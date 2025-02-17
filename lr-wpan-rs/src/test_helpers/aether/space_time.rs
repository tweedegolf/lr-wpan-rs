use crate::time::{Duration, TICKS_PER_SECOND};

#[derive(Debug, Default, Clone, Copy, PartialOrd, PartialEq)]
pub struct Meters(pub f64);

impl Meters {
    pub fn as_duration(&self) -> Duration {
        const C: f64 = 299_792_458.0;

        let secs = self.0 / C;
        Duration::from_ticks((secs * TICKS_PER_SECOND as f64) as i64)
    }
}

#[derive(Debug, Default, Clone, Copy, PartialOrd, PartialEq)]
pub struct Coordinate(pub [Meters; 2]);

impl Coordinate {
    pub const fn new(x: f64, y: f64) -> Self {
        Self([Meters(x), Meters(y)])
    }

    pub fn dist(&self, other: Self) -> Meters {
        let dist = self
            .0
            .into_iter()
            .zip(other.0)
            .map(|(a, b)| (a.0 - b.0).powi(2))
            .sum::<f64>()
            .sqrt();

        Meters(dist)
    }
}
