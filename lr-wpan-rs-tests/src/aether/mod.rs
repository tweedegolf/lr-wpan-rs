//! Radio simulation infrastructure
//!
//! This module provides a simulated [Aether](https://en.wikipedia.org/wiki/Luminiferous_aether) to connect several radios.
//!
//! # Example
//! ```
//! use lr_wpan_rs::phy::{Phy, SendContinuation, SendResult};
//! use lr_wpan_rs_tests::aether::{Aether, Coordinate, Meters};
//! use lr_wpan_rs_tests::run::run_mac_engine_multi;
//! use lr_wpan_rs::time::Duration;
//!
//! let (_, mut aether, mut runner) = run_mac_engine_multi(0);
//!
//! runner.attach_test_task(async {
//!     // Create two new radios connected to the aether
//!     let mut alice = aether.radio();
//!     let mut bob = aether.radio();
//!     bob.move_to(Coordinate::new(0.0, 299_792_458.0));
//!
//!     bob.start_receive().await.unwrap();
//!
//!     let tx_res = alice.send(b"Hello, world!", None, false, false, SendContinuation::Idle).await.unwrap();
//!     let SendResult::Success(tx_time) = tx_res else { unreachable!() };
//!
//!     let mut got_message = false;
//!     let ctx = bob.wait().await.unwrap();
//!
//!     if let Some(msg) = bob.process(ctx).await.unwrap() {
//!         assert_eq!(&msg.data[..], b"Hello, world!");
//!         assert_eq!(msg.timestamp, tx_time + Duration::from_seconds(1));
//!         got_message = true;
//!     }
//!
//!     assert!(got_message);
//! });
//!
//! runner.run()
//! ```

use core::fmt::Debug;
use std::{
    borrow::Cow,
    collections::HashMap,
    fs::File,
    io::{Seek, Write},
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex, MutexGuard,
    },
};

use async_channel::{bounded, Sender, TrySendError};
use byte::TryRead;
use heapless::Vec;
use lr_wpan_rs::{pib::PhyPib, time::Instant, wire::Frame};
use pcap_file::{
    pcapng::{
        blocks::{
            enhanced_packet::EnhancedPacketBlock,
            interface_description::{InterfaceDescriptionBlock, InterfaceDescriptionOption},
        },
        Block, PcapNgReader, PcapNgWriter,
    },
    DataLink,
};

mod radio;
mod space_time;

pub use radio::AetherRadio;
pub use space_time::{Coordinate, Meters};

use crate::time::SimulationTime;

/// A medium to which radios are connected
///
/// This takes care of routing the packets to the right radios.
pub struct Aether {
    inner: Arc<Mutex<AetherInner>>,
}

impl Aether {
    /// Create a new empty aether
    pub fn new(simulation_time: &'static SimulationTime) -> Self {
        let inner = AetherInner {
            nodes: Default::default(),
            pcap_trace: None,
            simulation_time,
        };

        Self {
            inner: Arc::new(Mutex::new(inner)),
        }
    }

    /// Create a new empty aether
    pub fn new_own_simulation_time() -> Self {
        let inner = AetherInner {
            nodes: Default::default(),
            pcap_trace: None,
            simulation_time: Box::leak(Box::new(SimulationTime::new())),
        };

        Self {
            inner: Arc::new(Mutex::new(inner)),
        }
    }

    /// Create a radio which lives in the Aether
    pub fn radio(&mut self) -> AetherRadio {
        let (tx, rx) = bounded(16);

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

    pub fn start_trace(&mut self, name: &str) {
        self.inner().start_trace(name);
    }

    pub fn stop_trace(&mut self) -> File {
        self.inner().stop_trace()
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
                                lr_wpan_rs::wire::FooterMode::None,
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
    pcap_trace: Option<(PcapNgWriter<File>, HashMap<NodeId, u32>)>,
    pub simulation_time: &'static SimulationTime,
}

impl Debug for AetherInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.debug_struct("AetherInner")
            .field("nodes", &self.nodes)
            .field("pcap_dump", &self.pcap_trace.as_ref().map(|(_, h)| ((), h)))
            .finish()
    }
}

