//! The constants defined in tables 51 and 70

/// The number of symbols forming a superframe slot
/// when the superframe order is equal to zero, as
/// described in 5.1.1.1.
#[doc(alias = "aBaseSlotDuration")]
pub const BASE_SLOT_DURATION: u32 = 60;

/// The number of symbols forming a superframe when
/// the superframe order is equal to zero.
#[doc(alias = "aBaseSuperframeDuration")]
pub const BASE_SUPERFRAME_DURATION: u32 = BASE_SLOT_DURATION * NUM_SUPERFRAME_SLOTS;

/// The number of superframes in which a GTS descriptor
/// exists in the beacon frame of the PAN coordinator.
#[doc(alias = "aGTSDescPersistenceTime")]
pub const GTS_DESC_PERSISTENCE_TIME: u32 = 4;

/// The maximum number of octets added by the MAC
/// sublayer to the MAC payload of a beacon frame.
#[doc(alias = "aMaxBeaconOverhead")]
pub const MAX_BEACON_OVERHEAD: usize = 75;

/// The maximum size, in octets, of a beacon payload.
#[doc(alias = "aMaxBeaconPayloadLength")]
pub const MAX_BEACON_PAYLOAD_LENGTH: usize = MAX_PHY_PACKET_SIZE - MAX_BEACON_OVERHEAD;

/// The number of consecutive lost beacons that will
/// cause the MAC sublayer of a receiving device to
/// declare a loss of synchronization.
#[doc(alias = "aMaxLostBeacons")]
pub const MAX_LOST_BEACONS: u32 = 4;

/// The maximum number of octets that can be transmitted in the MAC Payload field of an unsecured MAC
/// frame that will be guaranteed not to exceed aMaxPHYPacketSize.
#[doc(alias = "aMaxMACSafePayloadSize")]
pub const MAX_MAC_SAFE_PAYLOAD_SIZE: usize = MAX_PHY_PACKET_SIZE - MAX_MPDU_UNSECURED_OVERHEAD;

/// The maximum number of octets that can be transmitted in the MAC Payload field.
#[doc(alias = "aMaxMACPayloadSize")]
pub const MAX_MAC_PAYLOAD_SIZE: usize = MAX_PHY_PACKET_SIZE - MIN_MPDU_OVERHEAD;

/// The maximum number of octets added by the MAC
/// sublayer to the PSDU without security.
#[doc(alias = "aMaxMPDUUnsecuredOverhead")]
pub const MAX_MPDU_UNSECURED_OVERHEAD: usize = 25;

/// The maximum size of an MPDU, in octets, that can be
/// followed by a SIFS period.
#[doc(alias = "aMaxSIFSFrameSize")]
pub const MAX_SIFS_FRAME_SIZE: u32 = 18;

/// The minimum number of symbols forming the CAP.
/// This ensures that MAC commands can still be transferred to devices when GTSs are being used.
/// An exception to this minimum shall be allowed for the
/// accommodation of the temporary increase in the beacon frame length needed to perform GTS maintenance, as described in 5.2.2.1.3.
#[doc(alias = "aMinCAPLength")]
pub const MIN_CAP_LENGTH: u32 = 440;

/// The minimum number of octets added by the MAC
/// sublayer to the PSDU.
#[doc(alias = "aMinMPDUOverhead")]
pub const MIN_MPDU_OVERHEAD: usize = 9;

/// The number of slots contained in any superframe.
#[doc(alias = "aNumSuperframeSlots")]
pub const NUM_SUPERFRAME_SLOTS: u32 = 16;

/// The number of symbols forming the basic time period
/// used by the CSMA-CA algorithm.
#[doc(alias = "aUnitBackoffPeriod")]
pub const UNIT_BACKOFF_PERIOD: u32 = 20;

/// The maximum PSDU size (in octets) the PHY shall be able to receive.
#[doc(alias = "aMaxPHYPacketSize")]
pub const MAX_PHY_PACKET_SIZE: usize = 127;

/// RX-to-TX or TX-to-RX turnaround time (in symbol periods), as
/// defined in 8.2.1 and 8.2.2.
#[doc(alias = "aTurnaroundTime")]
pub const TURNAROUND_TIME: u32 = 12;
