use super::{MacError, commander::RequestResponder};
use crate::{
    phy::Phy,
    pib::{MacPib, PibValue},
    sap::{
        Status,
        get::{GetConfirm, GetRequest},
    },
};

pub async fn process_get_request(
    phy: &mut impl Phy,
    mac_pib: &MacPib,
    responder: RequestResponder<'_, GetRequest>,
) {
    let pib_attribute = responder.request.pib_attribute;
    let value = get_pib_value(phy, mac_pib, pib_attribute).await;

    match value {
        Ok(value) => responder.respond(GetConfirm {
            pib_attribute,
            status: Status::Success,
            value,
        }),
        Err(e) => responder.respond(GetConfirm {
            pib_attribute,
            status: e.into(),
            value: PibValue::None,
        }),
    }
}

async fn get_pib_value<P: Phy>(
    phy: &mut P,
    mac_pib: &MacPib,
    pib_attribute: &str,
) -> Result<PibValue, MacError<P::Error>> {
    let phy_pib = phy.get_phy_pib();

    if let Some(val) = phy_pib.get(pib_attribute) {
        return Ok(val);
    }

    if let Some(val) = mac_pib.get(pib_attribute, phy_pib) {
        return Ok(val);
    }

    Err(MacError::UnsupportedAttribute)
}
