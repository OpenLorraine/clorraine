//! Implementation boundaries beneath the independently frozen native contracts.

#![cfg(unix)]

// Shared fixtures serve the complete specification suite. These focused tests
// intentionally use only a subset of their capabilities, without changing them.
#[allow(dead_code)]
#[path = "native_driver_support/fixtures.rs"]
mod fixtures;
#[allow(dead_code)]
#[path = "native_driver_support/emulator_fixtures.rs"]
mod emulator_fixtures;
#[path = "native_driver_impl/paths.rs"]
mod paths;
#[path = "native_driver_impl/races.rs"]
mod races;
