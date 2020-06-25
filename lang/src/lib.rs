//! Stores `stdlib` export and bytecode verification procedures.

#![warn(missing_docs)]

#[macro_use]
extern crate anyhow;
extern crate libra;
extern crate include_dir;

// simply reexport stdlib for compatibility
pub extern crate stdlib;

/// Procedures to work with bytecode.
pub mod bytecode;
