//! Command-line entry point for the `clorraine` developer driver.

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(clc::clorraine::run(args));
}
