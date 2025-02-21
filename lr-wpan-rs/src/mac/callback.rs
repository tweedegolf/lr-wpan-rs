use super::{commander::RequestResponder, state::MacState};
use crate::{
    phy::{Phy, SendResult},
    pib::MacPib,
    sap::{associate::AssociateRequest, start::StartRequest},
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
    pub async fn run(self) {
        match self {
            DataRequestCallback::AssociationProcedure(_request_responder) => todo!(),
        }
    }
}
