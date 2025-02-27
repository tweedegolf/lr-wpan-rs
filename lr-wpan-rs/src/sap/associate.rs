use super::{
    ConfirmValue, DynamicRequest, Indication, IndicationValue, Request, RequestValue,
    ResponseValue, SecurityInfo, Status,
};
use crate::{
    wire::{
        command::{AssociationStatus, CapabilityInformation},
        Address, ExtendedAddress, ShortAddress,
    },
    ChannelPage,
};

/// The MLME-ASSOCIATE.request primitive is used by a device to request an association with a coordinator.
///
/// On receipt of the MLME-ASSOCIATE.request primitive, the MLME of an unassociated device first updates
/// the appropriate PHY and MAC PIB attributes, as described in 5.1.3.1, and then generates an association
/// request command, as defined in 5.3.1.
///
/// The SecurityLevel parameter specifies the level of security to be applied to the association request command
/// frame. Typically, the association request command should not be implemented using security. However, if
/// the device requesting association shares a key with the coordinator, then security may be specified
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssociateRequest {
    /// The channel number on which to attempt association.
    pub channel_number: u8,
    /// The channel page on which to attempt association.
    pub channel_page: ChannelPage,
    /// - The coordinator addressing mode for this primitive and subsequent MPDU.
    /// - The identifier of the PAN with which to associate.
    /// - The address of the coordinator with which to associate.
    pub coord_address: Address,
    /// Specifies the operational capabilities of the associating device.
    pub capability_information: CapabilityInformation,
    pub security_info: SecurityInfo,
}

impl From<RequestValue> for AssociateRequest {
    fn from(value: RequestValue) -> Self {
        match value {
            RequestValue::Associate(val) => val,
            _ => panic!("Bad cast"),
        }
    }
}

impl DynamicRequest for AssociateRequest {
    type Confirm = AssociateConfirm;
    type AllocationElement = core::convert::Infallible;
}

impl Request for AssociateRequest {}

/// The MLME-ASSOCIATE.indication primitive is used to indicate the reception of an association request
/// command.
///
/// When the next higher layer of a coordinator receives the MLME-ASSOCIATE.indication primitive, the
/// coordinator determines whether to accept or reject the unassociated device using an algorithm outside the
/// scope of this standard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssociateIndication {
    /// The address of the device requesting association.
    pub device_address: ExtendedAddress,
    /// The operational capabilities of the device requesting association.
    pub capability_information: CapabilityInformation,
    pub security_info: SecurityInfo,
}

impl From<IndicationValue> for AssociateIndication {
    fn from(value: IndicationValue) -> Self {
        match value {
            IndicationValue::Associate(val) => val,
            _ => panic!("Bad cast"),
        }
    }
}

impl Indication for AssociateIndication {
    type Response = AssociateResponse;
}

/// The MLME-ASSOCIATE.response primitive is used to initiate a response to an MLME-
/// ASSOCIATE.indication primitive.
///
/// When the MLME of a coordinator receives the MLME-ASSOCIATE.response primitive, it generates an
/// association response command, as described in 5.3.2, and attempts to send it to the device requesting
/// association, as described in 5.1.3.1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssociateResponse {
    /// The address of the device requesting association.
    pub device_address: ExtendedAddress,
    /// The short device address allocated by the
    /// coordinator on successful association. This
    /// parameter is set to 0xffff if the association
    /// was unsuccessful.
    pub assoc_short_address: ShortAddress,
    /// The status of the association attempt.
    pub status: AssociationStatus,
    pub security_info: SecurityInfo,
}

impl From<ResponseValue> for AssociateResponse {
    fn from(value: ResponseValue) -> Self {
        match value {
            ResponseValue::Associate(val) => val,
            _ => panic!("Bad cast"),
        }
    }
}

/// The MLME-ASSOCIATE.confirm primitive is used to inform the next higher layer of the initiating device
/// whether its request to associate was successful or unsuccessful.
///
/// If the association request was successful, then the status parameter will be set to SUCCESS. Otherwise, the
/// status parameter will be set to indicate the type of failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssociateConfirm {
    /// The short device address allocated by the
    /// coordinator on successful association. This
    /// parameter is set to 0xffff if the association
    /// was unsuccessful.
    pub assoc_short_address: ShortAddress,
    pub status: Status,
    pub security_info: SecurityInfo,
}

impl From<ConfirmValue> for AssociateConfirm {
    fn from(value: ConfirmValue) -> Self {
        match value {
            ConfirmValue::Associate(val) => val,
            _ => panic!("Bad cast"),
        }
    }
}
