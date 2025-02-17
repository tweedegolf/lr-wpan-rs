use super::{ConfirmValue, DynamicRequest, Request, RequestValue, Status};
use crate::time::{Duration, Instant};

/// The MLME-RX-ENABLE.request primitive allows the next higher layer to request that the receiver is either
/// enabled for a finite period of time or disabled.
///
/// The MLME-RX-ENABLE.request primitive is generated by the next higher layer and issued to the MLME
/// to enable the receiver for a fixed duration, at a time relative to the start of the current or next superframe on
/// a beacon-enabled PAN or immediately on a nonbeacon-enabled PAN. This primitive may also be generated
/// to cancel a previously generated request to enable the receiver. The receiver is enabled or disabled exactly
/// once per primitive request.
///
/// The MLME will treat the request to enable or disable the receiver as secondary to other responsibilities of
/// the device (e.g., GTSs, coordinator beacon tracking, or beacon transmissions). When the primitive is issued
/// to enable the receiver, the device will enable its receiver until either the device has a conflicting
/// responsibility or the time specified by RxOnDuration has expired. In the case of a conflicting responsibility,
/// the device will interrupt the receive operation. After the completion of the interrupting operation, the
/// RxOnDuration will be checked to determine whether the time has expired. If so, the operation is complete. If
/// not, the receiver is re-enabled until either the device has another conflicting responsibility or the time
/// specified by RxOnDuration has expired. When the primitive is issued to disable the receiver, the device will
/// disable its receiver unless the device has a conflicting responsibility.
///
/// On a nonbeacon-enabled PAN, the MLME ignores the DeferPermit and RxOnTime parameters and requests
/// that the PHY enable or disable the receiver immediately. If the request is to enable the receiver, the receiver
/// will remain enabled until RxOnDuration has elapsed.
///
/// Before attempting to enable the receiver on a beacon-enabled PAN, the MLME first determines whether
/// (RxOnTime + RxOnDuration) is less than the beacon interval, as defined by macBeaconOrder. If
/// (RxOnTime + RxOnDuration) is not less than the beacon interval, the MLME issues the MLME-RX-
/// ENABLE.confirm primitive with a status of ON_TIME_TOO_LONG.
///
/// The MLME then determines whether the receiver can be enabled in the current superframe. If the current
/// time measured from the start of the superframe is less than (RxOnTime – macSIFSPeriod), the MLME
/// attempts to enable the receiver in the current superframe. If the current time measured from the start of the
/// superframe is greater than or equal to (RxOnTime – macSIFSPeriod) and DeferPermit is equal to TRUE, the
/// MLME defers until the next superframe and attempts to enable the receiver in that superframe. Otherwise, if
/// the MLME cannot enable the receiver in the current superframe and is not permitted to defer the receive
/// operation until the next superframe, the MLME issues the MLME-RX-ENABLE.confirm primitive with a
/// status of PAST_TIME.
///
/// If the RxOnDuration parameter is equal to zero, the MLME requests that the PHY disable its receiver.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RxEnableRequest {
    /// TRUE if the requested operation can be deferred until
    /// the next superframe if the requested time has already
    /// passed. FALSE if the requested operation is only to be
    /// attempted in the current superframe. This parameter is
    /// ignored for nonbeacon-enabled PANs.
    /// If the issuing device is the PAN coordinator, the term
    /// superframe refers to its own superframe. Otherwise,
    /// the term refers to the superframe of the coordinator
    /// through which the issuing device is associated.
    pub defer_permit: bool,
    /// The number of symbols measured from the start of the
    /// superframe before the receiver is to be enabled or disabled. This is a 24-bit value, and the precision of this
    /// value shall be a minimum of 20 bits, with the lowest 4
    /// bits being the least significant. This parameter is
    /// ignored for nonbeacon-enabled PANs.
    /// If the issuing device is the PAN coordinator, the term
    /// superframe refers to its own superframe. Otherwise,
    /// the term refers to the superframe of the coordinator
    /// through which the issuing device is associated.
    pub rx_on_time: Instant,
    /// The number of symbols for which the receiver is to be
    /// enabled.
    /// If this parameter is equal to 0x000000, the receiver is
    /// to be disabled.
    pub rx_on_duration: Duration,
    /// Configure the transceiver to Rx with ranging for a
    /// value of RANGING_ON (true) or to not enable ranging for
    /// RANGING_OFF (false).
    pub ranging_rx_control: bool,
}

impl From<RequestValue> for RxEnableRequest {
    fn from(value: RequestValue) -> Self {
        match value {
            RequestValue::RxEnable(val) => val,
            _ => panic!("Bad cast"),
        }
    }
}

impl DynamicRequest for RxEnableRequest {
    type Confirm = RxEnableConfirm;
    type AllocationElement = core::convert::Infallible;
}

impl Request for RxEnableRequest {}

/// The MLME-RX-ENABLE.confirm primitive reports the results of the attempt to enable or disable the receiver.
///
/// The MLME-RX-ENABLE.confirm primitive is generated by the MLME and issued to its next higher layer
/// in response to an MLME-RX-ENABLE.request primitive. This primitive returns a status of either
/// SUCCESS, if the request to enable or disable the receiver was successful, or the appropriate error code. The
/// status values are fully described in 6.2.9.1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RxEnableConfirm {
    pub status: Status,
}

impl From<ConfirmValue> for RxEnableConfirm {
    fn from(value: ConfirmValue) -> Self {
        match value {
            ConfirmValue::RxEnable(val) => val,
            _ => panic!("Bad cast"),
        }
    }
}
