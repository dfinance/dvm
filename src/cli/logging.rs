use super::config::*;

pub fn init_logging(opts: &LoggingOptions) -> Result<(), log::SetLoggerError> {
    logging_builder(opts).try_init()
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

fn log_filters_with_verbosity(opts: &LoggingOptions) -> String {
    if opts.verbose {
        format!("{},trace", opts.log_filters)
    } else {
        opts.log_filters.to_string()
    }
}

fn rust_log_compat(rust_log: &str, rust_log_style: &str) {
    use std::env::set_var;
    set_var(RUST_LOG, rust_log);
    set_var(RUST_LOG_STYLE, rust_log_style);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_filters_verbose() {
        let opts = LoggingOptions {
            verbose: true,
            ..Default::default()
        };
        let log_filters = log_filters_with_verbosity(&opts);
        assert_eq!(",trace", &log_filters);
    }
}
