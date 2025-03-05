#![no_std]

use core::fmt::{Debug, Display};

pub use dw1000;
use dw1000::{
    configs::PulseRepetitionFrequency, AutoDoubleBufferReceiving, Ready, RxConfig, TxConfig,
};
use embassy_futures::select::{select, Either};
use embedded_hal::{delay::DelayNs as DelayNsSync, digital::ErrorType, spi::SpiDevice};
use embedded_hal_async::{delay::DelayNs, digital::Wait};
use lr_wpan_rs::{
    phy::{ModulationType, Phy, ReceivedMessage, SendContinuation},
    pib::{
        CcaMode, ChannelDescription, NativePrf, PhyPib, PhyPibWrite, TXPowerTolerance,
        UwbCurrentPulseShape,
    },
    time::{Duration, Instant},
    ChannelPage,
};
#[allow(unused_imports)]
use micromath::F32Ext;

const TIME_CHECK_INTERVAL_MILLIS: u32 = 5000;
const TIME_CHECK_MILLIS_PER_DELAY: u32 = 100;

const UWB_CHANNEL_PAGE: ChannelPage = ChannelPage::Uwb;

pub struct DW1000Phy<SPI: SpiDevice, IRQ: Wait, DELAY: DelayNs> {
    dw1000: DW1000<SPI>,
    irq: IRQ,
    delay: DELAY,
    last_instant: u64,
    millis_until_next_time_check: u32,

    current_tx_config: TxConfig,
    current_rx_config: RxConfig,
    phy_pib: PhyPib,
}

impl<SPI: SpiDevice, IRQ: Wait, DELAY: DelayNs> DW1000Phy<SPI, IRQ, DELAY> {
    pub async fn new(spi: SPI, irq: IRQ, mut delay: DELAY) -> Result<Self, Error<SPI, IRQ>>
    where
        DELAY: DelayNsSync,
    {
        let dw1000 = dw1000::DW1000::new(spi).init(&mut delay)?;

        Self::new_from_existing(dw1000, irq, delay).await
    }

    pub async fn new_from_existing(
        dw1000: dw1000::DW1000<SPI, Ready>,
        irq: IRQ,
        delay: DELAY,
    ) -> Result<Self, Error<SPI, IRQ>> {
        let mut s = Self {
            dw1000: DW1000::Ready(dw1000),
            irq,
            delay,
            last_instant: 0,
            millis_until_next_time_check: TIME_CHECK_INTERVAL_MILLIS,

            current_tx_config: TxConfig::default(),
            current_rx_config: RxConfig::default(),
            phy_pib: PhyPib::unspecified_new(), // TODO: Init with capabilities of this chip
        };

        s.reset().await?;

        Ok(s)
    }

    async fn convert_to_mac_time(
        &mut self,
        time: dw1000::time::Instant,
    ) -> Result<Instant, Error<SPI, IRQ>> {
        let current_time = self.get_instant().await?;
        let current_low_bits = current_time.ticks() & dw1000::time::TIME_MAX;
        let current_high_bits = current_time.ticks() & !dw1000::time::TIME_MAX;

        let time = time.value();

        let mac_time = match time > current_low_bits {
            true => current_high_bits | time,
            // Time has wrapped
            false => (current_high_bits + dw1000::time::TIME_MAX + 1) | time,
        };

        Ok(Instant::from_ticks(mac_time))
    }
}

impl<SPI: SpiDevice, IRQ: Wait, DELAY: DelayNs> Phy for DW1000Phy<SPI, IRQ, DELAY> {
    type Error = Error<SPI, IRQ>;

    type ProcessingContext = Either<Result<(), IRQ::Error>, ()>;

    const MODULATION: ModulationType = ModulationType::BPSK;

