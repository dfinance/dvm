use std::collections::HashMap;

use serde_derive::Serialize;

use crate::metrics::execution::ExecutionData;

/// Application metrics;
#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub struct Metrics {
    /// Inner metrics state: (name of a metric) -> ExecutionMetric
    pub execution_metrics: HashMap<&'static str, ExecutionMetric>,
}

impl Metrics {
    /// Calculate metrics based on a list of executions.
    pub fn calculate(executions: HashMap<&'static str, Vec<ExecutionData>>) -> Metrics {
        let execution_metrics = executions
            .into_iter()
            .map(|(name, metrics)| (name, ExecutionMetric::calculate(metrics)))
            .collect();

        Metrics {
            execution_metrics,
        }
    }
}

/// Time to process different chunks of executions.
#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub struct Percentiles {
    /// 50% percentile
    pub p_50: u64,
    /// 75%
    pub p_75: u64,
    /// 90%
    pub p_90: u64,
}

/// Average.
#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub struct Average {
    /// Average.
    pub avg: u64,
    ///Standard deviation.
    pub sd: f64,
}

/// Aggregate for the executions metrics.
#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub struct ExecutionMetric {
    /// The number of actions performed in the time interval.
    pub total_executions: u64,
    /// Number of actions without results. (Actions with panic.).
    pub executions_without_results: u64,
    /// The number of actions completed with success.
    pub success_actions: u64,
    /// struct -> count
    pub statuses: HashMap<u64, u64>,
    /// Total gas used in the interval.
    pub total_gas: u64,
    /// Percentiles.
    pub percentiles: Percentiles,
    /// Average.
    pub average: Average,
    /// Min time.
    pub min_time: u64,
    /// Max time.
    pub max_time: u64,
}

impl ExecutionMetric {
    /// Calculate metrics based on provided execution data.
    pub fn calculate(mut metrics: Vec<ExecutionData>) -> ExecutionMetric {
        let executions_count = metrics.len() as u64;
        let mut min_time = 0;
        let mut max_time = 0;
        let mut actions_without_results = 0;
        let mut success_actions = 0;
        let mut statuses = HashMap::new();
        let mut total_gas = 0;

        let mut percentiles = Percentiles::default();
        let mut average = Average::default();

        if !metrics.is_empty() {
            metrics.sort_by(|a, b| a.process_time.cmp(&b.process_time));
            min_time = metrics[0].process_time;
            max_time = metrics[executions_count as usize - 1].process_time;

            let mut total_time = 0;
            let mut quadratic_total_time = 0;

            percentiles.p_50 = metrics[executions_count as usize * 50 / 100].process_time;
            percentiles.p_75 = metrics[executions_count as usize * 75 / 100].process_time;
            percentiles.p_90 = metrics[executions_count as usize * 90 / 100].process_time;

            for metric in metrics {
                total_time += metric.process_time;
                quadratic_total_time += metric.process_time * metric.process_time;

                if let Some(res) = metric.result {
                    if res.is_success {
                        success_actions += 1;
                    }

                    *statuses.entry(res.status).or_insert(0) += 1;
                    total_gas += res.gas_used;
                } else {
                    actions_without_results += 1;
                }
            }

            average.avg = total_time / executions_count;
            quadratic_total_time /= executions_count;
            average.sd = ((quadratic_total_time - (average.avg * average.avg)) as f64).sqrt();
        }

        ExecutionMetric {
            total_executions: executions_count,
            executions_without_results: actions_without_results,
            success_actions,
            statuses,
            total_gas,
            percentiles,
            average,
            min_time,
            max_time,
        }
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use crate::metrics::execution::{ExecutionData, ExecutionResult};
    use crate::metrics::metric::{Average, ExecutionMetric, Metrics, Percentiles};

    #[test]
    fn test_empty_metrics_calculation() {
        assert_eq!(
            Metrics::calculate(HashMap::new()),
            Metrics {
                execution_metrics: Default::default(),
            }
        );

        let mut m = HashMap::new();
        m.insert("test", vec![]);
        let mut expected = HashMap::new();
        expected.insert("test", Default::default());
        assert_eq!(
            Metrics::calculate(m),
            Metrics {
                execution_metrics: expected,
            }
        );
    }

    fn panic(time: u64) -> ExecutionData {
        ExecutionData {
            process_time: time,
            result: None,
        }
    }

    fn success(time: u64, status: u64, gas_used: u64) -> ExecutionData {
        ExecutionData {
            process_time: time,
            result: Some(ExecutionResult {
                is_success: true,
                status,
                gas_used,
            }),
        }
    }

    fn fail(time: u64, status: u64, gas_used: u64) -> ExecutionData {
        ExecutionData {
            process_time: time,
            result: Some(ExecutionResult {
                is_success: false,
                status,
                gas_used,
            }),
        }
    }

    macro_rules! seq {
    ($($x:expr),+) => {
        [$($x,)+].iter().map(|&x| x).collect()
    }
}

    #[test]
    fn test_execution_metrics() {
        let data = vec![
            success(172, 200, 10),
            success(170, 400, 10),
            fail(169, 200, 0),
            success(169, 500, 10),
            success(167, 200, 700),
            panic(173),
        ];

        let metric = ExecutionMetric::calculate(data);
        assert_eq!(
            metric,
            ExecutionMetric {
                total_executions: 6,
                executions_without_results: 1,
                success_actions: 4,
                statuses: seq!((200, 3), (500, 1), (400, 1)),
                total_gas: 730,
                percentiles: Percentiles {
                    p_50: 170,
                    p_75: 172,
                    p_90: 173,
                },
                average: Average { avg: 170, sd: 2.0 },
                min_time: 167,
                max_time: 173,
            }
        );
    }
}
