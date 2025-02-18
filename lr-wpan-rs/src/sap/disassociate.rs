use super::{
    ConfirmValue, DynamicRequest, Indication, IndicationValue, Request, RequestValue, SecurityInfo,
    Status,
};
use crate::wire::{command::DisassociationReason, Address, ExtendedAddress};

/// The MLME-DISASSOCIATE.request primitive is used by an associated device to notify the coordinator of
/// its intent to leave the PAN. It is also used by the coordinator to instruct an associated device to leave the
/// PAN.
///
/// If the DeviceAddrMode parameter is equal to SHORT_ADDRESS and the DeviceAddress parameter is
/// equal to macCoordShortAddress or if the DeviceAddrMode parameter is equal to EXTENDED_ADDRESS
/// and the DeviceAddress parameter is equal to macCoordExtendedAddress, the TxIndirect parameter is
/// ignored, and the MLME sends a disassociation notification command, as defined in 5.3.3, to its coordinator
/// in the CAP for a beacon-enabled PAN or immediately for a nonbeacon-enabled PAN.
///
/// If the DeviceAddrMode parameter is equal to SHORT_ADDRESS and the DeviceAddress parameter is not
/// equal to macCoordShortAddress or if the DeviceAddrMode parameter is equal to EXTENDED_ADDRESS
/// and the DeviceAddress parameter is not equal to macCoordExtendedAddress, and if this primitive was
/// received by the MLME of a coordinator with the TxIndirect parameter set to TRUE, the disassociation
/// notification command will be sent using indirect transmission, as described in 5.1.5.
///
/// If the DeviceAddrMode parameter is equal to SHORT_ADDRESS and the DeviceAddress parameter is not
/// equal to macCoordShortAddress or if the DeviceAddrMode parameter is equal to EXTENDED_ADDRESS
/// and the DeviceAddress parameter is not equal to macCoordExtendedAddress, and if this primitive was
/// received by the MLME of a coordinator with the TxIndirect parameter set to FALSE, the MLME sends a
/// disassociation notification command to the device in the CAP for a beacon-enabled PAN or immediately for
/// a nonbeacon-enabled PAN.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisassociateRequest {
    /// The address of the device to which to send
    /// the disassociation notification command.
    pub device_address: Address,
    pub disassociate_reason: DisassociationReason,
    /// TRUE if the disassociation notification
    /// command is to be sent indirectly.
    pub tx_indirect: bool,
    pub security_info: SecurityInfo,
}

impl From<RequestValue> for DisassociateRequest {
    fn from(value: RequestValue) -> Self {
        match value {
            RequestValue::Disassociate(val) => val,
            _ => panic!("Bad cast"),
        }
    }
}

impl DynamicRequest for DisassociateRequest {
    type Confirm = DisassociateConfirm;
    type AllocationElement = core::convert::Infallible;
}

impl Request for DisassociateRequest {}

/// The MLME-DISASSOCIATE.indication primitive is used to indicate the reception of a disassociation
/// notification command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisassociateIndication {
    /// The address of the device requesting disassociation.
    pub device_address: ExtendedAddress,
    pub disassociate_reason: DisassociationReason,
    pub security_info: SecurityInfo,
}

impl From<IndicationValue> for DisassociateIndication {
    fn from(value: IndicationValue) -> Self {
        match value {
            IndicationValue::Disassociate(val) => val,
            _ => panic!("Bad cast"),
        }
    }
}

impl Indication for DisassociateIndication {
    type Response = ();
}

/// The MLME-DISASSOCIATE.confirm primitive reports the results of an MLME-DISASSOCIATE.request primitive.
///
/// This primitive returns a status of either SUCCESS, indicating that the disassociation request was successful,
/// or the appropriate status parameter value indicating the reason for failure.
///
/// If the DevicePANId parameter is not equal to macPANId in the MLME-DISASSOCIATE.request primitive,
/// the status parameter shall be set to INVALID_PARAMETER.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisassociateConfirm {
    pub status: Status,
    /// The address of the device that has
    /// either requested disassociation or
    /// been instructed to disassociate by its
    /// coordinator.
    pub device_address: Address,
}

impl From<ConfirmValue> for DisassociateConfirm {
    fn from(value: ConfirmValue) -> Self {
        match value {
            ConfirmValue::Disassociate(val) => val,
            _ => panic!("Bad cast"),
        }
    }
}
