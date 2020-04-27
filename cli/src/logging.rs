use crate::config::*;

#[cfg(feature = "sentry")]
pub use support_sentry::*;

#[cfg(feature = "sentry")]
mod support_sentry {
    use sentry::internals::Dsn;
    use sentry::internals::ClientInitGuard;
    use sentry::integrations::panic::register_panic_handler;
    use sentry::integrations::env_logger::init as sentry_log_init;
    use crate::config::*;
    use crate::logging;

    pub fn init(
        log: &LoggingOptions,
        integrations: &IntegrationsOptions,
    ) -> Option<ClientInitGuard> {
        let mut builder = logging::logging_builder(log);
        if let Some(sentry_dsn) = &integrations.sentry_dsn {
            sentry_log_init(Some(builder.build()), Default::default());

            let sentry = init_sentry(sentry_dsn, &integrations.sentry_env);
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
        }
    }

    pub fn init_sentry(dsn: &Dsn, env: &Option<String>) -> ClientInitGuard {
        // back-compat to default env var:
        std::env::set_var("SENTRY_DSN", format!("{}", &dsn));

        let client = {
            let mut options = sentry::ClientOptions::default();
            options.dsn = Some(dsn.to_owned());
            if let Some(ref env) = env {
                trace!("sentry env: {}", env);
                options.environment = Some(env.to_owned().into());
            }
            sentry::init(options)
        };
        if client.is_enabled() {
            register_panic_handler();
            trace!("Sentry integration enabled, panic handler registered.");
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
    set_var(RUST_LOG, rust_log);
    set_var(RUST_LOG_STYLE, rust_log_style);
}
