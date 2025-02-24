use std::{future::Future, sync::Arc};

use async_executor::{Executor, Task};
use log::trace;
use lr_wpan_rs::{
    mac::{MacCommander, MacConfig},
    wire::ExtendedAddress,
};
use rand::{rngs::StdRng, SeedableRng};

use super::aether::Aether;
use crate::time::SimulationTime;

/// Run multiple mac engines
pub fn run_mac_engine_multi<'a>(
    count: usize,
) -> (Arc<[&'static MacCommander]>, Aether, MultiRunner<'a>) {
    let commanders =
        Arc::from_iter((0..count).map(|_| Box::leak(Box::new(MacCommander::new())) as &_));

    let simulation_time = Box::leak(Box::new(SimulationTime::new())) as &_;

    let mut aether = Aether::new(simulation_time);
    let executor = Executor::new();

    let engine_handles = (0..count)
        .map(|i| {
            let commanders = commanders.clone();
            executor.spawn({
                let radio = aether.radio();
                async move {
                    lr_wpan_rs::mac::run_mac_engine(
                        radio,
                        commanders[i],
                        MacConfig {
                            extended_address: ExtendedAddress(i as _),
                            rng: StdRng::seed_from_u64(i as _),
                            delay: crate::time::Delay(simulation_time),
                        },
                    )
                    .await;
                }
            })
        })
        .collect();

    (
        commanders,
        aether,
        MultiRunner {
            executor,
            task_handles: Vec::new(),
            engine_handles,
            simulation_time,
        },
    )
}

pub struct MultiRunner<'a> {
    executor: Executor<'a>,
    engine_handles: Vec<Task<()>>,
    task_handles: Vec<Task<()>>,
    pub simulation_time: &'static SimulationTime,
}

impl<'a> MultiRunner<'a> {
    pub fn attach_test_task(&mut self, f: impl Future<Output = ()> + Send + 'a) {
        self.task_handles.push(self.executor.spawn(f));
    }

    pub fn run(mut self) {
        loop {
            if !self.executor.try_tick() {
                trace!("Ticking time along...");
                self.simulation_time.tick();
            }

            for i in (0..self.engine_handles.len()).rev() {
                if self.engine_handles[i].is_finished() {
                    // Check to see if it produced a result (and thus didn't panic)
                    futures::executor::block_on(self.engine_handles.remove(i).cancel());
                }
            }

            for i in (0..self.task_handles.len()).rev() {
                if self.task_handles[i].is_finished() {
                    // Check to see if it produced a result (and thus didn't panic)
                    futures::executor::block_on(self.task_handles.remove(i).cancel());
                }
            }

            if self.task_handles.is_empty() {
                // We're done
                break;
            }
        }
    }
}
