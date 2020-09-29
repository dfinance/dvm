use crate::config::*;

#[cfg(feature = "sentry")]
pub(crate) mod support_sentry {
    use super::*;
    use sentry::internals::Dsn;
    use sentry::internals::ClientInitGuard;
    use sentry::integrations::panic::register_panic_handler;
    use sentry::integrations::env_logger::init as sentry_log_init;

    /// Create standard logger, init integrations such as with sentry.
    /// At the end init Libra's logger.
    pub fn init(
        log: &LoggingOptions,
        integrations: &IntegrationsOptions,
    ) -> Option<ClientInitGuard> {
        let mut builder = logging_builder(log);
        let result = if let Some(sentry_dsn) = &integrations.sentry_dsn {
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
        };
        support_libra_logger::init();
        result
    }

    /// Init integration with Sentry:
    /// - integrate logger
    /// - register panic handler.
    ///
    /// Returns guard for panic handler and api-client.
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

mod support_libra_logger {
    use libra::logger::Logger;

    pub fn init() {
        Logger::builder().build();
    }
}

/// Try init `env_logger` and then Libra's logger.
pub fn init_logging(opts: &LoggingOptions) -> Result<(), log::SetLoggerError> {
    logging_builder(opts)
        .try_init()
        .map(|_| support_libra_logger::init())
}

/// Create and preconfigure `env_logger::Builder` using `LoggingOptions`
/// typically previously produced by arguments passed to cli.
pub fn logging_builder(opts: &LoggingOptions) -> env_logger::Builder {
    use env_logger::{Builder, Target};

    let log_filters = log_filters_with_verbosity(&opts);

    rust_log_compat(&opts.log_filters, &opts.log_style);

    let mut builder = Builder::new();
    builder.parse_filters(&log_filters);
    builder.parse_write_style(&opts.log_style);
    builder.target(Target::Stdout);
    builder
}

/// Produces log-level for all packages
/// including `dvm`,
/// excluding some third-party dependencies.
/// ```skip
/// 0     => Info,
/// 1     => Debug,
/// 2 | _ => Trace,
/// ```
fn log_level(level: u8) -> log::Level {
    use log::Level::*;
    match level {
        0 => Info,
        1 => Debug,
        _ => Trace,
    }
}

/// Produces log-level for third-party dependencies.
/// ```skip
/// 0 | 1 | 2 => Info,
/// 3         => Debug,
/// 4 | _     => Trace,
/// ```
fn log_level_deps(level: u8) -> log::Level {
    use log::Level::*;
    match level {
        0 | 1 | 2 => Info,
        3 => Debug,
        _ => Trace,
    }
}

/// Produces log-filters-string in standard `RUST_LOG`-format.
/// The log-level getting from `log_level_deps`.
///
/// List of filters contains crates:
/// - mio
/// - hyper
/// - reqwest
/// - tokio
/// - tokio_util
/// - h2
fn log_filters_deps_format(level: u8) -> String {
    format!(
        "mio={0:},hyper={0:},reqwest={0:},tokio={0:},tokio_util={0:},h2={0:}",
        log_level_deps(level)
    )
}

/// Produces log-filters-string in standard `RUST_LOG`-format
/// depending on the passed `LoggingOptions` including number of verbosity reqs, e.g. `-vvvv`
fn log_filters_with_verbosity(opts: &LoggingOptions) -> String {
    match opts.verbose {
        0 => opts.log_filters.to_string(),
        _ => format!(
            "{0:},{1:},{2:}",
            opts.log_filters,
            log_level(opts.verbose),
            log_filters_deps_format(opts.verbose)
        ),
    }
}

/// Set env vars `RUST_LOG` & `RUST_LOG_STYLE` for backward compatibility.
fn rust_log_compat(rust_log: &str, rust_log_style: &str) {
    use std::env::set_var;
    set_var(RUST_LOG, rust_log);
    set_var(RUST_LOG_STYLE, rust_log_style);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use clap::Clap;

    const DSN: &str = "https://foobar@test.test/0000000";

    #[test]
    fn parse_args_sentry() {
        // three tests in one because this needs to run on single thread
        // and we don't want set it up for all other tests.
        parse_args_sentry_off();
        parse_args_sentry_on();
        parse_args_sentry_override();
    }

    fn parse_args_sentry_off() {
        env::remove_var(DVM_SENTRY_DSN);
        let args = Vec::<String>::with_capacity(0).into_iter();
        let options = IntegrationsOptions::try_parse_from(args);
        assert!(options.is_ok());
        assert!(options.unwrap().sentry_dsn.is_none());
    }

    fn parse_args_sentry_on() {
        env::set_var(DVM_SENTRY_DSN, DSN);
        let args = Vec::<String>::with_capacity(0).into_iter();
        let options = IntegrationsOptions::try_parse_from(args);
        assert!(options.is_ok());

        let options = options.unwrap();
        assert!(options.sentry_dsn.is_some());
        assert_eq!("foobar", options.sentry_dsn.unwrap().public_key());
    }

    fn parse_args_sentry_override() {
        env::set_var(DVM_SENTRY_DSN, DSN);
        let args = ["", "--sentry-dsn", "https://0deedbeaf@test.test/0000000"].iter();
        let options = IntegrationsOptions::try_parse_from(args);
        assert!(options.is_ok());

        let options = options.unwrap();
        assert!(options.sentry_dsn.is_some());
        assert_eq!("0deedbeaf", options.sentry_dsn.unwrap().public_key());
    }

    const DEF_LOG_FILTERS: &str = "default";

    #[test]
    fn log_filters_verbose_0() {
        let log_filters = log_filters_verbose_v(0);
        assert_eq!(DEF_LOG_FILTERS, &log_filters);
    }

    #[test]
    fn log_filters_verbose_1() {
        let log_filters = log_filters_verbose_v(1);
        assert!(log_filters.starts_with(&format!("{},{}", DEF_LOG_FILTERS, log::Level::Debug)));
    }

    #[test]
    fn log_filters_verbose_2() {
        let expected = format!("{},{}", DEF_LOG_FILTERS, log::Level::Trace);
        assert!(log_filters_verbose_v(2).starts_with(&expected));
        assert!(log_filters_verbose_v(3).starts_with(&expected));
        assert!(log_filters_verbose_v(4).starts_with(&expected));
    }

    fn log_filters_verbose_v(v: u8) -> String {
        let opts = LoggingOptions {
            verbose: v,
            log_filters: DEF_LOG_FILTERS.to_owned(),
            ..Default::default()
        };
        log_filters_with_verbosity(&opts)
    }

    #[cfg(feature = "integrity-tests")]
    mod integrity {
        use super::*;

        /**
            Test integration with Sentry.
            Panics subthread, push to service, check existing, fails if report isn't not exists.

            Requires ENV vars:
            - `SENTRY_HOST_URI`: e.g. `"sentry.com"`
            - `SENTRY_COMPANY_PROJECT`: `"company/project"`
            - `SENTRY_API_TOKEN`
            - `SENTRY_CROSS_RANDOM_TAG`: e.g. `git rev-parse HEAD`

            Example:
            ```bash
            SENTRY_HOST_URI="sentry.com"
            SENTRY_COMPANY_PROJECT="company/project"
            SENTRY_API_TOKEN="foobartoken"
            export SENTRY_CROSS_RANDOM_TAG=`git rev-parse HEAD`
            DVM_SENTRY_DSN="https://$DSN_PUB_KEY@$SENTRY_HOST_URI/$PROJECT_ID"
            cargo test -p dvm-cli --manifest-path cli/Cargo.toml --lib --features integrity-tests panic_sentry_integrity -- --nocapture && \
            curl -H "Authorization: Bearer ${SENTRY_API_TOKEN}" https://$SENTRY_HOST_URI/api/0/projects/$SENTRY_COMPANY_PROJECT/events/ 2>&1 | grep "$SENTRY_CROSS_RANDOM_TAG" 1>/dev/null
            ```
        */
        #[test]
        #[cfg(feature = "sentry")]
        fn panic_sentry_integrity() {
            use std::{time::Duration, env};
            use std::thread::{spawn, sleep};

            static DSN: &str = env!("DVM_SENTRY_DSN");
            static TEXT: &str = env!("SENTRY_CROSS_RANDOM_TAG");
            let env = env::var(DVM_SENTRY_ENV).unwrap_or("testing".to_owned());

            let logging = LoggingOptions {
                log_filters: "trace".to_owned(),
                ..Default::default()
            };
            let integrations = IntegrationsOptions {
                sentry_dsn: Some(DSN.parse().expect("invalid DVM_SENTRY_DSN")),
                sentry_env: env.into(),
            };

            let _guard = crate::init(&logging, &integrations);
            let handle = spawn(move || panic!(TEXT));
            sleep(Duration::from_secs(5));
            sleep(Duration::from_secs(5));

            assert!(handle.join().is_err());
        }
    }
}
