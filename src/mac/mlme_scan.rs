use ieee802154::mac::{Frame, FrameContent, PanId};

use crate::{
    consts::BASE_SUPERFRAME_DURATION,
    phy::Phy,
    pib::MacPib,
    sap::{
        beacon_notify::BeaconNotifyIndication,
        scan::{ScanConfirm, ScanRequest, ScanType},
        PanDescriptor, SecurityInfo, Status,
    },
    time::{DelayNsExt, Duration, Instant},
    ChannelPage,
};

use super::{commander::RequestResponder, state::MacState, MacHandler};

pub async fn process_scan_request<'a>(
    phy: &mut impl Phy,
    mac_pib: &mut MacPib,
    mac_state: &mut MacState<'a>,
    responder: RequestResponder<'a, ScanRequest>,
) {
    let request = responder.request.clone();

    let current_time = match phy.get_instant().await {
        Ok(time) => time,
        Err(e) => {
            error!("Could not read the current time: {}", e);
            responder.respond(ScanConfirm {
                status: Status::PhyError,
                scan_type: request.scan_type,
                channel_page: request.channel_page,
                ..Default::default()
            });
            return;
        }
    };

    // Only one scan can be in progress at a time
    if mac_state.current_scan_process.is_some() {
        responder.respond(ScanConfirm {
            status: Status::ScanInProgress,
            scan_type: request.scan_type,
            channel_page: request.channel_page,
            ..Default::default()
        });
        return;
    }

    // Create the process. Making this `Some` will mark the scan as 'in process' for the rest of the system
    mac_state.current_scan_process = Some(ScanProcess {
        responder,
        symbol_duration: phy.symbol_duration(),
        end_time: current_time, // This waits 0 time before the first scan begins
        results: ScanConfirm {
            status: Status::Success,
            scan_type: request.scan_type,
            channel_page: request.channel_page,
            unscanned_channels: request.scan_channels,
            ..Default::default()
        },
        original_mac_pan_id: mac_pib.pan_id,
        skipped_channels: 0,
        beacons_found: false,
    });

    if let ScanType::Passive | ScanType::Active = request.scan_type {
        mac_pib.pan_id = PanId::broadcast()
    }
}

/// A structure that manages the scan process.
///
/// Steps:
/// - action = wait_for_next_action
/// - register_action_as_executed(action)
/// - if action is finish -> additionally call finish_scan
///
/// Meanwhile:
/// - Beacon received -> register_received_beacon
pub struct ScanProcess<'a> {
    /// Responder to the request we got. Eventually this must be answered.
    responder: RequestResponder<'a, ScanRequest>,
    /// The symbol duration of the phy. This is cached so we don't need to pass the phy around
    symbol_duration: Duration,
    /// The end time of the *current*  channel scan
    end_time: Instant,
    /// Work in progress result that we'll send back to the user
    results: ScanConfirm,
    /// Cache of the pan id we need to restore at the end
    original_mac_pan_id: PanId,
    /// The amount of channels that were skipped for some reason.
    /// This can be used to index into the unscanned channels to get the next channel to scan.
    skipped_channels: usize,
    /// True if some beacon was found at some point
    beacons_found: bool,
}

