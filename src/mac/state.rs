use arraydeque::ArrayDeque;
use arrayvec::ArrayVec;
use ieee802154::mac::{
    beacon::{GuaranteedTimeSlotInformation, PendingAddress},
    security::{default::Unimplemented, SecurityContext},
    FooterMode, FrameSerDesContext,
};
use rand_core::RngCore;

use crate::{sap::SecurityInfo, time::DelayNsExt};

use super::{callback::SendCallback, mlme_scan::ScanProcess, MacConfig};

pub struct MacState<'a> {
    pub message_scheduler: MessageScheduler<'a>,
    /// The security info of the beacons this mac is sending
    pub beacon_security_info: SecurityInfo,
    /// If true, the beacon of the coordinator this device is associated to is actively being tracked
    pub coordinator_beacon_tracked: bool,
    /// If and how this device sends out beacons
    pub beacon_mode: BeaconMode,
    /// Are we the pan coordinator?
    pub is_pan_coordinator: bool,
    /// Our current GTS setup we send out in our beacons
    pub current_gts: GuaranteedTimeSlotInformation,
    /// Are we currently in our own superframe?
    pub own_superframe_active: bool,

    pub current_scan_process: Option<ScanProcess<'a>>,

    security_context: SecurityContext<Unimplemented, Unimplemented>,
}

impl MacState<'_> {
    pub fn new<Rng: RngCore, Delay: DelayNsExt>(config: &MacConfig<Rng, Delay>) -> Self {
        Self {
            message_scheduler: MessageScheduler {
                scheduled_broadcasts: ArrayDeque::new(),
            },
            beacon_security_info: Default::default(),
            coordinator_beacon_tracked: false,
            beacon_mode: BeaconMode::Off,
            security_context: SecurityContext::new(config.extended_address.0, 0, Unimplemented),
            is_pan_coordinator: false,
            current_gts: GuaranteedTimeSlotInformation::new(),
            own_superframe_active: false,
            current_scan_process: None,
        }
    }

    fn frame_ser_des_context(&mut self) -> FrameSerDesContext<'_, Unimplemented, Unimplemented> {
        FrameSerDesContext::new(FooterMode::None, Some(&mut self.security_context))
    }

    pub fn serialize_frame(
        &mut self,
        frame: ieee802154::mac::Frame<'_>,
    ) -> ArrayVec<u8, { crate::consts::MAX_PHY_PACKET_SIZE }> {
        use byte::TryWrite;

        let mut buffer = ArrayVec::from([0; crate::consts::MAX_PHY_PACKET_SIZE]);
        let length = frame
            .try_write(&mut buffer, &mut self.frame_ser_des_context())
            .expect("Buffer is always big enough");
        buffer.truncate(length);

        buffer
    }

    pub fn deserialize_frame<'data>(
        &mut self,
        data: &'data mut [u8],
    ) -> Option<ieee802154::mac::Frame<'data>> {
        match ieee802154::mac::Frame::try_read_and_unsecure(
            data,
            &mut self.frame_ser_des_context(),
            &mut Unimplemented,
        ) {
            Ok((frame, _)) => Some(frame),
            Err(e) => {
                #[cfg(feature = "defmt-03")]
                warn!("Could not deserialize a frame: {}", defmt::Debug2Format(&e));
                #[cfg(not(feature = "defmt-03"))]
                warn!("Could not deserialize a frame: {:?}", e);

                None
            }
        }
    }
}

/// The central coordinator for scheduling messages
pub struct MessageScheduler<'a> {
    /// All the broadcast messages that are scheduled.
    ///
    /// If the PAN is beacon-enabled, one of the messages are popped off
    /// and sent after the beacon (which will have its frame-pending bit set).
    ///
    /// If the PAN is not beacon-enabled, the message will be sent immediately.
    ///
    /// The messages are sent using CSMA-CA.
    scheduled_broadcasts: ArrayDeque<ScheduledMessage<'a>, 4>,
}

impl<'a> MessageScheduler<'a> {
    pub fn schedule_broadcast_priority(
        &mut self,
        data: ArrayVec<u8, { crate::consts::MAX_PHY_PACKET_SIZE }>,
        callback: SendCallback<'a>,
    ) {
        if self
            .scheduled_broadcasts
            .push_front(ScheduledMessage { data, callback })
            .is_err()
        {
            panic!("scheduled_broadcasts reached capacity");
        }
    }

    #[expect(dead_code, reason = "for future use")]
    pub fn schedule_broadcast(
        &mut self,
        data: ArrayVec<u8, { crate::consts::MAX_PHY_PACKET_SIZE }>,
        callback: SendCallback<'a>,
    ) {
        if self
            .scheduled_broadcasts
            .push_front(ScheduledMessage { data, callback })
            .is_err()
        {
            panic!("scheduled_broadcasts reached capacity");
        }
    }

    pub fn has_broadcast_scheduled(&self) -> bool {
        !self.scheduled_broadcasts.is_empty()
    }

    pub fn take_scheduled_broadcast(&mut self) -> Option<ScheduledMessage<'a>> {
        self.scheduled_broadcasts.pop_front()
    }

    pub fn get_pending_addresses(&self) -> PendingAddress {
        PendingAddress::new()
    }
}

pub struct ScheduledMessage<'a> {
    pub data: ArrayVec<u8, { crate::consts::MAX_PHY_PACKET_SIZE }>,
    pub callback: SendCallback<'a>,
}

#[derive(Debug, Clone, Copy)]
pub enum BeaconMode {
    /// No beacon will be sent out
    Off,
    /// A beacon will be sent out according to the mac pib on its own time schedule.
    OnAutonomous,
    /// A beacon will be sent out after every tracked beacon with the given `start_time` offset.
    /// This is only valid if [MacState::pan_coordinator_beacon_tracked] is set to true.
    #[expect(dead_code, reason = "for future use")]
    OnTracking { start_time: u32 },
}
