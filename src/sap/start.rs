use super::{ConfirmValue, Request, RequestValue, SecurityInfo, Status};
use crate::{
    wire::{
        beacon::{BeaconOrder, SuperframeOrder},
        PanId,
    },
    ChannelPage,
};

/// The MLME-START.request primitive is used by the PAN coordinator to initiate a new PAN or to begin
/// using a new superframe configuration. This primitive is also used by a device already associated with an
/// existing PAN to begin using a new superframe configuration.
///
/// When the CoordRealignment parameter is set to TRUE, the coordinator attempts to transmit a coordinator
/// realignment command frame as described in 5.1.2.3.2. If the transmission of the coordinator realignment
/// command fails due to a channel access failure, the MLME will not make any changes to the superframe
/// configuration. (i.e., no PIB attributes will be changed). If the coordinator realignment command is
/// successfully transmitted, the MLME updates the PIB attributes BeaconOrder, SuperframeOrder, PANId,
/// ChannelPage, and ChannelNumber parameters.
///
/// When the CoordRealignment parameter is set to FALSE, the MLME updates the appropriate PIB attributes
/// with the values of the BeaconOrder, SuperframeOrder, PANId, ChannelPage, and ChannelNumber
/// parameters, as described in 5.1.2.3.4.
///
/// The address used by the coordinator in its beacon frames is determined by the current value of
/// macShortAddress, which is set by the next higher layer before issuing this primitive. If the BeaconOrder
/// parameter is less than 15, the MLME sets macBattLifeExt to the value of the BatteryLifeExtension
/// parameter. If the BeaconOrder parameter equals 15, the value of the BatteryLifeExtension parameter is
/// ignored.
///
/// If the CoordRealignment parameter is set to TRUE, the CoordRealignSecurityLevel, CoordRealignKeyId-
/// Mode, CoordRealignKeySource, and CoordRealignKeyIndex parameters will be used to process the MAC
/// command frame. If the BeaconOrder parameter indicates a beacon-enabled network, the BeaconSecurity-
/// Level, BeaconKeyIdMode, BeaconKeySource, and BeaconKeyIndex parameters will be used to process the
/// beacon frame.
///
/// The MLME shall ignore the StartTime parameter if the BeaconOrder parameter is equal to 15 because this
/// indicates a nonbeacon-enabled PAN. If the BeaconOrder parameter is less than 15, the MLME examines the
/// StartTime parameter to determine the time to begin transmitting beacons. If the PAN coordinator parameter
/// is set to TRUE, the MLME ignores the StartTime parameter and begins beacon transmissions immediately.
/// Setting the StartTime parameter to 0x000000 also causes the MLME to begin beacon transmissions
/// immediately. If the PANCoordinator parameter is set to FALSE and the StartTime parameter is nonzero, the
/// MLME calculates the beacon transmission time by adding StartTime to the time, obtained from the local
/// clock, when the MLME receives the beacon of the coordinator through which it is associated. If the time
/// calculated causes the outgoing superframe to overlap the incoming superframe, the MLME shall not begin
/// beacon transmissions. Otherwise, the MLME then begins beacon transmissions when the current time,
/// obtained from the local clock, equals the calculated time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartRequest {
    /// The PAN identifier to be used by the device.
    pub pan_id: PanId,
    pub channel_number: u8,
    pub channel_page: ChannelPage,
    /// The time at which to begin transmitting beacons. If this parameter is
    /// equal to 0x000000, beacon transmissions will begin immediately. Otherwise, the specified time is relative to
    /// the received beacon of the coordinator with which the device synchronizes. This parameter is ignored if
    /// either the BeaconOrder parameter has
    /// a value of 15 or the PANCoordinator
    /// parameter is TRUE. The time is specified in symbols and is rounded to a
    /// backoff period boundary. The precision of this value shall be a minimum
    /// of 20 bits, with the lowest 4 bits being
    /// the least significant.
    pub start_time: u32,
    /// Indicates the frequency with which
    /// the beacon is transmitted, as defined
    /// in 5.1.1.1.
    ///
    /// ## Range
    ///
    /// 0–15
    pub beacon_order: BeaconOrder,
    /// The length of the active portion of the
    /// superframe, including the beacon
    /// frame, as defined in 5.1.1.1.
    ///
    /// ## Range
    ///
    /// 0–beacon_order or 15
    pub superframe_order: SuperframeOrder,
    /// If this value is TRUE, the device will
    /// become the PAN coordinator of a new
    /// PAN. If this value is FALSE, the
    /// device will begin using a new superframe configuration on the PAN with
    /// which it is associated.
    pub pan_coordinator: bool,
    /// If this value is TRUE, the receiver of
    /// the beaconing device is disabled macBattLifeExtPeriods full backoff periods after the interframe spacing (IFS)
    /// period following the beacon frame. If
    /// this value is FALSE, the receiver of
    /// the beaconing device remains enabled
    /// for the entire CAP. This parameter is
    /// ignored if the BeaconOrder parameter
    /// has a value of 15.
    pub battery_life_extension: bool,
    /// TRUE if a coordinator realignment
    /// command is to be transmitted prior to
    /// changing the superframe configuration or FALSE otherwise.
    pub coord_realignment: bool,
    /// The security level to be used for coordinator realignment command
    /// frames, as described in Table 58.
    pub coord_realign_security_info: SecurityInfo,
    /// The security level to be used for beacon frames, as described in Table 58.
    pub beacon_security_info: SecurityInfo,
}

impl From<RequestValue> for StartRequest {
    fn from(value: RequestValue) -> Self {
        match value {
            RequestValue::Start(val) => val,
            _ => panic!("Bad cast"),
        }
    }
}

impl Request for StartRequest {
    type Confirm = StartConfirm;
}

/// The MLME-START.confirm primitive reports the results of the attempt to start using a new superframe configuration.
///
/// The MLME-START.confirm primitive is generated by the MLME and issued to its next higher layer in
/// response to an MLME-START.request primitive. The MLME-START.confirm primitive returns a status of
/// either SUCCESS, indicating that the MAC sublayer has started using the new superframe configuration, or
/// the appropriate error code as follows:
///
/// - NO_SHORT_ADDRESS – The macShortAddress is set to 0xffff.
/// - CHANNEL_ACCESS_FAILURE – The transmission of the coordinator realignment frame failed.
/// - FRAME_TOO_LONG – The length of the beacon frame exceeds aMaxPHYPacketSize.
/// - SUPERFRAME_OVERLAP – The outgoing superframe overlaps the incoming superframe.
/// - TRACKING_OFF – The StartTime parameter is nonzero, and the MLME is not currently tracking
///   the beacon of the coordinator through which it is associated.
/// - A security error code, as defined in 7.2.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartConfirm {
    pub status: Status,
}

impl From<ConfirmValue> for StartConfirm {
    fn from(value: ConfirmValue) -> Self {
        match value {
            ConfirmValue::Start(val) => val,
            _ => panic!("Bad cast"),
        }
    }
}
