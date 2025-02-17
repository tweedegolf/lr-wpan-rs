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
