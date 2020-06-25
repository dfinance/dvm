//! gRPC services definitions.

#![warn(missing_docs)]

#[macro_use]
extern crate anyhow;

use dvm_net::{api, tonic};

/// gRPC service for compiler.
pub mod compiler;

/// gRPC service for script signature parameters.
pub mod metadata;

/// gRPC service for vm script execution.
pub mod vm;
