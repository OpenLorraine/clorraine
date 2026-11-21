//! Language-server entry, with an exact informational version fast path.
//!
//! Every other argument vector retains stdio protocol startup. Version queries
//! must not wait for stdin or a client handshake during local package checks.

use std::ffi::OsStr;

fn main() {
    let mut arguments = std::env::args_os().skip(1);
    if arguments.next().as_deref() == Some(OsStr::new("--version"))
        && arguments.next().is_none()
    {
        println!("clorraine-lsp {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    std::process::exit(clc::lsp::run_stdio());
}
