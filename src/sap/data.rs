use arrayvec::ArrayVec;

use super::{
    ConfirmValue, Indication, IndicationValue, Request, RequestValue, SecurityInfo, Status,
};
use crate::{
    time::{Duration, Instant},
    wire::{AddressMode, PanId},
    DeviceAddress,
};

/// The MCPS-DATA.request primitive requests the transfer of data to another device.
///
/// On receipt of the MCPS-DATA.request primitive, the MAC sublayer entity begins the transmission of the
/// supplied MSDU.
///
/// If the msduLength parameter is greater than aMaxMACSafePayloadSize, the MAC sublayer will set the
/// Frame Version field to one.
///
/// The TxOptions parameter indicates the method used by the MAC sublayer data service to transmit the
/// supplied MSDU. If the TxOptions parameter specifies that an acknowledged transmission is required, the
/// AR field will be set appropriately, as described in 5.1.6.4.
///
/// If the TxOptions parameter specifies that a GTS transmission is required, the MAC sublayer will determine
/// whether it has a valid GTS as described 5.1.7.3. If a valid GTS could not be found, the MAC sublayer will
/// discard the MSDU. If a valid GTS was found, the MAC sublayer will defer, if necessary, until the GTS. If
/// the TxOptions parameter specifies that a GTS transmission is not required, the MAC sublayer will transmit
/// the MSDU using either slotted CSMA-CA in the CAP for a beacon-enabled PAN or unslotted CSMA-CA
/// for a nonbeacon-enabled PAN. Specifying a GTS transmission in the TxOptions parameter overrides an
/// indirect transmission request.
///
/// If the TxOptions parameter specifies that an indirect transmission is required and this primitive is received
/// by the MAC sublayer of a coordinator, the data frame is sent using indirect transmission, as described in
/// 5.1.5 and 5.1.6.3.
///
/// If the TxOptions parameter specifies that an indirect transmission is required and if the device receiving this
/// primitive is not a coordinator, the destination address is not present, or the TxOptions parameter also
/// specifies a GTS transmission, the indirect transmission option will be ignored.
///
/// If the TxOptions parameter specifies that an indirect transmission is not required, the MAC sublayer will
/// transmit the MSDU using CSMA-CA either in the CAP for a beacon-enabled PAN or immediately for a
/// nonbeacon-enabled PAN.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataRequest {
    /// The source addressing mode for this MPDU.
    pub src_addr_mode: AddressMode,
    /// The PAN identifier of the entity to which the MSDU is being transferred.
    pub dst_pan_id: PanId,
    /// The individual device address of the entity to which the MSDU is being transferred.
    pub dst_addr: Option<DeviceAddress>,
    /// The set of octets forming the MSDU to be transmitted by the MAC sublayer entity.
    pub msdu: ArrayVec<u8, { crate::consts::MAX_MAC_PAYLOAD_SIZE }>,
    /// The handle associated with the MSDU to be transmitted by the MAC sublayer entity.
    pub msdu_handle: u8,
    /// TRUE if acknowledged transmission is used, FALSE otherwise.
    pub ack_tx: bool,
    /// TRUE if a GTS is to be used for transmission. FALSE indicates that the CAP will be used.
    pub gtstx: bool,
    /// TRUE if indirect transmission is to be used, FALSE otherwise.
    pub indirect_tx: bool,
    pub security_info: SecurityInfo,
    pub uwbprf: UwbPrf,
    /// A value of NON_RANGING indicates that ranging
    /// is not to be used. A value of ALL_RANGING
    /// indicates that ranging operations using both the
    /// ranging bit in the PHR and the counter operation are
    /// enabled. A value of PHY_HEADER_ONLY
    /// indicates that only the ranging bit in the PHR will be
    /// used. A value of NON_RANGING is PHYs that do
    /// not support ranging.
    pub ranging: Ranging,
    /// The preamble symbol repetitions of the UWB PHY
    /// frame. A zero value is used for non-UWB PHYs.
    pub uwb_preamble_symbol_repetitions: UwbPreambleSymbolRepetitions,
    /// Indicates the data rate. For CSS PHYs, a value of
    /// one indicates 250 kb/s while a value of two
    /// indicates 1 Mb/s. For UWB PHYs, values 1–4 are
    /// valid and are defined in 14.2.6.1. For all other
    /// PHYs, the parameter is set to zero.
    pub data_rate: u8,
}

