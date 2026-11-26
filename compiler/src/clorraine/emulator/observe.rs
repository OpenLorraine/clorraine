//! Guest marker observation and bounded emulator termination.
//!
//! The loop reads the rc and status files without ever using an emulator exit
//! status as a guest result. A complete result is rc 0 plus status `ok`; every
//! other observed state is a failure or a timeout.

use std::fs;
use std::io;
use std::path::Path;
use std::process::Child;
use std::time::{Duration, Instant};

use super::storage;

/// How often the host checks the guest markers.
const POLL_INTERVAL: Duration = Duration::from_millis(250);

/// How long a partial marker may stay incomplete before it is a failure.
const PARTIAL_GRACE: Duration = Duration::from_secs(1);

/// How long a killed emulator may take to be reaped before the run is retained.
const KILL_GRACE: Duration = Duration::from_secs(5);

/// One observed guest marker.
enum Marker {
    /// The file does not exist yet.
    Missing,
    /// The file exists but is empty or not yet well formed.
    Partial,
    /// A numeric return code.
    Number(i64),
    /// The exact success status.
    Ok,
    /// The file could not be read.
    Error(String),
}

/// What the poll loop observed.
pub(super) enum Outcome {
    /// rc 0 plus status `ok`.
    Success(Vec<u8>),
    /// A nonzero guest return code.
    GuestFailure(i64, Vec<u8>),
    /// Missing, malformed, or partial markers.
    Incomplete(Vec<u8>),
    /// No marker appeared before the timeout.
    TimedOut(Vec<u8>),
    /// A marker or result could not be read.
    ReadError(String),
}

/// Polls the rc and status markers until a complete result or a failure.
pub(super) fn poll(child: &mut Child, drive: &Path, timeout: Duration) -> Outcome {
    let rc_path = drive.join("clorraine-rc.txt");
    let status_path = drive.join("clorraine-status.txt");
    let deadline = Instant::now() + timeout;
    let mut partial_since: Option<Instant> = None;
    loop {
        let rc = read_rc(&rc_path);
        let status = read_status(&status_path);
        if let Some(outcome) = complete(&rc, &status, drive) {
            return outcome;
        }
        if let Marker::Error(message) = &rc {
            return Outcome::ReadError(message.clone());
        }
        if let Marker::Error(message) = &status {
            return Outcome::ReadError(message.clone());
        }
        let is_partial = matches!(&rc, Marker::Partial) || matches!(&status, Marker::Partial);
        if is_partial {
            let started = *partial_since.get_or_insert_with(Instant::now);
            if started.elapsed() >= PARTIAL_GRACE {
                return incomplete(drive);
            }
        } else {
            partial_since = None;
        }
        match child.try_wait() {
            Ok(Some(_)) => {
                // Re-read once: the wrapper may have written its last marker
                // immediately before exiting.
                let rc = read_rc(&rc_path);
                let status = read_status(&status_path);
                return complete(&rc, &status, drive).unwrap_or_else(|| incomplete(drive));
            }
            Ok(None) => {}
            Err(error) => {
                return Outcome::ReadError(format!("cannot wait for FS-UAE: {error}"));
            }
        }
        if Instant::now() >= deadline {
            let any = !matches!(&rc, Marker::Missing) || !matches!(&status, Marker::Missing);
            return if any {
                incomplete(drive)
            } else {
                timed_out(drive)
            };
        }
        std::thread::sleep(POLL_INTERVAL);
    }
}

/// Returns the outcome when the marker pair proves success or a guest failure.
fn complete(rc: &Marker, status: &Marker, drive: &Path) -> Option<Outcome> {
    match (rc, status) {
        (Marker::Number(0), Marker::Ok) => {
            Some(with_result(read_result(drive), Outcome::Success))
        }
        (Marker::Number(code), _) if *code != 0 => Some(with_result(
            read_result(drive),
            |result| Outcome::GuestFailure(*code, result),
        )),
        _ => None,
    }
}

/// Reads the result bytes, treating a missing file as empty output.
fn read_result(drive: &Path) -> Result<Vec<u8>, String> {
    let path = drive.join("clorraine-result.txt");
    match fs::read(&path) {
        Ok(bytes) => Ok(strip_carriage_returns(bytes)),
        // An empty result is valid: a program may print nothing.
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(error) => Err(format!("cannot read '{}': {error}", storage::shown(&path))),
    }
}

/// Wraps result bytes in the requested outcome, or reports the read failure.
fn with_result<F>(result: Result<Vec<u8>, String>, complete: F) -> Outcome
where
    F: FnOnce(Vec<u8>) -> Outcome,
{
    match result {
        Ok(bytes) => complete(bytes),
        Err(error) => Outcome::ReadError(error),
    }
}

/// Builds an incomplete-marker outcome with the readable result bytes.
fn incomplete(drive: &Path) -> Outcome {
    with_result(read_result(drive), Outcome::Incomplete)
}

/// Builds a timeout outcome with the readable result bytes.
fn timed_out(drive: &Path) -> Outcome {
    with_result(read_result(drive), Outcome::TimedOut)
}

/// Reads the return-code marker.
fn read_rc(path: &Path) -> Marker {
    match read_text(path) {
        Err(message) => Marker::Error(message),
        Ok(None) => Marker::Missing,
        Ok(Some(text)) => match trim_marker(&text).parse::<i64>() {
            Ok(code) => Marker::Number(code),
            Err(_) => Marker::Partial,
        },
    }
}

/// Reads the status marker.
fn read_status(path: &Path) -> Marker {
    match read_text(path) {
        Err(message) => Marker::Error(message),
        Ok(None) => Marker::Missing,
        Ok(Some(text)) => {
            if trim_marker(&text) == "ok" {
                Marker::Ok
            } else {
                Marker::Partial
            }
        }
    }
}

/// Reads a marker file, distinguishing absence from a real read failure.
fn read_text(path: &Path) -> Result<Option<String>, String> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("cannot read '{}': {error}", storage::shown(&path))),
    }
}

/// Trims one trailing newline and an optional carriage return.
fn trim_marker(text: &str) -> &str {
    let text = text.strip_suffix('\n').unwrap_or(text);
    text.strip_suffix('\r').unwrap_or(text)
}

/// Removes carriage returns so Amiga line endings do not reach the terminal.
fn strip_carriage_returns(bytes: Vec<u8>) -> Vec<u8> {
    if bytes.contains(&b'\r') {
        bytes.into_iter().filter(|byte| *byte != b'\r').collect()
    } else {
        bytes
    }
}

/// Ends the child with a bounded wait for reaping.
///
/// The standard library exposes only a hard kill, and this crate forbids unsafe
/// code, so the runner kills the child directly and bounds how long it waits
/// for the process to disappear. The guest markers are already complete before
/// this point, so no result depends on a graceful emulator shutdown.
pub(super) fn terminate(child: &mut Child) -> bool {
    if matches!(child.try_wait(), Ok(Some(_))) {
        return true;
    }
    let _ = child.kill();
    reap(child, KILL_GRACE)
}

/// Waits at most `grace` for the child to be reaped.
fn reap(child: &mut Child, grace: Duration) -> bool {
    let deadline = Instant::now() + grace;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return true,
            Ok(None) => {}
            Err(_) => return false,
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// The result bytes carried by an outcome, when it has any.
pub(super) fn outcome_result(outcome: &Outcome) -> Option<&[u8]> {
    match outcome {
        Outcome::Success(result)
        | Outcome::GuestFailure(_, result)
        | Outcome::Incomplete(result)
        | Outcome::TimedOut(result) => Some(result),
        Outcome::ReadError(_) => None,
    }
}