    async fn reset(&mut self) -> Result<(), Self::Error> {
        // Assumptions:
        // Always using 850kbps datarate
        // Always using 16mhz PRF
        // Always using 1024 preamble length

        const NUM_PREAMBLE_SYMBOLS: u32 = 31; // Valid for PRF16
        const NUM_SFD_SYMBOLS: u32 = 8; // For 850kbps datarate
        const SYMBOLS_PER_OCTET: f32 = 9.17648; // Not too sure... This is `8 * (1s / symbol period in secs (Tdsym)) / 850_000`
        const SHR_DURATION: u32 = NUM_PREAMBLE_SYMBOLS + NUM_SFD_SYMBOLS;
        let max_frame_duration = SHR_DURATION
            + (((lr_wpan_rs::consts::MAX_PHY_PACKET_SIZE + 1) as f32 * SYMBOLS_PER_OCTET).ceil()
                as u32);

        self.phy_pib = PhyPib {
            pib_write: PhyPibWrite {
                current_channel: 5,
                tx_power_tolerance: TXPowerTolerance::DB6, // TODO: Not reflected in hardware
                tx_power: 0,                               // TODO: Not reflected in hardware
                cca_mode: CcaMode::Aloha,                  // TODO: Not reflected in driver
                current_page: UWB_CHANNEL_PAGE,
                uwb_current_pulse_shape: UwbCurrentPulseShape::Mandatory, // Only supported shape
                uwb_cou_pulse: lr_wpan_rs::pib::UwbCouPulse::CCh1,
                uwb_cs_pulse: lr_wpan_rs::pib::UwbCsPulse::No1,
                uwb_lcp_weight1: 0,
                uwb_lcp_weight2: 0,
                uwb_lcp_weight3: 0,
                uwb_lcp_weight4: 0,
                uwb_lcp_delay2: 0,
                uwb_lcp_delay3: 0,
                uwb_lcp_delay4: 0,
                current_code: 0, // TODO: Not reflected in hardware
                native_prf: NativePrf::Prf16,
                uwb_scan_bins_per_channel: 0,
                uwb_inserted_preamble_interval: 0,
                tx_rmarker_offset: 0,
                rx_rmarker_offset: 0,
                rframe_processing_time: 0,
                cca_duration: 0,
            },
            channels_supported: &[ChannelDescription {
                page: UWB_CHANNEL_PAGE,
                channel_numbers: &[1, 2, 3, 4, 5, 7],
            }],
            max_frame_duration,
            shr_duration: SHR_DURATION,
            symbols_per_octet: SYMBOLS_PER_OCTET,
            preamble_symbol_length: 0, // 31 for PRF16 and 127 for PRF64 (but only PRF16 is ever used)
            uwb_data_rates_supported: &[0b00, 0b01, 0b10],
            css_low_data_rate_supported: false,
            uwb_cou_supported: false,
            uwb_cs_supported: false,
            uwb_lcp_supported: false,
            ranging: true,
            ranging_crystal_offset: false, // TODO: Not yet implemented
            ranging_dps: true,
        };

        self.current_rx_config = RxConfig {
            bitrate: dw1000::configs::BitRate::Kbps850,
            frame_filtering: false,
            pulse_repetition_frequency: PulseRepetitionFrequency::Mhz16,
            expected_preamble_length: dw1000::configs::PreambleLength::Symbols1024,
            channel: dw1000::configs::UwbChannel::Channel5,
            sfd_sequence: dw1000::configs::SfdSequence::IEEE,
            append_crc: false,
        };
        self.current_tx_config = TxConfig {
            bitrate: dw1000::configs::BitRate::Kbps850,
            ranging_enable: true,
            pulse_repetition_frequency: PulseRepetitionFrequency::Mhz16,
            preamble_length: dw1000::configs::PreambleLength::Symbols1024,
            channel: dw1000::configs::UwbChannel::Channel5,
            sfd_sequence: dw1000::configs::SfdSequence::IEEE,
            append_crc: false,
        };

        // Apply the configs
        self.update_phy_pib(|_| {}).await?;

        Ok(())
    }

