// TODO: #![warn(missing_docs)]

#[macro_use]
pub extern crate log;
pub mod config;
pub mod logging;

use config::*;

/// Init standard handlers for cli-executable.
/// Create standard logger, init integrations such as with sentry if enabled.
/// By default - init logging without extra integrations.
pub fn init(
    log: &LoggingOptions,
    integrations: &IntegrationsOptions,
) -> (ShutdownSignal, Option<impl Drop>) {
    use logging::*;
    let guard = {
        #[cfg(feature = "sentry")]
        {
            support_sentry::init(log, integrations)
        }
        #[cfg(not(feature = "sentry"))]
        {
            init_logging(log)
                .map(|_| trace!("Logging system initialized."))
                .map_err(|err| eprintln!("Attempt to init global logger once more. {:?}", err))
                .ok()
        }
    };
    (init_signals(), guard)
}

pub type ShutdownSignal = std::sync::mpsc::Receiver<signal_notify::Signal>;

pub fn init_signals() -> ShutdownSignal {
    use signal_notify::{notify, Signal};
    notify(&[Signal::TERM])
}
