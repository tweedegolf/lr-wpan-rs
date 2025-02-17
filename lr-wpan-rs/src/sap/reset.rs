use super::{ConfirmValue, Request, RequestValue, Status};

/// The MLME-RESET.request primitive is used by the next higher layer to request that the MLME performs a
/// reset operation.
///
/// On receipt of the MLME-RESET.request primitive, the MLME resets the PHY in an implementation-
/// dependent manner.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ResetRequest {
    /// If TRUE, the MAC sublayer is reset, and all MAC
    /// PIB attributes are set to their default values. If
    /// FALSE, the MAC sublayer is reset, but all MAC PIB
    /// attributes retain their values prior to the generation of
    /// the MLME-RESET.request primitive.
    pub set_default_pib: bool,
}

impl From<RequestValue> for ResetRequest {
    fn from(value: RequestValue) -> Self {
        match value {
            RequestValue::Reset(val) => val,
            _ => panic!("Bad cast"),
        }
    }
}

impl Request for ResetRequest {
    type Confirm = ResetConfirm;
}

/// The MLME-RESET.confirm primitive reports the results of the reset operation.
///
/// The status parameter is set to SUCCESS on completion of the reset procedure.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ResetConfirm {
    pub status: Status,
}

impl From<ConfirmValue> for ResetConfirm {
    fn from(value: ConfirmValue) -> Self {
        match value {
            ConfirmValue::Reset(val) => val,
            _ => panic!("Bad cast"),
        }
    }
}
