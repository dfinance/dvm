// TODO: #![warn(missing_docs)]

#[macro_use]
pub extern crate log;
pub mod config;
pub mod logging;

use config::*;
use futures::future::{lazy, Future, FutureExt};

/// Init standard handlers for cli-executable.
/// Create standard logger, init integrations such as with sentry if enabled.
/// By default - init logging without extra integrations.
pub fn init(log: &LoggingOptions, integrations: &IntegrationsOptions) -> Option<impl Drop> {
    use logging::*;
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
}

pub fn init_sigterm_handler() -> std::sync::mpsc::Receiver<signal_notify::Signal> {
    use signal_notify::{notify, Signal};
    notify(&[Signal::TERM])
}

pub fn init_sigterm_handler_fut<F>(f: F) -> impl Future<Output = ()> + Send
where
    F: Send + FnOnce() -> (),
{
    let rx = init_sigterm_handler();
    lazy(move |_| rx.recv()).map(|sig| {
        if let Ok(sig) = sig {
            info!("Received signal {:?}. Shutting down services", sig);

            f();

            use std::time::Duration;
            std::thread::sleep(Duration::from_secs(3));

            println!("exitting complete");
            std::process::exit(130);
        }
    })
}
