//! Compiler library behind the `clc` command.
//!
//! The binary in `main.rs` is a thin wrapper; the compiler stages live here so
//! they can be exercised directly by unit tests. Front-end stages are added as
//! the compiler grows.

#![forbid(unsafe_code)]

pub mod diag;
pub mod lex;
pub mod span;
pub mod token;
