pub mod config;
use config::*;

#[cfg(feature = "sentry")]
pub use support_sentry::*;

#[cfg(feature = "sentry")]
mod support_sentry {
    use super::*;
    use sentry::internals::ClientInitGuard;
    use sentry::integrations::panic::register_panic_handler;
    use sentry::integrations::env_logger::init as sentry_log_init;

    pub fn init(
        log: &LoggingOptions,
        integrations: &IntegrationsOptions,
    ) -> Option<ClientInitGuard> {
        let mut builder = logging_builder(log);
        let guard = if let Some(sentry_dsn) = &integrations.sentry_dsn {
            let sentry = init_sentry(sentry_dsn);
            sentry_log_init(Some(builder.build()), Default::default());
            trace!("Logging system initialized with Sentry.");
            Some(sentry)
        } else {
            builder
                .try_init()
                .map(|_| trace!("Logging system initialized."))
                .map_err(|err| eprintln!("Attempt to init global logger once more. {:?}", err))
                .err();
            info!(
                "{} environment variable is not provided, Sentry integration is going to be disabled",
                DVM_SENTRY_DSN
            );
            None
        };
        guard
    }

    pub fn init_sentry(dsn: &str) -> ClientInitGuard {
        let client = sentry::init(dsn);
        if client.is_enabled() {
            register_panic_handler();
            info!("Sentry integration enabled, panic handler registered.");
        } else {
            trace!("Sentry client disabled");
        }
        client
    }
}

// fallback, just init logging //
#[cfg(not(feature = "sentry"))]
pub fn init(log: &LoggingOptions, _: &IntegrationsOptions) -> Option<()> {
    init_logging(log)
        .map(|_| trace!("Logging system initialized."))
        .map_err(|err| eprintln!("Attempt to init global logger once more. {:?}", err))
        .ok()
}

// logging //

pub fn init_logging(opts: &LoggingOptions) -> Result<(), log::SetLoggerError> {
    logging_builder(opts).try_init()
}

pub fn logging_builder(opts: &LoggingOptions) -> env_logger::Builder {
    use env_logger::{Builder, Target};

    let log_filters = if opts.verbose {
        format!("{},trace", opts.log_filters)
    } else {
        opts.log_filters.to_string()
    };

    rust_log_compat(&opts.log_filters, &opts.log_style);

    let mut builder = Builder::new();
    builder.parse_filters(&log_filters);
    builder.parse_write_style(&opts.log_style);
    builder.target(Target::Stdout);
    builder
}

fn rust_log_compat(rust_log: &str, rust_log_style: &str) {
    use std::env::set_var;
    set_var("RUST_LOG", rust_log);
    set_var("RUST_LOG_STYLE", rust_log_style);
}
