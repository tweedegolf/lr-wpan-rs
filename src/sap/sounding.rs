use super::{ConfirmValue, Request, RequestValue, Status};
use alloc::vec::Vec;

/// The MLME-SOUNDING.request primitive is used by the next higher layer to request that the PHY respond
/// with channel sounding information. The MLME-SOUNDING.request primitive shall be supported by all
/// RDEVs; however, the underlying sounding capability is optional in all cases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoundingRequest {
    // Intentionally empty
}

impl From<RequestValue> for SoundingRequest {
    fn from(value: RequestValue) -> Self {
        match value {
            RequestValue::Sounding(val) => val,
            _ => panic!("Bad cast"),
        }
    }
}

impl Request for SoundingRequest {
    type Confirm = SoundingConfirm;
}

/// The MLME-CHANNEL.confirm primitive reports the result of a request to the PHY to provide channel
/// sounding information. The MLME-SOUNDING.confirm primitive shall be supported by all RDEVs;
/// however, the underlying sounding capability is optional in all cases.
///
/// If the channel sounding information is available, the status parameter will be set to SUCCESS and the
/// SoundingList will contain valid data.
///
/// If the MLME-SOUNDING.request primitive is received when there is no information present, e.g., when
/// the PHY is in the process of performing a measurement, the status parameter will be set to NO_DATA.
///
/// If the channel sounding capability is not supported by the PHY, the status parameters will be set to
/// UNSUPPORTED_ATTRIBUTE.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoundingConfirm {
    pub sounding_list: Vec<SoundingData>,
    pub status: Status,
}

impl From<ConfirmValue> for SoundingConfirm {
    fn from(value: ConfirmValue) -> Self {
        match value {
            ConfirmValue::Sounding(val) => val,
            _ => panic!("Bad cast"),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct SoundingData {
    /// 16 ps per tick
    time: i16,
    amplitude: i16,
}
