//! Command-line entry point for the `clc` compiler.

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(clc::driver::run(args));
}
