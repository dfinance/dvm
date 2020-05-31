use std::net::SocketAddr;
use structopt::StructOpt;

#[derive(Debug, Default, StructOpt, Clone)]
pub struct InfoServiceConfig {
    /// Info service address  in the form of HOST_ADDRESS:PORT.
    /// Optional parameter. If the address is set, the web service starts.
    #[structopt(
        name = "info service listen address. HOST_ADDRESS:PORT",
        long = "info-service-addr",
        short = "i",
        verbatim_doc_comment
    )]
    pub info_service_addr: Option<SocketAddr>,

    /// Metric refresh interval in seconds.
    #[structopt(
        name = "Metric refresh interval in seconds.",
        default_value = "5",
        long = "metric-update-interval",
        verbatim_doc_comment
    )]
    pub metric_update_interval: u64,

    /// Maximum period between heartbeats. In seconds.
    #[structopt(
        name = "Maximum period between heartbeats.",
        default_value = "5",
        long = "max-heartbeat-interval",
        verbatim_doc_comment
    )]
    pub heartbeat_max_interval: u64,

    /// The interval between ping requests to dvm. In seconds.
    #[structopt(
        name = "The interval between ping requests to dvm. In seconds.",
        default_value = "4",
        long = "heartbeat_stimulation_interval",
        verbatim_doc_comment
    )]
    pub heartbeat_stimulation_interval: u64,
}
