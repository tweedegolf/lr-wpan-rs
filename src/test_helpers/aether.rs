//! Radio simulation infrastructure
//!
//! This module provides a simulated [Aether](https://en.wikipedia.org/wiki/Luminiferous_aether) to connect several radios.
//!
//! # Example
//! ```
//! use ieee_802_15_4_mac::phy::{Phy, SendContinuation, SendResult};
//! use ieee_802_15_4_mac::test_helpers::aether::{Aether, Coordinate, Meters};
//! use ieee_802_15_4_mac::time::Duration;
//!
//! # tokio::runtime::Builder::new_current_thread().enable_time().start_paused(true).build().unwrap().block_on(async {
//! let mut aether = Aether::new();
//!
//! // Create two new radios connected to the aether
//! let mut alice = aether.radio();
//! let mut bob = aether.radio();
//! bob.move_to(Coordinate::new(0.0, 299_792_458.0));
//!
//! bob.start_receive().await.unwrap();
//!
//! let tx_res = alice.send(b"Hello, world!", None, false, false, SendContinuation::Idle).await.unwrap();
//! let SendResult::Success(tx_time) = tx_res else { unreachable!() };
//!
//! let mut got_message = false;
//! let ctx = bob.wait().await.unwrap();
//!
//! if let Some(msg) = bob.process(ctx).await.unwrap() {
//!     assert_eq!(&msg.data[..], b"Hello, world!");
//!     assert_eq!(msg.timestamp, tx_time + Duration::from_seconds(1));
//!     got_message = true;
//! }
//!
//! assert!(got_message);
//! # });
//! ```

use crate::pib::PhyPib;
use crate::time::{Duration, Instant};
use alloc::borrow::Cow;
use arrayvec::ArrayVec;
use byte::TryRead;
use core::fmt::Debug;
use ieee802154::mac::Frame;
use pcap_file::pcapng::blocks::enhanced_packet::EnhancedPacketBlock;
use pcap_file::pcapng::blocks::interface_description::{
    InterfaceDescriptionBlock, InterfaceDescriptionOption,
};
use pcap_file::pcapng::{Block, PcapNgReader, PcapNgWriter};
use pcap_file::DataLink;
use std::collections::HashMap;
use std::fs::File;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use tokio::sync::mpsc::error::TrySendError;
use tokio::sync::mpsc::{channel, Sender};

mod radio;
mod space_time;

pub use radio::AetherRadio;
pub use space_time::{Coordinate, Meters};

/// A medium to which radios are connected
///
/// This takes care of routing the packets to the right radios.
pub struct Aether {
    inner: Arc<Mutex<AetherInner>>,
}

impl Default for Aether {
    fn default() -> Self {
        Self::new()
    }
}

impl Aether {
    /// Create a new empty aether
    pub fn new() -> Self {
        let inner = AetherInner {
            nodes: Default::default(),
            started: tokio::time::Instant::now(),
            pcap_dump: None,
        };

        Self {
            inner: Arc::new(Mutex::new(inner)),
        }
    }

    /// Create a radio which lives in the Aether
    pub fn radio(&mut self) -> AetherRadio {
        let (tx, rx) = channel(16);

        let pib = PhyPib::unspecified_new();
        let local_pib = pib.clone();
        let node = Node {
            position: Coordinate::default(),
            antenna: tx,
            pib,
            rx_enable: false,
        };
        let inner = Arc::clone(&self.inner);
        let node_id = NodeId::new();

        let old = self.inner().nodes.insert(node_id.clone(), node);
        assert!(old.is_none(), "node_id must be unique");

        AetherRadio {
            inner,
            node_id,
            antenna: rx,
            local_pib,
        }
    }

    pub fn start_trace(&mut self, file: File) {
        self.inner().start_trace(file);
    }

    pub fn stop_trace(&mut self) {
        self.inner().stop_trace();
    }

    pub fn parse_trace(&mut self, file: File) -> impl Iterator<Item = Frame<'static>> {
        let mut reader = PcapNgReader::new(file).unwrap();
        let mut current_data_link = DataLink::IEEE802_15_4_NOFCS;

        std::iter::from_fn(move || {
            while let Some(b) = reader.next_block() {
                let block = b.unwrap();

                match block {
                    Block::InterfaceDescription(interface_description_block) => {
                        current_data_link = interface_description_block.linktype
                    }
                    Block::EnhancedPacket(enhanced_packet_block) => {
                        if !matches!(
                            current_data_link,
                            DataLink::IEEE802_15_4_NOFCS
                                | DataLink::IEEE802_15_4
                                | DataLink::IEEE802_15_4_LINUX
                                | DataLink::IEEE802_15_4_NONASK_PHY
                                | DataLink::IEEE802_15_4_TAP
                        ) {
                            continue;
                        }
                        return Some(
                            Frame::try_read(
                                enhanced_packet_block.data.to_vec().leak(),
                                ieee802154::mac::FooterMode::None,
                            )
                            .unwrap()
                            .0,
                        );
                    }
                    _ => todo!(),
                }
            }

            None
        })
    }

    fn inner(&self) -> MutexGuard<AetherInner> {
        self.inner.lock().unwrap()
    }
}

