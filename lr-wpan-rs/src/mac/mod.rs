use core::{
    fmt::{Debug, Display},
    pin::{pin, Pin},
};

use crate::{
    phy::{Phy, ReceivedMessage, SendContinuation, SendResult},
    pib::MacPib,
    sap::{
        associate::AssociateConfirm, scan::ScanType, RequestValue, ResponseValue, SecurityInfo,
        Status,
    },
    time::{DelayNsExt, Duration, Instant},
    wire::{command::Command, Address, FrameType},
    DeviceAddress,
};

mod callback;
mod commander;
mod mlme_associate;
mod mlme_get;
mod mlme_reset;
mod mlme_scan;
mod mlme_set;
mod mlme_start;
mod state;

pub use commander::{IndicationResponder, MacCommander};
use commander::{IndirectIndicationCollection, MacHandler};
use embassy_futures::select::{select3, Either, Either3};
use futures::FutureExt;
use mlme_associate::{process_associate_request, process_associate_response};
use mlme_get::process_get_request;
use mlme_reset::process_reset_request;
use mlme_scan::{process_scan_request, ScanAction};
use mlme_set::process_set_request;
use mlme_start::process_start_request;
use rand_core::RngCore;
use state::{BeaconMode, DataRequestMode, MacState, PendingDataValue, ScheduledDataRequest};

use crate::wire::{ExtendedAddress, Frame, FrameContent, PanId, ShortAddress};

