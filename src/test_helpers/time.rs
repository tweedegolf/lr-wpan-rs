use crate::time::{Duration, Instant, TICKS_PER_SECOND};
use std::sync::OnceLock;

static START_TIME: OnceLock<std::time::Instant> = OnceLock::new();

impl From<std::time::Instant> for Instant {
    fn from(value: std::time::Instant) -> Self {
        let start = START_TIME.get_or_init(std::time::Instant::now);
        let since_start = value.duration_since(*start);

        let ticks = since_start.as_secs_f64() * TICKS_PER_SECOND as f64;

        Instant::from_ticks(ticks as u64)
    }
}

impl From<Instant> for std::time::Instant {
    fn from(value: Instant) -> Self {
        let start = *START_TIME.get_or_init(std::time::Instant::now);
        let seconds = value.ticks() as f64 / TICKS_PER_SECOND as f64;

        start + std::time::Duration::from_secs_f64(seconds)
    }
}

impl From<tokio::time::Instant> for Instant {
    fn from(value: tokio::time::Instant) -> Self {
        value.into_std().into()
    }
}

impl From<Instant> for tokio::time::Instant {
    fn from(value: Instant) -> Self {
        std::time::Instant::from(value).into()
    }
}

impl From<Duration> for std::time::Duration {
    fn from(value: Duration) -> Self {
        let seconds = value.ticks() as f64 / TICKS_PER_SECOND as f64;

        std::time::Duration::from_secs_f64(seconds)
    }
}

#[derive(Clone, Copy)]
pub struct Delay;

impl embedded_hal_async::delay::DelayNs for Delay {
    async fn delay_ns(&mut self, ns: u32) {
        tokio::time::sleep(std::time::Duration::from_nanos(ns as u64)).await
    }

    async fn delay_us(&mut self, us: u32) {
        tokio::time::sleep(std::time::Duration::from_micros(us as u64)).await
    }

    async fn delay_ms(&mut self, ms: u32) {
        tokio::time::sleep(std::time::Duration::from_millis(ms as u64)).await
    }
}