impl AetherInner {
    pub fn start_trace(&mut self, name: &str) {
        if self.pcap_trace.is_some() {
            panic!("Already capturing pcap");
        }

        let output_folder = PathBuf::from(env!("OUT_DIR")).join("test-output");
        if !output_folder.exists() {
            std::fs::create_dir_all(&output_folder).unwrap();
        }

        let trace_file_path = output_folder.join(name).with_extension("pcap");

        let file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .read(true)
            .open(&trace_file_path)
            .unwrap();

        log::info!("Writing aether trace to: {}", trace_file_path.display());

        self.pcap_trace = Some((PcapNgWriter::new(file).unwrap(), HashMap::new()));
    }

    /// Stops the trace and returns the file handle that was written to
    pub fn stop_trace(&mut self) -> File {
        let (trace_file, _) = self.pcap_trace.take().expect("No trace in progress");
        let mut file = trace_file.into_inner();
        file.seek(std::io::SeekFrom::Start(0)).unwrap();
        file.flush().unwrap();

        file
    }

    fn trace(&mut self, node_id: &NodeId, pkt: &AirPacket) {
        let Some((pcap, nodes)) = &mut self.pcap_trace else {
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

        self.simulation_time.now()
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
    pub data: Vec<u8, 127>,
    pub time_stamp: Instant,
    pub channel: u8,
}

impl AirPacket {
    pub fn new(data: impl TryInto<Vec<u8, 127>>, time_stamp: Instant, channel: u8) -> Self {
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
    use byte::TryWrite;
    use lr_wpan_rs::{
        phy::{Phy, ReceivedMessage, SendContinuation, SendResult},
        wire::{
            self,
            beacon::{
                BeaconOrder::BeaconOrder, GuaranteedTimeSlotInformation, PendingAddress,
                SuperframeOrder,
            },
            security::default::Unimplemented,
            FooterMode, FrameVersion,
        },
    };
    use pcap_file::pcapng::PcapNgReader;

    use super::*;

    async fn receive_one(bob: &mut AetherRadio) -> ReceivedMessage {
        let (tx, rx) = bounded(1);
        let ctx = bob.wait().await.unwrap();
        if let Some(pkt) = bob.process(ctx).await.unwrap() {
            tx.try_send(pkt).unwrap();
            drop(tx);
        }

        let pkt = rx.try_recv().unwrap();

        assert!(rx.is_closed());
        pkt
    }

    #[futures_test::test]
    async fn radios_are_connected() {
        let mut a = Aether::new_own_simulation_time();

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

    // #[futures_test::test]
    // async fn ignored_if_not_listening() {
    //     let mut a = Aether::new();

    //     let mut alice = a.radio();
    //     let mut bob = a.radio();

    //     alice
    //         .send(b"Hello!", None, false, false, SendContinuation::Idle)
    //         .await
    //         .unwrap();

    //     timeout(core::time::Duration::from_secs(1), async move {
    //         bob.wait().await.unwrap();
    //     })
    //     .await
    //     .unwrap_err();
    // }

    // #[futures_test::test]
    // async fn arrives_delayed() {
    //     let mut a = Aether::new();
    //     let mut alice = a.radio();
    //     let mut bob = a.radio();
    //     bob.move_to(Coordinate::new(0.0, 299_792_458.0));

    //     bob.start_receive().await.unwrap();
    //     let before_send = tokio::time::Instant::now();

    //     let tx_res = alice
    //         .send(b"Hello!", None, false, false, SendContinuation::Idle)
    //         .await
    //         .unwrap();
    //     let SendResult::Success(tx_time) = tx_res else {
    //         panic!("Failed to send packet!")
    //     };

    //     let pkt = receive_one(&mut bob).await;
    //     assert!(before_send.elapsed() >= core::time::Duration::from_secs(1));
    //     assert_eq!(pkt.timestamp, tx_time + Duration::from_millis(1_000));
    // }

    #[futures_test::test]
    async fn log_beacon() {
        let beacon_frame = wire::Frame {
            header: wire::Header {
                frame_type: wire::FrameType::Beacon,
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
            content: wire::FrameContent::Beacon(wire::beacon::Beacon {
                superframe_spec: wire::beacon::SuperframeSpecification {
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

        let written = {
            let mut a = Aether::new_own_simulation_time();
            a.start_trace("log_beacon");
            let mut alice = a.radio();
            let mut bob = a.radio();

            let mut buffer = Vec::<_, { lr_wpan_rs::consts::MAX_PHY_PACKET_SIZE }>::new();
            buffer
                .resize_default(lr_wpan_rs::consts::MAX_PHY_PACKET_SIZE)
                .unwrap();
            let mut ctx = wire::FrameSerDesContext::<Unimplemented, Unimplemented>::new(
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

            a.stop_trace()
        };

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
