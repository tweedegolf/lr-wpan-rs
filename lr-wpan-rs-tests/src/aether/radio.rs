use std::sync::{Arc, Mutex, MutexGuard};

use log::trace;
use lr_wpan_rs::{
    phy::{ModulationType, Phy, ReceivedMessage, SendContinuation, SendResult},
    pib::{PhyPib, PhyPibWrite},
    time::Instant,
};
use tokio::sync::mpsc::Receiver;

use crate::aether::{AetherInner, AirPacket, Coordinate, Node, NodeId};

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
        Ok(crate::time::SIMULATION_TIME.now())
    }

    fn symbol_duration(&self) -> lr_wpan_rs::time::Duration {
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

        let now = send_time
            .unwrap_or_else(|| std::time::Instant::from(tokio::time::Instant::now()).into());
        tokio::time::sleep_until(now.into_std().into()).await;

        // TODO: Handle more than just data
        let channel = self.local_pib.current_channel;
        self.aether().send(AirPacket::new(data, now, channel));

        match continuation {
            SendContinuation::Idle => {}
            SendContinuation::WaitForResponse { .. } => todo!(),
            SendContinuation::ReceiveContinuous => self.start_receive().await?,
        }

        // TODO: Handle congestion
        Ok(SendResult::Success(now))
    }

    async fn start_receive(&mut self) -> Result<(), Self::Error> {
        trace!("Radio start_receive {:?}", self.node_id);

        self.with_node(|node| {
            node.rx_enable = true;
        });

        Ok(())
    }

    async fn stop_receive(&mut self) -> Result<(), Self::Error> {
        trace!("Radio stop_receive {:?}", self.node_id);

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

            tokio::time::sleep_until(msg.timestamp.into_std().into()).await;

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

    async fn get_phy_pib(&mut self) -> Result<&PhyPib, Self::Error> {
        Ok(&self.local_pib)
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
}

#[cfg(test)]
mod tests {
    use crate::aether::Aether;

    use super::*;
    use lr_wpan_rs::time::{DelayNsExt, Duration};
    use test_log::test;

    #[test(tokio::test(unhandled_panic = "shutdown_runtime"))]
    async fn radio_time_consistent() {
        let mut aether = Aether::new();

        let mut radio = aether.radio();
        let instant = radio.get_instant().await.unwrap();
        crate::time::Delay.delay_duration(Duration::from_millis(10)).await;
        let instant2 = radio.get_instant().await.unwrap();

        let duration = instant2.duration_since(instant);
        println!("{duration}");

        assert!(duration > Duration::from_millis(9));
        assert!(duration < Duration::from_millis(11));
    }
}
