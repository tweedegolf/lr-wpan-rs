use core::fmt::{Debug, Display};

use crate::{
    phy::{Phy, ReceivedMessage, SendContinuation, SendResult},
    pib::MacPib,
    sap::{scan::ScanType, RequestValue, Status},
    time::{DelayNsExt, Duration, Instant},
};

mod callback;
mod commander;
mod mlme_get;
mod mlme_reset;
mod mlme_scan;
mod mlme_set;
mod mlme_start;
mod state;

use commander::MacHandler;
pub use commander::{IndicationResponder, MacCommander};
use embassy_futures::select::{select, Either};
use futures::FutureExt;
use mlme_get::process_get_request;
use mlme_reset::process_reset_request;
use mlme_scan::{process_scan_request, ScanAction};
use mlme_set::process_set_request;
use mlme_start::process_start_request;
use rand_core::RngCore;
use state::{BeaconMode, MacState};

use crate::wire::{ExtendedAddress, Frame, FrameContent, PanId, ShortAddress};

const BEACON_PLANNING_HEADROOM: Duration = Duration::from_millis(20);

/// Run the MAC layer of the IEEE protocol.
///
/// This is an async function that should always be polled in the background.
/// The given [MacCommander] is the method of communicating with the MAC.
pub async fn run_mac_engine<'a, Rng: RngCore, Delay: DelayNsExt>(
    mut phy: impl Phy + 'a,
    commander: &'a MacCommander,
    mut config: MacConfig<Rng, Delay>,
) -> ! {
    let handler = commander.get_handler();
    let mut mac_pib = MacPib::dummy_new();
    let mut mac_state = MacState::new(&config);

    loop {
        let result = select(
            wait_for_radio_event(&mut phy, &mut mac_pib, &mut mac_state, &mut config.delay),
            handler.wait_for_request(),
        )
        .await;

        match result {
            Either::First(event) => {
                handle_radio_event(event, &mut phy, &mut mac_pib, &mut mac_state, &handler).await
            }
            Either::Second(responder) => {
                handle_request(
                    responder,
                    &mut phy,
                    &mut mac_pib,
                    &mut mac_state,
                    &mut config,
                )
                .await;
            }
        }
    }
}

async fn handle_request<'a, Rng: RngCore, Delay: DelayNsExt>(
    responder: commander::RequestResponder<'a, RequestValue>,
    phy: &mut (impl Phy + 'a),
    mac_pib: &mut MacPib,
    mac_state: &mut MacState<'a>,
    config: &mut MacConfig<Rng, Delay>,
) {
    match &responder.request {
        RequestValue::Associate(_) => todo!(),
        RequestValue::Disassociate(_) => todo!(),
        RequestValue::Get(_) => {
            process_get_request(phy, &*mac_pib, responder.into_concrete()).await
        }
        RequestValue::Gts(_) => todo!(),
        RequestValue::Reset(_) => {
            process_reset_request(phy, mac_pib, mac_state, config, responder.into_concrete()).await
        }
        RequestValue::RxEnable(_) => todo!(),
        RequestValue::Scan(_) => {
            process_scan_request(phy, mac_pib, mac_state, responder.into_concrete()).await
        }
        RequestValue::Set(_) => {
            process_set_request(phy, &mut mac_pib.pib_write, responder.into_concrete()).await
        }
        RequestValue::Start(_) => {
            process_start_request(phy, mac_pib, mac_state, responder.into_concrete()).await
        }
        RequestValue::Sync(_) => todo!(),
        RequestValue::Poll(_) => todo!(),
        RequestValue::Dps(_) => todo!(),
        RequestValue::Sounding(_) => todo!(),
        RequestValue::Calibrate(_) => todo!(),
        RequestValue::Data(_) => todo!(),
        RequestValue::Purge(_) => todo!(),
    }
}