impl ScanProcess<'_> {
    /// Wait for the next action. This function may be cancelled.
    pub async fn wait_for_next_action(
        &self,
        current_time: Instant,
        mut delay: impl DelayNsExt,
    ) -> ScanAction {
        trace!(
            "Waiting for scan: {}",
            self.end_time.duration_since(current_time)
        );

        delay
            .delay_duration(self.end_time.duration_since(current_time))
            .await;

        if let Some(channel) = self.results.unscanned_channels.get(self.skipped_channels) {
            ScanAction::StartScan {
                channel: *channel,
                page: self.results.channel_page,
                scan_type: self.results.scan_type,
                current_code: (),
            }
        } else {
            ScanAction::Finish
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn register_received_beacon(
        &mut self,
        receive_time: Instant,
        lqi: u8,
        channel: u8,
        page: ChannelPage,
        frame: Frame<'_>,
        mac_pib: &MacPib,
        mac_handler: &MacHandler<'_>,
    ) {
        let FrameContent::Beacon(beacon_data) = frame.content else {
            panic!("Must be a beacon!");
        };

        self.beacons_found = true;

        let beacon_source = frame
            .header
            .source
            .expect("Beacons always have a source address");

        let pan_descriptor = PanDescriptor {
            coord_address: beacon_source,
            channel_number: channel,
            channel_page: page,
            super_frame_spec: beacon_data.superframe_spec,
            gts_permit: beacon_data.guaranteed_time_slot_info.permit,
            link_quality: lqi,
            timestamp: receive_time,
            security_status: None, // TODO: What's the expected behaviour here?
            security_info: frame
                .header
                .auxiliary_security_header
                .map(|h| SecurityInfo {
                    security_level: h.control.security_level(),
                    key_id_mode: h.control.key_id_mode(),
                    key_identifier: h.key_identifier,
                })
                .unwrap_or_default(),
            code_list: (),
        };

        // We either need to store the beacon in the results or send out a beacon notification
        if mac_pib.auto_request {
            // Store them

            // Ignore duplicates (5.1.2.1.2)
            let duplicate = self.results.pan_descriptor_list.iter().any(|descr| {
                descr.coord_address == beacon_source && descr.channel_number == channel
            });

            if duplicate {
                return;
            }

            // Push the descriptor
            self.results.pan_descriptor_list.push(pan_descriptor);
            self.results.result_list_size += 1;

            // End the scan if full
            if self.results.pan_descriptor_list.is_full() {
                // Next wait for action will return the Finish action
                self.skipped_channels = self.results.unscanned_channels.len();
                self.end_time = Instant::from_ticks(0);
                self.results.status = Status::LimitReached;
            }
        } else {
            // Output a beacon notification as per the spec
            mac_handler
                .indicate(BeaconNotifyIndication {
                    beacon_sequence_number: frame.header.seq,
                    pan_descriptor,
                    address_list: beacon_data.pending_address,
                    sdu: frame
                        .payload
                        .try_into()
                        .expect("Payload is never bigger than SDU"),
                })
                .await;
        }
    }

    pub fn register_action_as_executed(&mut self, action: ScanAction) {
        let scan_duration = self.symbol_duration
            * (BASE_SUPERFRAME_DURATION
                * ((1 << self.responder.request.scan_duration.min(14) as u32) + 1))
                as i64;
        self.end_time += scan_duration;

        match action {
            ScanAction::StartScan { .. } => {
                self.results
                    .unscanned_channels
                    .remove(self.skipped_channels);
            }
            ScanAction::Finish => {
                info!("Scan has been finished!")
            }
        }
    }

    pub async fn register_action_as_failed(&mut self, action: ScanAction, phy: &mut impl Phy) {
        let current_time = phy.get_instant().await.ok();

        match action {
            ScanAction::StartScan { .. } => {
                self.skipped_channels += 1;
                if let Some(current_time) = current_time {
                    // We skip the current channel, so we can continue with the next one
                    self.end_time = current_time;
                }
            }
            ScanAction::Finish => panic!("Cannot fail a finish"),
        }
    }

    pub async fn abort_scan(mut self, mac_pib: &mut MacPib, status: Status, phy: &mut impl Phy) {
        mac_pib.pan_id = self.original_mac_pan_id;
        self.results.status = status;

        // We set the status to NoBeacon if:
        // - there's no other error yet
        // - we have scanned at least one channel (unscanned_channels is initialized with the scan_channels of the request)
        // - we've not seen a beacon yet
        if self.results.status == Status::Success
            && self.results.unscanned_channels != self.responder.request.scan_channels
            && !self.beacons_found
        {
            self.results.status = Status::NoBeacon;
        }

        self.responder.respond(self.results);

        if let Err(e) = phy.stop_receive().await {
            error!("Could not stop receiving after scan: {}", e);
        }
    }

    pub async fn finish_scan(self, mac_pib: &mut MacPib, phy: &mut impl Phy) {
        self.abort_scan(mac_pib, Status::Success, phy).await
    }
}

pub enum ScanAction {
    StartScan {
        channel: u8,
        page: ChannelPage,
        scan_type: ScanType,
        /// TODO: According to the spec we should also go through all of the `phyCurrentCode` options
        /// for UWB and CSS. But this has not been implemented in the radio driver yet and we don't *really*
        /// need it. So ignore for now.
        #[expect(dead_code, reason = "Not yet implemented")]
        current_code: (),
    },
    Finish,
}