pub struct AetherInner {
    nodes: HashMap<NodeId, Node>,
    started: tokio::time::Instant,
    pcap_dump: Option<(PcapNgWriter<File>, HashMap<NodeId, u32>)>,
}

impl Debug for AetherInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.debug_struct("AetherInner")
            .field("nodes", &self.nodes)
            .field("started", &self.started)
            .field("pcap_dump", &self.pcap_dump.as_ref().map(|(_, h)| ((), h)))
            .finish()
    }
}

impl AetherInner {
    fn now(&mut self) -> Instant {
        let since_start = tokio::time::Instant::now()
            .duration_since(self.started)
            .as_millis()
            .try_into()
            .expect("tests never run longer than i64::MAX milliseconds");

        // TODO: make this more accurate
        Instant::from_ticks(0) + Duration::from_millis(since_start)
    }

    pub fn start_trace(&mut self, file: File) {
        if self.pcap_dump.is_some() {
            panic!("Already capturing pcap");
        }
        self.pcap_dump = Some((PcapNgWriter::new(file).unwrap(), HashMap::new()));
    }

    pub fn stop_trace(&mut self) {
        self.pcap_dump = None;
    }

    fn trace(&mut self, node_id: &NodeId, pkt: &AirPacket) {
        let Some((pcap, nodes)) = &mut self.pcap_dump else {
            return;
        };

        let len = nodes.len();
        let interface_id = *nodes.entry(node_id.clone()).or_insert_with(|| {
            pcap.write_pcapng_block(InterfaceDescriptionBlock {
                linktype: DataLink::IEEE802_15_4_NOFCS,
                snaplen: 127,
                options: vec![InterfaceDescriptionOption::IfName(
                    format!("{node_id:?}").into(),
                )],
            })
            .unwrap();

            len as u32
        });

        let block = EnhancedPacketBlock {
            interface_id,
            timestamp: pkt.time_stamp.duration_since_epoch().into(),
            original_len: pkt.data.len().try_into().unwrap(),
            data: Cow::Borrowed(pkt.data.as_ref()),
            options: vec![],
        };
        pcap.write_pcapng_block(block).unwrap();
    }

    fn send(&mut self, from: &NodeId, data: AirPacket) -> Instant {
        self.trace(from, &data);

        let mut closed_radios = vec![];
        let from_pos = self.nodes.get(from).expect("sender always exists").position;

        for (to, node) in &self.nodes {
            if from == to || !node.rx_enable {
                continue;
            }

            let mut delayed_data = data.clone();
            let dist = node.position.dist(from_pos);
            delayed_data.time_stamp += dist.as_duration();

            match node.antenna.try_send(delayed_data) {
                Ok(()) => {}
                Err(TrySendError::Closed(_)) => closed_radios.push(to.clone()),
                Err(TrySendError::Full(_)) => {
                    log::warn!("Radio antenna of {to:?} is full")
                }
            }
        }

        for closed_radio in closed_radios {
            self.nodes.remove(&closed_radio);
        }

        self.now()
    }
}

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Clone)]
pub struct NodeId(usize);

