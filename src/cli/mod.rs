pub mod config;
pub mod logging;

#[cfg(feature = "sentry")]
pub use support_sentry::*;

#[cfg(feature = "sentry")]
mod support_sentry {
    use super::logging;
    use super::config::*;
    use sentry::internals::Dsn;
    use sentry::internals::ClientInitGuard;
    use sentry::integrations::panic::register_panic_handler;
    use sentry::integrations::env_logger::init as sentry_log_init;

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
    logging::init_logging(log)
        .map(|_| trace!("Logging system initialized."))
        .map_err(|err| eprintln!("Attempt to init global logger once more. {:?}", err))
        .ok()
}