/// Configuration for the MAC layer
#[derive(Debug, Clone)]
pub struct MacConfig<Rng: RngCore, Delay: DelayNsExt> {
    /// The unique EUI-64 address used by the mac layer
    pub extended_address: ExtendedAddress,
    pub rng: Rng,
    pub delay: Delay,
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt-03", derive(defmt::Format))]
pub enum MacError<PE> {
    PhyError(PE),
    UnsupportedAttribute,
    UnknownChannelPage(u8),
}

impl<PE: Debug> Display for MacError<PE> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl<PE> From<PE> for MacError<PE> {
    fn from(v: PE) -> Self {
        Self::PhyError(v)
    }
}

impl<PE> From<MacError<PE>> for Status {
    fn from(value: MacError<PE>) -> Self {
        match value {
            MacError::PhyError(_) => Status::PhyError,
            MacError::UnsupportedAttribute => Status::UnsupportedAttribute,
            MacError::UnknownChannelPage(_) => Status::InvalidParameter,
        }
    }
}

/// Wait for a radio event. The event must be processed by the [handle_radio_event] function.
/// The split is there because it allows this function to be cancellable.
async fn wait_for_radio_event<P: Phy>(
    phy: &mut P,
    mac_pib: &mut MacPib,
    mac_state: &mut MacState<'_>,
    delay: &mut impl DelayNsExt,
) -> RadioEvent<P> {
    let current_time = match phy.get_instant().await {
        Ok(current_time) => current_time,
        Err(e) => {
            error!("Could not get current time: {}", e);
            return RadioEvent::Error;
        }
    };
    let symbol_duration = phy.symbol_duration();
    let current_time_symbols = current_time / symbol_duration;

    // TODO: Figure out whether we should put the radio in RX
    // - For example when PAN coordinator, but with beaconorder 15
    // - For example when PIB says so

    let own_superframe_start = wait_for_own_superframe_start(
        mac_pib,
        mac_state,
        current_time,
        current_time_symbols,
        symbol_duration,
        delay.clone(),
    );

    let own_superframe_end = wait_for_own_super_frame_end(
        mac_state,
        mac_pib,
        current_time_symbols,
        delay.clone(),
        symbol_duration,
    );

    let scan_action = wait_for_channel_scan_action(mac_state, current_time, delay.clone());

    let phy_wait = phy.wait();

    futures::select_biased! {
        wait_result = phy_wait.fuse() => {
            match wait_result {
                Ok(context) => RadioEvent::PhyWaitDone { context },
                Err(e) => {
                    error!("Phy wait error: {}", e);
                    RadioEvent::Error
                }
            }
        },
        event = own_superframe_start.fuse() => {
            event
        }
        event = own_superframe_end.fuse() => {
            event
        }
        event = scan_action.fuse() => {
            event
        }
    }
}

async fn handle_radio_event<P: Phy>(
    event: RadioEvent<P>,
    phy: &mut P,
    mac_pib: &mut MacPib,
    mac_state: &mut MacState<'_>,
    mac_handler: &MacHandler<'_>,
) {
    match event {
        RadioEvent::Error => todo!(),
        RadioEvent::OwnSuperframeStart { start_time } => {
            own_superframe_start(mac_state, mac_pib, phy, start_time).await
        }
        RadioEvent::OwnSuperframeStartMissed { start_time } => {
            // Reset so hopefully the next time works out
            mac_pib.beacon_tx_time = start_time / phy.symbol_duration();
        }
        RadioEvent::OwnSuperframeEnd => {
            mac_state.own_superframe_active = false;

            if !mac_pib.rx_on_when_idle {
                if let Err(e) = phy.stop_receive().await {
                    error!(
                        "Could not stop the radio receiving at the end of the superframe: {}",
                        e
                    );
                }
            }
        }
        RadioEvent::PhyWaitDone { context } => match phy.process(context).await {
            Ok(Some(message)) => process_message(message, mac_state, mac_pib, mac_handler).await,
            Ok(None) => {}
            Err(e) => {
                error!("Phy process error: {}", e);
            }
        },
        RadioEvent::ScanAction(scan_action) => {
            perform_scan_action(scan_action, phy, mac_state, mac_pib).await
        }
    }
}

async fn perform_scan_action(
    scan_action: ScanAction,
    phy: &mut impl Phy,
    mac_state: &mut MacState<'_>,
    mac_pib: &mut MacPib,
) {
    use crate::wire;

    match scan_action {
        action @ ScanAction::StartScan {
            channel,
            page,
            scan_type,
            current_code: _,
        } => {
            // Update the radio so it uses the correct channel and page
            if let Err(e) = phy
                .update_phy_pib(|pib| {
                    pib.current_channel = channel;
                    pib.current_page = page;
                    // TODO: pib.current_code = current_code;
                })
                .await
            {
                error!("Could not update the pib for the scan: {}", e);
                mac_state
                    .current_scan_process
                    .take()
                    .unwrap()
                    .abort_scan(mac_pib, Status::PhyError, phy)
                    .await;
                return;
            }

            let mut scan_type = scan_type;
            loop {
                match scan_type {
                    ScanType::Ed => {
                        todo!("Pick up later since it requires more phy implementation")
                    }
                    ScanType::Active => {
                        let data = mac_state.serialize_frame(Frame {
                            header: wire::Header {
                                frame_type: wire::FrameType::MacCommand,
                                frame_pending: false,
                                ack_request: false,
                                pan_id_compress: false,
                                seq_no_suppress: false,
                                ie_present: false,
                                version: wire::FrameVersion::Ieee802154_2003,
                                seq: 0,
                                destination: Some(wire::Address::Short(
                                    PanId::broadcast(),
                                    ShortAddress::BROADCAST,
                                )),
                                source: None,
                                auxiliary_security_header: None,
                            },
                            content: wire::FrameContent::Command(
                                wire::command::Command::BeaconRequest,
                            ),
                            payload: &[],
                            footer: [0, 0],
                        });

                        match phy
                            .send(
                                &data,
                                None,
                                false,
                                true,
                                SendContinuation::ReceiveContinuous,
                            )
                            .await
                        {
                            Ok(SendResult::Success(_)) => {
                                // Cool, continue
                            }
                            Ok(SendResult::ChannelAccessFailure) => {
                                // We could not send the beacon request, so let the scan process know it failed
                                // and should continue with the next channel
                                mac_state
                                    .current_scan_process
                                    .as_mut()
                                    .unwrap()
                                    .register_action_as_failed(action, phy)
                                    .await;
                                return;
                            }
                            Err(e) => {
                                error!("Start listening for scan: {}", e);
                                mac_state
                                    .current_scan_process
                                    .take()
                                    .unwrap()
                                    .abort_scan(mac_pib, Status::PhyError, phy)
                                    .await;
                                return;
                            }
                        }

                        // Continue just like the passive scan
                        scan_type = ScanType::Passive;
                        continue;
                    }
                    ScanType::Passive => {
                        if let Err(e) = phy.start_receive().await {
                            error!("Start listening for scan: {}", e);
                            mac_state
                                .current_scan_process
                                .take()
                                .unwrap()
                                .abort_scan(mac_pib, Status::PhyError, phy)
                                .await;
                            return;
                        }
                        break;
                    }
                    ScanType::Orphan => {
                        todo!("Pick up later when we implement orphan messages")
                    }
                }
            }

            mac_state
                .current_scan_process
                .as_mut()
                .unwrap()
                .register_action_as_executed(action);
        }
        action @ ScanAction::Finish => {
            let mut scan_process = mac_state.current_scan_process.take().unwrap();
            scan_process.register_action_as_executed(action);
            scan_process.finish_scan(mac_pib, phy).await;
        }
    }
}

async fn own_superframe_start(
    mac_state: &mut MacState<'_>,
    mac_pib: &mut MacPib,
    phy: &mut impl Phy,
    start_time: Instant,
) {
    use crate::wire;

    let has_broadcast_scheduled = mac_state.message_scheduler.has_broadcast_scheduled();
    mac_state.own_superframe_active = true;
    let beacon_frame = wire::Frame {
        header: wire::Header {
            frame_type: wire::FrameType::Beacon,
            frame_pending: has_broadcast_scheduled,
            ack_request: false,
            pan_id_compress: false,
            seq_no_suppress: false,
            ie_present: false,
            version: mac_state.beacon_security_info.get_frame_version(),
            seq: mac_pib.bsn.increment(),
            destination: None,
            source: Some(if mac_pib.short_address == ShortAddress(0xFFFE) {
                wire::Address::Extended(mac_pib.pan_id, mac_pib.extended_address)
            } else {
                wire::Address::Short(mac_pib.pan_id, mac_pib.short_address)
            }),
            auxiliary_security_header: mac_state.beacon_security_info.into(),
        },
        content: wire::FrameContent::Beacon(wire::beacon::Beacon {
            superframe_spec: wire::beacon::SuperframeSpecification {
                beacon_order: mac_pib.beacon_order,
                superframe_order: mac_pib.superframe_order,
                final_cap_slot: (crate::consts::NUM_SUPERFRAME_SLOTS
                    - mac_state
                        .current_gts
                        .slots()
                        .iter()
                        .map(|slot| slot.length as u32)
                        .sum::<u32>()) as u8,
                battery_life_extension: mac_pib.batt_life_ext,
                pan_coordinator: mac_state.is_pan_coordinator,
                association_permit: mac_pib.association_permit,
            },
            guaranteed_time_slot_info: mac_state.current_gts.clone(),
            pending_address: mac_state.message_scheduler.get_pending_addresses(),
        }),
        payload: &mac_pib.beacon_payload[..mac_pib.beacon_payload_length],
        footer: Default::default(),
    };
    if let Err(e) = phy
        .send(
            &mac_state.serialize_frame(beacon_frame),
            Some(start_time),
            mac_pib.ranging_supported,
            false,
            if !has_broadcast_scheduled {
                SendContinuation::ReceiveContinuous // The superframe of the beacon continues
            } else {
                SendContinuation::Idle
            },
        )
        .await
    {
        error!("Could not send beacon: {}", e);
        // Let's just act as though we did send it
    }
    if let Some(broadcast) = mac_state.message_scheduler.take_scheduled_broadcast() {
        match phy
            .send(
                &broadcast.data,
                Some(start_time),
                mac_pib.ranging_supported,
                false,
                SendContinuation::ReceiveContinuous, // The superframe of the beacon continues
            )
            .await
        {
            Err(e) => {
                error!("Could not send broadcast: {}", e);
                broadcast
                    .callback
                    .run(
                        crate::phy::SendResult::ChannelAccessFailure,
                        phy,
                        mac_pib,
                        mac_state,
                    )
                    .await
            }
            Ok(send_result) => {
                broadcast
                    .callback
                    .run(send_result, phy, mac_pib, mac_state)
                    .await
            }
        }
    }
    mac_pib.beacon_tx_time = start_time / phy.symbol_duration();
}

enum RadioEvent<P: Phy> {
    Error,
    OwnSuperframeStart { start_time: Instant },
    OwnSuperframeStartMissed { start_time: Instant },
    OwnSuperframeEnd,
    PhyWaitDone { context: P::ProcessingContext },
    ScanAction(ScanAction),
}

async fn wait_for_own_superframe_start<P: Phy>(
    mac_pib: &MacPib,
    mac_state: &MacState<'_>,
    current_time: Instant,
    current_time_symbols: i64,
    symbol_duration: Duration,
    mut delay: impl DelayNsExt,
) -> RadioEvent<P> {
    // Calculate if we have a timeout and for how long
    let timeout = match (mac_pib.beacon_interval(), mac_state.beacon_mode) {
        (None, BeaconMode::Off) => None,
        (None, BeaconMode::OnAutonomous | BeaconMode::OnTracking { .. }) => {
            panic!("No beacon interval while the beacon mode is on")
        }
        (Some(_), BeaconMode::Off) => {
            panic!("Beacon interval is valid while the beacon mode is off")
        }
        (Some(bi), BeaconMode::OnAutonomous) => {
            let next_start_time_symbols = mac_pib.beacon_tx_time + bi.get() as i64;
            let timeout_symbols = next_start_time_symbols - current_time_symbols;
            Some(timeout_symbols * symbol_duration)
        }
        (Some(_), BeaconMode::OnTracking { .. }) => {
            // This beacon tracks another beacon, so will be done in response to a tracked beacon event
            None
        }
    };

    let scan_active = mac_state.current_scan_process.is_some();

    match (scan_active, timeout) {
        // When the scan is active we must not send out beacons
        (true, Some(timeout)) => {
            delay
                .delay_duration(timeout - BEACON_PLANNING_HEADROOM)
                .await;
            warn!("Beacon is missed due to active scan in progress");
            RadioEvent::OwnSuperframeStartMissed {
                start_time: current_time + timeout,
            }
        }
        (false, Some(timeout)) if timeout > BEACON_PLANNING_HEADROOM => {
            delay
                .delay_duration(timeout - BEACON_PLANNING_HEADROOM)
                .await;
            RadioEvent::OwnSuperframeStart {
                start_time: current_time + timeout,
            }
        }
        (false, Some(timeout)) if timeout > Duration::from_ticks(0) => {
            warn!(
                "Beacon timeout is within headroom: {} millis",
                timeout.millis()
            );
            RadioEvent::OwnSuperframeStart {
                start_time: current_time + timeout,
            }
        }
        (false, Some(timeout)) => {
            error!("Beacon is too late by {} millis", timeout.abs().millis());
            RadioEvent::OwnSuperframeStartMissed {
                start_time: current_time + timeout,
            }
        }
        (_, None) => core::future::pending().await,
    }
}

async fn wait_for_own_super_frame_end<P: Phy>(
    mac_state: &MacState<'_>,
    mac_pib: &MacPib,
    current_time_symbols: i64,
    mut delay: impl DelayNsExt,
    symbol_duration: Duration,
) -> RadioEvent<P> {
    match (
        mac_state.own_superframe_active,
        mac_pib.superframe_duration(),
    ) {
        (true, None) => unreachable!(),
        (true, Some(superframe_duration)) => {
            let superframe_end_time = mac_pib.beacon_tx_time + superframe_duration.get() as i64;
            let duration_to_go = superframe_end_time - current_time_symbols;
            delay.delay_duration(duration_to_go * symbol_duration).await;
            RadioEvent::OwnSuperframeEnd
        }
        (false, _) => core::future::pending().await,
    }
}

async fn wait_for_channel_scan_action<P: Phy>(
    mac_state: &MacState<'_>,
    current_time: Instant,
    delay: impl DelayNsExt,
) -> RadioEvent<P> {
    match &mac_state.current_scan_process {
        Some(scan_process) => {
            let action = scan_process.wait_for_next_action(current_time, delay).await;
            RadioEvent::ScanAction(action)
        }
        None => core::future::pending().await,
    }
}

async fn process_message(
    mut message: ReceivedMessage,
    mac_state: &mut MacState<'_>,
    mac_pib: &MacPib,
    mac_handler: &MacHandler<'_>,
) {
    info!("Received a message");

    let Some(frame) = mac_state.deserialize_frame(&mut message.data) else {
        return;
    };

    // Now decide what to do with the frame...

    // TODO: Filtering as in 5.1.6.2

    if let Some(scan_process) = mac_state.current_scan_process.as_mut() {
        if let FrameContent::Beacon(_) = frame.content {
            scan_process
                .register_received_beacon(
                    message.timestamp,
                    message.lqi,
                    message.channel,
                    message.page,
                    frame,
                    mac_pib,
                    mac_handler,
                )
                .await;
        }
    }
}
