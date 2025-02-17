use core::num::{NonZero, NonZeroU32};

use crate::{
    consts::{MAX_BEACON_PAYLOAD_LENGTH, TURNAROUND_TIME, UNIT_BACKOFF_PERIOD},
    sap::Status,
    wire::{
        beacon::{BeaconOrder, SuperframeOrder},
        ExtendedAddress, PanId, ShortAddress,
    },
    ChannelPage,
};

#[derive(Debug, Clone)]
pub struct PhyPib {
    pub pib_write: PhyPibWrite,
    /// Read only
    ///
    /// Each entry in the list consists of a channel page
    /// and a list of channel numbers supported for that
    /// channel page.
    #[doc(alias = "phyChannelsSupported")]
    pub channels_supported: &'static [ChannelDescription],
    /// Read only
    ///
    /// The maximum number of symbols in a frame, as defined in 9.4.
    #[doc(alias = "phyMaxFrameDuration")]
    pub max_frame_duration: u32,
    /// Read only
    ///
    /// The duration of the synchronization header (SHR) in symbols for the current PHY.
    #[doc(alias = "phySHRDuration")]
    pub shr_duration: u32,
    /// Read only
    ///
    /// The number of symbols per octet for the current
    /// PHY. For the UWB PHY this is defined in 14.2.3.
    /// For the CSS PHY, 1.3 corresponds to 1 Mb/s
    /// while 5.3 corresponds to 250 kb/s.
    #[doc(alias = "phySymbolsPerOctet")]
    pub symbols_per_octet: f32,
    /// Read only
    ///
    /// Zero indicates preamble symbol length is 31, and
    /// one indicates that length 127 symbol is used.
    /// Present for UWB PHY.
    #[doc(alias = "phyPreambleSymbolLength")]
    pub preamble_symbol_length: u32,
    /// Read only
    ///
    /// A list of the data rates available in the operating
    /// channel as defined in Table 105.
    #[doc(alias = "phyUWBDataRatesSupported")]
    pub uwb_data_rates_supported: &'static [u8],
    /// Read only
    ///
    /// A value of TRUE indicates that 250 kb/s is supported. Present for CSS PHY.
    #[doc(alias = "phyCSSLowDataRateSupported")]
    pub css_low_data_rate_supported: bool,
    /// Read only
    ///
    /// TRUE if CoU pulses are supported, FALSE otherwise.
    #[doc(alias = "phyUWBCoUSupported")]
    pub uwb_cou_supported: bool,
    /// Read only
    ///
    /// TRUE if CS pulses are supported, FALSE otherwise.
    #[doc(alias = "phyUWBCSSupported")]
    pub uwb_cs_supported: bool,
    /// Read only
    ///
    /// TRUE if LCP pulses are supported, FALSE otherwise.
    #[doc(alias = "phyUWBLCPSupported")]
    pub uwb_lcp_supported: bool,
    /// Read only
    ///
    /// TRUE if ranging is supported, FALSE otherwise.
    #[doc(alias = "phyRanging")]
    pub ranging: bool,
    /// Read only
    ///
    /// TRUE if crystal offset characterization is supported, FALSE otherwise.
    #[doc(alias = "phyRangingCrystalOffset")]
    pub ranging_crystal_offset: bool,
    /// Read only
    ///
    /// TRUE if DPS is supported, FALSE otherwise.
    #[doc(alias = "phyRangingDPS")]
    pub ranging_dps: bool,
}

impl PhyPib {
    /// A pib containing reasonable dummy values
    pub fn unspecified_new() -> Self {
        #[allow(unused_imports)]
        use micromath::F32Ext;

        const NUM_PREAMBLE_SYMBOLS: u32 = 31;
        const NUM_SFD_SYMBOLS: u32 = 8;
        const SYMBOLS_PER_OCTET: f32 = 9.17648;
        const SHR_DURATION: u32 = NUM_PREAMBLE_SYMBOLS + NUM_SFD_SYMBOLS;
        let max_frame_duration = SHR_DURATION
            + (((crate::consts::MAX_PHY_PACKET_SIZE + 1) as f32 * SYMBOLS_PER_OCTET).ceil() as u32);

        Self {
            pib_write: PhyPibWrite {
                current_channel: 5,
                tx_power_tolerance: TXPowerTolerance::DB6,
                tx_power: 0,
                cca_mode: CcaMode::Aloha,
                current_page: ChannelPage::Uwb,
                uwb_current_pulse_shape: UwbCurrentPulseShape::Mandatory,
                uwb_cou_pulse: crate::pib::UwbCouPulse::CCh1,
                uwb_cs_pulse: crate::pib::UwbCsPulse::No1,
                uwb_lcp_weight1: 0,
                uwb_lcp_weight2: 0,
                uwb_lcp_weight3: 0,
                uwb_lcp_weight4: 0,
                uwb_lcp_delay2: 0,
                uwb_lcp_delay3: 0,
                uwb_lcp_delay4: 0,
                current_code: 0,
                native_prf: NativePrf::Prf16,
                uwb_scan_bins_per_channel: 0,
                uwb_inserted_preamble_interval: 0,
                tx_rmarker_offset: 0,
                rx_rmarker_offset: 0,
                rframe_processing_time: 0,
                cca_duration: 0,
            },
            channels_supported: &[ChannelDescription {
                page: ChannelPage::Uwb,
                channel_numbers: &[1, 2, 3, 4, 5, 7],
            }],
            max_frame_duration,
            shr_duration: SHR_DURATION,
            symbols_per_octet: SYMBOLS_PER_OCTET,
            preamble_symbol_length: 0,
            uwb_data_rates_supported: &[0b00, 0b01, 0b10],
            css_low_data_rate_supported: false,
            uwb_cou_supported: false,
            uwb_cs_supported: false,
            uwb_lcp_supported: false,
            ranging: true,
            ranging_crystal_offset: false,
            ranging_dps: true,
        }
    }

    #[rustfmt::skip]
    pub fn get(&self, attribute: &str) -> Option<PibValue> {
        if !attribute.starts_with("phy") {
            return None;
        }

        match attribute {
            PibValue::PHY_CHANNELS_SUPPORTED => Some(PibValue::PhyChannelsSupported(self.channels_supported)),
            PibValue::PHY_MAX_FRAME_DURATION => Some(PibValue::PhyMaxFrameDuration(self.max_frame_duration)),
            PibValue::PHY_SHR_DURATION => Some(PibValue::PhyShrDuration(self.shr_duration)),
            PibValue::PHY_SYMBOLS_PER_OCTET => Some(PibValue::PhySymbolsPerOctet(self.symbols_per_octet)),
            PibValue::PHY_PREAMBLE_SYMBOL_LENGTH => Some(PibValue::PhyPreambleSymbolLength(self.preamble_symbol_length)),
            PibValue::PHY_UWB_DATA_RATES_SUPPORTED => Some(PibValue::PhyUwbDataRatesSupported(self.uwb_data_rates_supported)),
            PibValue::PHY_CSS_LOW_DATA_RATE_SUPPORTED => Some(PibValue::PhyCssLowDataRateSupported(self.css_low_data_rate_supported)),
            PibValue::PHY_UWB_COU_SUPPORTED => Some(PibValue::PhyUwbCouSupported(self.uwb_cou_supported)),
            PibValue::PHY_UWB_CS_SUPPORTED => Some(PibValue::PhyUwbCsSupported(self.uwb_cs_supported)),
            PibValue::PHY_UWB_LCP_SUPPORTED => Some(PibValue::PhyUwbLcpSupported(self.uwb_lcp_supported)),
            PibValue::PHY_RANGING => Some(PibValue::PhyRanging(self.ranging)),
            PibValue::PHY_RANGING_CRYSTAL_OFFSET => Some(PibValue::PhyRangingCrystalOffset(self.ranging_crystal_offset)),
            PibValue::PHY_RANGING_DPS => Some(PibValue::PhyRangingDps(self.ranging_dps)),
            PibValue::PHY_CURRENT_CHANNEL => Some(PibValue::PhyCurrentChannel(self.current_channel)),
            PibValue::PHY_TX_POWER_TOLERANCE => Some(PibValue::PhyTxPowerTolerance(self.tx_power_tolerance)),
            PibValue::PHY_TX_POWER => Some(PibValue::PhyTxPower(self.tx_power)),
            PibValue::PHY_CCA_MODE => Some(PibValue::PhyCcaMode(self.cca_mode)),
            PibValue::PHY_CURRENT_PAGE => Some(PibValue::PhyCurrentPage(self.current_page)),
            PibValue::PHY_UWB_CURRENT_PULSE_SHAPE => Some(PibValue::PhyUwbCurrentPulseShape(self.uwb_current_pulse_shape)),
            PibValue::PHY_UWB_COU_PULSE => Some(PibValue::PhyUwbCouPulse(self.uwb_cou_pulse)),
            PibValue::PHY_UWB_CS_PULSE => Some(PibValue::PhyUwbCsPulse(self.uwb_cs_pulse)),
            PibValue::PHY_UWB_LCP_WEIGHT1 => Some(PibValue::PhyUwbLcpWeight1(self.uwb_lcp_weight1)),
            PibValue::PHY_UWB_LCP_WEIGHT2 => Some(PibValue::PhyUwbLcpWeight2(self.uwb_lcp_weight2)),
            PibValue::PHY_UWB_LCP_WEIGHT3 => Some(PibValue::PhyUwbLcpWeight3(self.uwb_lcp_weight3)),
            PibValue::PHY_UWB_LCP_WEIGHT4 => Some(PibValue::PhyUwbLcpWeight4(self.uwb_lcp_weight4)),
            PibValue::PHY_UWB_LCP_DELAY2 => Some(PibValue::PhyUwbLcpDelay2(self.uwb_lcp_delay2)),
            PibValue::PHY_UWB_LCP_DELAY3 => Some(PibValue::PhyUwbLcpDelay3(self.uwb_lcp_delay3)),
            PibValue::PHY_UWB_LCP_DELAY4 => Some(PibValue::PhyUwbLcpDelay4(self.uwb_lcp_delay4)),
            PibValue::PHY_CURRENT_CODE => Some(PibValue::PhyCurrentCode(self.current_code)),
            PibValue::PHY_NATIVE_PRF => Some(PibValue::PhyNativePrf(self.native_prf)),
            PibValue::PHY_UWB_SCAN_BINS_PER_CHANNEL => Some(PibValue::PhyUwbScanBinsPerChannel(self.uwb_scan_bins_per_channel)),
            PibValue::PHY_UWB_INSERTED_PREAMBLE_INTERVAL => Some(PibValue::PhyUwbInsertedPreambleInterval(self.uwb_inserted_preamble_interval)),
            PibValue::PHY_TX_RMARKER_OFFSET => Some(PibValue::PhyTxRmarkerOffset(self.tx_rmarker_offset)),
            PibValue::PHY_RX_RMARKER_OFFSET => Some(PibValue::PhyRxRmarkerOffset(self.rx_rmarker_offset)),
            PibValue::PHY_RFRAME_PROCESSING_TIME => Some(PibValue::PhyRframeProcessingTime(self.rframe_processing_time)),
            PibValue::PHY_CCA_DURATION => Some(PibValue::PhyCcaDuration(self.cca_duration)),
            _ => None,
        }
    }
}

