#![no_std]

use embassy_time::Delay;
use embedded_hal::{
    digital::{InputPin, OutputPin},
    spi::SpiDevice,
};
use embedded_hal_async::digital::Wait;
use lr_wpan_rs::{
    phy::{Phy, ReceivedMessage},
    pib::{ChannelDescription, PhyPib, PhyPibWrite},
    time::{Duration, Instant},
};
use s2lp::{
    S2lp,
    packet_format::{Ieee802154G, Ieee802154GConfig},
    states::{Ready, Rx, Shutdown, Tx, rx::RxResult},
};

const NUM_PREAMBLE_BITS: u16 = 16 * 8;
const NUM_SFD_BITS: u8 = 16;
const DATARATE: u32 = 50_000;

pub struct S2lpPhy<'buffer, Spi: SpiDevice, Sdn: OutputPin, Gpio: InputPin + Wait> {
    radio: RuntimeS2lp<'buffer, Spi, Sdn, Gpio>,
    buffer: &'buffer mut [u8],
    phy_pib: PhyPib,
}

impl<'buffer, Spi: SpiDevice, Sdn: OutputPin, Gpio: InputPin + Wait>
    S2lpPhy<'buffer, Spi, Sdn, Gpio>
{
    pub async fn new(
        radio: S2lp<Shutdown, Spi, Sdn, Gpio, Delay>,
        radio_xtal_frequency: u32,
        buffer: &'buffer mut [u8],
    ) -> Result<Self, <Self as Phy>::Error> {
        // Setup according to SUN FSK PHY

        // phyFskPreambleLength = 16
        // phySunFskSfd = 0
        // No mode switch (not supported on radio)
        // Band 866 Mhz, operating mode #1

        let radio = radio
            .init(s2lp::states::shutdown::Config {
                xtal_frequency: radio_xtal_frequency,
                base_frequency: 865_100_000,
                modulation: s2lp::ll::ModulationType::Fsk2,
                datarate: DATARATE,
                frequency_deviation: 12_500,
                bandwidth: 100_000,
            })
            .await?;

        let radio = radio.set_format(&Ieee802154GConfig {
            preamble_length: NUM_PREAMBLE_BITS,
            preamble_pattern: s2lp::packet_format::PreamblePattern::Pattern0,
            sync_length: NUM_SFD_BITS,
            sync_pattern: 0b0110_1111_0100_1110,
            crc_mode: s2lp::ll::CrcMode::CrcPoly0X04C011Bb7,
            data_whitening: true,
        })?;

        let mut s = Self {
            radio: RuntimeS2lp::Ready(radio),
            buffer,
            phy_pib: PhyPib::unspecified_new(),
        };

        s.reset().await?;

        Ok(s)
    }
}

