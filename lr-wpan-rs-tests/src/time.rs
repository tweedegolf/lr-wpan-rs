use std::sync::atomic::{AtomicU64, Ordering};

use log::{debug, trace};
use lr_wpan_rs::time::{Duration, Instant};

#[derive(Clone, Copy)]
pub struct Delay(pub &'static SimulationTime);

impl embedded_hal_async::delay::DelayNs for Delay {
    async fn delay_ns(&mut self, ns: u32) {
        self.0.delay(Duration::from_nanos(ns as _)).await;
    }

    async fn delay_us(&mut self, us: u32) {
        self.0.delay(Duration::from_micros(us as _)).await;
    }

    async fn delay_ms(&mut self, ms: u32) {
        self.0.delay(Duration::from_millis(ms as _)).await;
    }
}

pub struct SimulationTime {
    now_ticks: AtomicU64,
    delay_waits: maitake_sync::WaitQueue,
    next_smallest_end_time: AtomicU64,
}

impl SimulationTime {
    pub const fn new() -> Self {
        Self {
            now_ticks: AtomicU64::new(0),
            delay_waits: maitake_sync::WaitQueue::new(),
            next_smallest_end_time: AtomicU64::new(u64::MAX),
        }
    }

    pub fn now(&'static self) -> Instant {
        let now_ticks = self.now_ticks.load(Ordering::SeqCst);
        Instant::from_ticks(now_ticks)
    }

    pub async fn delay(&'static self, duration: Duration) {
        if duration.ticks().is_negative() {
            panic!("Cannot delay a negative amount of time: {}", duration);
        }

        let end_time = self.now() + duration;

        self.delay_waits
            .wait_for_value(|| {
                if self.now() >= end_time {
                    Some(())
                } else {
                    trace!(
                        "Setting next smallest end time: {}",
                        end_time.duration_since_epoch()
                    );
                    self.next_smallest_end_time
                        .fetch_min(end_time.ticks(), Ordering::SeqCst);
                    None
                }
            })
            .await
            .unwrap();

        trace!(
            "Delay done. Now: {}, endtime: {}",
            self.now().duration_since_epoch(),
            end_time.duration_since_epoch()
        );
    }

    pub(crate) fn tick(&'static self) {
        let next_time = self.next_smallest_end_time.swap(u64::MAX, Ordering::SeqCst);

        if next_time == u64::MAX {
            // Nothing has set the delay, so we're probably not ready to move the time yet
            return;
        }

        self.now_ticks.store(next_time, Ordering::SeqCst);

        self.delay_waits.wake_all();

        debug!("Time updated. Now = {}", self.now().duration_since_epoch());
    }
}

impl Default for SimulationTime {
    fn default() -> Self {
        Self::new()
    }
}