    async fn get_instant(&mut self) -> Result<lr_wpan_rs::time::Instant, Self::Error> {
        let sys_time = match &mut self.dw1000 {
            DW1000::Empty => return Err(Error::WrongState),
            DW1000::Ready(dw1000) => dw1000.sys_time()?.value(),
            DW1000::Receiving(dw1000) => dw1000.sys_time()?.value(),
        };

        let mut last_major_bits = self.last_instant & !dw1000::time::TIME_MAX;
        let last_minor_bits = self.last_instant & dw1000::time::TIME_MAX;

        if sys_time < last_minor_bits {
            // Wraparound has happened
            last_major_bits += dw1000::time::TIME_MAX + 1;
        }

        let current_time = last_major_bits | sys_time;

        self.last_instant = current_time;
        self.millis_until_next_time_check = TIME_CHECK_INTERVAL_MILLIS;

        Ok(Instant::from_ticks(current_time))
    }

    fn symbol_period(&self) -> Duration {
        Duration::from_ticks(65536)
    }

    async fn send(
        &mut self,
        data: &[u8],
        send_time: Option<lr_wpan_rs::time::Instant>,
        ranging: bool,
        use_csma: bool,
        continuation: lr_wpan_rs::phy::SendContinuation,
    ) -> Result<lr_wpan_rs::phy::SendResult, Self::Error> {
        assert!(!use_csma, "Not supported");
        assert!(
            !matches!(continuation, SendContinuation::WaitForResponse { .. }),
            "Not yet implemented"
        );

        let send_time = match send_time {
            Some(target_time) => {
                let now = self.get_instant().await?;
                let time_diff = target_time.duration_since(now);
                const MAX_TIME_DIFF: Duration = Duration::from_ticks(dw1000::time::TIME_MAX as i64);
                const MIN_TIME_DIFF: Duration = Duration::from_millis(10);

                if time_diff > MAX_TIME_DIFF {
                    return Err(Error::TimeTooFarInFuture);
                }

                if time_diff < MIN_TIME_DIFF {
                    return Err(Error::TimeTooCloseInFuture);
                }

                dw1000::hl::SendTime::Delayed(
                    dw1000::time::Instant::new(target_time.ticks() & dw1000::time::TIME_MAX)
                        .unwrap(),
                )
            }
            None => dw1000::hl::SendTime::Now,
        };

        self.stop_receive().await?;

        self.current_tx_config.ranging_enable = ranging;
        let mut dw1000 = self.dw1000.take_ready().ok_or(Error::WrongState)?;
        dw1000.enable_tx_interrupts()?;

        let mut dw1000 = dw1000.send_raw(
            |buffer| {
                buffer[..data.len()].copy_from_slice(data);
                data.len()
            },
            send_time,
            self.current_tx_config,
        )?;

        let raw_tx_time = loop {
            self.irq.wait_for_high().await.map_err(|e| Error::Irq(e))?;
            match dw1000.wait_transmit() {
                Ok(raw_tx_time) => break raw_tx_time,
                Err(nb::Error::WouldBlock) => continue,
                Err(nb::Error::Other(e)) => return Err(e.into()),
            }
        };

        self.dw1000 = match dw1000.finish_sending() {
            Ok(dw1000) => DW1000::Ready(dw1000),
            Err((_dw1000, e)) => {
                // No real recovery possible...
                #[cfg(feature = "defmt-03")]
                defmt::panic!("Could not finish sending: {}", defmt::Debug2Format(&e));
                #[cfg(not(feature = "defmt-03"))]
                panic!("Could not finish sending: {:?}", e);
            }
        };

        let tx_time = self.convert_to_mac_time(raw_tx_time).await?;

        if matches!(continuation, SendContinuation::ReceiveContinuous) {
            // This should use the hardware acceleration, but driver doesn't implement that
            self.start_receive().await?;
        }

        Ok(lr_wpan_rs::phy::SendResult::Success(tx_time, None))
    }

