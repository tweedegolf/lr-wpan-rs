use arrayvec::ArrayVec;

use super::{ConfirmValue, PanDescriptor, Request, RequestValue, SecurityInfo, Status};
use crate::ChannelPage;

/// The MLME-SCAN.request primitive is used to initiate a channel scan over a given list of channels
///
/// When the MLME receives this primitive, it begins the appropriate scan procedure, as defined in 5.1.2.
///
/// The security info parameters are used only in an orphan scan
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanRequest {
    pub scan_type: ScanType,
    pub scan_channels: ArrayVec<u8, 16>,
    /// A value used to calculate the length of time to
    /// spend scanning each channel for ED, active,
    /// and passive scans. This parameter is ignored for
    /// orphan scans. The time spent scanning each
    /// channel is [aBaseSuperframeDuration × (2^n +
    /// 1)], where n is the value of the ScanDuration
    /// parameter.
    ///
    /// ## Range
    ///
    /// 0-14
    pub scan_duration: u8,
    pub channel_page: ChannelPage,
    pub security_info: SecurityInfo,
}

impl From<RequestValue> for ScanRequest {
    fn from(value: RequestValue) -> Self {
        match value {
            RequestValue::Scan(val) => val,
            _ => panic!("Bad cast"),
        }
    }
}

impl Request for ScanRequest {
    type Confirm = ScanConfirm;
}

/// The MLME-SCAN.confirm primitive reports the result of the channel scan request.
///
/// If the requested scan was successful, the status parameter will be set to SUCCESS.
/// If the MLME receives the MLME-SCAN.request primitive while performing a previously initiated scan
/// operation, the MLME will not perform the scan and the status parameter will be set to
/// SCAN_IN_PROGRESS.
///
/// If, during an active scan, the MLME is unable to transmit a beacon request command on a channel specified
/// by the ScanChannels parameter due to a channel access failure, the channel will appear in the list of
/// unscanned channels returned by the MLME-SCAN.confirm primitive. If the MLME was able to send a
/// beacon request command on at least one of the channels but no beacons were found, the MLME-
/// SCAN.confirm primitive will contain a null set of PAN descriptor values, regardless of the value of
/// macAutoRequest, and a status of NO_BEACON.
///
/// If the MLME-SCAN.request primitive requested an orphan scan, the ResultListSize parameter will be set to
/// zero. If the MLME is unable to transmit an orphan notification command on a channel specified by the
/// ScanChannels parameter due to a channel access failure, the channel will appear in the list of unscanned
/// channels returned by the MLME-SCAN.confirm primitive. If the MLME was able to send an orphan
/// notification command on at least one of the channels but the device did not receive a coordinator
/// realignment command, the MLME-SCAN.confirm primitive will contain a status of NO_BEACON.
///
/// If the MLME-SCAN.request primitive requested an active, passive, or orphan scan, the EnergyDetectList
/// and UWBEnergyDetectList parameters will be null. If the MLME-SCAN.request primitive requested an ED
/// or orphan scan, the PANDescriptorList parameter will be null.
///
/// If, during an ED, active, or passive scan, the implementation-specified maximum of PAN descriptors is
/// reached thus terminating the scan procedure, the MAC sublayer will issue the MLME-SCAN.confirm
/// primitive with a status of LIMIT_REACHED.
///
/// If the MLME-SCAN.request primitive requested an ED and the PHY type is UWB, as indicated by the
/// phyChannelPage, then the UWBEnergyDetectList contains the results for the UWB channels scanned, and
/// the EnergyDetectList and PANDescriptorList are null. The UWB scan is fully described in 5.1.2.1.
#[derive(Debug, Clone, Default)]
pub struct ScanConfirm {
    pub status: Status,
    pub scan_type: ScanType,
    /// The channel page on which the scan
    /// was performed, as defined in 8.1.2.
    pub channel_page: ChannelPage,
    /// A list of the channels given in the
    /// request which were not scanned. This
    /// parameter is not valid for ED scans.
    pub unscanned_channels: ArrayVec<u8, 16>,
    /// The number of elements returned in
    /// the appropriate result lists. This value
    /// is zero for the result of an orphan scan.
    pub result_list_size: u8,
    /// The list of energy measurements, one
    /// for each channel searched during an
    /// ED scan. This parameter is null for
    /// active, passive, and orphan scans.
    pub energy_detect_list: ArrayVec<u8, 16>,
    /// The list of PAN descriptors, one for
    /// each beacon found during an active or
    /// passive scan if macAutoRequest is set
    /// to TRUE. This parameter is null for
    /// ED and orphan scans or when macAutoRequest is set to FALSE during an
    /// active or passive scan.
    pub pan_descriptor_list: alloc::boxed::Box<ArrayVec<PanDescriptor, 16>>,
    /// Categorization of energy detected in
    /// channel with the following values:
    /// - 0: Category detection is not supported
    /// - 1: UWB PHY detected
    /// - 2: Non-UWB PHY signal source detected
    /// - 3–25: Reserved for future use
    pub detected_category: u8,
    /// For UWB PHYs, the list of energy
    /// measurements taken. The total number
    /// of measurements is indicated by
    /// ResultListSize. This parameter is null
    /// for active, passive, and orphan scans. It
    /// is also null for non-UWB PHYs.
    pub uwb_energy_detect_list: ArrayVec<u8, 16>,
}

impl From<ConfirmValue> for ScanConfirm {
    fn from(value: ConfirmValue) -> Self {
        match value {
            ConfirmValue::Scan(val) => val,
            _ => panic!("Bad cast"),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub enum ScanType {
    Ed,
    #[default]
    Active,
    Passive,
    Orphan,
}