impl core::ops::DerefMut for PhyPib {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.pib_write
    }
}

impl core::ops::Deref for PhyPib {
    type Target = PhyPibWrite;

    fn deref(&self) -> &Self::Target {
        &self.pib_write
    }
}

#[derive(Debug, Clone)]
pub struct PhyPibWrite {
    /// The RF channel to use for all following transmissions and receptions, 8.1.2.
    #[doc(alias = "phyCurrentChannel")]
    pub current_channel: u8,
    /// The tolerance on the transmit power setting, plus
    /// or minus the indicated value.
    #[doc(alias = "phyTXPowerTolerance")]
    pub tx_power_tolerance: TXPowerTolerance,
    /// The transmit power of the device in dBm.
    #[doc(alias = "phyTXPower")]
    pub tx_power: i16,
    /// The CCA mode, as defined in 8.2.7.
    #[doc(alias = "phyCCAMode")]
    pub cca_mode: CcaMode,
    /// This is the current PHY channel page. This is
    /// used in conjunction with phyCurrentChannel to
    /// uniquely identify the channel currently being
    /// used.
    #[doc(alias = "phyCurrentPage")]
    pub current_page: ChannelPage,
    /// Indicates the current pulse shape setting of the
    /// UWB PHY. The mandatory pulse is described in
    /// 14.4.5. Optional pulse shapes include CoU, as
    /// defined in 14.5.1, CS, as defined in 14.5.2, and
    /// LCP, as defined in 14.5.3.
    #[doc(alias = "phyUWBCurrentPulseShape")]
    pub uwb_current_pulse_shape: UwbCurrentPulseShape,
    /// Defines the slope of the frequency chirp and
    /// bandwidth of pulse. CCh.3–CCh.6 are valid only
    /// for wideband UWB channels, e.g., 4, 7, 11, or 15,
    /// as defined in 14.5.1.
    #[doc(alias = "phyUWBCoUpulse")]
    pub uwb_cou_pulse: UwbCouPulse,
    /// Defines the group delay of the continuous spectrum filter.
    /// No.3–No.6 are valid only for wideband UWB channels, e.g., 4, 7, 11, or 15, as
    /// described in 14.5.2.
    #[doc(alias = "phyUWBCSpulse")]
    pub uwb_cs_pulse: UwbCsPulse,
    /// The weights are represented in twos-complement
    /// form. A value of 0x80 represents –1 while a
    /// value of 0x7F represents 1.
    #[doc(alias = "phyUWBLCPWeight1")]
    pub uwb_lcp_weight1: i8,
    /// The weights are represented in twos-complement
    /// form. A value of 0x80 represents –1 while a
    /// value of 0x7F represents 1.
    #[doc(alias = "phyUWBLCPWeight2")]
    pub uwb_lcp_weight2: i8,
    /// The weights are represented in twos-complement
    /// form. A value of 0x80 represents –1 while a
    /// value of 0x7F represents 1.
    #[doc(alias = "phyUWBLCPWeight3")]
    pub uwb_lcp_weight3: i8,
    /// The weights are represented in twos-complement
    /// form. A value of 0x80 represents –1 while a
    /// value of 0x7F represents 1.
    #[doc(alias = "phyUWBLCPWeight4")]
    pub uwb_lcp_weight4: i8,
    /// The range is from 0 to 4 ns with a resolution is 4/255 = 15.625 ps. For example, a value of 0x00
    /// represents 0 while 0x02 represents 31.25 ps, as
    /// defined in 14.5.3.
    #[doc(alias = "phyUWBLCPDelay2")]
    pub uwb_lcp_delay2: u8,
    /// The range is from 0 to 4 ns with a resolution is 4/255 = 15.625 ps. For example, a value of 0x00
    /// represents 0 while 0x02 represents 31.25 ps, as
    /// defined in 14.5.3.
    #[doc(alias = "phyUWBLCPDelay3")]
    pub uwb_lcp_delay3: u8,
    /// The range is from 0 to 4 ns with a resolution is 4/255 = 15.625 ps. For example, a value of 0x00
    /// represents 0 while 0x02 represents 31.25 ps, as
    /// defined in 14.5.3.
    #[doc(alias = "phyUWBLCPDelay4")]
    pub uwb_lcp_delay4: u8,
    /// This value is zero for PHYs other than UWB or
    /// CSS. For UWB PHYs, this represents the current
    /// preamble code index in use by the transmitter, as
    /// defined in Table 102 and Table 103. For the CSS
    /// PHY, the value indicates the subchirp, as defined
    /// in 13.3.
    #[doc(alias = "phyCurrentCode")]
    pub current_code: u8,
    /// For UWB PHYs, the native PRF. Zero is for nonUWB PHYs; one is for PRF of 4; two is for a
    /// PRF of 16; and three is for PHYs that have no
    /// preference.
    #[doc(alias = "phyNativePRF")]
    pub native_prf: NativePrf,
    /// Number of frequency intervals used to scan each
    /// UWB channel (scan resolution). Set to zero for
    /// non-UWB PHYs.
    #[doc(alias = "phyUWBScanBinsPerChannel")]
    pub uwb_scan_bins_per_channel: u8,
    /// The time interval between two neighboring
    /// inserted preamble symbols in the data portion, as
    /// defined in 14.6, for UWB PHYs operating with
    /// CCA mode 6. The resolution is a data symbol
    /// duration at a data rate of 850 kb/s for all channels. Set to four for UWB PHY in CCA mode 6;
    /// otherwise, set to zero.
    #[doc(alias = "phyUWBInsertedPreambleInterval")]
    pub uwb_inserted_preamble_interval: u8,
    /// A count of the propagation time from the ranging
    /// counter to the transmit antenna. The LSB of a
    /// time value represents 1/128 of a chip time at the
    /// mandatory chipping rate of 499.2 MHz.
    #[doc(alias = "phyTXRMARKEROffset")]
    pub tx_rmarker_offset: u32,
    /// A count of the propagation time from the receive
    /// antenna to the ranging counter. The LSB of a
    /// time value represents 1/128 of a chip time at the
    /// mandatory chipping rate of 499.2 MHz.
    #[doc(alias = "phyRXRMARKEROffset")]
    pub rx_rmarker_offset: u32,
    /// A count of the processing time required by the
    /// PHY to handle an arriving RFRAME. The LSB
    /// represents 2 ms. The meaning of the value is that
    /// if a sequence of RFRAMEs arrive separated by
    /// phyRFRAMEProcessingTime, then the PHY can
    /// keep up with the processing indefinitely.
    #[doc(alias = "phyRFRAMEProcessingTime")]
    pub rframe_processing_time: u8,
    /// The duration for CCA, specified in symbols. This
    /// attribute shall only be implemented with PHYs
    /// operating in the 950 MHz band.
    #[doc(alias = "phyCCADuration")]
    pub cca_duration: u16,
}

