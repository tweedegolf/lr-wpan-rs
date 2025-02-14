use associate::{AssociateConfirm, AssociateIndication, AssociateRequest, AssociateResponse};
use beacon_notify::BeaconNotifyIndication;
use calibrate::{CalibrateConfirm, CalibrateRequest};
use comm_status::CommStatusIndication;
use data::{DataConfirm, DataIndication, DataRequest};
use disassociate::{DisassociateConfirm, DisassociateIndication, DisassociateRequest};
use dps::{DpsConfirm, DpsIndication, DpsRequest};
use get::{GetConfirm, GetRequest};
use gts::{GtsConfirm, GtsIndication, GtsRequest};
use orphan::{OrphanIndication, OrphanResponse};
use poll::{PollConfirm, PollRequest};
use purge::{PurgeConfirm, PurgeRequest};
use reset::{ResetConfirm, ResetRequest};
use rx_enable::{RxEnableConfirm, RxEnableRequest};
use scan::{ScanConfirm, ScanRequest};
use set::{SetConfirm, SetRequest};
use sounding::{SoundingConfirm, SoundingRequest};
use start::{StartConfirm, StartRequest};
use sync::{SyncLossIndication, SyncRequest};

use crate::{
    time::Instant,
    wire::{
        beacon::SuperframeSpecification,
        security::{
            AuxiliarySecurityHeader, KeyIdentifier, KeyIdentifierMode, SecurityControl,
            SecurityError, SecurityLevel,
        },
        Address,
    },
    ChannelPage,
};

pub mod associate;
pub mod beacon_notify;
pub mod calibrate;
pub mod comm_status;
pub mod data;
pub mod disassociate;
pub mod dps;
pub mod get;
pub mod gts;
pub mod orphan;
pub mod poll;
pub mod purge;
pub mod reset;
pub mod rx_enable;
pub mod scan;
pub mod set;
pub mod sounding;
pub mod start;
pub mod sync;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub enum Status {
    #[default]
    Success,
    NoAck,
    TransactionOverflow,
    TransactionExpired,
    ChannelAccessFailure,
    CounterError,
    FrameTooLong,
    UnavailableKey,
    UnsupportedSecurity,
    InvalidParameter,
    ImproperKeyType,
    ImproperSecurityLevel,
    NetworkAtCapacity,
    AccessDenied,
    NoData,
    SecurityError,
    UnsupportedLegacy,
    UnsupportedAttribute,
    Denied,
    NoShortAddress,
    OnTimeTooLong,
    PastTime,
    RangingNotSupported,
    LimitReached,
    NoBeacon,
    ScanInProgress,
    SuperframeOverlap,
    TrackingOff,
    DpsNotSupported,
    SoundingNotSupported,
    ComputationNeeded,
    InvalidAddress,
    InvalidGts,
    InvalidHandle,
    PhyError,
    ReadOnly,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct SecurityInfo {
    pub security_level: SecurityLevel,
    pub key_id_mode: KeyIdentifierMode,
    pub key_identifier: Option<KeyIdentifier>,
}

impl SecurityInfo {
    pub fn new_none_security() -> Self {
        Self {
            security_level: SecurityLevel::None,
            key_id_mode: KeyIdentifierMode::None,
            key_identifier: None,
        }
    }

    pub fn has_security(&self) -> bool {
        self.security_level != SecurityLevel::None
    }

