use core::{
    fmt::Display,
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign},
};

use embedded_hal_async::delay::DelayNs;

pub const TICKS_PER_SECOND: u64 = 499200000 * 128;
pub const TICKS_PER_MILLI: u64 = TICKS_PER_SECOND / 1000;

/// An instant of time.
///
/// Every tick is 1/128th of a chip time at the mandatory
/// chipping rate of 499.2 MHz (~15.65 ps)
///
/// Wraps every ~288_692_283.8 seconds or every ~9 years
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Instant {
    ticks: u64,
}

impl Instant {
    pub const fn from_ticks(ticks: u64) -> Self {
        Self { ticks }
    }

    pub const fn from_seconds(seconds: u64) -> Self {
        Self::from_ticks(seconds * TICKS_PER_SECOND)
    }

    pub const fn ticks(&self) -> u64 {
        self.ticks
    }

    #[must_use]
    pub const fn checked_duration_since(&self, other: Self) -> Option<Duration> {
        let negative = other.ticks > self.ticks;
        let diff = self.ticks.abs_diff(other.ticks);

        if diff > i64::MAX as u64 {
            return None;
        }

        Some(Duration {
            ticks: diff as i64 * if negative { -1 } else { 1 },
        })
    }

    #[must_use]
    pub fn duration_since(&self, other: Self) -> Duration {
        unwrap!(self.checked_duration_since(other))
    }

    #[must_use]
    pub fn duration_since_epoch(&self) -> Duration {
        self.duration_since(Instant { ticks: 0 })
    }

    #[must_use]
    pub const fn checked_add_duration(self, duration: Duration) -> Option<Self> {
        match self.ticks.checked_add_signed(duration.ticks) {
            Some(ticks) => Some(Self { ticks }),
            None => None,
        }
    }

    #[must_use]
    pub const fn checked_sub_duration(self, duration: Duration) -> Option<Self> {
        match self.ticks.checked_add_signed(-duration.ticks) {
            Some(ticks) => Some(Self { ticks }),
            None => None,
        }
    }

    #[cfg(feature = "std")]
    pub fn into_std(self) -> std::time::Instant {
        self.into()
    }
}

impl Add<Duration> for Instant {
    type Output = Instant;

    fn add(self, rhs: Duration) -> Self::Output {
        unwrap!(self.checked_add_duration(rhs))
    }
}

impl AddAssign<Duration> for Instant {
    fn add_assign(&mut self, rhs: Duration) {
        *self = *self + rhs;
    }
}

impl Sub<Duration> for Instant {
    type Output = Instant;

    fn sub(self, rhs: Duration) -> Self::Output {
        unwrap!(self.checked_sub_duration(rhs))
    }
}

impl Sub<Instant> for Instant {
    type Output = Duration;

    fn sub(self, rhs: Instant) -> Self::Output {
        rhs.duration_since(self)
    }
}

impl SubAssign<Duration> for Instant {
    fn sub_assign(&mut self, rhs: Duration) {
        *self = *self - rhs;
    }
}

impl Div<Duration> for Instant {
    type Output = i64;

    fn div(self, rhs: Duration) -> Self::Output {
        let div = self.ticks / rhs.ticks.unsigned_abs();
        i64::try_from(div).expect("Overflow") * rhs.ticks.signum()
    }
}

#[cfg(feature = "std")]
static START_TIME: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();

#[cfg(feature = "std")]
impl From<std::time::Instant> for Instant {
    fn from(value: std::time::Instant) -> Self {
        let start = START_TIME.get_or_init(std::time::Instant::now);
        let since_start = value.duration_since(*start);

        let ticks = since_start.as_secs_f64() * TICKS_PER_SECOND as f64;

        Instant::from_ticks(ticks as u64)
    }
}

#[cfg(feature = "std")]
impl From<Instant> for std::time::Instant {
    fn from(value: Instant) -> Self {
        let start = *START_TIME.get_or_init(std::time::Instant::now);
        let seconds = value.ticks() as f64 / TICKS_PER_SECOND as f64;

        start + std::time::Duration::from_secs_f64(seconds)
    }
}

/// A span of time.
///
/// Every tick is 1/128th of a chip time at the mandatory
/// chipping rate of 499.2 MHz (~15.65 ps)
///
/// Ranges ~144_346_141.9 seconds or ~4.5 years
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Duration {
    ticks: i64,
}

impl Display for Duration {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let neg = self.ticks < 0;

        let value = self.abs();

        let s = value.secs();
        let ms = (value - Self::from_seconds(s)).millis();

        if neg {
            write!(f, "-{s}.{ms} secs")
        } else {
            write!(f, "{s}.{ms} secs")
        }
    }
}

#[cfg(feature = "defmt-03")]
impl defmt::Format for Duration {
    fn format(&self, f: defmt::Formatter) {
        let neg = self.ticks < 0;

        let value = self.abs();

        let s = value.secs();
        let ms = (value - Self::from_seconds(s)).millis();

        if neg {
            defmt::write!(f, "-{}.{} secs", s, ms)
        } else {
            defmt::write!(f, "{}.{} secs", s, ms)
        }
    }
}

impl Duration {
    pub const fn from_ticks(ticks: i64) -> Self {
        Self { ticks }
    }

    pub const fn from_seconds(seconds: i64) -> Self {
        Self::from_ticks(seconds * TICKS_PER_SECOND as i64)
    }

    pub const fn from_millis(millis: i64) -> Self {
        Self::from_ticks(millis * TICKS_PER_MILLI as i64)
    }

    pub const fn ticks(&self) -> i64 {
        self.ticks
    }

