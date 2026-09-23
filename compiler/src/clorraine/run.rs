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

use super::display;
use super::toolchain::{self, Runner};

/// The observable result of one program run.
pub struct RunResult {
    /// The program's exit code, or 1 when the driver could not run it.
    pub code: i32,
    /// True only when `--capture` was requested and its output was written.
    ///
    /// The caller prints the `captured <path>` success line on this flag, so a
    /// failed write is never reported as a capture.
    pub captured: bool,
}

/// Runs `bin` with `runner`, streaming or capturing stdout.
///
/// When `capture` is set, the CR-stripped stdout is written to that path and
/// nothing is streamed. The reported code is the program's exit code, or 1
/// when it was killed by a signal or the driver could not spawn it.
pub fn run_program(runner: &Runner, bin: &Path, capture: Option<&Path>) -> RunResult {
    let argv = toolchain::runner_command(runner, bin);
    let mut command = Command::new(&argv[0]);
    command.args(&argv[1..]);
    command.stderr(Stdio::inherit());
    // The host binary streams straight to the terminal. The Amiga run and any
    // capture both need the bytes in hand: the first to strip `\r`, the second
    // to write a file.
    let piped = capture.is_some() || matches!(runner, Runner::Vamos);
    command.stdout(if piped {
        Stdio::piped()
    } else {
        Stdio::inherit()
    });
    let child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            eprintln!(
                "clorraine: cannot run '{}': {error}",
                display::sanitize(&argv[0])
            );
            return RunResult {
                code: 1,
                captured: false,
            };
        }
    };
    let output = match child.wait_with_output() {
        Ok(output) => output,
        Err(error) => {
            eprintln!(
                "clorraine: cannot wait for '{}': {error}",
                display::sanitize(&argv[0])
            );
            return RunResult {
                code: 1,
                captured: false,
            };
        }
    };
    let mut written = false;
    if piped {
        // `vamos` emits Amiga line endings; the terminal wants plain lines.
        let text = String::from_utf8_lossy(&output.stdout).replace('\r', "");
        match capture {
            Some(path) => {
                if let Err(error) = std::fs::write(path, text.as_bytes()) {
                    eprintln!(
                        "clorraine: cannot write '{}': {error}",
                        display::sanitize(&path.display().to_string())
                    );
                    return RunResult {
                        code: 1,
                        captured: false,
                    };
                }
                written = true;
            }
            None => {
                let mut stdout = std::io::stdout();
                let _ = stdout.write_all(text.as_bytes());
                let _ = stdout.flush();
            }
        }
    }
    RunResult {
        code: output.status.code().unwrap_or(1),
        captured: written,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn a_failed_capture_write_is_not_reported_as_captured() {
        // `/bin/true` runs and prints nothing, so the only failure is the
        // capture write into a directory that does not exist.
        let result = run_program(
            &Runner::Native,
            Path::new("/bin/true"),
            Some(Path::new("/no/such/clorraine/dir/out.txt")),
        );
        assert!(
            !result.captured,
            "a failed capture write must not be reported as a capture"
        );
        assert_eq!(result.code, 1, "the driver reports the write failure");
    }
}