impl PhyPibWrite {
    #[rustfmt::skip]
    pub fn try_set(&mut self, attribute: &str, value: &PibValue) -> Option<Status> {
        if !attribute.starts_with("phy") {
            return None;
        }

        let result = match (attribute, value) {
            (PibValue::PHY_CHANNELS_SUPPORTED, _) => Status::ReadOnly,
            (PibValue::PHY_MAX_FRAME_DURATION, _) => Status::ReadOnly,
            (PibValue::PHY_SHR_DURATION, _) => Status::ReadOnly,
            (PibValue::PHY_SYMBOLS_PER_OCTET, _) => Status::ReadOnly,
            (PibValue::PHY_PREAMBLE_SYMBOL_LENGTH, _) => Status::ReadOnly,
            (PibValue::PHY_UWB_DATA_RATES_SUPPORTED, _) => Status::ReadOnly,
            (PibValue::PHY_CSS_LOW_DATA_RATE_SUPPORTED, _) => Status::ReadOnly,
            (PibValue::PHY_UWB_COU_SUPPORTED, _) => Status::ReadOnly,
            (PibValue::PHY_UWB_CS_SUPPORTED, _) => Status::ReadOnly,
            (PibValue::PHY_UWB_LCP_SUPPORTED, _) => Status::ReadOnly,
            (PibValue::PHY_RANGING, _) => Status::ReadOnly,
            (PibValue::PHY_RANGING_CRYSTAL_OFFSET, _) => Status::ReadOnly,
            (PibValue::PHY_RANGING_DPS, _) => Status::ReadOnly,
            (PibValue::PHY_CURRENT_CHANNEL, value @ PibValue::PhyCurrentChannel(_)) => self.set(value),
            (PibValue::PHY_TX_POWER_TOLERANCE, value @ PibValue::PhyTxPowerTolerance(_)) => self.set(value),
            (PibValue::PHY_TX_POWER, value @ PibValue::PhyTxPower(_)) => self.set(value),
            (PibValue::PHY_CCA_MODE, value @ PibValue::PhyCcaMode(_)) => self.set(value),
            (PibValue::PHY_CURRENT_PAGE, value @ PibValue::PhyCurrentPage(_)) => self.set(value),
            (PibValue::PHY_UWB_CURRENT_PULSE_SHAPE, value @ PibValue::PhyUwbCurrentPulseShape(_)) => self.set(value),
            (PibValue::PHY_UWB_COU_PULSE, value @ PibValue::PhyUwbCouPulse(_)) => self.set(value),
            (PibValue::PHY_UWB_CS_PULSE, value @ PibValue::PhyUwbCsPulse(_)) => self.set(value),
            (PibValue::PHY_UWB_LCP_WEIGHT1, value @ PibValue::PhyUwbLcpWeight1(_)) => self.set(value),
            (PibValue::PHY_UWB_LCP_WEIGHT2, value @ PibValue::PhyUwbLcpWeight2(_)) => self.set(value),
            (PibValue::PHY_UWB_LCP_WEIGHT3, value @ PibValue::PhyUwbLcpWeight3(_)) => self.set(value),
            (PibValue::PHY_UWB_LCP_WEIGHT4, value @ PibValue::PhyUwbLcpWeight4(_)) => self.set(value),
            (PibValue::PHY_UWB_LCP_DELAY2, value @ PibValue::PhyUwbLcpDelay2(_)) => self.set(value),
            (PibValue::PHY_UWB_LCP_DELAY3, value @ PibValue::PhyUwbLcpDelay3(_)) => self.set(value),
            (PibValue::PHY_UWB_LCP_DELAY4, value @ PibValue::PhyUwbLcpDelay4(_)) => self.set(value),
            (PibValue::PHY_CURRENT_CODE, value @ PibValue::PhyCurrentCode(_)) => self.set(value),
            (PibValue::PHY_NATIVE_PRF, value @ PibValue::PhyNativePrf(_)) => self.set(value),
            (PibValue::PHY_UWB_SCAN_BINS_PER_CHANNEL, value @ PibValue::PhyUwbScanBinsPerChannel(_)) => self.set(value),
            (PibValue::PHY_UWB_INSERTED_PREAMBLE_INTERVAL, value @ PibValue::PhyUwbInsertedPreambleInterval(_)) => self.set(value),
            (PibValue::PHY_TX_RMARKER_OFFSET, value @ PibValue::PhyTxRmarkerOffset(_)) => self.set(value),
            (PibValue::PHY_RX_RMARKER_OFFSET, value @ PibValue::PhyRxRmarkerOffset(_)) => self.set(value),
            (PibValue::PHY_RFRAME_PROCESSING_TIME, value @ PibValue::PhyRframeProcessingTime(_)) => self.set(value),
            (PibValue::PHY_CCA_DURATION, value @ PibValue::PhyCcaDuration(_)) => self.set(value),
            (PibValue::PHY_CURRENT_CHANNEL, _) => Status::InvalidParameter,
            (PibValue::PHY_TX_POWER_TOLERANCE, _) => Status::InvalidParameter,
            (PibValue::PHY_TX_POWER, _) => Status::InvalidParameter,
            (PibValue::PHY_CCA_MODE, _) => Status::InvalidParameter,
            (PibValue::PHY_CURRENT_PAGE, _) => Status::InvalidParameter,
            (PibValue::PHY_UWB_CURRENT_PULSE_SHAPE, _) => Status::InvalidParameter,
            (PibValue::PHY_UWB_COU_PULSE, _) => Status::InvalidParameter,
            (PibValue::PHY_UWB_CS_PULSE, _) => Status::InvalidParameter,
            (PibValue::PHY_UWB_LCP_WEIGHT1, _) => Status::InvalidParameter,
            (PibValue::PHY_UWB_LCP_WEIGHT2, _) => Status::InvalidParameter,
            (PibValue::PHY_UWB_LCP_WEIGHT3, _) => Status::InvalidParameter,
            (PibValue::PHY_UWB_LCP_WEIGHT4, _) => Status::InvalidParameter,
            (PibValue::PHY_UWB_LCP_DELAY2, _) => Status::InvalidParameter,
            (PibValue::PHY_UWB_LCP_DELAY3, _) => Status::InvalidParameter,
            (PibValue::PHY_UWB_LCP_DELAY4, _) => Status::InvalidParameter,
            (PibValue::PHY_CURRENT_CODE, _) => Status::InvalidParameter,
            (PibValue::PHY_NATIVE_PRF, _) => Status::InvalidParameter,
            (PibValue::PHY_UWB_SCAN_BINS_PER_CHANNEL, _) => Status::InvalidParameter,
            (PibValue::PHY_UWB_INSERTED_PREAMBLE_INTERVAL, _) => Status::InvalidParameter,
            (PibValue::PHY_TX_RMARKER_OFFSET, _) => Status::InvalidParameter,
            (PibValue::PHY_RX_RMARKER_OFFSET, _) => Status::InvalidParameter,
            (PibValue::PHY_RFRAME_PROCESSING_TIME, _) => Status::InvalidParameter,
            (PibValue::PHY_CCA_DURATION, _) => Status::InvalidParameter,
            _ => Status::UnsupportedAttribute,
        };

        Some(result)
    }

    fn set(&mut self, value: &PibValue) -> Status {
        match value {
            PibValue::PhyCurrentChannel(value) => self.current_channel = *value,
            PibValue::PhyTxPowerTolerance(value) => self.tx_power_tolerance = *value,
            PibValue::PhyTxPower(value) => self.tx_power = *value,
            PibValue::PhyCcaMode(value) => self.cca_mode = *value,
            PibValue::PhyCurrentPage(value) => self.current_page = *value,
            PibValue::PhyUwbCurrentPulseShape(value) => self.uwb_current_pulse_shape = *value,
            PibValue::PhyUwbCouPulse(value) => self.uwb_cou_pulse = *value,
            PibValue::PhyUwbCsPulse(value) => self.uwb_cs_pulse = *value,
            PibValue::PhyUwbLcpWeight1(value) => self.uwb_lcp_weight1 = *value,
            PibValue::PhyUwbLcpWeight2(value) => self.uwb_lcp_weight2 = *value,
            PibValue::PhyUwbLcpWeight3(value) => self.uwb_lcp_weight3 = *value,
            PibValue::PhyUwbLcpWeight4(value) => self.uwb_lcp_weight4 = *value,
            PibValue::PhyUwbLcpDelay2(value) => self.uwb_lcp_delay2 = *value,
            PibValue::PhyUwbLcpDelay3(value) => self.uwb_lcp_delay3 = *value,
            PibValue::PhyUwbLcpDelay4(value) => self.uwb_lcp_delay4 = *value,
            PibValue::PhyCurrentCode(value) => self.current_code = *value,
            PibValue::PhyNativePrf(value) => self.native_prf = *value,
            _ => unreachable!(),
        }

        Status::Success
    }
}

#[derive(Debug, Clone)]
pub struct MacPib {
    pub pib_write: MacPibWrite,

    /// The extended address assigned to the device.
    ///
    /// ## Range
    /// Device specific
    #[doc(alias = "macExtendedAddress")]
    pub extended_address: ExtendedAddress,
    /// The time that the device transmitted its
    /// last beacon frame, in symbol periods.
    /// The measurement shall be taken at the
    /// same symbol boundary within every
    /// transmitted beacon frame, the location of
    /// which is implementation specific. The
    /// precision of this value shall be a minimum of 20 bits, with the lowest four bits
    /// being the least significant.
    #[doc(alias = "macBeaconTxTime")]
    pub beacon_tx_time: i64,
    /// The minimum time forming a LIFS
    /// period.
    ///
    /// ## Range
    /// As defined in 8.1.3
    #[doc(alias = "macLIFSPeriod")]
    pub lifs_period: u8,
    /// The minimum time forming a SIFS
    /// period.
    ///
    /// ## Range
    /// As defined in 8.1.3
    #[doc(alias = "macSIFSPeriod")]
    pub sifs_period: u8,
    /// This indicates whether the MAC sublayer supports the optional ranging features.
    #[doc(alias = "macRangingSupported")]
    pub ranging_supported: bool,
    /// The length of the active portion of the
    /// outgoing superframe, including the beacon frame, as defined in 5.1.1.1
    ///
    /// ## Range
    /// 0–15
    #[doc(alias = "macSuperframeOrder")]
    pub superframe_order: SuperframeOrder,
    /// The offset, measured in symbols,
    /// between the symbol boundary at which
    /// the MLME captures the timestamp of
    /// each transmitted or received frame, and
    /// the onset of the first symbol past the
    /// SFD, namely, the first symbol of the
    /// Length field.
    ///
    /// ## Range
    ///
    /// - 0x000–0x100 for the 2.4 GHz band
    /// - 0x000–0x400 for the 868 MHz and 915 MHz bands
    #[doc(alias = "macSyncSymbolOffset")]
    pub sync_symbol_offset: u16,
    /// Indication of whether the MAC sublayer
    /// supports the optional timestamping feature for incoming and outgoing data
    /// frames.
    #[doc(alias = "macTimestampSupported")]
    pub timestamp_supported: bool,
}

