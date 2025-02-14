use arrayvec::ArrayVec;
use ieee802154::mac::beacon::PendingAddress;

use crate::consts::MAX_BEACON_PAYLOAD_LENGTH;

use super::{Indication, IndicationValue, PanDescriptor};

/// The MLME-BEACON-NOTIFY.indication primitive is used to send parameters contained within a beacon
/// frame received by the MAC sublayer to the next higher layer when either `macAutoRequest` is set to FALSE
/// or when the beacon frame contains one or more octets of payload. The primitive also sends a measure of the
/// LQI and the time the beacon frame was received.
#[derive(Debug, Clone)]
pub struct BeaconNotifyIndication {
    pub beacon_sequence_number: u8,
    pub pan_descriptor: PanDescriptor,
    /// The list of addresses of the devices for which the beacon source has data.
    pub address_list: PendingAddress,
    /// The set of octets comprising the beacon
    /// payload to be transferred from the MAC
    /// sublayer entity to the next higher layer.
    pub sdu: ArrayVec<u8, MAX_BEACON_PAYLOAD_LENGTH>,
}

impl From<IndicationValue> for BeaconNotifyIndication {
    fn from(value: IndicationValue) -> Self {
        match value {
            IndicationValue::BeaconNotify(val) => val,
            _ => panic!("Bad cast"),
        }
    }
}

impl Indication for BeaconNotifyIndication {
    type Response = ();
}