    async fn start_receive(&mut self) -> Result<(), Self::Error> {
        let mut ready_radio = self.dw1000.take_ready().ok_or(Error::WrongState)?;

        ready_radio.enable_rx_interrupts()?;

        self.dw1000 =
            DW1000::Receiving(ready_radio.receive_auto_double_buffered(self.current_rx_config)?);

        Ok(())
    }

    async fn stop_receive(&mut self) -> Result<(), Self::Error> {
        if let Some(dw1000) = self.dw1000.take_receiving() {
            match dw1000.finish_receiving() {
                Ok(dw1000) => self.dw1000 = DW1000::Ready(dw1000),
                Err((dw1000, e)) => {
                    self.dw1000 = DW1000::Receiving(dw1000);
                    return Err(e.into());
                }
            };
        }

        Ok(())
    }

    async fn wait(&mut self) -> Result<Self::ProcessingContext, Self::Error> {
        let wait_for_time = async {
            while self.millis_until_next_time_check > 0 {
                self.millis_until_next_time_check = self
                    .millis_until_next_time_check
                    .saturating_sub(TIME_CHECK_MILLIS_PER_DELAY);
                self.delay.delay_ms(TIME_CHECK_MILLIS_PER_DELAY).await;
            }
        };

        // Do the cancellable waiting
        Ok(select(self.irq.wait_for_high(), wait_for_time).await)
    }

    async fn process(
        &mut self,
        ctx: Self::ProcessingContext,
    ) -> Result<Option<ReceivedMessage>, Self::Error> {
        match ctx {
            Either::First(irq_result) => {
                // Propagate the irq error if any
                irq_result.map_err(Error::Irq)?;

                match &mut self.dw1000 {
                    DW1000::Empty => {
                        // Spurious interrupt?
                    }
                    DW1000::Ready(dw1000) => {
                        // Spurious interrupt?
                        dw1000.disable_interrupts()?;
                    }
                    DW1000::Receiving(dw1000) => {
                        let mut buffer = [0; 127];
                        return match dw1000.wait_receive_raw(&mut buffer) {
                            Ok(message) => {
                                let timestamp = self.convert_to_mac_time(message.rx_time).await?;

                                Ok(Some(lr_wpan_rs::phy::ReceivedMessage {
                                    timestamp,
                                    data: message.bytes.try_into().unwrap(),
                                    lqi: 255, // TODO
                                    channel: self.phy_pib.current_channel,
                                    page: self.phy_pib.current_page,
                                }))
                            }
                            Err(nb::Error::WouldBlock) => {
                                // Just wait a bit more
                                Ok(None)
                            }
                            Err(nb::Error::Other(e)) => Err(e.into()),
                        };
                    }
                }

                Ok(None)
            }
            Either::Second(_check_for_time) => {
                // Get the current time so it can do the wraparound bookkeeping
                self.get_instant().await?;
                Ok(None)
            }
        }
    }

