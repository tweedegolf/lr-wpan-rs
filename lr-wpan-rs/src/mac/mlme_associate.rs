use super::{
    callback::DataRequestCallback,
    commander::{MacHandler, RequestResponder},
    state::{DataRequestMode, MacState, ScheduledDataRequest},
};
use crate::{
    phy::{Phy, SendContinuation, SendResult},
    pib::MacPib,
    sap::{
        associate::{AssociateConfirm, AssociateIndication, AssociateRequest},
        SecurityInfo, Status,
    },
    wire::{
        command::{CapabilityInformation, Command}, Address, ExtendedAddress, Frame, FrameContent, FrameType, FrameVersion, Header, PanId, ShortAddress
    }, DeviceAddress,
};

pub async fn process_associate_request<'a>(
    phy: &mut impl Phy,
    mac_pib: &mut MacPib,
    mac_state: &mut MacState<'a>,
    responder: RequestResponder<'a, AssociateRequest>,
) {
    if mac_pib.pan_id != PanId::broadcast() {
        // We are already associated, this is not allowed
        // The spec doesn't really say what to do in this case...
        responder.respond(AssociateConfirm {
            assoc_short_address: ShortAddress::BROADCAST,
            status: Status::AlreadyAssociated,
            security_info: SecurityInfo::new_none_security(),
        });
        return;
    }

    // Take the data from the request and reflect them into the pibs
    let result = phy
        .update_phy_pib(|phy_pib| {
            phy_pib.current_channel = responder.request.channel_number;
            phy_pib.current_page = responder.request.channel_page;
        })
        .await;

    if let Err(e) = result {
        error!(
            "Could not update the phy pib for the associate request: {}",
            e
        );
        responder.respond(AssociateConfirm {
            assoc_short_address: ShortAddress::BROADCAST,
            status: Status::PhyError,
            security_info: SecurityInfo::new_none_security(),
        });
        return;
    }

    mac_pib.pan_id = responder.request.coord_address.pan_id();
    match responder.request.coord_address {
        Address::Short(_, short_address) => mac_pib.coord_short_address = short_address,
        Address::Extended(_, extended_address) => mac_pib.coord_extended_address = extended_address,
    }

    // Generate the associate request and send it
    let dsn = mac_pib.dsn.increment();
    let associate_request_frame = Frame {
        header: Header {
            frame_type: FrameType::MacCommand,
            frame_pending: false,
            ack_request: true,
            pan_id_compress: false,
            seq_no_suppress: false,
            ie_present: false,
            version: FrameVersion::Ieee802154_2003,
            seq: dsn,
            destination: Some(responder.request.coord_address),
            source: Some(Address::Extended(
                PanId::broadcast(),
                mac_pib.extended_address,
            )),
            auxiliary_security_header: responder.request.security_info.into(),
        },
        content: FrameContent::Command(Command::AssociationRequest(
            responder.request.capability_information,
        )),
        payload: &[],
        footer: [0, 0],
    };
    let associate_request_frame_data = mac_state.serialize_frame(associate_request_frame);

    let ack_wait_duration = mac_pib.ack_wait_duration(phy.get_phy_pib()) as i64;

    debug!("Sending association request");

    // We send with ack request, but we won't retry if the ack is not received
    let send_result = phy
        .send(
            &associate_request_frame_data,
            None,
            false,
            true,
            SendContinuation::WaitForResponse {
                turnaround_time: phy.symbol_duration() * crate::consts::TURNAROUND_TIME as i64,
                timeout: phy.symbol_duration() * ack_wait_duration,
            },
        )
        .await;

    let ack_timestamp = match send_result {
        Ok(SendResult::Success(_, None)) => None,
        Ok(SendResult::Success(_, Some(mut response))) => {
            // See if what we received was an Ack for us

            match mac_state.deserialize_frame(&mut response.data) {
                Some(frame) => {
                    if matches!(frame.header.frame_type, FrameType::Acknowledgement)
                        && frame.header.seq == dsn
                    {
                        Some(response.timestamp)
                    } else {
                        None
                    }
                }
                None => None,
            }
        }
        Ok(SendResult::ChannelAccessFailure) => {
            responder.respond(AssociateConfirm {
                assoc_short_address: ShortAddress::BROADCAST,
                status: Status::ChannelAccessFailure,
                security_info: SecurityInfo::new_none_security(),
            });
            return;
        }
        Err(e) => {
            error!("Could not send the association request: {}", e);
            responder.respond(AssociateConfirm {
                assoc_short_address: ShortAddress::BROADCAST,
                status: Status::PhyError,
                security_info: SecurityInfo::new_none_security(),
            });
            return;
        }
    };

    // We did not get an ack, so let the higher level layer know
    let Some(ack_timestamp) = ack_timestamp else {
        responder.respond(AssociateConfirm {
            assoc_short_address: ShortAddress::BROADCAST,
            status: Status::NoAck,
            security_info: SecurityInfo::new_none_security(),
        });
        return;
    };

    debug!("Association procedure now waiting until the response can be requested");

    // We have received the ack to our association request.
    // Now we must wait and request our data later.

    mac_state
        .message_scheduler
        .schedule_data_request(ScheduledDataRequest {
            mode: DataRequestMode::Independent {
                timestamp: Some(
                    ack_timestamp
                        + phy.symbol_duration()
                            * crate::consts::BASE_SUPERFRAME_DURATION as i64
                            * mac_pib.response_wait_time as i64,
                ),
            },
            used_security_info: responder.request.security_info,
            callback: DataRequestCallback::AssociationProcedure(responder),
        });
}

// Received from the radio, not as an MLME request
pub async fn process_received_associate_request(mac_handler: &MacHandler<'_>, device_address: ExtendedAddress, capability_information: CapabilityInformation) {
    let indirect_response = mac_handler.indicate_indirect(AssociateIndication {
        device_address,
        capability_information,
        security_info: SecurityInfo::new_none_security(),
    });

    // TODO: Store the indirect_response and await it later
}
