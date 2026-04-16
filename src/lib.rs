//! World Compute — core library crate.
//!
//! This crate provides the shared types, modules, and infrastructure used by
//! the agent daemon, CLI, GUI, and adapters.

pub mod error;
pub mod types;

pub mod acceptable_use;
pub mod agent;
pub mod cli;
pub mod credits;
pub mod data_plane;
pub mod governance;
pub mod ledger;
pub mod network;
pub mod preemption;
pub mod sandbox;
pub mod scheduler;
pub mod telemetry;
pub mod verification;