impl<'buffer, Spi: SpiDevice, Sdn: OutputPin, Gpio: InputPin + Wait> Phy
    for S2lpPhy<'buffer, Spi, Sdn, Gpio>
{
    type Error = s2lp::Error<Spi::Error, Sdn::Error, Gpio::Error>;

    type ProcessingContext = embassy_time::Instant;

    const MODULATION: lr_wpan_rs::phy::ModulationType = lr_wpan_rs::phy::ModulationType::GFSK;

    async fn reset(&mut self) -> Result<(), Self::Error> {
        const SHR_DURATION: u32 = NUM_PREAMBLE_BITS + NUM_SFD_BITS;
        const SYMBOLS_PER_OCTET: f32 = 8.0;

        self.phy_pib = PhyPib {
            pib_write: PhyPibWrite {
                current_channel: 0,
                tx_power_tolerance: lr_wpan_rs::pib::TXPowerTolerance::DB3,
                tx_power: 0, // TODO: Not yet implemented in driver
                cca_mode: lr_wpan_rs::pib::CcaMode::EnergyAboveThreshold,
                current_page: lr_wpan_rs::ChannelPage::Sun866MhzMode1,
                uwb_current_pulse_shape: lr_wpan_rs::pib::UwbCurrentPulseShape::Mandatory,
                uwb_cou_pulse: lr_wpan_rs::pib::UwbCouPulse::CCh1,
                uwb_cs_pulse: lr_wpan_rs::pib::UwbCsPulse::No1,
                uwb_lcp_weight1: 0,
                uwb_lcp_weight2: 0,
                uwb_lcp_weight3: 0,
                uwb_lcp_weight4: 0,
                uwb_lcp_delay2: 0,
                uwb_lcp_delay3: 0,
                uwb_lcp_delay4: 0,
                current_code: 0,
                native_prf: lr_wpan_rs::pib::NativePrf::NoPreference,
                uwb_scan_bins_per_channel: 0,
                uwb_inserted_preamble_interval: 0,
                tx_rmarker_offset: 0,
                rx_rmarker_offset: 0,
                rframe_processing_time: 1,
                cca_duration: 0,
            },
            channels_supported: &[ChannelDescription {
                page: lr_wpan_rs::ChannelPage::Sun866MhzMode1,
                channel_numbers: &[0],
            }],
            max_frame_duration: const {
                SHR_DURATION
                    + f32::ceil(
                        (lr_wpan_rs::consts::MAX_PHY_PACKET_SIZE + 1) as f32 * SYMBOLS_PER_OCTET,
                    ) as u32
            },
            shr_duration: SHR_DURATION,
            symbols_per_octet: SYMBOLS_PER_OCTET,
            preamble_symbol_length: 0,
            uwb_data_rates_supported: &[],
            css_low_data_rate_supported: false,
            uwb_cou_supported: false,
            uwb_cs_supported: false,
            uwb_lcp_supported: false,
            ranging: false,
            ranging_crystal_offset: false,
            ranging_dps: false,
        };

        self.radio
            .if_rx(async |radio| Ok(radio.abort().ok().unwrap()))
            .await?;
        self.radio
            .if_tx(async |radio| Ok(radio.abort().ok().unwrap()))
            .await?;

        Ok(())
    }

    async fn get_instant(&mut self) -> Result<Instant, Self::Error> {
        Ok(convert_embassy_time_instant(embassy_time::Instant::now()))
    }

    fn symbol_period(&self) -> Duration {
        Duration::from_ticks(ticks)
    }

    async fn send(
        &mut self,
        data: &[u8],
        send_time: Option<Instant>,
        ranging: bool,
        use_csma: bool,
        continuation: lr_wpan_rs::phy::SendContinuation,
    ) -> Result<lr_wpan_rs::phy::SendResult, Self::Error> {
        if ranging {
            panic!("Ranging not supported");
        }

        if let Some(send_time) = send_time {
            let now = embassy_time::Instant::now();
            let sleep_duration = send_time.duration_since(convert_embassy_time_instant(now));

            if sleep_duration.ticks() > 0 {
                embassy_time::Timer::at(now + convert_wpan_duration(sleep_duration)).await;
            }
        }

        todo!("Send message")
    }

    async fn start_receive(&mut self) -> Result<(), Self::Error> {
        todo!()
    }

    async fn stop_receive(&mut self) -> Result<(), Self::Error> {
        todo!()
    }

    async fn wait(&mut self) -> Result<Self::ProcessingContext, Self::Error> {
        if let Some(radio) = self.radio.as_rx() {
            radio.wait_for_irq().await?;
        }

        Ok(embassy_time::Instant::now())
    }

    async fn process(
        &mut self,
        ctx: Self::ProcessingContext,
    ) -> Result<Option<lr_wpan_rs::phy::ReceivedMessage>, Self::Error> {
        let mut received_message = None;

        let current_channel = self.get_phy_pib().current_channel;
        let current_page = self.get_phy_pib().current_page;

        self.radio
            .if_rx(
                async |mut radio: S2lp<Rx<'buffer, Ieee802154G>, Spi, Sdn, Gpio, Delay>| {
                    let rx_result = match radio.wait().await {
                        Ok(rx_result) => rx_result,
                        Err(e) => return Err((radio.abort().unwrap(), e)),
                    };

                    match rx_result {
                        RxResult::Ok {
                            packet_size: packet_size @ 0..128,
                            rssi_value,
                            meta_data: _,
                        } => {
                            received_message = Some(ReceivedMessage {
                                timestamp: convert_embassy_time_instant(ctx),
                                data: self.buffer[..packet_size].try_into().unwrap(),
                                lqi: (rssi_value.clamp(-192, 64) + 192) as u8,
                                channel: current_channel,
                                page: current_page,
                            });
                        }
                        RxResult::RxAlreadyDone => unreachable!(),
                        _ => {
                            // Not something we can do anything with
                        }
                    }

                    Ok(radio.finish().ok().unwrap())
                },
            )
            .await?;

        Ok(received_message)
    }

    async fn update_phy_pib<U>(
        &mut self,
        f: impl FnOnce(&mut lr_wpan_rs::pib::PhyPibWrite) -> U,
    ) -> Result<U, Self::Error> {
        todo!()
    }

    fn get_phy_pib(&mut self) -> &lr_wpan_rs::pib::PhyPib {
        todo!()
    }
}