    /// The amount of *full* seconds in this duration.
    /// Always rounds down.
    pub const fn secs(&self) -> i64 {
        if self.ticks().is_negative() {
            -(self.ticks().unsigned_abs().div_ceil(TICKS_PER_SECOND) as i64)
        } else {
            self.ticks() / TICKS_PER_SECOND as i64
        }
    }

    /// The amount of *full* milliseconds in this duration.
    /// Always rounds down.
    pub const fn millis(&self) -> i64 {
        if self.ticks().is_negative() {
            -(self.ticks().unsigned_abs().div_ceil(TICKS_PER_MILLI) as i64)
        } else {
            self.ticks() / TICKS_PER_MILLI as i64
        }
    }

    #[must_use]
    pub const fn checked_add(self, duration: Duration) -> Option<Self> {
        match self.ticks.checked_add(duration.ticks) {
            Some(ticks) => Some(Self { ticks }),
            None => None,
        }
    }

    #[must_use]
    pub const fn checked_sub(self, duration: Duration) -> Option<Self> {
        match self.ticks.checked_sub(duration.ticks) {
            Some(ticks) => Some(Self { ticks }),
            None => None,
        }
    }

    #[must_use]
    pub const fn abs(self) -> Self {
        Self {
            ticks: self.ticks.abs(),
        }
    }

    #[cfg(feature = "std")]
    pub fn into_std(self) -> std::time::Duration {
        self.into()
    }
}

impl Add for Duration {
    type Output = Duration;

    fn add(self, rhs: Duration) -> Self::Output {
        unwrap!(self.checked_add(rhs))
    }
}

impl AddAssign for Duration {
    fn add_assign(&mut self, rhs: Duration) {
        *self = *self + rhs;
    }
}

impl Sub for Duration {
    type Output = Duration;

    fn sub(self, rhs: Duration) -> Self::Output {
        unwrap!(self.checked_sub(rhs))
    }
}

impl SubAssign for Duration {
    fn sub_assign(&mut self, rhs: Duration) {
        *self = *self - rhs;
    }
}

impl Mul<i64> for Duration {
    type Output = Duration;

    fn mul(self, rhs: i64) -> Self::Output {
        Self {
            ticks: unwrap!(self.ticks.checked_mul(rhs)),
        }
    }
}

impl Mul<Duration> for i64 {
    type Output = Duration;

    fn mul(self, rhs: Duration) -> Self::Output {
        rhs * self
    }
}

impl MulAssign<i64> for Duration {
    fn mul_assign(&mut self, rhs: i64) {
        *self = *self * rhs;
    }
}

impl Div<i64> for Duration {
    type Output = Duration;

    fn div(self, rhs: i64) -> Self::Output {
        Self {
            ticks: unwrap!(self.ticks.checked_div(rhs)),
        }
    }
}

impl DivAssign<i64> for Duration {
    fn div_assign(&mut self, rhs: i64) {
        *self = *self / rhs;
    }
}

#[cfg(feature = "std")]
impl From<Duration> for std::time::Duration {
    fn from(value: Duration) -> Self {
        let seconds = value.ticks() as f64 / TICKS_PER_SECOND as f64;

        std::time::Duration::from_secs_f64(seconds)
    }
}

pub trait DelayNsExt: DelayNs + Clone {
    /// Delay for the duration. Accurate to the millisecond
    async fn delay_duration(&mut self, mut duration: Duration) {
        if duration.ticks().is_negative() {
            return;
        }

        let limit = u32::MAX as i64 - 1;

        while duration.millis() > limit {
            self.delay_ms(limit as u32).await;
            duration -= Duration::from_millis(limit);
        }

        // We want to wait *at least* the duration, so add another milli if we have time left over
        let left_over = (Duration::from_millis(duration.millis()) - duration)
            .ticks()
            .is_positive();

        self.delay_ms(duration.millis() as u32 + if left_over { 1 } else { 0 })
            .await;
    }
}

impl<T: DelayNs + Clone> DelayNsExt for T {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duration_since() {
        assert_eq!(
            Instant::from_ticks(0).duration_since(Instant::from_ticks(5)),
            Duration::from_ticks(-5)
        );
        assert_eq!(
            Instant::from_ticks(10).duration_since(Instant::from_ticks(5)),
            Duration::from_ticks(5)
        );
    }

    #[test]
    fn add() {
        assert_eq!(
            Instant::from_ticks(0) + Duration::from_ticks(5),
            Instant::from_ticks(5)
        );
        assert_eq!(
            Instant::from_ticks(10) + Duration::from_ticks(-5),
            Instant::from_ticks(5)
        );

        assert_eq!(
            Duration::from_ticks(0) + Duration::from_ticks(5),
            Duration::from_ticks(5)
        );
        assert_eq!(
            Duration::from_ticks(10) + Duration::from_ticks(-5),
            Duration::from_ticks(5)
        );
    }

    #[test]
    fn sub() {
        assert_eq!(
            Instant::from_ticks(0) - Duration::from_ticks(-5),
            Instant::from_ticks(5)
        );
        assert_eq!(
            Instant::from_ticks(10) - Duration::from_ticks(5),
            Instant::from_ticks(5)
        );

        assert_eq!(
            Duration::from_ticks(0) - Duration::from_ticks(-5),
            Duration::from_ticks(5)
        );
        assert_eq!(
            Duration::from_ticks(10) - Duration::from_ticks(5),
            Duration::from_ticks(5)
        );
    }

    #[test]
    fn mul() {
        assert_eq!(Duration::from_ticks(10) * 5, Duration::from_ticks(50));
        assert_eq!(Duration::from_ticks(10) * -5, Duration::from_ticks(-50));
    }

    #[test]
    fn div() {
        assert_eq!(Duration::from_ticks(10) / 5, Duration::from_ticks(2));
        assert_eq!(Duration::from_ticks(10) / -5, Duration::from_ticks(-2));
    }
}
