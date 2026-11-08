//! The Clorraine language server.
//!
//! The server speaks JSON-RPC over stdio, reuses the compiler's shared
//! analysis entry point, and keeps the protocol layer std-only. It exposes
//! diagnostics, hover, goto definition, completion, and document symbols.
//!
//! # Examples
//!
//! ```text
//! $ clorraine-lsp
//! Content-Length: 123
//!
//! {"jsonrpc":"2.0","id":1,"method":"initialize","params":{...}}
//! ```

pub mod json;
pub mod protocol;
pub mod server;
pub mod text;
pub mod uri;
pub mod workspace;

/// Runs the server over the process standard streams.
///
/// Returns the process exit code: zero after `shutdown` then `exit`, one when
/// `exit` arrives without `shutdown` or the stream fails.
pub fn run_stdio() -> i32 {
    server::run_stdio()
}
