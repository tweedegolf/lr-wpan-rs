use heapless::Vec;

use crate::{
    pib::{PhyPib, PhyPibWrite},
    time::{Duration, Instant},
    ChannelPage,
};

pub trait Phy {
    #[cfg(not(feature = "defmt-03"))]
    type Error: core::error::Error;
    #[cfg(feature = "defmt-03")]
    type Error: core::error::Error + defmt::Format;

    type ProcessingContext;

    const MODULATION: ModulationType;

    /// Reset the phy and the pib back to the defeaults as if it was newly created.
    async fn reset(&mut self) -> Result<(), Self::Error>;

    /// Get the current time of the radio.
    /// This is not very accurate, but can be used for e.g. logging.
    async fn get_instant(&mut self) -> Result<Instant, Self::Error>;

    /// Get the amount of time each symbol takes.
    fn symbol_period(&self) -> Duration;

    /// Send some data.
    ///
    /// If the radio was receiving, it will automatically stop to do the transmission.
    ///
    /// - The `data` must be a valid MAC frame.
    /// - If `send_time` is some, then that must be the time at which the data is sent. This must be done as accurately as possible.
    /// - If `ranging` is true, then the ranging bit must be set.
    /// - If `use_csma` is true, then the carrier sense mechanism should be used. If the channel is busy, then the send is aborted and [SendResult::ChannelAccessFailure] is returned
    /// - The `continuation` specifies what the radio should do after the transmission
    ///
    /// The actual time the data frame was sent is returned. This needs to be accurate, especially when `ranging` is true
    async fn send(
        &mut self,
        data: &[u8],
        send_time: Option<Instant>,
        ranging: bool,
        use_csma: bool,
        continuation: SendContinuation,
    ) -> Result<SendResult, Self::Error>;

    /// Start the receiver of the radio.
    ///
    /// It will continuously receive messages according to the PIB settings.
    /// When PIB attributes are updated, the receiver must reflect them immediately,
    /// even if that disrupts the operation for a little bit.
    ///
    /// If this function is called when the radio is already receiving, then nothing should happen and the
    /// radio should continue receiving.
    ///
    /// A received message is returned in the [Self::process] function.
    async fn start_receive(&mut self) -> Result<(), Self::Error>;

    /// Stop the receiver and go back to idle mode
    async fn stop_receive(&mut self) -> Result<(), Self::Error>;

    /// Wait on something to happen. When not doing anything with the phy, this function should be running.
    /// The function is cancellable, so you can use it in a select while remaining to have access to the other functions
    /// of this trait.
    ///
    /// When this function is done, it returns a context that should be passed to [Self::process].
    async fn wait(&mut self) -> Result<Self::ProcessingContext, Self::Error>;

    /// Do some processing. This function ought to be called after the [Self::wait] function returned.
    /// This function is not cancel-safe.
    ///
    /// If a message was received, it is returned.
    async fn process(
        &mut self,
        ctx: Self::ProcessingContext,
    ) -> Result<Option<ReceivedMessage>, Self::Error>;

    /// Update the PIB values that are updatable accessible from the outside
    async fn update_phy_pib<U>(
        &mut self,
        f: impl FnOnce(&mut PhyPibWrite) -> U,
    ) -> Result<U, Self::Error>;
    /// Get all the PIB values available for reading
    fn get_phy_pib(&mut self) -> &PhyPib;
}

pub enum SendResult {
    /// The message has been sent successfully at the given time.
    ///
    /// If the [SendContinuation::WaitForResponse] was used, the response message, if received, is also passed back.
    /// Otherwise is must always be None.
    Success(Instant, Option<ReceivedMessage>),
    /// CSMA-CA was used and no suitable time to send the message was found
    ChannelAccessFailure,
}

#[derive(Clone, Copy, Debug)]
pub enum SendContinuation {
    /// Go back to idle
    Idle,
    /// Go into receive mode to receive one message.
    /// The radio must wait for the turnaround time to actually start the receiver.
    /// After that, the receiver stays on until a message is received or rx time exceeds the timeout value.
    ///
    /// This is useful for receiving acks.
    WaitForResponse {
        turnaround_time: Duration,
        timeout: Duration,
    },
    /// Immediately go back to receiving messages
    ReceiveContinuous,
}

pub struct ReceivedMessage {
    /// The time at which the message was received
    pub timestamp: Instant,
    pub data: Vec<u8, 127>,
    /// The LQI at which the network beacon was received. Lower values represent lower LQI, as defined in 8.2.6.
    pub lqi: u8,
    /// The channel on which the message was received
    pub channel: u8,
    pub page: ChannelPage,
}

pub enum ModulationType {
    BPSK,
    GFSK,
}

impl ModulationType {
    pub fn tx_control_active_duration(&self) -> u32 {
        match self {
            ModulationType::BPSK => 2000,
            ModulationType::GFSK => 10000,
        }
    }

    pub fn tx_control_pause_duration(&self) -> u32 {
        match self {
            ModulationType::BPSK => 2000,
            ModulationType::GFSK => 10000,
        }
    }
}
