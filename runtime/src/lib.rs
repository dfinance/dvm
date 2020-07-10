//! Integration with MoveVM.

#![warn(missing_docs)]

#[macro_use]
pub extern crate log;

/// Defines dvm `CostTable`.
pub mod gas_schedule;

/// Defines structures for script execution inside VM.
pub mod move_vm;
/// Chain resources.
pub mod resources;
