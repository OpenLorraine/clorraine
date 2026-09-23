//! The `clorraine` developer driver: build and run Clorraine programs.
//!
//! The driver fronts the emit-only compiler (`clc`) and owns the build/run
//! orchestration: it emits the C, compiles it with the profile's toolchain, and
//! runs it, so a developer sees only the program's output. This module is the
//! testable entry point; the `clorraine` binary is a thin wrapper.

/// Runs the driver with the given arguments (without the program name).
///
/// Not implemented yet. Every invocation reports the driver as unavailable so
/// the specification tests can compile and fail before the behaviour lands.
pub fn run(_args: Vec<String>) -> i32 {
    eprintln!("clorraine: not implemented");
    2
}