impl MacPib {
    /// Dummy values to create just any old mac pib without caring about the values.
    /// TODO: Remove later when there are better PIB init apis
    pub fn dummy_new() -> Self {
        Self {
            pib_write: MacPibWrite {
                associated_pan_coord: false,
                association_permit: false,
                auto_request: false,
                batt_life_ext: false,
                beacon_payload: [0; MAX_BEACON_PAYLOAD_LENGTH],
                beacon_payload_length: 0,
                beacon_order: BeaconOrder::OnDemand,
                bsn: SequenceNumber::new(0),
                coord_extended_address: ExtendedAddress::BROADCAST,
                coord_short_address: ShortAddress::BROADCAST,
                dsn: SequenceNumber::new(0),
                gts_permit: false,
                max_be: 0,
                max_csma_backoffs: 0,
                max_frame_retries: 0,
                min_be: 0,
                pan_id: PanId::broadcast(),
                promiscuous_mode: false,
                response_wait_time: 0,
                rx_on_when_idle: false,
                security_enabled: false,
                short_address: ShortAddress::BROADCAST,
                transaction_persistence_time: 0,
                tx_control_active_duration: 0,
                tx_control_pause_duration: 0,
                tx_total_duration: 0,
            },
            extended_address: ExtendedAddress::BROADCAST,
            beacon_tx_time: 0,
            lifs_period: 0,
            sifs_period: 0,
            ranging_supported: false,
            superframe_order: SuperframeOrder::Inactive,
            sync_symbol_offset: 0,
            timestamp_supported: false,
        }
    }

    #[rustfmt::skip]
    pub fn get(&self, attribute: &str, phy_pib: &PhyPib) -> Option<PibValue> {
        if !attribute.starts_with("mac") {
            return None;
        }

        match attribute {
            PibValue::MAC_EXTENDED_ADDRESS => Some(PibValue::MacExtendedAddress(self.extended_address)),
            PibValue::MAC_ACK_WAIT_DURATION => Some(PibValue::MacAckWaitDuration(self.ack_wait_duration(phy_pib))),
            PibValue::MAC_ASSOCIATED_PAN_COORD => Some(PibValue::MacAssociatedPanCoord(self.associated_pan_coord)),
            PibValue::MAC_BEACON_PAYLOAD => Some(PibValue::MacBeaconPayload(self.beacon_payload)),
            PibValue::MAC_BEACON_PAYLOAD_LENGTH => Some(PibValue::MacBeaconPayloadLength(self.beacon_payload_length)),
            PibValue::MAC_BEACON_TX_TIME => Some(PibValue::MacBeaconTxTime(self.beacon_tx_time)),
            PibValue::MAC_BSN => Some(PibValue::MacBsn(self.bsn.value)),
            PibValue::MAC_COORD_EXTENDED_ADDRESS => Some(PibValue::MacCoordExtendedAddress(self.coord_extended_address)),
            PibValue::MAC_COORD_SHORT_ADDRESS => Some(PibValue::MacCoordShortAddress(self.coord_short_address)),
            PibValue::MAC_DSN => Some(PibValue::MacDsn(self.dsn.value)),
            PibValue::MAC_MAX_FRAME_TOTAL_WAIT_TIME => Some(PibValue::MacMaxFrameTotalWaitTime(self.max_frame_total_wait_time(phy_pib))),
            PibValue::MAC_LIFS_PERIOD => Some(PibValue::MacLifsPeriod(self.lifs_period)),
            PibValue::MAC_SIFS_PERIOD => Some(PibValue::MacSifsPeriod(self.sifs_period)),
            PibValue::MAC_PAN_ID => Some(PibValue::MacPanId(self.pan_id)),
            PibValue::MAC_RANGING_SUPPORTED => Some(PibValue::MacRangingSupported(self.ranging_supported)),
            PibValue::MAC_SHORT_ADDRESS => Some(PibValue::MacShortAddress(self.short_address)),
            PibValue::MAC_SUPERFRAME_ORDER => Some(PibValue::MacSuperframeOrder(self.superframe_order)),
            PibValue::MAC_SYNC_SYMBOL_OFFSET => Some(PibValue::MacSyncSymbolOffset(self.sync_symbol_offset)),
            PibValue::MAC_TIMESTAMP_SUPPORTED => Some(PibValue::MacTimestampSupported(self.timestamp_supported)),
            PibValue::MAC_TRANSACTION_PERSISTENCE_TIME => Some(PibValue::MacTransactionPersistenceTime(self.transaction_persistence_time)),
            PibValue::MAC_TX_CONTROL_ACTIVE_DURATION => Some(PibValue::MacTxControlActiveDuration(self.tx_control_active_duration)),
            PibValue::MAC_TX_CONTROL_PAUSE_DURATION => Some(PibValue::MacTxControlPauseDuration(self.tx_control_pause_duration)),
            PibValue::MAC_TX_TOTAL_DURATION => Some(PibValue::MacTxTotalDuration(self.tx_total_duration)),
            PibValue::MAC_ASSOCIATION_PERMIT => Some(PibValue::MacAssociationPermit(self.association_permit)),
            PibValue::MAC_AUTO_REQUEST => Some(PibValue::MacAutoRequest(self.auto_request)),
            PibValue::MAC_BATT_LIFE_EXT => Some(PibValue::MacBattLifeExt(self.batt_life_ext)),
            PibValue::MAC_BATT_LIFE_EXT_PERIODS => Some(PibValue::MacBattLifeExtPeriods(self.batt_life_ext_periods(phy_pib))),
            PibValue::MAC_BEACON_ORDER => Some(PibValue::MacBeaconOrder(self.beacon_order)),
            PibValue::MAC_GTS_PERMIT => Some(PibValue::MacGtsPermit(self.gts_permit)),
            PibValue::MAC_MAX_BE => Some(PibValue::MacMaxBe(self.max_be)),
            PibValue::MAC_MAX_CSMA_BACKOFFS => Some(PibValue::MacMaxCsmaBackoffs(self.max_csma_backoffs)),
            PibValue::MAC_MAX_FRAME_RETRIES => Some(PibValue::MacMaxFrameRetries(self.max_frame_retries)),
            PibValue::MAC_MIN_BE => Some(PibValue::MacMinBe(self.min_be)),
            PibValue::MAC_PROMISCUOUS_MODE => Some(PibValue::MacPromiscuousMode(self.promiscuous_mode)),
            PibValue::MAC_RESPONSE_WAIT_TIME => Some(PibValue::MacResponseWaitTime(self.response_wait_time)),
            PibValue::MAC_RX_ON_WHEN_IDLE => Some(PibValue::MacRxOnWhenIdle(self.rx_on_when_idle)),
            PibValue::MAC_SECURITY_ENABLED => Some(PibValue::MacSecurityEnabled(self.security_enabled)),
            _ => None,
        }
    }

    /// The maximum number of symbols to
    /// wait for an acknowledgment frame to
    /// arrive following a transmitted data
    /// frame. This value is dependent on the
    /// supported PHY, which determines both
    /// the selected channel and channel page.
    /// The calculated value is the time to commence transmitting the ACK plus the
    /// length of the ACK frame. The commencement time is described in
    /// 5.1.6.4.2.
    ///
    /// ## Range
    ///
    /// As defined in 6.4.3
    #[doc(alias = "macAckWaitDuration")]
    pub fn ack_wait_duration(&self, phy_pib: &PhyPib) -> u32 {
        #[allow(unused)]
        use micromath::F32Ext;

        UNIT_BACKOFF_PERIOD
            + TURNAROUND_TIME
            + phy_pib.shr_duration
            + (6.0 * phy_pib.symbols_per_octet).ceil() as u32
    }

    /// The maximum time to wait either for a
    /// frame intended as a response to a data
    /// request frame or for a broadcast frame
    /// following a beacon with the Frame Pending field set to one.
    ///
    /// ## Range
    /// As defined in 6.4.3
    #[doc(alias = "macMaxFrameTotalWaitTime")]
    pub fn max_frame_total_wait_time(&self, phy_pib: &PhyPib) -> u32 {
        let m = (self.max_be - self.min_be).min(self.max_csma_backoffs);

        let mut max_frame_total_wait_time =
            (self.max_csma_backoffs - m) as u32 * ((1 << self.max_be as u32) - 1);

        for k in 0..m {
            max_frame_total_wait_time += 1 << (self.min_be + k);
        }

        max_frame_total_wait_time *= UNIT_BACKOFF_PERIOD;
        max_frame_total_wait_time += phy_pib.max_frame_duration;
        max_frame_total_wait_time
    }

    /// In BLE mode, the number of backoff
    /// periods during which the receiver is
    /// enabled after the IFS following a beacon.
    /// This value is dependent on the supported
    /// PHY and is the sum of three terms:
    /// Term 1: The value 2x – 1, where x is the
    /// maximum value of macMinBE in BLE
    /// mode (equal to two). This term is thus
    /// equal to threebackoff periods. Term 2:
    /// The duration of the initial contention
    /// window length, as described in 5.1.1.4.
    /// Term 3: The Preamble field length and
    /// the SFD field length of the supported
    /// PHY summed together and rounded up
    /// (if necessary) to an integer number of
    /// backoff periods.
    ///
    /// ## Range
    /// 6-41
    #[doc(alias = "macBattLifeExtPeriods")]
    pub fn batt_life_ext_periods(&self, phy_pib: &PhyPib) -> u8 {
        (
            // Term one
            3
            // Term two
            + phy_pib.current_page.cw0() as u32
            // Term three in unit backoff periods rounded up
            + ((phy_pib.shr_duration + UNIT_BACKOFF_PERIOD / 2) / UNIT_BACKOFF_PERIOD)
        ) as u8
    }

    #[doc(alias = "SD")]
    pub fn superframe_duration(&self) -> Option<NonZeroU32> {
        match self.superframe_order {
            SuperframeOrder::Inactive => None,
            SuperframeOrder::SuperframeOrder(so) => {
                NonZeroU32::new(crate::consts::BASE_SUPERFRAME_DURATION << so)
            }
        }
    }
}

