// TODO: #![warn(missing_docs)]
#[macro_use]
pub extern crate log;
extern crate tokio;
pub extern crate dvm_api as api;
pub use api::tonic;

pub type StdError = Box<dyn std::error::Error + Send + Sync>;

pub mod endpoint;
pub mod serve;

#[cfg(any(unix, macos))]
mod unix;

#[cfg(windows)]
mod win;

pub mod transport {
    #[cfg(any(unix, macos))]
    pub use super::unix::*;

    #[cfg(windows)]
    pub use super::win::*;
}

pub mod prelude {
    // pub use dvm_api as api;

    pub use crate::serve::*;
    pub use crate::endpoint::*;
    pub use crate::transport::*;

    pub use std::convert::{TryInto, TryFrom};
}
