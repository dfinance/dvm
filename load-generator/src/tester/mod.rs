use crate::tester::pipeline::{Handler, Pipeline, perform};
use crate::dvm::Dvm;
use crate::tester::stat::StatWriter;
use crate::ds::InMemoryDataSource;
use std::sync::Arc;

pub mod mv_template;
pub mod pipeline;
pub mod stat;

pub struct LoadHandler {
    handlers: Vec<Handler>,
}

pub fn run_load(
    dvm: Arc<Dvm>,
    workers_count: usize,
    stat: StatWriter,
    ds: InMemoryDataSource,
) -> LoadHandler {
    let handlers = (0..workers_count)
        .into_iter()
        .map(|_| Pipeline::new(stat.clone(), dvm.clone(), ds.clone()))
        .map(perform)
        .collect();

    LoadHandler { handlers }
}

impl LoadHandler {
    pub fn is_run(&self) -> bool {
        self.handlers.iter().any(|h| h.is_run())
    }

    pub fn shutdown(&self) {
        for handler in &self.handlers {
            handler.stop();
        }
    }
}
