// TODO: #![warn(missing_docs)]
#[macro_use]
pub extern crate log;
extern crate tokio;
pub extern crate grpc;
pub use grpc::tonic;

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
    pub use crate::serve::*;
    pub use crate::endpoint::*;
    pub use crate::transport::*;
}