pub enum RuntimeS2lp<'buffer, Spi: SpiDevice, Sdn: OutputPin, Gpio: InputPin + Wait> {
    Empty,
    Ready(S2lp<Ready<Ieee802154G>, Spi, Sdn, Gpio, Delay>),
    Rx(S2lp<Rx<'buffer, Ieee802154G>, Spi, Sdn, Gpio, Delay>),
    Tx(S2lp<Tx<'buffer, Ieee802154G>, Spi, Sdn, Gpio, Delay>),
}

impl<'buffer, Spi: SpiDevice, Sdn: OutputPin, Gpio: InputPin + Wait>
    From<S2lp<Tx<'buffer, Ieee802154G>, Spi, Sdn, Gpio, Delay>>
    for RuntimeS2lp<'buffer, Spi, Sdn, Gpio>
{
    fn from(v: S2lp<Tx<'buffer, Ieee802154G>, Spi, Sdn, Gpio, Delay>) -> Self {
        Self::Tx(v)
    }
}

impl<'buffer, Spi: SpiDevice, Sdn: OutputPin, Gpio: InputPin + Wait>
    From<S2lp<Rx<'buffer, Ieee802154G>, Spi, Sdn, Gpio, Delay>>
    for RuntimeS2lp<'buffer, Spi, Sdn, Gpio>
{
    fn from(v: S2lp<Rx<'buffer, Ieee802154G>, Spi, Sdn, Gpio, Delay>) -> Self {
        Self::Rx(v)
    }
}

impl<'buffer, Spi: SpiDevice, Sdn: OutputPin, Gpio: InputPin + Wait>
    From<S2lp<Ready<Ieee802154G>, Spi, Sdn, Gpio, Delay>> for RuntimeS2lp<'buffer, Spi, Sdn, Gpio>
{
    fn from(v: S2lp<Ready<Ieee802154G>, Spi, Sdn, Gpio, Delay>) -> Self {
        Self::Ready(v)
    }
}

impl<'buffer, Spi: SpiDevice, Sdn: OutputPin, Gpio: InputPin + Wait>
    RuntimeS2lp<'buffer, Spi, Sdn, Gpio>
{
    pub async fn take_ready<Out: Into<Self>>(
        &mut self,
        f: impl AsyncFnOnce(S2lp<Ready<Ieee802154G>, Spi, Sdn, Gpio, Delay>) -> Out,
    ) {
        let Self::Ready(radio) = core::mem::replace(self, Self::Empty) else {
            panic!("Radio not in ready state");
        };

        *self = f(radio).await.into()
    }

    pub async fn take_rx<Out: Into<Self>>(
        &mut self,
        f: impl AsyncFnOnce(S2lp<Rx<'buffer, Ieee802154G>, Spi, Sdn, Gpio, Delay>) -> Out,
    ) {
        let Self::Rx(radio) = core::mem::replace(self, Self::Empty) else {
            panic!("Radio not in rx state");
        };

        *self = f(radio).await.into()
    }

    pub async fn take_tx<Out: Into<Self>>(
        &mut self,
        f: impl AsyncFnOnce(S2lp<Tx<'buffer, Ieee802154G>, Spi, Sdn, Gpio, Delay>) -> Out,
    ) {
        let Self::Tx(radio) = core::mem::replace(self, Self::Empty) else {
            panic!("Radio not in tx state");
        };

        *self = f(radio).await.into()
    }

    pub async fn if_ready<Out: Into<Self>>(
        &mut self,
        f: impl AsyncFnOnce(
            S2lp<Ready<Ieee802154G>, Spi, Sdn, Gpio, Delay>,
        )
            -> Result<Out, (Out, s2lp::Error<Spi::Error, Sdn::Error, Gpio::Error>)>,
    ) -> Result<(), s2lp::Error<Spi::Error, Sdn::Error, Gpio::Error>> {
        match core::mem::replace(self, Self::Empty) {
            RuntimeS2lp::Ready(radio) => match f(radio).await {
                Ok(out) => *self = out.into(),
                Err((out, e)) => {
                    *self = out.into();
                    return Err(e);
                }
            },
            radio => *self = radio,
        }

        Ok(())
    }

    pub async fn if_rx<Out: Into<Self>>(
        &mut self,
        f: impl AsyncFnOnce(
            S2lp<Rx<'buffer, Ieee802154G>, Spi, Sdn, Gpio, Delay>,
        )
            -> Result<Out, (Out, s2lp::Error<Spi::Error, Sdn::Error, Gpio::Error>)>,
    ) -> Result<(), s2lp::Error<Spi::Error, Sdn::Error, Gpio::Error>> {
        match core::mem::replace(self, Self::Empty) {
            RuntimeS2lp::Rx(radio) => match f(radio).await {
                Ok(out) => *self = out.into(),
                Err((out, e)) => {
                    *self = out.into();
                    return Err(e);
                }
            },
            radio => *self = radio,
        }

        Ok(())
    }

    pub async fn if_tx<Out: Into<Self>>(
        &mut self,
        f: impl AsyncFnOnce(
            S2lp<Tx<'buffer, Ieee802154G>, Spi, Sdn, Gpio, Delay>,
        )
            -> Result<Out, (Out, s2lp::Error<Spi::Error, Sdn::Error, Gpio::Error>)>,
    ) -> Result<(), s2lp::Error<Spi::Error, Sdn::Error, Gpio::Error>> {
        match core::mem::replace(self, Self::Empty) {
            RuntimeS2lp::Tx(radio) => match f(radio).await {
                Ok(out) => *self = out.into(),
                Err((out, e)) => {
                    *self = out.into();
                    return Err(e);
                }
            },
            radio => *self = radio,
        }

        Ok(())
    }

    pub fn as_ready(&mut self) -> Option<&mut S2lp<Ready<Ieee802154G>, Spi, Sdn, Gpio, Delay>> {
        if let Self::Ready(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_rx(&mut self) -> Option<&mut S2lp<Rx<'buffer, Ieee802154G>, Spi, Sdn, Gpio, Delay>> {
        if let Self::Rx(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_tx(&mut self) -> Option<&mut S2lp<Tx<'buffer, Ieee802154G>, Spi, Sdn, Gpio, Delay>> {
        if let Self::Tx(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

fn convert_embassy_time_instant(instant: embassy_time::Instant) -> Instant {
    Instant::from_ticks(
        (instant.as_ticks() as f64 / embassy_time::TICK_HZ as f64
            * lr_wpan_rs::time::TICKS_PER_SECOND as f64) as u64,
    )
}

fn convert_wpan_duration(duration: Duration) -> embassy_time::Duration {
    embassy_time::Duration::from_ticks(
        (duration.ticks() as f64 / lr_wpan_rs::time::TICKS_PER_SECOND as f64
            * embassy_time::TICK_HZ as f64) as u64,
    )
}