impl core::ops::DerefMut for MacPib {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.pib_write
    }
}

impl core::ops::Deref for MacPib {
    type Target = MacPibWrite;

    fn deref(&self) -> &Self::Target {
        &self.pib_write
    }
}

#[derive(Debug, Clone)]
pub struct MacPibWrite {
    /// Indication of whether the device is associated to the PAN through the PAN coordinator. A value of TRUE indicates the
    /// device has associated through the PAN
    /// coordinator. Otherwise, the value is set to
    /// FALSE.
    #[doc(alias = "macAssociatedPANCoord")]
    pub associated_pan_coord: bool,
    /// Indication of whether a coordinator is
    /// currently allowing association. A value
    /// of TRUE indicates that association is
    /// permitted.
    #[doc(alias = "macAssociationPermit")]
    pub association_permit: bool,
    /// Indication of whether a device automatically sends a data request command if its
    /// address is listed in the beacon frame. A
    /// value of TRUE indicates that the data
    /// request command is automatically sent.
    /// This attribute also affects the generation
    /// of the MLME-BEACON-NOTIFY.indication primitive, as described in 6.2.4.1.
    #[doc(alias = "macAutoRequest")]
    pub auto_request: bool,
    /// Indication of whether BLE, through the
    /// reduction of coordinator receiver operation time during the CAP, is enabled. A
    /// value of TRUE indicates that it is
    /// enabled. The effect of this attribute on
    /// the backoff exponent in the CSMA-CA
    /// algorithm is explained in 5.1.1.4.
    #[doc(alias = "macBattLifeExt")]
    pub batt_life_ext: bool,
    /// The contents of the beacon payload.
    #[doc(alias = "macBeaconPayload")]
    pub beacon_payload: [u8; MAX_BEACON_PAYLOAD_LENGTH],
    /// The length, in octets, of the beacon payload.
    ///
    /// ## Range
    ///
    /// `0 – aMaxBeaconPayloadLength`
    #[doc(alias = "macBeaconPayloadLength")]
    pub beacon_payload_length: usize,
    /// Indicates the frequency with which the
    /// beacon is transmitted, as defined in
    /// 5.1.1.1.
    ///
    /// ## Range
    /// 0–15
    #[doc(alias = "macBeaconOrder")]
    pub beacon_order: BeaconOrder,
    /// The sequence number added to the transmitted beacon frame.
    #[doc(alias = "macBSN")]
    pub bsn: SequenceNumber,
    /// The address of the coordinator through
    /// which the device is associated.
    #[doc(alias = "macCoordExtendedAddress")]
    pub coord_extended_address: ExtendedAddress,
    /// The short address assigned to the coordinator through which the device is associated. A value of 0xfffe indicates that the
    /// coordinator is only using its extended
    /// address. A value of 0xffff indicates that
    /// this value is unknown.
    #[doc(alias = "macCoordShortAddress")]
    pub coord_short_address: ShortAddress,
    /// The sequence number added to the transmitted data or MAC command frame.
    #[doc(alias = "macDSN")]
    pub dsn: SequenceNumber,
    /// TRUE if the PAN coordinator is to
    /// accept GTS requests. FALSE otherwise.
    #[doc(alias = "macGTSPermit")]
    pub gts_permit: bool,
    /// The maximum value of the backoff
    /// exponent, BE, in the CSMA-CA algorithm, as defined in 5.1.1.4.
    ///
    /// ## Range
    /// 3–8
    #[doc(alias = "macMaxBE")]
    pub max_be: u8,
    /// The maximum number of backoffs the
    /// CSMA-CA algorithm will attempt before
    /// declaring a channel access failure.
    ///
    /// ## Range
    /// 0–5
    #[doc(alias = "macMaxCSMABackoffs")]
    pub max_csma_backoffs: u8,
    /// The maximum number of retries allowed
    /// after a transmission failure.
    ///
    /// ## Range
    /// 0–7
    #[doc(alias = "macMaxFrameRetries")]
    pub max_frame_retries: u8,
    /// The minimum value of the backoff exponent (BE) in the CSMA-CA algorithm,
    /// as described in 5.1.1.4.
    ///
    /// ## Range
    /// 0–macMaxBE
    #[doc(alias = "macMinBE")]
    pub min_be: u8,
    /// The identifier of the PAN on which the
    /// device is operating. If this value is 0xffff,
    /// the device is not associated.
    #[doc(alias = "macPANId")]
    pub pan_id: PanId,
    /// Indication of whether the MAC sublayer
    /// is in a promiscuous (receive all) mode. A
    /// value of TRUE indicates that the MAC
    /// sublayer accepts all frames received
    /// from the PHY.
    #[doc(alias = "macPromiscuousMode")]
    pub promiscuous_mode: bool,
    /// The maximum time, in multiples of
    /// aBaseSuperframeDuration, a device
    /// shall wait for a response command frame
    /// to be available following a request command frame.
    ///
    /// ## Range
    /// 2-64
    #[doc(alias = "macResponseWaitTime")]
    pub response_wait_time: u8,
    /// Indication of whether the MAC sublayer
    /// is to enable its receiver during idle periods. For a beacon-enabled PAN, this
    /// attribute is relevant only during the CAP
    /// of the incoming superframe. For a nonbeacon-enabled PAN, this attribute is relevant at all times.
    #[doc(alias = "macRxOnWhenIdle")]
    pub rx_on_when_idle: bool,
    /// Indication of whether the MAC sublayer
    /// has security enabled.
    ///
    /// A value of TRUE indicates that security
    /// is enabled, while a value of FALSE indicates that security is disabled.
    #[doc(alias = "macSecurityEnabled")]
    pub security_enabled: bool,
    /// The address that the device uses to communicate in the PAN. If the device is the
    /// PAN coordinator, this value shall be chosen before a PAN is started. Otherwise,
    /// the short address is allocated by a coordinator during association.
    ///
    /// A value of 0xfffe indicates that the
    /// device has associated but has not been
    /// allocated an address. A value of 0xffff
    /// indicates that the device does not have a
    /// short address.
    #[doc(alias = "macShortAddress")]
    pub short_address: ShortAddress,
    /// The maximum time (in unit periods) that
    /// a transaction is stored by a coordinator
    /// and indicated in its beacon.
    ///
    /// The unit period is governed by macBeaconOrder, BO, as follows: For 0 ≤ BO ≤
    /// 14, the unit period will be aBase-SuperframeDuration × 2BO. For BO = 15, the
    /// unit period will be aBaseSuperframeDuration.
    #[doc(alias = "macTransactionPersistenceTime")]
    pub transaction_persistence_time: u16,
    /// The duration for which transmit is
    /// permitted without pause specified in
    /// symbols.
    ///
    /// ## Range
    /// 0–100000
    #[doc(alias = "macTxControlActiveDuration")]
    pub tx_control_active_duration: u32,
    /// The duration after transmission before
    /// another transmission is permitted specified in symbols.
    ///
    /// ## Range
    /// 2000 or 10000
    #[doc(alias = "macTxControlPauseDuration")]
    pub tx_control_pause_duration: u32,
    /// The total transmit duration (including
    /// PHY header and FCS) specified in symbols. This can be read and cleared by
    /// NHL.
    #[doc(alias = "macTxTotalDuration")]
    pub tx_total_duration: u32,
}

