//! Native SDK command-line contracts, observed through real driver processes.
//!
//! Distinct executable helpers prove tool selection, argument boundaries and
//! status propagation. They do not establish Python-wheel compatibility or an
//! Amiga boot; those require the separate installed-artifact acceptance proof.

#![cfg(unix)]

#[path = "native_driver_support/compatibility.rs"]
mod compatibility;
#[path = "native_driver_support/diagnostics.rs"]
mod diagnostics;
#[path = "native_driver_support/emulator_artifact_alias.rs"]
mod emulator_artifact_alias;
#[path = "native_driver_support/emulator_fixtures.rs"]
mod emulator_fixtures;
#[path = "native_driver_support/emulator_lifetime.rs"]
mod emulator_lifetime;
#[path = "native_driver_support/emulator_media.rs"]
mod emulator_media;
#[path = "native_driver_support/emulator_options.rs"]
mod emulator_options;
#[path = "native_driver_support/emulator_staging.rs"]
mod emulator_staging;
#[path = "native_driver_support/emulator_storage_safety.rs"]
mod emulator_storage_safety;
#[path = "native_driver_support/fixtures.rs"]
mod fixtures;
#[path = "native_driver_support/resolution.rs"]
mod resolution;
#[path = "native_driver_support/validation.rs"]
mod validation;
