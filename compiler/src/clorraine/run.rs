//! Running a built program for the `clorraine` developer driver.
//!
//! The host profile inherits the terminal, so the program streams exactly like
//! a normal process. The `hosted-amiga` profile runs under `vamos`, whose
//! Amiga-newline output is captured and stripped of carriage returns before it
//! reaches the terminal. With `--capture` the output is written to the
//! artifact directory instead of streamed.
//!
//! The program's own exit code is returned unchanged, so a failing run is
//! visible to scripts.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use super::toolchain::{self, Runner};

/// Runs `bin` with `runner`, streaming or capturing stdout.
///
/// When `capture` is set, the CR-stripped stdout is written to that path and
/// nothing is streamed. Returns the program's exit code, or 1 when it was
/// killed by a signal or the driver could not spawn it.
pub fn run_program(runner: &Runner, bin: &Path, capture: Option<&Path>) -> i32 {
    let argv = toolchain::runner_command(runner, bin);
    let mut command = Command::new(&argv[0]);
    command.args(&argv[1..]);
    command.stderr(Stdio::inherit());
    // The host binary streams straight to the terminal. The Amiga run and any
    // capture both need the bytes in hand: the first to strip `\r`, the second
    // to write a file.
    let captured = capture.is_some() || matches!(runner, Runner::Vamos);
    command.stdout(if captured {
        Stdio::piped()
    } else {
        Stdio::inherit()
    });
    let child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            eprintln!("clorraine: cannot run '{}': {error}", argv[0]);
            return 1;
        }
    };
    let output = match child.wait_with_output() {
        Ok(output) => output,
        Err(error) => {
            eprintln!("clorraine: cannot wait for '{}': {error}", argv[0]);
            return 1;
        }
    };
    if captured {
        // `vamos` emits Amiga line endings; the terminal wants plain lines.
        let text = String::from_utf8_lossy(&output.stdout).replace('\r', "");
        match capture {
            Some(path) => {
                if let Err(error) = std::fs::write(path, text.as_bytes()) {
                    eprintln!("clorraine: cannot write '{}': {error}", path.display());
                    return 1;
                }
            }
            None => {
                let mut stdout = std::io::stdout();
                let _ = stdout.write_all(text.as_bytes());
                let _ = stdout.flush();
            }
        }
    }
    output.status.code().unwrap_or(1)
}
