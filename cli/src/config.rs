use clap::Clap;

// rust env variables
pub const RUST_LOG: &str = "RUST_LOG";
pub const RUST_LOG_STYLE: &str = "RUST_LOG_STYLE";
// dvm env variables
pub const DVM_LOG: &str = "DVM_LOG";
pub const DVM_LOG_STYLE: &str = "DVM_LOG_COLOR";
pub const DVM_DATA_SOURCE: &str = "DVM_DATA_SOURCE";
pub const DVM_SENTRY_DSN: &str = "DVM_SENTRY_DSN";
pub const DVM_SENTRY_ENV: &str = "DVM_SENTRY_ENVIRONMENT";

pub const MAX_LOG_VERBOSE: u8 = 4;

#[derive(Debug, Default, Clone, Clap)]
pub struct LoggingOptions {
    /// Enables verbosity logging mode.
    /// Sets level of verbosity, and can be used multiple times.
    /// Overrides or extends values passed as the `--log` parameter or `DVM_LOG` environment variable.
    /// To set maximum level of verbosity use `-vvvv`.
    #[clap(short, long, parse(from_occurrences), verbatim_doc_comment)]
    pub verbose: u8,

    /// Log filters.
    /// The same as standard RUST_LOG environment variable.
    /// Possible values in verbosity ordering: error, warn, info, debug and trace.
    /// For complex filters see documentation: https://docs.rs/env_logger/#filtering-results
    #[clap(
        long = "log",
        env = DVM_LOG,
        default_value = "info,dvm=info,hyper=warn,mio=warn",
        verbatim_doc_comment
    )]
    pub log_filters: String,

    /// Log colors and other styles.
    /// The same as standard RUST_LOG_STYLE environment variable.
    /// Possible values in verbosity ordering: auto, always, never.
    #[clap(
        long = "log-color",
        env = DVM_LOG_STYLE,
        default_value = "auto",
        verbatim_doc_comment
    )]
    pub log_style: String,
}

#[derive(Debug, Default, Clone, Clap)]
pub struct IntegrationsOptions {
    /// Optional key-uri, enables crash logging service integration.
    /// If value ommited, crash logging service will not be initialized.
    #[clap(name = "Sentry DSN", long = "sentry-dsn", env = DVM_SENTRY_DSN)]
    #[cfg(feature = "sentry")]
    pub sentry_dsn: Option<sentry::internals::Dsn>,

    /// Sets the environment code to separate events from testnet and production. Optional.
    /// Works with Sentry integration.
    #[clap(name = "Sentry environment", long = "sentry-env", env = DVM_SENTRY_ENV)]
    #[cfg(feature = "sentry")]
    pub sentry_env: Option<String>,
}
