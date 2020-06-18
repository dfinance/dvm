use crate::config::*;

#[cfg(feature = "sentry")]
pub(crate) mod support_sentry {
    use super::*;
    use sentry::internals::Dsn;
    use sentry::internals::ClientInitGuard;
    use sentry::integrations::panic::register_panic_handler;
    use sentry::integrations::env_logger::init as sentry_log_init;

    /// Create standard logger, init integrations such as with sentry.
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
    use libra::libra_logger as logger;
    use logger::{StructLogSink, StructuredLogEntry};
    use logger::{struct_logger_set, set_struct_logger};

    pub fn init() {
        let logger = TraceLog;
        let logger = Box::leak(Box::new(logger));
        set_struct_logger(logger)
            .map(|_| trace!("internall logger initialized: {}", struct_logger_set()))
            .map_err(|_| {
                warn!("unable to initialize sub-logger");
            })
            .ok();
        // logger::init_println_struct_log()
    }

    struct TraceLog;
    impl StructLogSink for TraceLog {
        fn send(&self, entry: StructuredLogEntry) {
            trace!("{}", serde_json::to_string(&entry).unwrap());
        }
    }
}

pub fn init_logging(opts: &LoggingOptions) -> Result<(), log::SetLoggerError> {
    logging_builder(opts).try_init().and_then(|_| {
        support_libra_logger::init();
        Ok(())
    })
}

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

fn log_level(level: u8) -> log::Level {
    use log::Level::*;
    match level {
        0 => Info,
        1 => Debug,
        2 | _ => Trace,
    }
}

fn log_level_deps(level: u8) -> log::Level {
    use log::Level::*;
    match level {
        0 | 1 | 2 => Info,
        3 => Debug,
        4 | _ => Trace,
    }
}
fn log_filters_deps_format(level: u8) -> String {
    // mio=info,hyper=info,reqwest=info,tokio=info,tokio_util=info,h2=info
    format!(
        "mio={0:},hyper={0:},reqwest={0:},tokio={0:},tokio_util={0:},h2={0:}",
        log_level_deps(level)
    )
}

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
    // if opts.verbose {
    //     format!("{},trace", opts.log_filters)
    // } else {
    //     opts.log_filters.to_string()
    // }
}

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
        // env::remove_var(DVM_SENTRY_DSN);
        // let args = Vec::<String>::with_capacity(0).into_iter();
        // let options = IntegrationsOptions::from_iter_safe(args);
        // assert!(options.is_ok());
        // assert!(options.unwrap().sentry_dsn.is_none());
    }

    fn parse_args_sentry_on() {
        // env::set_var(DVM_SENTRY_DSN, DSN);
        // let args = Vec::<String>::with_capacity(0).into_iter();
        // let options = IntegrationsOptions::from_iter_safe(args);
        // assert!(options.is_ok());

        // let options = options.unwrap();
        // assert!(options.sentry_dsn.is_some());
        // assert_eq!("foobar", options.sentry_dsn.unwrap().public_key());
    }

    fn parse_args_sentry_override() {
        // env::set_var(DVM_SENTRY_DSN, DSN);
        // let args = ["", "--sentry-dsn", "https://0deedbeaf@test.test/0000000"].iter();
        // let options = IntegrationsOptions::from_iter_safe(args);
        // assert!(options.is_ok());

        // let options = options.unwrap();
        // assert!(options.sentry_dsn.is_some());
        // assert_eq!("0deedbeaf", options.sentry_dsn.unwrap().public_key());
    }

    #[test]
    fn log_filters_verbose() {
        let opts = LoggingOptions {
            verbose: MAX_LOG_VERBOSE,
            ..Default::default()
        };
        let log_filters = log_filters_with_verbosity(&opts);
        assert_eq!(",trace", &log_filters);
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