const BEACON_PLANNING_HEADROOM: Duration = Duration::from_millis(20);
const DATA_REQUEST_PLANNING_HEADROOM: Duration = Duration::from_millis(20);

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
    let mut indirect_indications = core::pin::pin!(IndirectIndicationCollection::new());

    loop {
        let current_time = match phy.get_instant().await {
            Ok(current_time) => current_time,
            Err(e) => {
                error!("Could not get the current time: {}", e);
                continue;
            }
        };

        let result = select3(
            wait_for_radio_event(&mut phy, &mac_pib, &mac_state, &config.delay),
            indirect_indications.as_mut().wait(current_time),
            handler.wait_for_request(),
        )
        .await;

        match result {
            Either3::First(event) => {
                handle_radio_event(
                    event,
                    &mut phy,
                    &mut mac_pib,
                    &mut mac_state,
                    &handler,
                    indirect_indications.as_mut(),
                    &mut config.delay,
                )
                .await
            }
            Either3::Second(indication_response_value) => {
                handle_response(indication_response_value, &mut phy, &mut mac_state).await
            }
            Either3::Third(responder) => {
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
        RequestValue::Associate(_) => {
            process_associate_request(phy, mac_pib, mac_state, responder.into_concrete()).await
        }
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

async fn handle_response(
    indication_response_value: ResponseValue,
    phy: &mut impl Phy,
    mac_state: &mut MacState<'_>,
) {
    let current_time = match phy.get_instant().await {
        Ok(current_time) => current_time,
        Err(e) => {
            error!("Could not get the current time, so we can't process the indication_response_value: {}", e);
            return;
        }
    };

    match indication_response_value {
        crate::sap::ResponseValue::Associate(associate_response) => {
            process_associate_response(associate_response, current_time, mac_state).await
        }
        crate::sap::ResponseValue::Orphan(_orphan_response) => todo!(),
        crate::sap::ResponseValue::None => todo!(),
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
    mac_pib: &MacPib,
    mac_state: &MacState<'_>,
    delay: &impl DelayNsExt,
) -> RadioEvent<P> {
    let current_time = match phy.get_instant().await {
        Ok(current_time) => current_time,
        Err(e) => {
            error!("Could not get current time: {}", e);
            return RadioEvent::Error;
        }
    };
    let symbol_period = phy.symbol_period();
    let current_time_symbols = current_time / symbol_period;

    // TODO: Figure out when exactly we should put the radio in RX
    // - For example when PAN coordinator
    // - For example when PIB says so
    if mac_state.is_pan_coordinator || mac_pib.rx_on_when_idle {
        if let Err(e) = phy.start_receive().await {
            error!("Could not start receiving: {}", e);
            return RadioEvent::Error;
        }
    }

    let own_superframe_start = wait_for_own_superframe_start(
        mac_pib,
        mac_state,
        current_time,
        current_time_symbols,
        symbol_period,
        delay.clone(),
    );

    let own_superframe_end = wait_for_own_super_frame_end(
        mac_state,
        mac_pib,
        current_time_symbols,
        delay.clone(),
        symbol_period,
    );

    let scan_action = wait_for_channel_scan_action(mac_state, current_time, delay.clone());

    let independent_data_request =
        wait_for_independent_data_request(mac_state, current_time, delay.clone());

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
        event = independent_data_request.fuse() => {
            event
        }
    }
}

async fn handle_radio_event<'a, P: Phy>(
    event: RadioEvent<P>,
    phy: &mut P,
    mac_pib: &mut MacPib,
    mac_state: &mut MacState<'a>,
    mac_handler: &MacHandler<'a>,
    mut indirect_indications: Pin<&mut IndirectIndicationCollection<'a>>,
    delay: &mut impl DelayNsExt,
) {
    let mut next_events = arraydeque::ArrayDeque::<_, 4>::new();
    next_events.push_back(event).unwrap();

    while let Some(event) = next_events.pop_front() {
        match event {
            RadioEvent::Error => todo!(),
            RadioEvent::BeaconRequested => send_beacon(mac_state, mac_pib, phy, None, true).await,
            RadioEvent::OwnSuperframeStart { start_time } => {
                send_beacon(mac_state, mac_pib, phy, Some(start_time), false).await
            }
            RadioEvent::OwnSuperframeStartMissed { start_time } => {
                // Reset so hopefully the next time works out
                mac_pib.beacon_tx_time = start_time / phy.symbol_period();
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
                Ok(Some(message)) => {
                    process_message::<P>(
                        message,
                        mac_state,
                        mac_pib,
                        mac_handler,
                        indirect_indications.as_mut(),
                        phy.symbol_period(),
                        &mut next_events,
                    )
                    .await;
                }
                Ok(None) => {}
                Err(e) => {
                    error!("Phy process error: {}", e);
                }
            },
            RadioEvent::ScanAction(scan_action) => {
                debug!("Performing scan action");
                perform_scan_action(scan_action, phy, mac_state, mac_pib).await
            }
            RadioEvent::SendScheduledIndependentDataRequest => {
                debug!("Sending data request");
                perform_data_request(
                    mac_state
                        .message_scheduler
                        .take_scheduled_independent_data_request()
                        .unwrap(),
                    phy,
                    mac_state,
                    mac_pib,
                    delay,
                )
                .await
            }
            RadioEvent::SendAck {
                receive_time,
                seq,
                frame_pending,
            } => {
                debug!("Sending ack");
                send_ack(phy, mac_pib, mac_state, receive_time, seq, frame_pending).await
            }
            RadioEvent::SendPendingData {
                request_receive_time,
                device_address,
            } => {
                debug!("Sending pending data");
                send_pending_data(
                    phy,
                    mac_pib,
                    mac_state,
                    request_receive_time,
                    device_address,
                )
                .await
            }
        }
    }
}

async fn send_pending_data(
    phy: &mut impl Phy,
    mac_pib: &mut MacPib,
    mac_state: &mut MacState<'_>,
    #[expect(unused, reason = "TODO to use")] request_receive_time: Instant,
    device_address: DeviceAddress,
) {
    use crate::wire;

    let data = mac_state
        .message_scheduler
        .take_pending_data(device_address);
    let has_more_data = mac_state.message_scheduler.has_pending_data(device_address);

    let dsn = mac_pib.dsn.increment();

    let frame = match data.as_ref().map(|pd| &pd.data_value) {
        Some(PendingDataValue::AssociationResponse {
            short_address,
            association_status,
        }) => Frame {
            header: wire::Header {
                frame_type: wire::FrameType::MacCommand,
                frame_pending: has_more_data,
                ack_request: true,
                pan_id_compress: true,
                seq_no_suppress: false,
                ie_present: false,
                version: wire::FrameVersion::Ieee802154_2003,
                seq: dsn,
                destination: Some(device_address.with_pan(mac_pib.pan_id)),
                source: Some(wire::Address::Extended(
                    mac_pib.pan_id,
                    mac_pib.extended_address,
                )),
                auxiliary_security_header: None,
            },
            content: wire::FrameContent::Command(Command::AssociationResponse(
                *short_address,
                *association_status,
            )),
            payload: &[],
            footer: [0, 0],
        },
        // If no pending data, send an empty data response
        None => Frame {
            header: wire::Header {
                frame_type: wire::FrameType::Data,
                frame_pending: has_more_data,
                ack_request: false,
                pan_id_compress: true,
                seq_no_suppress: false,
                ie_present: false,
                version: wire::FrameVersion::Ieee802154_2003,
                seq: dsn,
                destination: Some(device_address.with_pan(mac_pib.pan_id)),
                source: Some(wire::Address::Extended(
                    mac_pib.pan_id,
                    mac_pib.extended_address,
                )),
                auxiliary_security_header: None,
            },
            content: wire::FrameContent::Data,
            payload: &[],
            footer: [0, 0],
        },
    };

    let ack_required = frame.header.ack_request;
    let message = mac_state.serialize_frame(frame);

    let ack_wait_duration = mac_pib.ack_wait_duration(phy.get_phy_pib()) as i64;

    // TODO: This can be sent without CSMA too if we're in a superframe and there's time remaining, and then only on a backoff period boundary: 5.1.6.3
    // That should probably be done if we're in a superframe since it's nice and efficient
    let ack = match phy
        .send(
            &message,
            None,
            false,
            true,
            if ack_required {
                SendContinuation::WaitForResponse {
                    turnaround_time: phy.symbol_period() * crate::consts::TURNAROUND_TIME as i64,
                    timeout: phy.symbol_period() * ack_wait_duration,
                }
            } else {
                SendContinuation::Idle
            },
        )
        .await
    {
        Ok(SendResult::Success(_, None)) => None,
        Ok(SendResult::Success(_, Some(mut response))) => {
            // See if what we received was an Ack for us
            match mac_state.deserialize_frame(&mut response.data) {
                Some(frame) => {
                    if matches!(frame.header.frame_type, FrameType::Acknowledgement)
                        && frame.header.seq == dsn
                    {
                        Some((response.timestamp, frame.header.frame_pending))
                    } else {
                        None
                    }
                }
                None => None,
            }
        }
        Ok(SendResult::ChannelAccessFailure) => {
            warn!("CSMA failed for sending request data response");
            if let Some(data) = data {
                // We could not send, so push back onto the queue
                mac_state.message_scheduler.push_pending_data(data).unwrap();
            }
            // TODO: We probably need to do something here
            return;
        }
        Err(e) => {
            error!("Could not send an ack: {}", e);
            // TODO: Not sure how we can recover
            return;
        }
    };

    if ack_required && ack.is_none() {
        todo!("No ack received. No retry implemented yet");
    }
}

async fn send_ack(
    phy: &mut impl Phy,
    mac_pib: &mut MacPib,
    mac_state: &mut MacState<'_>,
    receive_time: Instant,
    seq: u8,
    frame_pending: bool,
) {
    use crate::wire;

    let data = mac_state.serialize_frame(Frame {
        header: wire::Header {
            frame_type: wire::FrameType::Acknowledgement,
            frame_pending,
            ack_request: false,
            pan_id_compress: false,
            seq_no_suppress: false,
            ie_present: false,
            version: wire::FrameVersion::Ieee802154_2003,
            seq,
            destination: None,
            source: None,
            auxiliary_security_header: None,
        },
        content: wire::FrameContent::Acknowledgement,
        payload: &[],
        footer: [0, 0],
    });

    // TODO: Actually schedule this according to the rules (5.1.6.4.2)
    let ack_send_time = receive_time + phy.symbol_period() * mac_pib.sifs_period as i64;

    match phy
        .send(
            &data,
            Some(ack_send_time),
            false,
            false,
            SendContinuation::Idle,
        )
        .await
    {
        Ok(SendResult::Success(_, _)) => {
            // Cool, continue
        }
        Ok(SendResult::ChannelAccessFailure) => {
            unreachable!();
        }
        Err(e) => {
            error!("Could not send an ack: {}", e);
        }
    }
}

// 5.1.6.3
async fn perform_data_request(
    data_request: ScheduledDataRequest<'_>,
    phy: &mut impl Phy,
    mac_state: &mut MacState<'_>,
    mac_pib: &mut MacPib,
    delay: &mut impl DelayNsExt,
) {
    let send_time = match data_request.mode {
        DataRequestMode::InSuperFrame => todo!(),
        DataRequestMode::Independent { timestamp } => timestamp,
    };

    let (destination_address, source_address) = match data_request.trigger {
        state::DataRequestTrigger::BeaconPendingDataIndication => todo!(),
        state::DataRequestTrigger::MlmePoll => todo!(),
        state::DataRequestTrigger::Association => {
            let destination = if mac_pib.coord_short_address.0 == 0xFFFE {
                Address::Extended(mac_pib.pan_id, mac_pib.coord_extended_address)
            } else {
                Address::Short(mac_pib.pan_id, mac_pib.coord_short_address)
            };

            let source = Address::Extended(mac_pib.pan_id, mac_pib.extended_address);

            (Some(destination), source)
        }
    };

    let dsn = mac_pib.dsn.increment();
    let data_request_frame = Frame {
        header: crate::wire::Header {
            frame_type: crate::wire::FrameType::MacCommand,
            frame_pending: false,
            ack_request: true,
            pan_id_compress: destination_address.is_none(),
            seq_no_suppress: false,
            ie_present: false,
            version: crate::wire::FrameVersion::Ieee802154_2003,
            seq: dsn,
            destination: destination_address,
            source: Some(source_address),
            auxiliary_security_header: None,
        },
        content: FrameContent::Command(Command::DataRequest),
        payload: &[],
        footer: [0; 2],
    };

    let message = mac_state.serialize_frame(data_request_frame);

    let ack_wait_duration = mac_pib.ack_wait_duration(phy.get_phy_pib()) as i64;

    let send_result = phy
        .send(
            &message,
            send_time,
            false,
            true, // TODO: Unless in superframe
            SendContinuation::WaitForResponse {
                turnaround_time: phy.symbol_period() * crate::consts::TURNAROUND_TIME as i64,
                timeout: phy.symbol_period() * ack_wait_duration,
            },
        )
        .await;

    let ack = match send_result {
        Ok(SendResult::Success(_, None)) => None,
        Ok(SendResult::Success(_, Some(mut response))) => {
            // See if what we received was an Ack for us
            match mac_state.deserialize_frame(&mut response.data) {
                Some(frame) => {
                    if matches!(frame.header.frame_type, FrameType::Acknowledgement)
                        && frame.header.seq == dsn
                    {
                        Some((response.timestamp, frame.header.frame_pending))
                    } else {
                        None
                    }
                }
                None => None,
            }
        }
        Ok(SendResult::ChannelAccessFailure) => {
            warn!("Could not send the data request: ChannelAccessFailure");
            data_request
                .callback
                .run_associate(Err(Err(Status::ChannelAccessFailure)), mac_pib)
                .await;
            return;
        }
        Err(e) => {
            error!("Could not send the data request: {}", e);
            data_request
                .callback
                .run_associate(Err(Err(Status::PhyError)), mac_pib)
                .await;
            return;
        }
    };

    let Some((_ack_timestamp, frame_pending)) = ack else {
        todo!("No ack received for data request. Retransmission: TODO");
    };

    if !frame_pending {
        trace!("No data available at the coordinator");
        data_request
            .callback
            .run_associate(Err(Err(Status::NoData)), mac_pib)
            .await;
        return;
    }

    // TODO: Refactor listening to common function

    // Turn on receiver for macMaxFrameTotalWaitTime to receive the association response
    let on_duration =
        phy.symbol_period() * mac_pib.max_frame_total_wait_time(phy.get_phy_pib()).into();
    let mut on_delay = pin!(delay.delay_duration(on_duration));

    if let Err(e) = phy.start_receive().await {
        error!(
            "Could not turn on phy for receiving association response: {}",
            e
        );
        data_request
            .callback
            .run_associate(Err(Err(Status::PhyError)), mac_pib)
            .await;
        return;
    }

    let response = loop {
        match embassy_futures::select::select(phy.wait(), &mut on_delay).await {
            Either::First(Ok(processing_context)) => match phy.process(processing_context).await {
                Ok(Some(mut received_message)) => {
                    let Some(frame) = mac_state.deserialize_frame(&mut received_message.data)
                    else {
                        trace!("Received a frame that can't be deserialized");
                        continue;
                    };

                    trace!("Received a frame in the data request routine: {:?}", frame);

                    if !filter_frame(&frame) {
                        // Frame not for us
                        continue;
                    }

                    let FrameContent::Command(Command::AssociationResponse(
                        assoc_short_address,
                        association_status,
                    )) = frame.content
                    else {
                        warn!("Received something other than the expected AssociationResponse");
                        continue;
                    };

                    if frame.header.ack_request {
                        send_ack(
                            phy,
                            mac_pib,
                            mac_state,
                            received_message.timestamp,
                            frame.header.seq,
                            false,
                        )
                        .await;
                    }

                    break Ok(AssociateConfirm {
                        assoc_short_address,
                        status: Ok(association_status),
                        security_info: SecurityInfo::new_none_security(),
                    });
                }
                Ok(None) => {
                    continue;
                }
                Err(e) => {
                    error!("Could not process phy: {}", e);
                    break Err(Err(Status::PhyError));
                }
            },
            Either::First(Err(e)) => {
                error!("Could not wait on phy: {}", e);
                break Err(Err(Status::PhyError));
            }
            Either::Second(()) => {
                // Timeout
                break Err(Err(Status::NoData));
            }
        }
    };

    if let Err(e) = phy.stop_receive().await {
        error!(
            "Could not turn off phy for receiving association response: {}",
            e
        );
        data_request
            .callback
            .run_associate(Err(Err(Status::PhyError)), mac_pib)
            .await;
        return;
    }

    data_request.callback.run_associate(response, mac_pib).await;
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
            debug!(
                "Scanning channel '{}' of page '{:?}' with type '{:?}'",
                channel, page, scan_type
            );
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

                        trace!("Sending beacon request");
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
                            Ok(SendResult::Success(_, _)) => {
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

async fn send_beacon(
    mac_state: &mut MacState<'_>,
    mac_pib: &mut MacPib,
    phy: &mut impl Phy,
    send_time: Option<Instant>,
    use_beacon_csma: bool,
) {
    use crate::wire;

    let has_broadcast_scheduled = mac_state.message_scheduler.has_broadcast_scheduled();
    mac_state.own_superframe_active = !mac_pib.superframe_order.is_inactive();

    if mac_state.own_superframe_active {
        trace!("Starting a new superframe");
    } else {
        trace!("Sending a beacon")
    }

    let beacon_send_continuation = if mac_state.own_superframe_active || mac_pib.rx_on_when_idle {
        SendContinuation::ReceiveContinuous
    } else {
        SendContinuation::Idle
    };

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

    let send_time = match phy
        .send(
            &mac_state.serialize_frame(beacon_frame),
            send_time,
            mac_pib.ranging_supported,
            use_beacon_csma,
            if !has_broadcast_scheduled {
                beacon_send_continuation
            } else {
                SendContinuation::Idle
            },
        )
        .await
    {
        Ok(SendResult::Success(send_time, _)) => send_time,
        Ok(SendResult::ChannelAccessFailure) => {
            warn!("Could not send beacon due to channel access failure");
            return;
        }
        Err(e) => {
            error!("Could not send beacon: {}", e);
            return;
        }
    };

    if let Some(broadcast) = mac_state.message_scheduler.take_scheduled_broadcast() {
        match phy
            .send(
                &broadcast.data,
                Some(send_time),
                mac_pib.ranging_supported,
                false,
                beacon_send_continuation,
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

    mac_pib.beacon_tx_time = send_time / phy.symbol_period();
}

enum RadioEvent<P: Phy> {
    Error,
    BeaconRequested,
    OwnSuperframeStart {
        start_time: Instant,
    },
    OwnSuperframeStartMissed {
        start_time: Instant,
    },
    OwnSuperframeEnd,
    PhyWaitDone {
        context: P::ProcessingContext,
    },
    ScanAction(ScanAction),
    SendScheduledIndependentDataRequest,
    SendAck {
        /// The time the message we're acking was received
        receive_time: Instant,
        /// The sequence number of the received message
        seq: u8,
        /// True if the frame pending bit should be set
        frame_pending: bool,
    },
    SendPendingData {
        /// The time at which we received the data request
        request_receive_time: Instant,
        /// The address of the requester
        device_address: DeviceAddress,
    },
}

async fn wait_for_own_superframe_start<P: Phy>(
    mac_pib: &MacPib,
    mac_state: &MacState<'_>,
    current_time: Instant,
    current_time_symbols: i64,
    symbol_period: Duration,
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
            Some(timeout_symbols * symbol_period)
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
    symbol_period: Duration,
) -> RadioEvent<P> {
    match (
        mac_state.own_superframe_active,
        mac_pib.superframe_duration(),
    ) {
        (true, None) => unreachable!(),
        (true, Some(superframe_duration)) => {
            let superframe_end_time = mac_pib.beacon_tx_time + superframe_duration.get() as i64;
            let duration_to_go = superframe_end_time - current_time_symbols;
            delay.delay_duration(duration_to_go * symbol_period).await;
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

async fn wait_for_independent_data_request<P: Phy>(
    mac_state: &MacState<'_>,
    current_time: Instant,
    mut delay: impl DelayNsExt,
) -> RadioEvent<P> {
    match mac_state
        .message_scheduler
        .get_scheduled_independent_data_request()
    {
        Some(ScheduledDataRequest {
            mode:
                DataRequestMode::Independent {
                    timestamp: Some(send_time),
                },
            ..
        }) => {
            delay
                .delay_duration(
                    send_time.duration_since(current_time) - DATA_REQUEST_PLANNING_HEADROOM,
                )
                .await;
            RadioEvent::SendScheduledIndependentDataRequest
        }
        Some(ScheduledDataRequest {
            mode: DataRequestMode::Independent { timestamp: None },
            ..
        }) => RadioEvent::SendScheduledIndependentDataRequest,
        Some(_) => todo!(),
        None => core::future::pending().await,
    }
}

async fn process_message<'a, P: Phy>(
    mut message: ReceivedMessage,
    mac_state: &mut MacState<'a>,
    mac_pib: &MacPib,
    mac_handler: &MacHandler<'a>,
    indirect_indications: Pin<&mut IndirectIndicationCollection<'a>>,
    symbol_period: Duration,
    next_events: &mut arraydeque::ArrayDeque<RadioEvent<P>, 4>,
) {
    let Some(frame) = mac_state.deserialize_frame(&mut message.data) else {
        trace!("Received a frame that could not be deserialized");
        return;
    };

    trace!("Received a frame: {:?}", frame);

    // Now decide what to do with the frame...

    if !filter_frame(&frame) {
        // Frame not for us
        return;
    }

    if mac_state.current_scan_process.is_some() {
        // During a scan, all non-beacon frames are rejected
        if !matches!(frame.content, FrameContent::Beacon(_)) {
            trace!("Ignoring a beacon");
            return;
        }
    }

    if matches!(frame.content, FrameContent::Command(Command::BeaconRequest)) {
        if mac_state.is_pan_coordinator && mac_pib.beacon_order.is_on_demand() {
            debug!("Got a beacon request to respond to");
            next_events.push_back(RadioEvent::BeaconRequested).unwrap();
            return;
        } else {
            trace!("Ignoring a beacon request");
            return;
        }
    }

    if let Some(scan_process) = mac_state.current_scan_process.as_mut() {
        debug!("Received a beacon for the scan");

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

        return;
    }

    let frame_pending = match frame.content {
        FrameContent::Command(Command::AssociationRequest(capability_information)) => {
            match frame.header.source {
                Some(Address::Extended(_, device_address)) => {
                    mlme_associate::process_received_associate_request(
                        mac_handler,
                        mac_pib,
                        indirect_indications,
                        device_address,
                        capability_information,
                        message.timestamp,
                        symbol_period,
                    )
                    .await
                }
                _ => warn!(
                    "Association request came from frame without correct source field. Ignored"
                ),
            }

            false
        }
        FrameContent::Command(Command::DataRequest) => {
            if let Some(source) = frame.header.source {
                next_events
                    .push_back(RadioEvent::SendPendingData {
                        request_receive_time: message.timestamp,
                        device_address: source.into(),
                    })
                    .unwrap();

                mac_state.message_scheduler.has_pending_data(source.into())
            } else {
                warn!("Got a datarequest without source address. Ignored");
                false
            }
        }
        content => {
            warn!(
                "Received frame has content we don't yet process: {}",
                content
            );
            false
        }
    };

    // Filtering has been done, so we know this is meant for us.
    // If it needs to be acked, we should do it now.
    // TODO: Look at the exact rules, because this is currently likely not correct
    if frame.header.ack_request {
        // Push to the front because acks need to processed first
        next_events
            .push_front(RadioEvent::SendAck {
                receive_time: message.timestamp,
                seq: frame.header.seq,
                frame_pending,
            })
            .unwrap();
    }
}

/// Filtering as in 5.1.6.2
///
/// If the frame should be processed, this function returns true.
/// If the frame can be discarded, this function returns false.
fn filter_frame(_frame: &Frame<'_>) -> bool {
    // TODO: Actually implement
    true
}
