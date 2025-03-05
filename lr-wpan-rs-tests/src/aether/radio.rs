use std::{
    pin::pin,
    sync::{Arc, Mutex, MutexGuard},
};

use async_channel::Receiver;
use futures::FutureExt;
use log::trace;
use lr_wpan_rs::{
    phy::{ModulationType, Phy, ReceivedMessage, SendContinuation, SendResult},
    pib::{PhyPib, PhyPibWrite},
    time::Instant,
};

use crate::{
    aether::{AetherInner, AirPacket, Coordinate, Node, NodeId},
    time::SimulationTime,
};

/// Single radio connected to an [`super::Aether`]
#[derive(Debug)]
pub struct AetherRadio {
    pub(super) inner: Arc<Mutex<AetherInner>>,
    pub(super) node_id: NodeId,
    pub(super) antenna: Receiver<AirPacket>,
    pub(super) local_pib: PhyPib,
}

impl AetherRadio {
    pub fn move_to(&mut self, position: Coordinate) {
        self.with_node(|node| node.position = position);
    }

    fn aether(&mut self) -> AetherGuard {
        AetherGuard {
            aether: self.inner.lock().unwrap(),
            node_id: self.node_id.clone(),
        }
    }

    fn simulation_time(&self) -> &'static SimulationTime {
        self.inner.lock().unwrap().simulation_time
    }

    fn with_node<R>(&mut self, f: impl FnOnce(&mut Node) -> R) -> R {
        let AetherGuard {
            mut aether,
            node_id,
        } = self.aether();
        let node = aether
            .nodes
            .get_mut(&node_id)
            .expect("we exist therefore there must be a node with out id");

        f(node)
    }
}

impl Phy for AetherRadio {
    type Error = core::convert::Infallible;
    type ProcessingContext = ReceivedMessage;

    const MODULATION: ModulationType = ModulationType::BPSK;

    async fn reset(&mut self) -> Result<(), Self::Error> {
        trace!("Radio reset {:?}", self.node_id);

        self.stop_receive().await?;
        let new_pib = PhyPib::unspecified_new();
        self.with_node(|node| {
            node.pib = new_pib;
        });

        Ok(())
    }

    async fn get_instant(&mut self) -> Result<Instant, Self::Error> {
        Ok(self.aether().simulation_time().now())
    }

    fn symbol_period(&self) -> lr_wpan_rs::time::Duration {
        lr_wpan_rs::time::Duration::from_ticks(10000)
    }

    async fn send(
        &mut self,
        data: &[u8],
        send_time: Option<Instant>,
        _ranging: bool,
        _use_csma: bool,
        continuation: SendContinuation,
    ) -> Result<SendResult, Self::Error> {
        trace!("Radio send {:?}", self.node_id);

        if let Some(send_time) = send_time {
            self.simulation_time().delay_until(send_time).await;
        }

        let now = self.simulation_time().now();

        trace!("Radio send {:?} at: {}", self.node_id, now);

        // TODO: Handle more than just data
        let channel = self.local_pib.current_channel;
        self.aether().send(AirPacket::new(data, now, channel));

        let response = match continuation {
            SendContinuation::Idle => None,
            SendContinuation::WaitForResponse {
                turnaround_time,
                timeout,
            } => {
                let receive_start_time = self.simulation_time().delay(turnaround_time).await;
                trace!("Wait for response start at: {}", receive_start_time);
                self.start_receive().await?;

                let mut timeout = pin!(self.simulation_time().delay(timeout).fuse());

                let response = loop {
                    futures::select! {
                        _ = &mut timeout => {
                            break None;
                        }
                        processing_context = self.wait().fuse() => {
                            match self.process(processing_context?).await? {
                                Some(received_message) => break Some(received_message),
                                None => continue,
                            }
                        }
                    }
                };

                self.stop_receive().await?;

                response
            }
            SendContinuation::ReceiveContinuous => {
                self.start_receive().await?;
                None
            }
        };

        // TODO: Handle congestion
        Ok(SendResult::Success(now, response))
    }

    async fn start_receive(&mut self) -> Result<(), Self::Error> {
        trace!(
            "Radio start_receive {:?} at: {}",
            self.node_id,
            self.simulation_time().now(),
        );

        self.with_node(|node| {
            node.rx_enable = true;
        });

        Ok(())
    }

    async fn stop_receive(&mut self) -> Result<(), Self::Error> {
        trace!(
            "Radio stop_receive {:?} at: {}",
            self.node_id,
            self.simulation_time().now(),
        );

        self.with_node(|node| {
            node.rx_enable = false;
        });

        Ok(())
    }

    async fn wait(&mut self) -> Result<Self::ProcessingContext, Self::Error> {
        loop {
            let msg = self
                .antenna
                .recv()
                .await
                .expect("only we can close the antenna");

            if msg.channel != self.local_pib.current_channel {
                continue;
            }

            let msg = ReceivedMessage {
                timestamp: msg.time_stamp,
                data: msg.data,
                lqi: 255,
                channel: msg.channel,
                page: lr_wpan_rs::ChannelPage::Uwb,
            };

            self.simulation_time()
                .delay_until_at_least(msg.timestamp)
                .await;

            return Ok(msg);
        }
    }

    async fn process(
        &mut self,
        ctx: Self::ProcessingContext,
    ) -> Result<Option<ReceivedMessage>, Self::Error> {
        trace!("Radio process {:?}", self.node_id);

        Ok(Some(ctx))
    }

    async fn update_phy_pib<U>(
        &mut self,
        f: impl FnOnce(&mut PhyPibWrite) -> U,
    ) -> Result<U, Self::Error> {
        let res = f(&mut self.local_pib);

        let new_pib = self.local_pib.clone();
        self.with_node(|node| {
            node.pib = new_pib;
        });

        Ok(res)
    }

    fn get_phy_pib(&mut self) -> &PhyPib {
        &self.local_pib
    }
}

struct AetherGuard<'a> {
    aether: MutexGuard<'a, AetherInner>,
    node_id: NodeId,
}

impl AetherGuard<'_> {
    fn send(&mut self, data: AirPacket) -> Instant {
        self.aether.send(&self.node_id, data)
    }

    fn simulation_time(&self) -> &'static SimulationTime {
        self.aether.simulation_time
    }
}
