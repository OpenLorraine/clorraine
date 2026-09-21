//! Compiler library behind the `clc` command.
//!
//! The binary in `main.rs` is a thin wrapper; the compiler stages live here so
//! they can be exercised directly by unit tests. Front-end stages are added as
//! the compiler grows.

#![forbid(unsafe_code)]

pub mod abi;
pub mod ast;
pub mod borrow;
pub mod codegen;
pub mod consts;
pub mod diag;
pub mod driver;
pub mod ffi;
pub mod init_graph;
pub mod ir;
pub mod lex;
pub mod load;
pub mod own;
pub mod parse;
pub mod profile;
pub mod resolve;
pub mod sema;
pub mod span;
pub mod stdlib;
pub mod token;
pub mod types;
