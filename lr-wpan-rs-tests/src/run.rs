use std::{future::Future, sync::Arc};

use async_executor::{Executor, Task};
use lr_wpan_rs::{
    mac::{MacCommander, MacConfig},
    wire::ExtendedAddress,
};
use rand::{rngs::StdRng, SeedableRng};

use super::aether::Aether;
use crate::{aether::Coordinate, time::SimulationTime};

/// Run multiple mac engines
pub fn create_test_runner<'a>(
    mac_stack_count: usize,
) -> (Arc<[&'static MacCommander]>, Aether, TestRunner<'a>) {
    let commanders = Arc::from_iter(
        (0..mac_stack_count).map(|_| Box::leak(Box::new(MacCommander::new())) as &_),
    );

    let simulation_time = Box::leak(Box::new(SimulationTime::new())) as &_;

    let mut aether = Aether::new(simulation_time);
    let executor = Executor::new();

    let engine_handles = (0..mac_stack_count)
        .map(|i| {
            let commanders = commanders.clone();
            executor.spawn({
                let mut radio = aether.radio();
                radio.move_to(Coordinate::new(i as f64, 0.0));
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
        TestRunner {
            executor,
            task_handles: Vec::new(),
            engine_handles,
            simulation_time,
        },
    )
}

pub struct TestRunner<'a> {
    executor: Executor<'a>,
    engine_handles: Vec<Task<()>>,
    task_handles: Vec<Task<()>>,
    pub simulation_time: &'static SimulationTime,
}

impl<'a> TestRunner<'a> {
    pub fn attach_test_task(&mut self, f: impl Future<Output = ()> + Send + 'a) {
        self.task_handles.push(self.executor.spawn(f));
    }

    pub fn run(mut self) {
        loop {
            if !self.executor.try_tick() {
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
