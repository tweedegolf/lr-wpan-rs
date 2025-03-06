use super::{
    commander::RequestResponder,
    state::{BeaconMode, MacState},
    MacError,
};
use crate::{
    consts,
    mac::callback::SendCallback,
    phy::{Phy, SendResult},
    pib::MacPib,
    sap::{
        start::{StartConfirm, StartRequest},
        Status,
    },
    wire::{
        beacon::{BeaconOrder, SuperframeOrder},
        ShortAddress,
    },
};

pub async fn process_start_request<'a>(
    phy: &mut impl Phy,
    mac_pib: &mut MacPib,
    mac_state: &mut MacState<'a>,
    mut responder: RequestResponder<'a, StartRequest>,
) {
    assert!(
        u8::from(responder.request.superframe_order) <= u8::from(responder.request.beacon_order)
            || responder.request.superframe_order == SuperframeOrder::Inactive,
        "SuperframeOrder out of range"
    );

    // Start time must be rounded to the backoff period
    responder.request.start_time = (responder.request.start_time + consts::UNIT_BACKOFF_PERIOD / 2)
        / consts::UNIT_BACKOFF_PERIOD
        * consts::UNIT_BACKOFF_PERIOD;

    // Reject if the short address hasn't been set yet, according to the spec
    if mac_pib.short_address == ShortAddress::BROADCAST {
        responder.respond(StartConfirm {
            status: Status::NoShortAddress,
        });
        return;
    }

    if responder.request.coord_realignment {
        use crate::wire::{
            command::{Command, CoordinatorRealignmentData},
            Address, Frame, FrameContent, FrameType, FrameVersion, Header, PanId,
        };
        // We need to send a realignment message and only after that change apply the changes.
        // This happens in the callback
        let coord_realignment_message = Frame {
            header: Header {
                ie_present: false,
                seq_no_suppress: false,
                frame_type: FrameType::MacCommand,
                frame_pending: false,
                ack_request: false,
                pan_id_compress: false,
                version: FrameVersion::Ieee802154_2006, // Realignment command with channel page present

                seq: mac_pib.dsn.increment(),
                destination: Some(Address::Short(PanId::broadcast(), ShortAddress::BROADCAST)),
                source: Some(Address::Extended(mac_pib.pan_id, mac_pib.extended_address)),
                auxiliary_security_header: responder.request.coord_realign_security_info.into(),
            },
            content: FrameContent::Command(Command::CoordinatorRealignment(
                CoordinatorRealignmentData {
                    pan_id: responder.request.pan_id,
                    coordinator_address: mac_pib.short_address,
                    channel: responder.request.channel_number,
                    device_address: ShortAddress::BROADCAST,
                    channel_page: Some(responder.request.channel_page as u8),
                },
            )),
            payload: &[],
            footer: [0, 0],
        };

        let serialized_frame = mac_state.serialize_frame(coord_realignment_message);
        mac_state
            .message_scheduler
            .schedule_broadcast_priority(serialized_frame, SendCallback::StartProcedure(responder));
    } else {
        // We can apply the changes immediately
        apply_changes(phy, mac_pib, mac_state, responder).await;
    }
}

pub async fn coord_realignment_sent_callback<'a>(
    send_result: SendResult,
    phy: &mut impl Phy,
    mac_pib: &mut MacPib,
    mac_state: &mut MacState<'a>,
    responder: RequestResponder<'a, StartRequest>,
) {
    match send_result {
        SendResult::Success(_, _) => {
            apply_changes(phy, mac_pib, mac_state, responder).await;
        }
        SendResult::ChannelAccessFailure => {
            responder.respond(StartConfirm {
                status: Status::ChannelAccessFailure,
            });
        }
    };
}

async fn apply_changes<'a>(
    phy: &mut impl Phy,
    mac_pib: &mut MacPib,
    mac_state: &mut MacState<'a>,
    responder: RequestResponder<'a, StartRequest>,
) {
    let request = &responder.request;

    if request.pan_coordinator
        || request.start_time == 0
        || request.beacon_order == BeaconOrder::OnDemand
    {
        // We are going to run our own independent beacon

        if let Err(e) = update_superframe_config(phy, mac_pib, request).await {
            error!("Updating superframe config returned an error: {}", e);
            responder.respond(StartConfirm { status: e.into() });
            return;
        }

        mac_state.is_pan_coordinator = request.pan_coordinator;
        mac_state.beacon_security_info = request.beacon_security_info;
        mac_state.beacon_mode = if request.beacon_order != BeaconOrder::OnDemand {
            BeaconMode::OnAutonomous
        } else {
            BeaconMode::Off
        };

        responder.respond(StartConfirm {
            status: Status::Success,
        });
    } else if request.start_time > 0 && mac_state.coordinator_beacon_tracked {
        // We are going to run our beacon at an offset to the tracked beacon

        let beacon_super_frame_symbols =
            consts::BASE_SUPERFRAME_DURATION << u8::from(request.superframe_order);

        if request.start_time < beacon_super_frame_symbols {
            responder.respond(StartConfirm {
                status: Status::SuperframeOverlap,
            });
            return;
        }

        if let Err(e) = update_superframe_config(phy, mac_pib, request).await {
            error!("Updating superframe config returned an error: {}", e);
            responder.respond(StartConfirm { status: e.into() });
            return;
        }

        mac_state.is_pan_coordinator = request.pan_coordinator;
        mac_state.beacon_security_info = request.beacon_security_info;
        mac_state.beacon_mode = BeaconMode::OnTracking {
            start_time: request.start_time,
        };

        responder.respond(StartConfirm {
            status: Status::Success,
        });
    } else {
        // The user specified it wanted to track the beacon, but we aren't tracking one
        responder.respond(StartConfirm {
            status: Status::TrackingOff,
        });
    }
}

async fn update_superframe_config<P: Phy>(
    phy: &mut P,
    mac_pib: &mut MacPib,
    request: &StartRequest,
) -> Result<(), MacError<P::Error>> {
    // Implementation as per 5.1.2.3.4

    mac_pib.beacon_order = request.beacon_order;
    mac_pib.superframe_order = if request.beacon_order == BeaconOrder::OnDemand {
        SuperframeOrder::Inactive
    } else {
        request.superframe_order
    };
    mac_pib.pan_id = request.pan_id;

    if request.beacon_order != BeaconOrder::OnDemand {
        mac_pib.batt_life_ext = request.battery_life_extension;
    }

    phy.update_phy_pib(|phy_pib| {
        phy_pib.current_page = request.channel_page;
        phy_pib.current_channel = request.channel_number;
    })
    .await?;

    Ok(())
}