impl MacPibWrite {
    #[rustfmt::skip]
    pub fn try_set(&mut self, attribute: &str, value: &PibValue) -> Option<Status> {
        if !attribute.starts_with("mac") {
            return None;
        }

        let result = match (attribute, value) {
            (PibValue::MAC_EXTENDED_ADDRESS, _) => Status::ReadOnly,
            (PibValue::MAC_ACK_WAIT_DURATION, _) => Status::ReadOnly,
            (PibValue::MAC_BEACON_TX_TIME, _) => Status::ReadOnly,
            (PibValue::MAC_LIFS_PERIOD, _) => Status::ReadOnly,
            (PibValue::MAC_SIFS_PERIOD, _) => Status::ReadOnly,
            (PibValue::MAC_RANGING_SUPPORTED, _) => Status::ReadOnly,
            (PibValue::MAC_SUPERFRAME_ORDER, _) => Status::ReadOnly,
            (PibValue::MAC_SYNC_SYMBOL_OFFSET, _) => Status::ReadOnly,
            (PibValue::MAC_TIMESTAMP_SUPPORTED, _) => Status::ReadOnly,

            (PibValue::MAC_ASSOCIATED_PAN_COORD, value @ PibValue::MacAssociatedPanCoord(_)) => self.set(value),
            (PibValue::MAC_ASSOCIATION_PERMIT, value @ PibValue::MacAssociationPermit(_)) => self.set(value),
            (PibValue::MAC_AUTO_REQUEST, value @ PibValue::MacAutoRequest(_)) => self.set(value),
            (PibValue::MAC_BATT_LIFE_EXT, value @ PibValue::MacBattLifeExt(_)) => self.set(value),
            (PibValue::MAC_BATT_LIFE_EXT_PERIODS, value @ PibValue::MacBattLifeExtPeriods(_)) => self.set(value),
            (PibValue::MAC_BEACON_PAYLOAD, value @ PibValue::MacBeaconPayload(_)) => self.set(value),
            (PibValue::MAC_BEACON_PAYLOAD_LENGTH, value @ PibValue::MacBeaconPayloadLength(_)) => self.set(value),
            (PibValue::MAC_BEACON_ORDER, value @ PibValue::MacBeaconOrder(_)) => self.set(value),
            (PibValue::MAC_BSN, value @ PibValue::MacBsn(_)) => self.set(value),
            (PibValue::MAC_COORD_EXTENDED_ADDRESS, value @ PibValue::MacCoordExtendedAddress(_)) => self.set(value),
            (PibValue::MAC_COORD_SHORT_ADDRESS, value @ PibValue::MacCoordShortAddress(_)) => self.set(value),
            (PibValue::MAC_DSN, value @ PibValue::MacDsn(_)) => self.set(value),
            (PibValue::MAC_GTS_PERMIT, value @ PibValue::MacGtsPermit(_)) => self.set(value),
            (PibValue::MAC_MAX_BE, value @ PibValue::MacMaxBe(_)) => self.set(value),
            (PibValue::MAC_MAX_CSMA_BACKOFFS, value @ PibValue::MacMaxCsmaBackoffs(_)) => self.set(value),
            (PibValue::MAC_MAX_FRAME_TOTAL_WAIT_TIME, value @ PibValue::MacMaxFrameTotalWaitTime(_)) => self.set(value),
            (PibValue::MAC_MAX_FRAME_RETRIES, value @ PibValue::MacMaxFrameRetries(_)) => self.set(value),
            (PibValue::MAC_MIN_BE, value @ PibValue::MacMinBe(_)) => self.set(value),
            (PibValue::MAC_PAN_ID, value @ PibValue::MacPanId(_)) => self.set(value),
            (PibValue::MAC_PROMISCUOUS_MODE, value @ PibValue::MacPromiscuousMode(_)) => self.set(value),
            (PibValue::MAC_RESPONSE_WAIT_TIME, value @ PibValue::MacResponseWaitTime(_)) => self.set(value),
            (PibValue::MAC_RX_ON_WHEN_IDLE, value @ PibValue::MacRxOnWhenIdle(_)) => self.set(value),
            (PibValue::MAC_SECURITY_ENABLED, value @ PibValue::MacSecurityEnabled(_)) => self.set(value),
            (PibValue::MAC_SHORT_ADDRESS, value @ PibValue::MacShortAddress(_)) => self.set(value),
            (PibValue::MAC_TRANSACTION_PERSISTENCE_TIME, value @ PibValue::MacTransactionPersistenceTime(_)) => self.set(value),
            (PibValue::MAC_TX_CONTROL_ACTIVE_DURATION, value @ PibValue::MacTxControlActiveDuration(_)) => self.set(value),
            (PibValue::MAC_TX_CONTROL_PAUSE_DURATION, value @ PibValue::MacTxControlPauseDuration(_)) => self.set(value),
            (PibValue::MAC_TX_TOTAL_DURATION, value @ PibValue::MacTxTotalDuration(_)) => self.set(value),

            (PibValue::MAC_ASSOCIATED_PAN_COORD, _) => Status::InvalidParameter,
            (PibValue::MAC_ASSOCIATION_PERMIT, _) => Status::InvalidParameter,
            (PibValue::MAC_AUTO_REQUEST, _) => Status::InvalidParameter,
            (PibValue::MAC_BATT_LIFE_EXT, _) => Status::InvalidParameter,
            (PibValue::MAC_BATT_LIFE_EXT_PERIODS, _) => Status::InvalidParameter,
            (PibValue::MAC_BEACON_PAYLOAD, _) => Status::InvalidParameter,
            (PibValue::MAC_BEACON_PAYLOAD_LENGTH, _) => Status::InvalidParameter,
            (PibValue::MAC_BEACON_ORDER, _) => Status::InvalidParameter,
            (PibValue::MAC_BSN, _) => Status::InvalidParameter,
            (PibValue::MAC_COORD_EXTENDED_ADDRESS, _) => Status::InvalidParameter,
            (PibValue::MAC_COORD_SHORT_ADDRESS, _) => Status::InvalidParameter,
            (PibValue::MAC_DSN, _) => Status::InvalidParameter,
            (PibValue::MAC_GTS_PERMIT, _) => Status::InvalidParameter,
            (PibValue::MAC_MAX_BE, _) => Status::InvalidParameter,
            (PibValue::MAC_MAX_CSMA_BACKOFFS, _) => Status::InvalidParameter,
            (PibValue::MAC_MAX_FRAME_TOTAL_WAIT_TIME, _) => Status::InvalidParameter,
            (PibValue::MAC_MAX_FRAME_RETRIES, _) => Status::InvalidParameter,
            (PibValue::MAC_MIN_BE, _) => Status::InvalidParameter,
            (PibValue::MAC_PAN_ID, _) => Status::InvalidParameter,
            (PibValue::MAC_PROMISCUOUS_MODE, _) => Status::InvalidParameter,
            (PibValue::MAC_RESPONSE_WAIT_TIME, _) => Status::InvalidParameter,
            (PibValue::MAC_RX_ON_WHEN_IDLE, _) => Status::InvalidParameter,
            (PibValue::MAC_SECURITY_ENABLED, _) => Status::InvalidParameter,
            (PibValue::MAC_SHORT_ADDRESS, _) => Status::InvalidParameter,
            (PibValue::MAC_TRANSACTION_PERSISTENCE_TIME, _) => Status::InvalidParameter,
            (PibValue::MAC_TX_CONTROL_ACTIVE_DURATION, _) => Status::InvalidParameter,
            (PibValue::MAC_TX_CONTROL_PAUSE_DURATION, _) => Status::InvalidParameter,
            (PibValue::MAC_TX_TOTAL_DURATION, _) => Status::InvalidParameter,

            _ => Status::UnsupportedAttribute,
        };

        Some(result)
    }

    #[rustfmt::skip]
    fn set(&mut self, value: &PibValue) -> Status {
        let Self {
            associated_pan_coord,
            association_permit,
            auto_request,
            batt_life_ext,
            beacon_payload,
            beacon_payload_length,
            beacon_order,
            bsn,
            coord_extended_address,
            coord_short_address,
            dsn,
            gts_permit,
            max_be,
            max_csma_backoffs,
            max_frame_retries,
            min_be,
            pan_id,
            promiscuous_mode,
            response_wait_time,
            rx_on_when_idle,
            security_enabled,
            short_address,
            transaction_persistence_time,
            tx_control_active_duration,
            tx_control_pause_duration,
            tx_total_duration,
        } = self;

        match value {
            PibValue::MacAssociatedPanCoord(value) => *associated_pan_coord = *value,
            PibValue::MacAssociationPermit(value) => *association_permit = *value,
            PibValue::MacAutoRequest(value) => *auto_request = *value,
            PibValue::MacBattLifeExt(value) => *batt_life_ext = *value,
            PibValue::MacBattLifeExtPeriods(value) if (6..=41).contains(value) => {
                // Ignored since we do calculations manually
            }
            PibValue::MacBattLifeExtPeriods(_) => return Status::InvalidParameter,
            PibValue::MacBeaconPayload(value) => *beacon_payload = *value,
            PibValue::MacBeaconPayloadLength(value) => *beacon_payload_length = *value,
            PibValue::MacBeaconOrder(value) => *beacon_order = *value,
            PibValue::MacBsn(value) => bsn.value = *value,
            PibValue::MacCoordExtendedAddress(value) => *coord_extended_address = *value,
            PibValue::MacCoordShortAddress(value) => *coord_short_address = *value,
            PibValue::MacDsn(value) => dsn.value = *value,
            PibValue::MacGtsPermit(value) => *gts_permit = *value,
            PibValue::MacMaxBe(value) if (3..=8).contains(value) => *max_be = *value,
            PibValue::MacMaxBe(_) => return Status::InvalidParameter,
            PibValue::MacMaxCsmaBackoffs(value) if (0..=5).contains(value) => {
                *max_csma_backoffs = *value
            }
            PibValue::MacMaxCsmaBackoffs(_) => return Status::InvalidParameter,
            PibValue::MacMaxFrameTotalWaitTime(_value) => {
                // Ignored since we do calculations manually
            }
            PibValue::MacMaxFrameRetries(value) if (0..=7).contains(value) => {
                *max_frame_retries = *value
            }
            PibValue::MacMaxFrameRetries(_) => return Status::InvalidParameter,
            PibValue::MacMinBe(value) if (0..=*max_be).contains(value) => *min_be = *value,
            PibValue::MacMinBe(_) => return Status::InvalidParameter,
            PibValue::MacPanId(value) => *pan_id = *value,
            PibValue::MacPromiscuousMode(value) => *promiscuous_mode = *value,
            PibValue::MacResponseWaitTime(value) if (2..=64).contains(value) => {
                *response_wait_time = *value
            }
            PibValue::MacResponseWaitTime(_) => return Status::InvalidParameter,
            PibValue::MacRxOnWhenIdle(value) => *rx_on_when_idle = *value,
            PibValue::MacSecurityEnabled(value) => *security_enabled = *value,
            PibValue::MacShortAddress(value) => *short_address = *value,
            PibValue::MacTransactionPersistenceTime(value) => {
                *transaction_persistence_time = *value
            }
            PibValue::MacTxControlActiveDuration(value) if (0..=100000).contains(value) => {
                *tx_control_active_duration = *value
            }
            PibValue::MacTxControlActiveDuration(_) => return Status::InvalidParameter,
            PibValue::MacTxControlPauseDuration(value) if *value == 2000 || *value == 10000 => {
                *tx_control_pause_duration = *value
            }
            PibValue::MacTxControlPauseDuration(_) => return Status::InvalidParameter,
            PibValue::MacTxTotalDuration(value) => *tx_total_duration = *value,
            _ => unreachable!(),
        }

        Status::Success
    }

