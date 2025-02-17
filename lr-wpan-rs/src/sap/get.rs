use super::{ConfirmValue, Request, RequestValue, Status};
use crate::pib::PibValue;

/// The MLME-GET.request primitive requests information about a given PIB attribute.
///
/// On receipt of the MLME-GET.request primitive, the MLME checks to see whether the PIB attribute is a
/// MAC PIB attribute or PHY PIB attribute. If the requested attribute is a MAC attribute, the MLME attempts
/// to retrieve the requested MAC PIB attribute from its database. If the requested attribute is a PHY PIB
/// attribute, the MLME attempts to retrieve the value from the PHY.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetRequest {
    pub pib_attribute: &'static str,
}

impl From<RequestValue> for GetRequest {
    fn from(value: RequestValue) -> Self {
        match value {
            RequestValue::Get(val) => val,
            _ => panic!("Bad cast"),
        }
    }
}

impl Request for GetRequest {
    type Confirm = GetConfirm;
}

/// The MLME-GET.confirm primitive reports the results of an information request from the PIB.
///
/// If the request to read a PIB attribute was successful, the primitive returns with a status of SUCCESS. If the
/// identifier of the PIB attribute is not found, the primitive returns with a status of
/// UNSUPPORTED_ATTRIBUTE. When an error code of UNSUPPORTED_ATTRIBUTE is returned, the
/// PIBAttribute value parameter will be set to length zero.
#[derive(Debug, Clone, PartialEq)]
pub struct GetConfirm {
    pub status: Status,
    pub pib_attribute: &'static str,
    pub value: PibValue,
}

impl From<ConfirmValue> for GetConfirm {
    fn from(value: ConfirmValue) -> Self {
        match value {
            ConfirmValue::Get(val) => val,
            _ => panic!("Bad cast"),
        }
    }
}