    async fn update_phy_pib<U>(
        &mut self,
        f: impl FnOnce(&mut lr_wpan_rs::pib::PhyPibWrite) -> U,
    ) -> Result<U, Self::Error> {
        let old_pib = self.phy_pib.pib_write.clone();
        let old_rx_config = self.current_rx_config;
        let old_tx_config = self.current_tx_config;

        let return_value = f(&mut self.phy_pib.pib_write);

        let update_settings = || {
            let PhyPibWrite {
                current_channel,
                tx_power_tolerance,
                tx_power,
                cca_mode,
                current_page,
                uwb_current_pulse_shape,
                uwb_cou_pulse,
                uwb_cs_pulse,
                uwb_lcp_weight1,
                uwb_lcp_weight2,
                uwb_lcp_weight3,
                uwb_lcp_weight4,
                uwb_lcp_delay2,
                uwb_lcp_delay3,
                uwb_lcp_delay4,
                current_code,
                native_prf,
                uwb_scan_bins_per_channel,
                uwb_inserted_preamble_interval,
                tx_rmarker_offset,
                rx_rmarker_offset,
                rframe_processing_time,
                cca_duration,
            } = &self.phy_pib.pib_write;

            // Set current channel
            self.current_tx_config.channel = (*current_channel)
                .try_into()
                .map_err(|_| Error::UnsupportedChannelNumber)?;
            self.current_rx_config.channel = self.current_tx_config.channel;

            // TODO: TX power (not yet implemented in driver)
            let _ = (tx_power_tolerance, tx_power);

            // Ignore cca_mode and co (only used in transmit function)
            let _ = (cca_mode, uwb_inserted_preamble_interval, cca_duration);

            if *current_page != UWB_CHANNEL_PAGE {
                return Err(Error::UnsupportedChannelPage);
            }

            if *uwb_current_pulse_shape != UwbCurrentPulseShape::Mandatory {
                let _ = (uwb_cou_pulse, uwb_cs_pulse);
                let _ = (
                    uwb_lcp_weight1,
                    uwb_lcp_weight2,
                    uwb_lcp_weight3,
                    uwb_lcp_weight4,
                    uwb_lcp_delay2,
                    uwb_lcp_delay3,
                    uwb_lcp_delay4,
                );
                return Err(Error::UnsupportedCurrentPulseShape);
            }

            // TODO: SHR SYNC pattern code. Driver now autoselects: UwbChannel::get_recommended_preamble_code
            let _ = current_code;

            // Set the PRF
            // This is different in 2020 version where PRF is given along the sap messages instead of PIB
            // Also, 2011 doesn't support 64-Mhz
            self.current_tx_config.pulse_repetition_frequency = match native_prf {
                NativePrf::NonUwb => return Err(Error::UnsupportedPrf),
                NativePrf::Prf4 => return Err(Error::UnsupportedPrf),
                NativePrf::Prf16 => PulseRepetitionFrequency::Mhz16,
                NativePrf::NoPreference => PulseRepetitionFrequency::Mhz16,
            };
            self.current_rx_config.pulse_repetition_frequency =
                self.current_tx_config.pulse_repetition_frequency;

            // Used by scan, but not something we have to use now
            let _ = uwb_scan_bins_per_channel;

            // Nothing to react to
            let _ = rframe_processing_time;

            if let Some(dw1000) = self.dw1000.take_receiving() {
                self.dw1000 = DW1000::Ready(dw1000.finish_receiving().unwrap());
            }
            self.dw1000.as_ready_mut().unwrap().set_antenna_delay(
                (*rx_rmarker_offset)
                    .try_into()
                    .map_err(|_| Error::RMarkerOffsetTooLarge)?,
                (*tx_rmarker_offset)
                    .try_into()
                    .map_err(|_| Error::RMarkerOffsetTooLarge)?,
            )?;

            Ok(return_value)
        };

        match update_settings() {
            Ok(return_value) => Ok(return_value),
            Err(e) => {
                self.phy_pib.pib_write = old_pib;
                self.current_rx_config = old_rx_config;
                self.current_tx_config = old_tx_config;

                Err(e)
            }
        }
    }

    fn get_phy_pib(&mut self) -> &lr_wpan_rs::pib::PhyPib {
        &self.phy_pib
    }
}

enum DW1000<SPI> {
    Empty,
    Ready(dw1000::DW1000<SPI, Ready>),
    Receiving(dw1000::DW1000<SPI, AutoDoubleBufferReceiving>),
}

impl<SPI> DW1000<SPI> {
    fn take_ready(&mut self) -> Option<dw1000::DW1000<SPI, Ready>> {
        match core::mem::replace(self, DW1000::Empty) {
            Self::Ready(v) => Some(v),
            val => {
                *self = val;
                None
            }
        }
    }
    fn take_receiving(&mut self) -> Option<dw1000::DW1000<SPI, AutoDoubleBufferReceiving>> {
        match core::mem::replace(self, DW1000::Empty) {
            Self::Receiving(v) => Some(v),
            val => {
                *self = val;
                None
            }
        }
    }

