use super::{commander::RequestResponder, state::MacState};
use crate::{
    phy::{Phy, SendResult},
    pib::MacPib,
    sap::{
        associate::{AssociateConfirm, AssociateRequest},
        start::StartRequest,
        Status,
    },
    wire::command::AssociationStatus,
};

/// A callback that will be ran when a message has been sent.
pub enum SendCallback<'a> {
    StartProcedure(RequestResponder<'a, StartRequest>),
}

impl<'a> SendCallback<'a> {
    pub async fn run(
        self,
        send_result: SendResult,
        phy: &mut impl Phy,
        mac_pib: &mut MacPib,
        mac_state: &mut MacState<'a>,
    ) {
        match self {
            SendCallback::StartProcedure(responder) => {
                super::mlme_start::coord_realignment_sent_callback(
                    send_result,
                    phy,
                    mac_pib,
                    mac_state,
                    responder,
                )
                .await;
            }
        }
    }
}

pub enum DataRequestCallback<'a> {
    AssociationProcedure(RequestResponder<'a, AssociateRequest>),
}

impl DataRequestCallback<'_> {
    #[expect(unused, reason = "For now")]
    pub async fn run_data_request(self) {
        #[expect(clippy::match_single_binding, reason = "For now")]
        match self {
            _ => panic!("Should only be called on real data request callbacks"),
        }
    }

    pub async fn run_associate(
        self,
        associate_confirm: Result<AssociateConfirm, Result<AssociationStatus, Status>>,
        mac_pib: &mut MacPib,
    ) {
        match self {
            DataRequestCallback::AssociationProcedure(request_responder) => {
                super::mlme_associate::association_data_request_callback(
                    request_responder,
                    associate_confirm,
                    mac_pib,
                )
                .await;
            }
        }
    }
}
