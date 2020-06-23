use std::net::SocketAddr;
use clap::Clap;

/// Configuration for service that gathers metrics about VM execution.
#[derive(Debug, Default, Clone, Clap)]
pub struct InfoServiceConfig {
    /// Info service address  in the form of HOST_ADDRESS:PORT.
    /// Optional parameter. If the address is set, the web service starts.
    #[clap(
        name = "info service listen address. HOST_ADDRESS:PORT",
        long = "info-service-addr",
        short = "i",
        verbatim_doc_comment
    )]
    pub info_service_addr: Option<SocketAddr>,

    /// Metric refresh interval in seconds.
    #[clap(
        default_value = "5",
        name = "seconds between updates",
        long = "metric-update-interval",
        verbatim_doc_comment
    )]
    pub metric_update_interval: u64,

    /// Maximum period between heartbeats. In seconds.
    #[clap(
        default_value = "5",
        name = "max seconds between heartbeats",
        long = "heartbeat-interval-max",
        verbatim_doc_comment
    )]
    pub heartbeat_max_interval: u64,

    /// The interval between ping requests to dvm. In seconds.
    #[clap(
        default_value = "4",
        name = "seconds between heartbeats",
        long = "heartbeat-pressure",
        verbatim_doc_comment
    )]
    pub heartbeat_stimulation_interval: u64,
}
