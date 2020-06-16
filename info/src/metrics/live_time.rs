use std::sync::RwLock;
use std::collections::HashMap;
use std::{thread, process, panic};
use std::thread::ThreadId;
use std::cell::RefCell;
use once_cell::sync::Lazy;
use sysinfo::{System, SystemExt};
use serde_derive::{Serialize, Deserialize};
use std::sync::atomic::{AtomicBool, Ordering};

/// Live time metrics.
/// Recorded metrics for the current countdown.
pub(crate) static LIVE_METRICS: Lazy<RwLock<Metrics>> =
    Lazy::new(|| RwLock::new(Metrics::default()));
/// Save metrics flag.
pub(crate) static STORE_METRICS: AtomicBool = AtomicBool::new(false);

/// Execution data.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct ExecutionData {
    /// Time of processing.
    pub process_time: u64,
    /// Execution result.
    /// May be empty.
    /// Empty if the value is not set or the process ended with panic.
    pub result: Option<ExecutionResult>,
}

impl ExecutionData {
    /// Create a new ExecutionData.
    pub fn new(process_time: u64) -> ExecutionData {
        ExecutionData {
            process_time,
            result: None,
        }
    }

    /// Crate execution with result data.
    pub fn with_result(process_time: u64, result: Option<ExecutionResult>) -> ExecutionData {
        ExecutionData {
            process_time,
            result,
        }
    }
}

/// Result of the action.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct ExecutionResult {
    /// Is action completed successfully.
    pub is_success: bool,
    /// Status code.
    pub status: u64,
    /// Spent gas.
    pub gas_used: u64,
}

impl ExecutionResult {
    /// Create new action result
    pub fn new(is_success: bool, status: u64, gas_used: u64) -> ExecutionResult {
        ExecutionResult {
            is_success,
            status,
            gas_used,
        }
    }
}

/// System metrics.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SysMetrics {
    pub cpu_usage: f32,
    pub memory: u64,
    pub threads_count: usize,
}

/// Stores metric.
pub fn store_metric(name: &'static str, metric: ExecutionData) {
    let result = panic::catch_unwind(|| {
        if STORE_METRICS.load(Ordering::Relaxed) {
            let not_stored_metric = {
                let metrics = &LIVE_METRICS.read().unwrap();
                metrics.store_metric(name, metric)
            };
            if let Some(metric) = not_stored_metric {
                let metrics = &mut LIVE_METRICS.write().unwrap();
                metrics.crate_local_metrics();
                metrics.store_metric(name, metric);
            }
        }
    });

    if let Err(err) = result {
        error!("Failed to store metric: [{:?}]", err);
    }
}

/// Returns the live metrics by name.
pub fn get_action_metrics(name: &'static str) -> Vec<ExecutionData> {
    let metrics = &LIVE_METRICS.read().unwrap();
    metrics.0.iter().fold(Vec::new(), |mut acc, (_, local)| {
        if let Some(metric) = local.borrow().get(name) {
            acc.extend(metric.iter().cloned());
        }
        acc
    })
}

pub fn get_sys_metrics() -> SysMetrics {
    let sys = System::default();
    let process = sys.get_process(process::id() as i32).unwrap();
    SysMetrics {
        cpu_usage: process.cpu_usage,
        memory: process.memory,
        threads_count: palaver::thread::count(),
    }
}

/// Drain live metrics.
pub fn drain_action_metrics() -> HashMap<&'static str, Vec<ExecutionData>> {
    let metrics = &mut LIVE_METRICS.write().unwrap();
    metrics
        .0
        .drain()
        .fold(HashMap::new(), |mut acc, (_, metrics)| {
            for (name, live_metrics) in metrics.borrow_mut().iter_mut() {
                let metrics = acc.entry(name).or_insert_with(Vec::new);
                metrics.extend(live_metrics.drain(..));
            }
            acc
        })
}

/// List of metrics defined in the thread.
type ThreadLocalMetrics = HashMap<&'static str, Vec<ExecutionData>>;

/// Application metrics.
#[derive(Debug, Default)]
pub(crate) struct Metrics(HashMap<ThreadId, RefCell<ThreadLocalMetrics>>);

/// We use manual synchronization by thread identifier.
unsafe impl Sync for Metrics {}

unsafe impl Send for Metrics {}

impl Metrics {
    /// Stores thread local metric.
    /// Returns None if thread local metrics is available for current thread, given metric otherwise.
    fn store_metric(&self, name: &'static str, metric: ExecutionData) -> Option<ExecutionData> {
        if let Some(local_metrics) = self.0.get(&id()) {
            let mut metrics = local_metrics.borrow_mut();
            let metrics = metrics.entry(name);
            metrics.or_insert_with(Vec::new).push(metric);
            None
        } else {
            Some(metric)
        }
    }

    /// Creates metric for thread local.
    fn crate_local_metrics(&mut self) {
        self.0
            .entry(id())
            .or_insert_with(|| RefCell::new(Default::default()));
    }
}

/// Returns current thread id.
fn id() -> ThreadId {
    thread::current().id()
}

#[cfg(test)]
mod test {
    use std::thread;
    use std::collections::HashSet;
    use crate::metrics::live_time::{
        get_sys_metrics, drain_action_metrics, store_metric, ExecutionData, STORE_METRICS,
    };
    use std::sync::atomic::Ordering;

    #[test]
    pub fn test_multi_thread() {
        STORE_METRICS.store(true, Ordering::Relaxed);

        let handlers = (0..10)
            .map(|i| {
                thread::spawn(move || {
                    for j in 0..100 {
                        store_metric("execute_script", ExecutionData::new(j * i));
                        store_metric("publish_module", ExecutionData::new(j * i * 2));
                        store_metric("data_source_access", ExecutionData::new(j * i * 3));
                    }
                })
            })
            .collect::<Vec<_>>();
        for j in handlers {
            j.join().unwrap();
        }

        let mut metrics = drain_action_metrics();
        assert_eq!(metrics["execute_script"].len(), 1000);
        assert_eq!(metrics["publish_module"].len(), 1000);
        assert_eq!(metrics["data_source_access"].len(), 1000);

        let execute_script_metrics = metrics
            .remove("execute_script")
            .unwrap()
            .into_iter()
            .collect::<HashSet<_>>();
        let publish_module_metrics = metrics
            .remove("publish_module")
            .unwrap()
            .into_iter()
            .collect::<HashSet<_>>();
        let data_source_access_metrics = metrics
            .remove("data_source_access")
            .unwrap()
            .into_iter()
            .collect::<HashSet<_>>();

        for i in 0..10 {
            for j in 0..100 {
                assert!(execute_script_metrics.contains(&ExecutionData::new(j * i)));
                assert!(publish_module_metrics.contains(&ExecutionData::new(j * i * 2)));
                assert!(data_source_access_metrics.contains(&ExecutionData::new(j * i * 3)));
            }
        }
    }

    #[test]
    pub fn test_sys_metrics() {
        get_sys_metrics();
    }
}
