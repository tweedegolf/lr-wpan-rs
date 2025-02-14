use std::sync::Arc;

use ieee802154::mac::ExtendedAddress;
use rand::{rngs::StdRng, SeedableRng};
use tokio::task::AbortHandle;

use crate::mac::{MacCommander, MacConfig};

use super::aether::Aether;

/// Run a single mac engine
pub fn run_mac_engine_simple() -> Runner {
    let commander = Box::leak(Box::new(MacCommander::new()));
    let mut aether = Aether::new();

    let task_handle = tokio::spawn(crate::mac::run_mac_engine(
        aether.radio(),
        commander,
        MacConfig {
            extended_address: ExtendedAddress(0x0123456789abcdef),
            rng: StdRng::seed_from_u64(0),
            delay: crate::test_helpers::time::Delay,
        },
    ))
    .abort_handle();

    Runner {
        commander,
        aether,
        task_handle,
    }
}

pub struct Runner {
    pub commander: &'static MacCommander,
    pub aether: Aether,
    task_handle: AbortHandle,
}

impl Drop for Runner {
    fn drop(&mut self) {
        self.task_handle.abort();
    }
}

/// Run multiple mac engines
pub fn run_mac_engine_multi(count: usize) -> MultiRunner {
    let commanders =
        Arc::from_iter((0..count).map(|_| Box::leak(Box::new(MacCommander::new())) as &_));
    let mut aether = Aether::new();

    let task_handles = (0..count)
        .map(|i| {
            let commanders = commanders.clone();
            tokio::spawn(crate::mac::run_mac_engine(
                aether.radio(),
                commanders[i],
                MacConfig {
                    extended_address: ExtendedAddress(i as _),
                    rng: StdRng::seed_from_u64(i as _),
                    delay: crate::test_helpers::time::Delay,
                },
            ))
            .abort_handle()
        })
        .collect();

    MultiRunner {
        commanders,
        aether,
        task_handles,
    }
}

pub struct MultiRunner {
    pub commanders: Arc<[&'static MacCommander]>,
    pub aether: Aether,
    task_handles: Vec<AbortHandle>,
}

impl Drop for MultiRunner {
    fn drop(&mut self) {
        self.task_handles.iter().for_each(|handle| handle.abort());
    }
}
