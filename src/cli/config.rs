use structopt::StructOpt;
use anyhow::Result;

pub fn get_sentry_dsn() -> Result<String> {
    std::env::var("SENTRY_DSN")
        .map_err(|_| anyhow!("SENTRY_DSN environment variable is not provided, Sentry integration is going to be disabled"))
}


#[derive(Debug, StructOpt, Clone)]
pub struct LoggingOptions {
    /// Enables verbose logging mode.
    #[structopt(long = "verbose", short = "v")]
    pub verbose: bool,
}

#[derive(Debug, StructOpt, Clone)]
pub struct IntegrationsOptions {
    /// Optional crash logging service integration.
    // If value ommited, crash logging service will not be initialized.
    #[structopt(name = "Sentry DSN", env = "DVM_SENTRY_DSN")]
    pub sentry_dsn: Option<String>,
}
