use std::{
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    time::UNIX_EPOCH,
};

use lr_wpan_rs::time::{Duration, Instant};

#[derive(Clone, Copy)]
pub struct Delay;

impl embedded_hal_async::delay::DelayNs for Delay {
    async fn delay_ns(&mut self, ns: u32) {
        SIMULATION_TIME.delay(Duration::from_nanos(ns as _)).await;
    }

    async fn delay_us(&mut self, us: u32) {
        SIMULATION_TIME.delay(Duration::from_micros(us as _)).await;
    }

    async fn delay_ms(&mut self, ms: u32) {
        SIMULATION_TIME.delay(Duration::from_millis(ms as _)).await;
    }
}

pub struct SimulationTime {
    now_ticks: AtomicU64,
    delay_waits: maitake_sync::WaitQueue,
    next_smallest_end_time: AtomicU64,
    ticker_started: AtomicBool,
}

pub static SIMULATION_TIME: SimulationTime = SimulationTime {
    now_ticks: AtomicU64::new(0),
    delay_waits: maitake_sync::WaitQueue::new(),
    next_smallest_end_time: AtomicU64::new(u64::MAX),
    ticker_started: AtomicBool::new(false),
};

impl SimulationTime {
    pub fn now(&'static self) -> Instant {
        self.start_ticker();

        let now_ticks = self.now_ticks.load(Ordering::SeqCst);
        Instant::from_ticks(now_ticks)
    }

    pub async fn delay(&'static self, duration: Duration) {
        if duration.ticks().is_negative() {
            panic!("Cannot delay a negative amount of time");
        }

        let end_time = self.now() + duration;

        self.delay_waits
            .wait_for_value(|| {
                if self.now() >= end_time {
                    Some(())
                } else {
                    println!(
                        "{} Setting next smallest end time: {}",
                        std::time::SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_millis(),
                        end_time.duration_since_epoch()
                    );
                    self.next_smallest_end_time
                        .fetch_min(end_time.ticks(), Ordering::SeqCst);
                    None
                }
            })
            .await
            .unwrap();

        println!(
            "Delay done. Now: {}, endtime: {}",
            self.now().duration_since_epoch(),
            end_time.duration_since_epoch()
        );
    }

    fn start_ticker(&'static self) {
        if self
            .ticker_started
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            tokio::spawn(async {
                let mut interval = tokio::time::interval(std::time::Duration::from_millis(3000));
                interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

                loop {
                    interval.tick().await;

                    let next_time = self.next_smallest_end_time.swap(u64::MAX, Ordering::SeqCst);

                    if next_time == u64::MAX {
                        // Nothing has set the delay, so we're probably not ready to move the time yet
                        continue;
                    }

                    self.now_ticks.store(
                        next_time, //.min(self.now_ticks.load(Ordering::SeqCst) + Duration::from_millis(10).ticks() as u64),
                        Ordering::SeqCst,
                    );

                    self.delay_waits.wake_all();

                    println!(
                        "\n{} Time updated. Now = {}",
                        std::time::SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_millis(),
                        self.now().duration_since_epoch()
                    );
                }
            });
        }
    }
}
