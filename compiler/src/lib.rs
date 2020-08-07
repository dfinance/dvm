//! Move compiler.

#![deny(missing_docs)]

#[macro_use]
extern crate anyhow;
#[macro_use]
extern crate log;

/// Movec commands handler.
pub mod cmd;
/// Move embedded compiler.
mod embedded;
/// Movec configuration.
pub mod manifest;
/// Move compiler components.
mod mv;

pub use mv::*;
pub use embedded::Compiler;
pub use embedded::compile;
#[cfg(test)]
pub use disassembler;
