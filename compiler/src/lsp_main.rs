//! Command-line entry point for the Clorraine language server.

fn main() {
    std::process::exit(clc::lsp::run_stdio());
}