    #[doc(alias = "BI")]
    pub fn beacon_interval(&self) -> Option<NonZeroU32> {
        match self.beacon_order {
            BeaconOrder::OnDemand => None,
            BeaconOrder::BeaconOrder(bo) => {
                NonZero::new(crate::consts::BASE_SUPERFRAME_DURATION << bo)
            }
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ChannelDescription {
    pub page: ChannelPage,
    pub channel_numbers: &'static [u8],
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum TXPowerTolerance {
    /// One decibel
    DB1,
    /// Three decibels
    DB3,
    /// Six decibels
    DB6,
}

/// 8.2.7
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CcaMode {
    EnergyAboveThreshold = 1,
    CarrierSenseOnly,
    CarrierSenseEnergyAboveTheshold,
    Aloha,
    UwbPreambleSenseShr,
    UwbPreambleSensePacket,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum UwbCurrentPulseShape {
    Mandatory,
    Cou,
    Cs,
    Lcp,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum UwbCouPulse {
    CCh1 = 1,
    CCh2,
    CCh3,
    CCh4,
    CCh5,
    CCh6,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum UwbCsPulse {
    No1 = 1,
    No2,
    No3,
    No4,
    No5,
    No6,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum NativePrf {
    NonUwb,
    Prf4,
    Prf16,
    NoPreference,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PibValue {
    None,
    PhyChannelsSupported(&'static [ChannelDescription]),
    PhyMaxFrameDuration(u32),
    PhyShrDuration(u32),
    PhySymbolsPerOctet(f32),
    PhyPreambleSymbolLength(u32),
    PhyUwbDataRatesSupported(&'static [u8]),
    PhyCssLowDataRateSupported(bool),
    PhyUwbCouSupported(bool),
    PhyUwbCsSupported(bool),
    PhyUwbLcpSupported(bool),
    PhyRanging(bool),
    PhyRangingCrystalOffset(bool),
    PhyRangingDps(bool),
    PhyCurrentChannel(u8),
    PhyTxPowerTolerance(TXPowerTolerance),
    PhyTxPower(i16),
    PhyCcaMode(CcaMode),
    PhyCurrentPage(ChannelPage),
    PhyUwbCurrentPulseShape(UwbCurrentPulseShape),
    PhyUwbCouPulse(UwbCouPulse),
    PhyUwbCsPulse(UwbCsPulse),
    PhyUwbLcpWeight1(i8),
    PhyUwbLcpWeight2(i8),
    PhyUwbLcpWeight3(i8),
    PhyUwbLcpWeight4(i8),
    PhyUwbLcpDelay2(u8),
    PhyUwbLcpDelay3(u8),
    PhyUwbLcpDelay4(u8),
    PhyCurrentCode(u8),
    PhyNativePrf(NativePrf),
    PhyUwbScanBinsPerChannel(u8),
    PhyUwbInsertedPreambleInterval(u8),
    PhyTxRmarkerOffset(u32),
    PhyRxRmarkerOffset(u32),
    PhyRframeProcessingTime(u8),
    PhyCcaDuration(u16),
    MacExtendedAddress(ExtendedAddress),
    MacAckWaitDuration(u32),
    MacAssociatedPanCoord(bool),
    MacBeaconPayload([u8; MAX_BEACON_PAYLOAD_LENGTH]),
    MacBeaconPayloadLength(usize),
    MacBeaconTxTime(i64),
    MacBsn(u8),
    MacCoordExtendedAddress(ExtendedAddress),
    MacCoordShortAddress(ShortAddress),
    MacDsn(u8),
    MacMaxFrameTotalWaitTime(u32),
    MacLifsPeriod(u8),
    MacSifsPeriod(u8),
    MacPanId(PanId),
    MacRangingSupported(bool),
    MacShortAddress(ShortAddress),
    MacSuperframeOrder(SuperframeOrder),
    MacSyncSymbolOffset(u16),
    MacTimestampSupported(bool),
    MacTransactionPersistenceTime(u16),
    MacTxControlActiveDuration(u32),
    MacTxControlPauseDuration(u32),
    MacTxTotalDuration(u32),
    MacAssociationPermit(bool),
    MacAutoRequest(bool),
    MacBattLifeExt(bool),
    MacBattLifeExtPeriods(u8),
    MacBeaconOrder(BeaconOrder),
    MacGtsPermit(bool),
    MacMaxBe(u8),
    MacMaxCsmaBackoffs(u8),
    MacMaxFrameRetries(u8),
    MacMinBe(u8),
    MacPromiscuousMode(bool),
    MacResponseWaitTime(u8),
    MacRxOnWhenIdle(bool),
    MacSecurityEnabled(bool),
}

impl PibValue {
    pub const PHY_CHANNELS_SUPPORTED: &'static str = "phyChannelsSupported";
    pub const PHY_MAX_FRAME_DURATION: &'static str = "phyMaxFrameDuration";
    pub const PHY_SHR_DURATION: &'static str = "phySHRDuration";
    pub const PHY_SYMBOLS_PER_OCTET: &'static str = "phySymbolsPerOctet";
    pub const PHY_PREAMBLE_SYMBOL_LENGTH: &'static str = "phyPreambleSymbolLength";
    pub const PHY_UWB_DATA_RATES_SUPPORTED: &'static str = "phyUWBDataRatesSupported";
    pub const PHY_CSS_LOW_DATA_RATE_SUPPORTED: &'static str = "phyCSSLowDataRateSupported";
    pub const PHY_UWB_COU_SUPPORTED: &'static str = "phyUWBCoUSupported";
    pub const PHY_UWB_CS_SUPPORTED: &'static str = "phyUWBCSSupported";
    pub const PHY_UWB_LCP_SUPPORTED: &'static str = "phyUWBLCPSupported";
    pub const PHY_RANGING: &'static str = "phyRanging";
    pub const PHY_RANGING_CRYSTAL_OFFSET: &'static str = "phyRangingCrystalOffset";
    pub const PHY_RANGING_DPS: &'static str = "phyRangingDPS";
    pub const PHY_CURRENT_CHANNEL: &'static str = "phyCurrentChannel";
    pub const PHY_TX_POWER_TOLERANCE: &'static str = "phyTXPowerTolerance";
    pub const PHY_TX_POWER: &'static str = "phyTXPower";
    pub const PHY_CCA_MODE: &'static str = "phyCCAMode";
    pub const PHY_CURRENT_PAGE: &'static str = "phyCurrentPage";
    pub const PHY_UWB_CURRENT_PULSE_SHAPE: &'static str = "phyUWBCurrentPulseShape";
    pub const PHY_UWB_COU_PULSE: &'static str = "phyUWBCoUpulse";
    pub const PHY_UWB_CS_PULSE: &'static str = "phyUWBCSpulse";
    pub const PHY_UWB_LCP_WEIGHT1: &'static str = "phyUWBLCPWeight1";
    pub const PHY_UWB_LCP_WEIGHT2: &'static str = "phyUWBLCPWeight2";
    pub const PHY_UWB_LCP_WEIGHT3: &'static str = "phyUWBLCPWeight3";
    pub const PHY_UWB_LCP_WEIGHT4: &'static str = "phyUWBLCPWeight4";
    pub const PHY_UWB_LCP_DELAY2: &'static str = "phyUWBLCPDelay2";
    pub const PHY_UWB_LCP_DELAY3: &'static str = "phyUWBLCPDelay3";
    pub const PHY_UWB_LCP_DELAY4: &'static str = "phyUWBLCPDelay4";
    pub const PHY_CURRENT_CODE: &'static str = "phyCurrentCode";
    pub const PHY_NATIVE_PRF: &'static str = "phyNativePRF";
    pub const PHY_UWB_SCAN_BINS_PER_CHANNEL: &'static str = "phyUWBScanBinsPerChannel";
    pub const PHY_UWB_INSERTED_PREAMBLE_INTERVAL: &'static str = "phyUWBInsertedPreambleInterval";
    pub const PHY_TX_RMARKER_OFFSET: &'static str = "phyTXRMARKEROffset";
    pub const PHY_RX_RMARKER_OFFSET: &'static str = "phyRXRMARKEROffset";
    pub const PHY_RFRAME_PROCESSING_TIME: &'static str = "phyRFRAMEProcessingTime";
    pub const PHY_CCA_DURATION: &'static str = "phyCCADuration";
    pub const MAC_EXTENDED_ADDRESS: &'static str = "macExtendedAddress";
    pub const MAC_ACK_WAIT_DURATION: &'static str = "macAckWaitDuration";
    pub const MAC_ASSOCIATED_PAN_COORD: &'static str = "macAssociatedPANCoord";
    pub const MAC_BEACON_PAYLOAD: &'static str = "macBeaconPayload";
    pub const MAC_BEACON_PAYLOAD_LENGTH: &'static str = "macBeaconPayloadLength";
    pub const MAC_BEACON_TX_TIME: &'static str = "macBeaconTxTime";
    pub const MAC_BSN: &'static str = "macBSN";
    pub const MAC_COORD_EXTENDED_ADDRESS: &'static str = "macCoordExtendedAddress";
    pub const MAC_COORD_SHORT_ADDRESS: &'static str = "macCoordShortAddress";
    pub const MAC_DSN: &'static str = "macDSN";
    pub const MAC_MAX_FRAME_TOTAL_WAIT_TIME: &'static str = "macMaxFrameTotalWaitTime";
    pub const MAC_LIFS_PERIOD: &'static str = "macLIFSPeriod";
    pub const MAC_SIFS_PERIOD: &'static str = "macSIFSPeriod";
    pub const MAC_PAN_ID: &'static str = "macPANId";
    pub const MAC_RANGING_SUPPORTED: &'static str = "macRangingSupported";
    pub const MAC_SHORT_ADDRESS: &'static str = "macShortAddress";
    pub const MAC_SUPERFRAME_ORDER: &'static str = "macSuperframeOrder";
    pub const MAC_SYNC_SYMBOL_OFFSET: &'static str = "macSyncSymbolOffset";
    pub const MAC_TIMESTAMP_SUPPORTED: &'static str = "macTimestampSupported";
    pub const MAC_TRANSACTION_PERSISTENCE_TIME: &'static str = "macTransactionPersistenceTime";
    pub const MAC_TX_CONTROL_ACTIVE_DURATION: &'static str = "macTxControlActiveDuration";
    pub const MAC_TX_CONTROL_PAUSE_DURATION: &'static str = "macTxControlPauseDuration";
    pub const MAC_TX_TOTAL_DURATION: &'static str = "macTxTotalDuration";
    pub const MAC_ASSOCIATION_PERMIT: &'static str = "macAssociationPermit";
    pub const MAC_AUTO_REQUEST: &'static str = "macAutoRequest";
    pub const MAC_BATT_LIFE_EXT: &'static str = "macBattLifeExt";
    pub const MAC_BATT_LIFE_EXT_PERIODS: &'static str = "macBattLifeExtPeriods";
    pub const MAC_BEACON_ORDER: &'static str = "macBeaconOrder";
    pub const MAC_GTS_PERMIT: &'static str = "macGTSPermit";
    pub const MAC_MAX_BE: &'static str = "macMaxBE";
    pub const MAC_MAX_CSMA_BACKOFFS: &'static str = "macMaxCSMABackoffs";
    pub const MAC_MAX_FRAME_RETRIES: &'static str = "macMaxFrameRetries";
    pub const MAC_MIN_BE: &'static str = "macMinBE";
    pub const MAC_PROMISCUOUS_MODE: &'static str = "macPromiscuousMode";
    pub const MAC_RESPONSE_WAIT_TIME: &'static str = "macResponseWaitTime";
    pub const MAC_RX_ON_WHEN_IDLE: &'static str = "macRxOnWhenIdle";
    pub const MAC_SECURITY_ENABLED: &'static str = "macSecurityEnabled";

    pub const fn name(&self) -> &'static str {
        match self {
            PibValue::None => "none",
            PibValue::PhyChannelsSupported(_) => Self::PHY_CHANNELS_SUPPORTED,
            PibValue::PhyMaxFrameDuration(_) => Self::PHY_MAX_FRAME_DURATION,
            PibValue::PhyShrDuration(_) => Self::PHY_SHR_DURATION,
            PibValue::PhySymbolsPerOctet(_) => Self::PHY_SYMBOLS_PER_OCTET,
            PibValue::PhyPreambleSymbolLength(_) => Self::PHY_PREAMBLE_SYMBOL_LENGTH,
            PibValue::PhyUwbDataRatesSupported(_) => Self::PHY_UWB_DATA_RATES_SUPPORTED,
            PibValue::PhyCssLowDataRateSupported(_) => Self::PHY_CSS_LOW_DATA_RATE_SUPPORTED,
            PibValue::PhyUwbCouSupported(_) => Self::PHY_UWB_COU_SUPPORTED,
            PibValue::PhyUwbCsSupported(_) => Self::PHY_UWB_CS_SUPPORTED,
            PibValue::PhyUwbLcpSupported(_) => Self::PHY_UWB_LCP_SUPPORTED,
            PibValue::PhyRanging(_) => Self::PHY_RANGING,
            PibValue::PhyRangingCrystalOffset(_) => Self::PHY_RANGING_CRYSTAL_OFFSET,
            PibValue::PhyRangingDps(_) => Self::PHY_RANGING_DPS,
            PibValue::PhyCurrentChannel(_) => Self::PHY_CURRENT_CHANNEL,
            PibValue::PhyTxPowerTolerance(_) => Self::PHY_TX_POWER_TOLERANCE,
            PibValue::PhyTxPower(_) => Self::PHY_TX_POWER,
            PibValue::PhyCcaMode(_) => Self::PHY_CCA_MODE,
            PibValue::PhyCurrentPage(_) => Self::PHY_CURRENT_PAGE,
            PibValue::PhyUwbCurrentPulseShape(_) => Self::PHY_UWB_CURRENT_PULSE_SHAPE,
            PibValue::PhyUwbCouPulse(_) => Self::PHY_UWB_COU_PULSE,
            PibValue::PhyUwbCsPulse(_) => Self::PHY_UWB_CS_PULSE,
            PibValue::PhyUwbLcpWeight1(_) => Self::PHY_UWB_LCP_WEIGHT1,
            PibValue::PhyUwbLcpWeight2(_) => Self::PHY_UWB_LCP_WEIGHT2,
            PibValue::PhyUwbLcpWeight3(_) => Self::PHY_UWB_LCP_WEIGHT3,
            PibValue::PhyUwbLcpWeight4(_) => Self::PHY_UWB_LCP_WEIGHT4,
            PibValue::PhyUwbLcpDelay2(_) => Self::PHY_UWB_LCP_DELAY2,
            PibValue::PhyUwbLcpDelay3(_) => Self::PHY_UWB_LCP_DELAY3,
            PibValue::PhyUwbLcpDelay4(_) => Self::PHY_UWB_LCP_DELAY4,
            PibValue::PhyCurrentCode(_) => Self::PHY_CURRENT_CODE,
            PibValue::PhyNativePrf(_) => Self::PHY_NATIVE_PRF,
            PibValue::PhyUwbScanBinsPerChannel(_) => Self::PHY_UWB_SCAN_BINS_PER_CHANNEL,
            PibValue::PhyUwbInsertedPreambleInterval(_) => Self::PHY_UWB_INSERTED_PREAMBLE_INTERVAL,
            PibValue::PhyTxRmarkerOffset(_) => Self::PHY_TX_RMARKER_OFFSET,
            PibValue::PhyRxRmarkerOffset(_) => Self::PHY_RX_RMARKER_OFFSET,
            PibValue::PhyRframeProcessingTime(_) => Self::PHY_RFRAME_PROCESSING_TIME,
            PibValue::PhyCcaDuration(_) => Self::PHY_CCA_DURATION,
            PibValue::MacExtendedAddress(_) => Self::MAC_EXTENDED_ADDRESS,
            PibValue::MacAckWaitDuration(_) => Self::MAC_ACK_WAIT_DURATION,
            PibValue::MacAssociatedPanCoord(_) => Self::MAC_ASSOCIATED_PAN_COORD,
            PibValue::MacBeaconPayload(_) => Self::MAC_BEACON_PAYLOAD,
            PibValue::MacBeaconPayloadLength(_) => Self::MAC_BEACON_PAYLOAD_LENGTH,
            PibValue::MacBeaconTxTime(_) => Self::MAC_BEACON_TX_TIME,
            PibValue::MacBsn(_) => Self::MAC_BSN,
            PibValue::MacCoordExtendedAddress(_) => Self::MAC_COORD_EXTENDED_ADDRESS,
            PibValue::MacCoordShortAddress(_) => Self::MAC_COORD_SHORT_ADDRESS,
            PibValue::MacDsn(_) => Self::MAC_DSN,
            PibValue::MacMaxFrameTotalWaitTime(_) => Self::MAC_MAX_FRAME_TOTAL_WAIT_TIME,
            PibValue::MacLifsPeriod(_) => Self::MAC_LIFS_PERIOD,
            PibValue::MacSifsPeriod(_) => Self::MAC_SIFS_PERIOD,
            PibValue::MacPanId(_) => Self::MAC_PAN_ID,
            PibValue::MacRangingSupported(_) => Self::MAC_RANGING_SUPPORTED,
            PibValue::MacShortAddress(_) => Self::MAC_SHORT_ADDRESS,
            PibValue::MacSuperframeOrder(_) => Self::MAC_SUPERFRAME_ORDER,
            PibValue::MacSyncSymbolOffset(_) => Self::MAC_SYNC_SYMBOL_OFFSET,
            PibValue::MacTimestampSupported(_) => Self::MAC_TIMESTAMP_SUPPORTED,
            PibValue::MacTransactionPersistenceTime(_) => Self::MAC_TRANSACTION_PERSISTENCE_TIME,
            PibValue::MacTxControlActiveDuration(_) => Self::MAC_TX_CONTROL_ACTIVE_DURATION,
            PibValue::MacTxControlPauseDuration(_) => Self::MAC_TX_CONTROL_PAUSE_DURATION,
            PibValue::MacTxTotalDuration(_) => Self::MAC_TX_TOTAL_DURATION,
            PibValue::MacAssociationPermit(_) => Self::MAC_ASSOCIATION_PERMIT,
            PibValue::MacAutoRequest(_) => Self::MAC_AUTO_REQUEST,
            PibValue::MacBattLifeExt(_) => Self::MAC_BATT_LIFE_EXT,
            PibValue::MacBattLifeExtPeriods(_) => Self::MAC_BATT_LIFE_EXT_PERIODS,
            PibValue::MacBeaconOrder(_) => Self::MAC_BEACON_ORDER,
            PibValue::MacGtsPermit(_) => Self::MAC_GTS_PERMIT,
            PibValue::MacMaxBe(_) => Self::MAC_MAX_BE,
            PibValue::MacMaxCsmaBackoffs(_) => Self::MAC_MAX_CSMA_BACKOFFS,
            PibValue::MacMaxFrameRetries(_) => Self::MAC_MAX_FRAME_RETRIES,
            PibValue::MacMinBe(_) => Self::MAC_MIN_BE,
            PibValue::MacPromiscuousMode(_) => Self::MAC_PROMISCUOUS_MODE,
            PibValue::MacResponseWaitTime(_) => Self::MAC_RESPONSE_WAIT_TIME,
            PibValue::MacRxOnWhenIdle(_) => Self::MAC_RX_ON_WHEN_IDLE,
            PibValue::MacSecurityEnabled(_) => Self::MAC_SECURITY_ENABLED,
        }
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt-03", derive(defmt::Format))]
pub struct SequenceNumber {
    value: u8,
}

impl SequenceNumber {
    pub fn new(initial_value: u8) -> Self {
        Self {
            value: initial_value,
        }
    }

    pub fn increment(&mut self) -> u8 {
        self.value = self.value.wrapping_add(1);
        self.value
    }
}
