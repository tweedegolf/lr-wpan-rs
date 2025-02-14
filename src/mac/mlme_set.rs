use crate::{
    phy::Phy,
    pib::{MacPibWrite, PibValue},
    sap::{
        set::{SetConfirm, SetRequest},
        Status,
    },
};

use super::{commander::RequestResponder, MacError};

pub async fn process_set_request(
    phy: &mut impl Phy,
    mac_pib_write: &mut MacPibWrite,
    responder: RequestResponder<'_, SetRequest>,
) {
    let pib_attribute = responder.request.pib_attribute;

    match set_pib_value(
        phy,
        mac_pib_write,
        pib_attribute,
        responder.request.pib_attribute_value.clone(),
    )
    .await
    {
        Ok(status) => responder.respond(SetConfirm {
            status,
            pib_attribute,
        }),
        Err(e) => responder.respond(SetConfirm {
            status: e.into(),
            pib_attribute,
        }),
    }
}

async fn set_pib_value<P: Phy>(
    phy: &mut P,
    mac_pib_write: &mut MacPibWrite,
    pib_attribute: &str,
    pib_value: PibValue,
) -> Result<Status, MacError<P::Error>> {
    if let Some(status) = phy
        .update_phy_pib(|phy_pib| phy_pib.try_set(pib_attribute, &pib_value))
        .await?
    {
        return Ok(status);
    }

    if let Some(status) = mac_pib_write.try_set(pib_attribute, &pib_value) {
        return Ok(status);
    }

    Err(MacError::UnsupportedAttribute)
}