    fn as_ready_mut(&mut self) -> Option<&mut dw1000::DW1000<SPI, Ready>> {
        if let Self::Ready(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

pub enum Error<SPI: SpiDevice, IRQ: ErrorType> {
    DW1000(dw1000::Error<SPI>),
    Irq(IRQ::Error),
    WrongState,
    UnsupportedChannelNumber,
    UnsupportedChannelPage,
    UnsupportedCurrentPulseShape,
    UnsupportedPrf,
    RMarkerOffsetTooLarge,
    TimeTooFarInFuture,
    TimeTooCloseInFuture,
}

impl<SPI: SpiDevice, IRQ: ErrorType> From<dw1000::Error<SPI>> for Error<SPI, IRQ> {
    fn from(v: dw1000::Error<SPI>) -> Self {
        Self::DW1000(v)
    }
}

#[cfg(feature = "defmt-03")]
impl<SPI: SpiDevice, IRQ: ErrorType> defmt::Format for Error<SPI, IRQ> {
    fn format(&self, fmt: defmt::Formatter) {
        match self {
            Error::DW1000(error) => defmt::write!(fmt, "DW1000: {}", defmt::Debug2Format(error)),
            Error::Irq(error) => defmt::write!(fmt, "Irq: {}", defmt::Debug2Format(error)),
            Error::WrongState => defmt::write!(fmt, "WrongState"),
            Error::UnsupportedChannelNumber => defmt::write!(fmt, "UnsupportedChannelNumber"),
            Error::UnsupportedChannelPage => defmt::write!(fmt, "UnsupportedChannelPage"),
            Error::UnsupportedCurrentPulseShape => {
                defmt::write!(fmt, "UnsupportedCurrentPulseShape")
            }
            Error::UnsupportedPrf => defmt::write!(fmt, "UnsupportedPrf"),
            Error::RMarkerOffsetTooLarge => defmt::write!(fmt, "RMarkerOffsetTooLarge"),
            Error::TimeTooFarInFuture => defmt::write!(fmt, "TimeTooFarInFuture"),
            Error::TimeTooCloseInFuture => defmt::write!(fmt, "TimeTooCloseInFuture"),
        }
    }
}

impl<SPI: SpiDevice, IRQ: ErrorType> core::fmt::Debug for Error<SPI, IRQ> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::DW1000(arg0) => f.debug_tuple("DW1000").field(arg0).finish(),
            Error::Irq(arg0) => f.debug_tuple("Irq").field(arg0).finish(),
            Error::WrongState => f.debug_tuple("WrongState").finish(),
            Error::UnsupportedChannelNumber => f.debug_tuple("UnsupportedChannelNumber").finish(),
            Error::UnsupportedChannelPage => f.debug_tuple("UnsupportedChannelPage").finish(),
            Error::UnsupportedCurrentPulseShape => {
                f.debug_tuple("UnsupportedCurrentPulseShape").finish()
            }
            Error::UnsupportedPrf => f.debug_tuple("UnsupportedPrf").finish(),
            Error::RMarkerOffsetTooLarge => f.debug_tuple("RMarkerOffsetTooLarge").finish(),
            Error::TimeTooFarInFuture => f.debug_tuple("TimeTooFarInFuture").finish(),
            Error::TimeTooCloseInFuture => f.debug_tuple("TimeTooCloseInFuture").finish(),
        }
    }
}

impl<SPI: SpiDevice, IRQ: ErrorType> Display for Error<SPI, IRQ> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        <Self as Debug>::fmt(self, f)
    }
}

impl<SPI: SpiDevice, IRQ: ErrorType> core::error::Error for Error<SPI, IRQ> {}
