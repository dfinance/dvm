use crate::metrics::metric::{Metrics, ExecutionMetric};
use crate::metrics::live_time::SysMetrics;
use prometheus_exporter_base::{PrometheusMetric, MetricType};
use std::collections::HashMap;
use once_cell::sync::Lazy;
use sys_info::hostname;

static METRIC_HEADER: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert(
        "total_requests",
        "The number of requests performed in the time interval.",
    );
    m.insert(
        "executions_without_results",
        "Number of actions without results. (Actions with panic.)",
    );
    m.insert(
        "success_actions",
        "The number of actions completed with success.",
    );
    m.insert(
        "action_with_status",
        "The number of actions completed with status.",
    );
    m.insert("total_gas", "Total gas used in the interval.");
    m.insert("percentile", "percentiles");
    m.insert("average", "Average time. (im milliseconds)");
    m.insert(
        "standard_deviation",
        "Standard deviation. (im milliseconds)",
    );
    m.insert("min_time", "Minimum time. (im milliseconds)");
    m.insert("max_time", "Maximum time. (im milliseconds)");
    m
});
static HOST_NAME: Lazy<String> = Lazy::new(|| hostname().unwrap_or_else(|_| "None".to_string()));

macro_rules! store {
    ($buf:expr, $pm:expr, $metric_name:expr, $val:expr) => {
        $buf.push_str(&$pm.render_sample(
            Some(&[
                ("service_name", "dvm"),
                ("host_name", &HOST_NAME),
                ("process", $metric_name),
            ]),
            $val,
        ))
    };
    ($buf:expr, $pm:expr, $metric_name:expr, $name:expr, $p:expr, $val:expr) => {
        $buf.push_str(&$pm.render_sample(
            Some(&[
                ("service_name", "dvm"),
                ("host_name", &HOST_NAME),
                ("process", $metric_name),
                ($name, $p),
            ]),
            $val,
        ))
    };
}

/// Encode metrics.
pub fn encode_metrics(
    system_metrics: Option<SysMetrics>,
    metrics: Metrics,
    metrics_list: &[&str],
) -> String {
    let mut buf = String::new();

    if let Some(sys_metrics) = system_metrics {
        encode_sys_metrics(&mut buf, &sys_metrics);
        buf.push('\n');
    }

    let empty = ExecutionMetric::default();

    for (field, description) in METRIC_HEADER.iter() {
        let counter_name = format!("dvm_{}", field);
        let pm = PrometheusMetric::new(&counter_name, MetricType::Gauge, description);
        buf.push_str(&pm.render_header());
        for name in metrics_list {
            let metric = metrics.execution_metrics.get(name).unwrap_or(&empty);

            match *field {
                "total_requests" => store!(buf, pm, name, metric.total_executions),
                "executions_without_results" => {
                    store!(buf, pm, name, metric.executions_without_results)
                }
                "success_actions" => store!(buf, pm, name, metric.success_actions),
                "action_with_status" => {
                    for (status, count) in metric.statuses.iter() {
                        store!(buf, pm, name, "status", &status.to_string(), *count);
                    }
                }
                "total_gas" => store!(buf, pm, name, metric.total_gas),
                "percentile" => {
                    let percentiles = &metric.percentiles;
                    store!(buf, pm, name, "p", "50", percentiles.p_50);
                    store!(buf, pm, name, "p", "75", percentiles.p_75);
                    store!(buf, pm, name, "p", "90", percentiles.p_90);
                }
                "average" => store!(buf, pm, name, metric.average.avg),
                "standard_deviation" => store!(buf, pm, name, metric.average.sd),
                "min_time" => store!(buf, pm, name, metric.min_time),
                "max_time" => store!(buf, pm, name, metric.max_time),
                _ => {
                    // no-op
                }
            }
        }

        buf.push('\n');
    }

    buf
}

/// Encode system metrics.
fn encode_sys_metrics(buf: &mut String, metric: &SysMetrics) {
    let pc = PrometheusMetric::new(
        "dvm_sys_info_cpu_usage",
        MetricType::Gauge,
        "CPU used by the process",
    );
    buf.push_str(&pc.render_header());
    buf.push_str(&pc.render_sample(
        Some(&[("service_name", "dvm"), ("host_name", &HOST_NAME)]),
        metric.cpu_usage,
    ));

    let pc = PrometheusMetric::new(
        "dvm_sys_info_memory",
        MetricType::Gauge,
        "Memory used by the process (in kB).",
    );
    buf.push_str(&pc.render_header());
    buf.push_str(&pc.render_sample(
        Some(&[("service_name", "dvm"), ("host_name", &HOST_NAME)]),
        metric.memory,
    ));

    let pc = PrometheusMetric::new(
        "dvm_sys_info_threads_count",
        MetricType::Gauge,
        "Threads count.",
    );
    buf.push_str(&pc.render_header());
    buf.push_str(&pc.render_sample(
        Some(&[("service_name", "dvm"), ("host_name", &HOST_NAME)]),
        metric.threads_count,
    ));
}
