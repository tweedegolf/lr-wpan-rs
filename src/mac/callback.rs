use crate::{
    phy::{Phy, SendResult},
    pib::MacPib,
    sap::start::StartRequest,
};

use super::{commander::RequestResponder, state::MacState};

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