impl NodeId {
    fn new() -> Self {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

        Self(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }
}

#[derive(Debug)]
pub struct Node {
    position: Coordinate,
    antenna: Sender<AirPacket>,
    pib: PhyPib,
    rx_enable: bool,
}

#[derive(Debug, Clone)]
pub struct AirPacket {
    pub data: ArrayVec<u8, 127>,
    pub time_stamp: Instant,
    pub channel: u8,
}

impl AirPacket {
    pub fn new(data: impl TryInto<ArrayVec<u8, 127>>, time_stamp: Instant, channel: u8) -> Self {
        let Ok(data) = data.try_into() else {
            unreachable!("Test data always fits 127 bytes");
        };

        Self {
            data,
            time_stamp,
            channel,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::phy::{Phy, ReceivedMessage, SendContinuation, SendResult};
    use byte::TryWrite;
    use ieee802154::mac;
    use ieee802154::mac::beacon::BeaconOrder::BeaconOrder;
    use ieee802154::mac::beacon::{GuaranteedTimeSlotInformation, PendingAddress, SuperframeOrder};
    use ieee802154::mac::security::default::Unimplemented;
    use ieee802154::mac::{FooterMode, FrameVersion};
    use pcap_file::pcapng::PcapNgReader;
    use tokio::time::timeout;

    async fn receive_one(bob: &mut AetherRadio) -> ReceivedMessage {
        let (tx, mut rx) = channel(1);
        let ctx = bob.wait().await.unwrap();
        if let Some(pkt) = bob.process(ctx).await.unwrap() {
            tx.try_send(pkt).unwrap();
            drop(tx);
        }

        let pkt = rx.try_recv().unwrap();

        assert!(rx.is_closed());
        pkt
    }

    #[tokio::test(start_paused = true)]
    async fn radios_are_connected() {
        let mut a = Aether::new();

        let mut alice = a.radio();
        let mut bob = a.radio();

        let test_data = [1, 2, 3, 4];

        bob.start_receive().await.unwrap();

        let SendResult::Success(tx_time) = alice
            .send(&test_data, None, false, false, SendContinuation::Idle)
            .await
            .unwrap()
        else {
            panic!("Failed to send packet!")
        };

        let pkt = receive_one(&mut bob).await;
        assert_eq!(pkt.timestamp, tx_time);
        assert_eq!(&pkt.data[..], &test_data[..]);
    }

    #[tokio::test(start_paused = true)]
    async fn ignored_if_not_listening() {
        let mut a = Aether::new();

        let mut alice = a.radio();
        let mut bob = a.radio();

        alice
            .send(b"Hello!", None, false, false, SendContinuation::Idle)
            .await
            .unwrap();

        timeout(core::time::Duration::from_secs(1), async move {
            bob.wait().await.unwrap();
        })
        .await
        .unwrap_err();
    }

    #[tokio::test(start_paused = true)]
    async fn arrives_delayed() {
        let mut a = Aether::new();
        let mut alice = a.radio();
        let mut bob = a.radio();
        bob.move_to(Coordinate::new(0.0, 299_792_458.0));

        bob.start_receive().await.unwrap();
        let before_send = tokio::time::Instant::now();

        let tx_res = alice
            .send(b"Hello!", None, false, false, SendContinuation::Idle)
            .await
            .unwrap();
        let SendResult::Success(tx_time) = tx_res else {
            panic!("Failed to send packet!")
        };

        let pkt = receive_one(&mut bob).await;
        assert!(before_send.elapsed() >= core::time::Duration::from_secs(1));
        assert_eq!(pkt.timestamp, tx_time + Duration::from_millis(1_000));
    }

    #[tokio::test]
    async fn log_beacon() {
        let beacon_frame = mac::Frame {
            header: mac::Header {
                frame_type: mac::FrameType::Beacon,
                frame_pending: true,
                ack_request: false,
                pan_id_compress: false,
                seq_no_suppress: false,
                ie_present: false,
                version: FrameVersion::Ieee802154,
                seq: 42,
                destination: None,
                source: None,
                auxiliary_security_header: None,
            },
            content: mac::FrameContent::Beacon(mac::beacon::Beacon {
                superframe_spec: mac::beacon::SuperframeSpecification {
                    beacon_order: BeaconOrder(0),
                    superframe_order: SuperframeOrder::Inactive,
                    final_cap_slot: 5,
                    battery_life_extension: true,
                    pan_coordinator: false,
                    association_permit: true,
                },
                guaranteed_time_slot_info: GuaranteedTimeSlotInformation::new(),
                pending_address: PendingAddress::new(),
            }),
            payload: b"Hello!",
            footer: Default::default(),
        };

        {
            let mut a = Aether::new();
            a.start_trace(File::create("log_beacon.pcap").unwrap());
            let mut alice = a.radio();
            let mut bob = a.radio();

            let mut buffer = ArrayVec::from([0; crate::consts::MAX_PHY_PACKET_SIZE]);
            let mut ctx = mac::FrameSerDesContext::<Unimplemented, Unimplemented>::new(
                FooterMode::None,
                None,
            );
            let length = beacon_frame.try_write(&mut buffer, &mut ctx).unwrap();
            buffer.truncate(length);

            alice
                .send(&buffer, None, true, false, SendContinuation::Idle)
                .await
                .unwrap();
            bob.send(&buffer, None, true, false, SendContinuation::Idle)
                .await
                .unwrap();
        }

        let written = File::open("log_beacon.pcap").unwrap();
        let mut reader = PcapNgReader::new(written).unwrap();

        let mut blocks = vec![];
        while let Some(b) = reader.next_block() {
            blocks.push(b.unwrap().into_owned());
        }

        assert_eq!(blocks.len(), 4);

        let int0 = blocks[0].clone().into_interface_description().unwrap();
        assert_eq!(int0.linktype, DataLink::IEEE802_15_4_NOFCS);
        assert_eq!(int0.snaplen, 127);

        let data0 = blocks[1].clone().into_enhanced_packet().unwrap();
        assert_eq!(data0.interface_id, 0);
        assert!(data0.data.ends_with(b"Hello!"));

        let int1 = blocks[2].clone().into_interface_description().unwrap();
        assert_eq!(int1.linktype, DataLink::IEEE802_15_4_NOFCS);
        assert_eq!(int1.snaplen, 127);

        assert_ne!(int0, int1);

        let data1 = blocks[3].clone().into_enhanced_packet().unwrap();
        assert_eq!(data1.interface_id, 1);
        assert!(data1.data.ends_with(b"Hello!"));
    }
}
