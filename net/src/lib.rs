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

pub mod transport {
    #[cfg(any(unix, macos))]
    pub use super::unix::*;

    #[cfg(windows)]
    compile_error!("windows platform is not supported");
}

pub mod prelude {
    //! The DVM-NET Prelude
    //!
    //! The purpose of this module is to give imports-set for many common cases
    //! by adding a glob import to the top:
    //!
    //! ```
    //! # #![allow(unused_imports)]
    //! use dvm_net::prelude::*;
    //! ```
    //!
    //! Also contains reshared `TryInto` and `TryFrom` std traits.


    pub use crate::serve::*;
    pub use crate::endpoint::*;
    pub use crate::transport::*;

    pub use std::convert::{TryInto, TryFrom};
}