impl From<RequestValue> for DataRequest {
    fn from(value: RequestValue) -> Self {
        match value {
            RequestValue::Data(val) => val,
            _ => panic!("Bad cast"),
        }
    }
}

impl Request for DataRequest {
    type Confirm = DataConfirm;
}

/// The MCPS-DATA.confirm primitive reports the results of a request to transfer data to another device.
///
/// The MCPS-DATA.confirm primitive is generated by the MAC sublayer entity in response to an MCPS-
/// DATA.request primitive. The MCPS-DATA.confirm primitive returns a status of either SUCCESS,
/// indicating that the request to transmit was successful, or the appropriate error code.
///
/// If both the SrcAddrMode and the DstAddrMode parameters are set to NO_ADDRESS in the MCPS-
/// DATA.request primitive, the status shall be set to INVALID_ADDRESS.
///
/// If a valid GTS could not be found, the status shall be set to INVALID_GTS.
///
/// If there is no capacity to store the transaction, the status will be set to TRANSACTION_OVERFLOW. If the
/// transaction is not handled within the required time, the transaction information will be discarded and the
/// status will be set to TRANSACTION_EXPIRED.
///
/// If the TxOptions parameter specifies that a direct transmission is required and the MAC sublayer does not
/// receive an acknowledgment from the recipient after macMaxFrameRetries retransmissions, as described in
/// 5.1.6.4, it will discard the MSDU and issue the MCPS-DATA.confirm primitive with a status of NO_ACK.
///
/// If the requested transaction is too large to fit in the CAP or GTS, as appropriate, the MAC sublayer shall
/// discard the frame and issue the MCPS-DATA.confirm primitive with a status of FRAME_TOO_LONG.
///
/// If the transmission uses CSMA-CA and the CSMA-CA algorithm failed due to adverse conditions on the
/// channel, and the TxOptions parameter specifies that a direct transmission is required, the MAC
/// sublayer will discard the MSDU and the status will be set to CHANNEL_ACCESS_FAILURE.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataConfirm {
    /// The handle associated with the MSDU being confirmed.
    pub msdu_handle: u8,
    /// Optional. The time, in symbols, at which
    /// the data were transmitted, as described in
    /// 5.1.4.1. The value of this parameter will
    /// be considered valid only if the value of
    /// the status parameter is SUCCESS; if the
    /// status parameter is not equal to SUCCESS, the value of the Timestamp
    /// parameter shall not be used for any other
    /// purpose. The symbol boundary is
    /// described by macSyncSymbolOffset, as
    /// described in Table 52. The precision of
    /// this value shall be a minimum of 20 bits,
    /// with the lowest 4 bits being the least
    /// significant.
    pub timestamp: Instant,
    /// A value of FALSE indicates that ranging
    /// is either not supported by the PHY or that
    /// it was not indicated by the received
    /// PSDU. A value of TRUE indicates ranging operations were indicated for this
    /// PSDU.
    pub ranging_received: bool,
    /// A count of the time units corresponding
    /// to an RMARKER at the antenna at the
    /// beginning of a ranging exchange, as
    /// described in 14.7.1. A value of
    /// 0x00000000 is used if ranging is not supported, not enabled or if counter was not
    /// used for this PPDU.
    pub ranging_counter_start: Instant,
    /// A count of the time units corresponding
    /// to an RMARKER at the antenna at the
    /// end of a ranging exchange, as described
    /// in 14.7.1. A value of 0x00000000 is used
    /// if ranging is not supported, not enabled,
    /// or if the counter is not used for this
    /// PPDU.
    pub ranging_counter_stop: Instant,
    /// A count of the time units in a message
    /// exchange over which the tracking offset
    /// was measured, as described in 14.7.2.2. If
    /// tracking-based crystal characterization is
    /// not supported or if ranging is not supported, a value of 0x00000000 is used.
    pub ranging_tracking_interval: Duration,
    /// A count of the time units slipped or
    /// advanced by the radio tracking system
    /// over the course of the entire tracking
    /// interval, as described in 14.7.2.1. The top
    /// 4 bits are reserved and set to zero. The
    /// most significant of the active bits is the
    /// sign bit.
    pub ranging_offset: Duration,
    /// The FoM characterizing the ranging measurement, as described in 14.7.3. The
    /// most significant bit (MSB) is reserved
    /// and is zero. The remaining 7 bits are used
    /// in three fields: Confidence Level, Confidence Interval, and Confidence Interval
    /// Scaling Factor.
    pub ranging_fom: u8,
    pub status: Status,
}

