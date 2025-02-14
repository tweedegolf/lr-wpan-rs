use ieee802154::mac::{command::GuaranteedTimeSlotCharacteristics, ShortAddress};

use super::{
    ConfirmValue, Indication, IndicationValue, Request, RequestValue, SecurityInfo, Status,
};

/// The MLME-GTS.request primitive allows a device to send a request to the PAN coordinator to allocate a
/// new GTS or to deallocate an existing GTS. This primitive is also used by the PAN coordinator to initiate a
/// GTS deallocation.
///
/// On receipt of the MLME-GTS.request primitive by a device, the MLME of a device performs either the
/// GTS request procedure,as described in 5.1.7.2, or the GTS deallocation procedure, as described in 5.1.7.4,
/// depending on the value of the GTSCharacteristics field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GtsRequest {
    /// The characteristics of the GTS request, including
    /// whether the request is for the allocation of a new
    /// GTS or the deallocation of an existing GTS.
    pub gts_characteristics: GuaranteedTimeSlotCharacteristics,
    pub security_info: SecurityInfo,
}

impl From<RequestValue> for GtsRequest {
    fn from(value: RequestValue) -> Self {
        match value {
            RequestValue::Gts(val) => val,
            _ => panic!("Bad cast"),
        }
    }
}

impl Request for GtsRequest {
    type Confirm = GtsConfirm;
}

/// The MLME-GTS.confirm primitive reports the results of a request to allocate a new GTS or to deallocate an
/// existing GTS.
///
/// If the request to allocate or deallocate a GTS was successful, this primitive will return a status of SUCCESS
/// and the Characteristics Type field of the GTSCharacteristics parameter will have the value of one or zero,
/// respectively. Otherwise, the status parameter will indicate the appropriate error code, as defined in 5.1.7.2 or
/// 5.1.7.4.
///
/// If macShortAddress is equal to 0xfffe or 0xffff, the device is not permitted to request a GTS and the status
/// parameter will be set to NO_SHORT_ADDRESS.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GtsConfirm {
    pub gts_characteristics: GuaranteedTimeSlotCharacteristics,
    pub status: Status,
}

impl From<ConfirmValue> for GtsConfirm {
    fn from(value: ConfirmValue) -> Self {
        match value {
            ConfirmValue::Gts(val) => val,
            _ => panic!("Bad cast"),
        }
    }
}

/// The MLME-GTS.indication primitive indicates that a GTS has been allocated or that a previously allocated
/// GTS has been deallocated.
///
/// The value of the Characteristics Type field, as defined in 5.3.9.2, in the GTSCharacteristics parameter
/// indicates if the GTS has been allocated or if a GTS has been deallocated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GtsIndication {
    pub device_address: ShortAddress,
    pub gts_characteristics: GuaranteedTimeSlotCharacteristics,
    pub security_info: SecurityInfo,
}

impl From<IndicationValue> for GtsIndication {
    fn from(value: IndicationValue) -> Self {
        match value {
            IndicationValue::Gts(val) => val,
            _ => panic!("Bad cast"),
        }
    }
}

impl Indication for GtsIndication {
    type Response = ();
}