    pub fn get_frame_version(&self) -> crate::wire::FrameVersion {
        if self.has_security() {
            crate::wire::FrameVersion::Ieee802154_2006
        } else {
            crate::wire::FrameVersion::Ieee802154_2003
        }
    }
}

impl From<SecurityInfo> for Option<AuxiliarySecurityHeader> {
    fn from(value: SecurityInfo) -> Self {
        if !value.has_security() {
            None
        } else {
            Some(AuxiliarySecurityHeader::new(
                SecurityControl::new(value.security_level),
                value.key_identifier,
            ))
        }
    }
}

impl From<Option<AuxiliarySecurityHeader>> for SecurityInfo {
    fn from(value: Option<AuxiliarySecurityHeader>) -> Self {
        match value {
            Some(header) => SecurityInfo {
                security_level: header.control.security_level(),
                key_id_mode: header.control.key_id_mode(),
                key_identifier: header.key_identifier,
            },
            None => Self::default(),
        }
    }
}

impl Default for SecurityInfo {
    fn default() -> Self {
        Self::new_none_security()
    }
}

#[derive(Debug, Clone)]
pub struct PanDescriptor {
    /// The address of the coordinator as specified in the received beacon frame.
    pub coord_address: Address,
    /// The current channel number occupied by the network.
    pub channel_number: u8,
    /// The current channel page occupied by the network.
    pub channel_page: ChannelPage,
    /// The superframe specification as specified in the received beacon frame.
    pub super_frame_spec: SuperframeSpecification,
    /// TRUE if the beacon is from the PAN coordinator that is accepting GTS requests.
    pub gts_permit: bool,
    /// The LQI at which the network beacon was
    /// received. Lower values represent lower
    /// LQI, as defined in 8.2.6.
    pub link_quality: u8,
    /// The time at which the beacon frame was received
    pub timestamp: Instant,
    /// NONE/SUCCESS if there was no error in the security processing of the frame. One of the
    /// other status codes indicating an error in the
    /// security processing otherwise, as described
    /// in 7.2.3.
    pub security_status: Option<SecurityError>,
    pub security_info: SecurityInfo,
    /// Not implemented. Seemingly unused everywhere and quite big when not heap allocated
    pub code_list: (),
}

impl PartialEq for PanDescriptor {
    fn eq(&self, other: &Self) -> bool {
        #[expect(clippy::unit_cmp, reason = "Might not be unit in future")]
        let code_list_equal = self.code_list == other.code_list;

        self.coord_address == other.coord_address
            && self.channel_number == other.channel_number
            && self.channel_page == other.channel_page
            && self.super_frame_spec == other.super_frame_spec
            && self.gts_permit == other.gts_permit
            && self.link_quality == other.link_quality
            && self.timestamp == other.timestamp
            && self.security_info == other.security_info
            && code_list_equal
            && match (self.security_status, other.security_status) {
                (None, None) => true,
                (None, Some(_)) => false,
                (Some(_), None) => false,
                (Some(l), Some(r)) => core::mem::discriminant(&l) == core::mem::discriminant(&r),
            }
    }
}

#[allow(private_bounds)]
pub trait Request: From<RequestValue> + Into<RequestValue> {
    type Confirm: From<ConfirmValue> + Into<ConfirmValue>;
}

pub(crate) enum RequestValue {
    Associate(AssociateRequest),
    Disassociate(DisassociateRequest),
    Get(GetRequest),
    Gts(GtsRequest),
    Reset(ResetRequest),
    RxEnable(RxEnableRequest),
    Scan(ScanRequest),
    Set(SetRequest),
    Start(StartRequest),
    Sync(SyncRequest),
    Poll(PollRequest),
    Dps(DpsRequest),
    Sounding(SoundingRequest),
    Calibrate(CalibrateRequest),
    Data(DataRequest),
    Purge(PurgeRequest),
}

impl From<PurgeRequest> for RequestValue {
    fn from(v: PurgeRequest) -> Self {
        Self::Purge(v)
    }
}

impl From<DataRequest> for RequestValue {
    fn from(v: DataRequest) -> Self {
        Self::Data(v)
    }
}

impl From<CalibrateRequest> for RequestValue {
    fn from(v: CalibrateRequest) -> Self {
        Self::Calibrate(v)
    }
}

impl From<SoundingRequest> for RequestValue {
    fn from(v: SoundingRequest) -> Self {
        Self::Sounding(v)
    }
}

impl From<DpsRequest> for RequestValue {
    fn from(v: DpsRequest) -> Self {
        Self::Dps(v)
    }
}

impl From<PollRequest> for RequestValue {
    fn from(v: PollRequest) -> Self {
        Self::Poll(v)
    }
}

impl From<SyncRequest> for RequestValue {
    fn from(v: SyncRequest) -> Self {
        Self::Sync(v)
    }
}

impl From<StartRequest> for RequestValue {
    fn from(v: StartRequest) -> Self {
        Self::Start(v)
    }
}

impl From<SetRequest> for RequestValue {
    fn from(v: SetRequest) -> Self {
        Self::Set(v)
    }
}

impl From<ScanRequest> for RequestValue {
    fn from(v: ScanRequest) -> Self {
        Self::Scan(v)
    }
}

impl From<RxEnableRequest> for RequestValue {
    fn from(v: RxEnableRequest) -> Self {
        Self::RxEnable(v)
    }
}

impl From<ResetRequest> for RequestValue {
    fn from(v: ResetRequest) -> Self {
        Self::Reset(v)
    }
}

impl From<GtsRequest> for RequestValue {
    fn from(v: GtsRequest) -> Self {
        Self::Gts(v)
    }
}

impl From<GetRequest> for RequestValue {
    fn from(v: GetRequest) -> Self {
        Self::Get(v)
    }
}

impl From<DisassociateRequest> for RequestValue {
    fn from(v: DisassociateRequest) -> Self {
        Self::Disassociate(v)
    }
}

impl From<AssociateRequest> for RequestValue {
    fn from(v: AssociateRequest) -> Self {
        Self::Associate(v)
    }
}

pub(crate) enum ConfirmValue {
    Associate(AssociateConfirm),
    Disassociate(DisassociateConfirm),
    Get(GetConfirm),
    Gts(GtsConfirm),
    Reset(ResetConfirm),
    RxEnable(RxEnableConfirm),
    Scan(ScanConfirm),
    Set(SetConfirm),
    Start(StartConfirm),
    Poll(PollConfirm),
    Dps(DpsConfirm),
    Sounding(SoundingConfirm),
    Calibrate(CalibrateConfirm),
    Data(DataConfirm),
    Purge(PurgeConfirm),
    None,
}

impl From<ConfirmValue> for () {
    fn from(v: ConfirmValue) -> Self {
        match v {
            ConfirmValue::None => (),
            _ => panic!("Bad cast"),
        }
    }
}

impl From<()> for ConfirmValue {
    fn from(_: ()) -> Self {
        Self::None
    }
}

impl From<PurgeConfirm> for ConfirmValue {
    fn from(v: PurgeConfirm) -> Self {
        Self::Purge(v)
    }
}

impl From<DataConfirm> for ConfirmValue {
    fn from(v: DataConfirm) -> Self {
        Self::Data(v)
    }
}

impl From<CalibrateConfirm> for ConfirmValue {
    fn from(v: CalibrateConfirm) -> Self {
        Self::Calibrate(v)
    }
}

impl From<SoundingConfirm> for ConfirmValue {
    fn from(v: SoundingConfirm) -> Self {
        Self::Sounding(v)
    }
}

impl From<DpsConfirm> for ConfirmValue {
    fn from(v: DpsConfirm) -> Self {
        Self::Dps(v)
    }
}

impl From<PollConfirm> for ConfirmValue {
    fn from(v: PollConfirm) -> Self {
        Self::Poll(v)
    }
}

impl From<StartConfirm> for ConfirmValue {
    fn from(v: StartConfirm) -> Self {
        Self::Start(v)
    }
}

impl From<SetConfirm> for ConfirmValue {
    fn from(v: SetConfirm) -> Self {
        Self::Set(v)
    }
}

impl From<ScanConfirm> for ConfirmValue {
    fn from(v: ScanConfirm) -> Self {
        Self::Scan(v)
    }
}

impl From<RxEnableConfirm> for ConfirmValue {
    fn from(v: RxEnableConfirm) -> Self {
        Self::RxEnable(v)
    }
}

impl From<ResetConfirm> for ConfirmValue {
    fn from(v: ResetConfirm) -> Self {
        Self::Reset(v)
    }
}

impl From<GtsConfirm> for ConfirmValue {
    fn from(v: GtsConfirm) -> Self {
        Self::Gts(v)
    }
}

impl From<GetConfirm> for ConfirmValue {
    fn from(v: GetConfirm) -> Self {
        Self::Get(v)
    }
}

impl From<DisassociateConfirm> for ConfirmValue {
    fn from(v: DisassociateConfirm) -> Self {
        Self::Disassociate(v)
    }
}

impl From<AssociateConfirm> for ConfirmValue {
    fn from(v: AssociateConfirm) -> Self {
        Self::Associate(v)
    }
}

#[allow(private_bounds)]
pub trait Indication: From<IndicationValue> + Into<IndicationValue> {
    type Response: From<ResponseValue> + Into<ResponseValue>;
}

pub enum IndicationValue {
    Associate(AssociateIndication),
    Disassociate(DisassociateIndication),
    BeaconNotify(BeaconNotifyIndication),
    CommStatus(CommStatusIndication),
    Gts(GtsIndication),
    Orphan(OrphanIndication),
    SyncLoss(SyncLossIndication),
    Dps(DpsIndication),
    Data(DataIndication),
}

impl From<CommStatusIndication> for IndicationValue {
    fn from(v: CommStatusIndication) -> Self {
        Self::CommStatus(v)
    }
}

impl From<DataIndication> for IndicationValue {
    fn from(v: DataIndication) -> Self {
        Self::Data(v)
    }
}

impl From<DpsIndication> for IndicationValue {
    fn from(v: DpsIndication) -> Self {
        Self::Dps(v)
    }
}

impl From<SyncLossIndication> for IndicationValue {
    fn from(v: SyncLossIndication) -> Self {
        Self::SyncLoss(v)
    }
}

impl From<OrphanIndication> for IndicationValue {
    fn from(v: OrphanIndication) -> Self {
        Self::Orphan(v)
    }
}

impl From<GtsIndication> for IndicationValue {
    fn from(v: GtsIndication) -> Self {
        Self::Gts(v)
    }
}

impl From<BeaconNotifyIndication> for IndicationValue {
    fn from(v: BeaconNotifyIndication) -> Self {
        Self::BeaconNotify(v)
    }
}

impl From<DisassociateIndication> for IndicationValue {
    fn from(v: DisassociateIndication) -> Self {
        Self::Disassociate(v)
    }
}

impl From<AssociateIndication> for IndicationValue {
    fn from(v: AssociateIndication) -> Self {
        Self::Associate(v)
    }
}

pub(crate) enum ResponseValue {
    Associate(AssociateResponse),
    Orphan(OrphanResponse),
    None,
}

impl From<OrphanResponse> for ResponseValue {
    fn from(v: OrphanResponse) -> Self {
        Self::Orphan(v)
    }
}

impl From<AssociateResponse> for ResponseValue {
    fn from(v: AssociateResponse) -> Self {
        Self::Associate(v)
    }
}

impl From<ResponseValue> for () {
    fn from(v: ResponseValue) -> Self {
        match v {
            ResponseValue::None => (),
            _ => panic!("Bad cast"),
        }
    }
}

impl From<()> for ResponseValue {
    fn from(_: ()) -> Self {
        Self::None
    }
}