impl From<ConfirmValue> for DataConfirm {
    fn from(value: ConfirmValue) -> Self {
        match value {
            ConfirmValue::Data(val) => val,
            _ => panic!("Bad cast"),
        }
    }
}

/// The MCPS-DATA.indication primitive indicates the reception of data from another device.
///
/// The MCPS-DATA.indication primitive is generated by the MAC sublayer and issued to the next higher
/// layer on receipt of a data frame at the local MAC sublayer entity that passes the appropriate message
/// filtering operations as described in 5.1.6.2. If the primitive is received while the device is in promiscuous
/// mode, the parameters will be set as specified in 5.1.6.5.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataIndication {
    /// The PAN identifier of the entity from which the MSDU was received.
    pub src_pan_id: PanId,
    /// The individual device address of the entity from which the MSDU was received.
    pub src_addr: Option<DeviceAddress>,
    /// The PAN identifier of the entity to which the MSDU is being transferred.
    pub dst_pan_id: PanId,
    /// The individual device address of the entity to which the MSDU is being transferred.
    pub dst_addr: Option<DeviceAddress>,
    /// The set of octets forming the MSDU being indicated by the MAC sublayer entity.
    pub msdu: ArrayVec<u8, { crate::consts::MAX_MAC_PAYLOAD_SIZE }>,
    /// LQI value measured during reception of the MPDU.
    /// Lower values represent lower LQI, as described in 8.2.6.
    pub mpdu_link_quality: u8,
    /// The DSN of the received data frame.
    pub dsn: u8,
    /// Optional. The time, in symbols, at which the data
    /// were received, as described in 5.1.4.1. The symbol
    /// boundary is described by macSyncSymbolOffset, as
    /// described in Table 52. The precision of this value
    /// shall be a minimum of 20 bits, with the lowest 4 bits
    /// being the least significant.
    pub timestamp: Instant,
    /// The security info purportedly used by the received data frame
    pub security_info: SecurityInfo,
    /// The pulse repetition value of the received PPDU.
    /// This parameter shall be ignored by non-UWB PHYs.
    pub uwbprf: UwbPrf,
    pub uwb_preamble_symbol_repetitions: UwbPreambleSymbolRepetitions,
    pub data_rate: u8,
    /// A value of RANGING_REQUESTED_BUT_NOT_SUPPORTED indicates that ranging is
    /// not supported but has been requested. A value of
    /// NO_RANGING_REQUESTED indicates that no
    /// ranging is requested for the PSDU received. A value
    /// of RANGING_ACTIVE denotes ranging operations
    /// requested for this PSDU. A value of
    /// NO_RANGING_REQUESTED is used for PHYs
    /// that do not support ranging.
    pub ranging_received: ReceivedRanging,
    pub ranging_counter_start: Instant,
    pub ranging_counter_stop: Instant,
    pub ranging_tracking_interval: Duration,
    pub ranging_offset: Duration,
    pub ranging_fom: u8,
}

impl From<IndicationValue> for DataIndication {
    fn from(value: IndicationValue) -> Self {
        match value {
            IndicationValue::Data(val) => val,
            _ => panic!("Bad cast"),
        }
    }
}

impl Indication for DataIndication {
    type Response = ();
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum UwbPrf {
    Off,
    Nominal4M,
    Nominal16M,
    Nominal64M,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Ranging {
    NonRanging,
    AllRanging,
    PhyHeaderOnly,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ReceivedRanging {
    NoRangingRequested,
    RangingActive,
    RangingRequestedButNotSupported,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum UwbPreambleSymbolRepetitions {
    Reps0,
    Reps16,
    Reps64,
    Reps1024,
    Reps4096,
}
