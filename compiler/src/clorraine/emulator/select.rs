//! FS-UAE, Kickstart, and timeout selection for the verified runner.
//!
//! Every input is checked before a build starts. A selected invalid value is a
//! usage-class error (exit 2) and never falls through to a lower-priority
//! source.

use std::ffi::{OsStr, OsString};
use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use super::super::display;
use super::super::environment::{self, EnvironmentError};
use super::absolute;
use super::Rom;

/// Default guest timeout when the environment does not override it.
const DEFAULT_TIMEOUT_SECONDS: u64 = 90;

/// Largest accepted guest timeout.
const MAX_TIMEOUT_SECONDS: u64 = 3600;

/// The oldest supported FS-UAE release.
const MINIMUM_FS_UAE: (u64, u64, u64) = (3, 2, 0);

/// The standard raw 512 KB Kickstart size.
const ROM_512K: u64 = 524_288;

/// The tested Kickstart 3.1 A1200 image size.
const ROM_KS31_A1200: u64 = 524_299;

/// A 256 KB Kickstart 1.3 image.
const ROM_256K: u64 = 262_144;

/// Resolves the FS-UAE executable from the flag, environment, or `PATH`.
pub(super) fn resolve_executable(explicit: Option<&str>, path: &OsStr) -> Result<PathBuf, EnvironmentError> {
    let selected = explicit
        .map(OsString::from)
        .or_else(|| std::env::var_os("CLORRAINE_FS_UAE"));
    let executable = match selected {
        Some(selected) => absolute(Path::new(&selected))
            .map_err(|error| input_error(2, "--fs-uae", &error.to_string()))?,
        None => {
            let found = std::env::split_paths(path)
                .map(|dir| dir.join("fs-uae"))
                .find(|file| environment::is_executable_file(file))
                .ok_or_else(|| {
                    input_error(
                        2,
                        "--fs-uae",
                        "fs-uae was not found on PATH; install FS-UAE 3.2.0 or newer or set CLORRAINE_FS_UAE",
                    )
                })?;
            absolute(&found).map_err(|error| input_error(2, "--fs-uae", &error.to_string()))?
        }
    };
    safe_path(&executable, "--fs-uae")?;
    if !environment::is_executable_file(&executable) {
        return Err(input_error(
            2,
            "--fs-uae",
            &format!("'{}' is missing or not executable", executable.display()),
        ));
    }
    Ok(executable)
}

/// Runs `fs-uae --version` once and requires 3.2.0 or newer.
pub(super) fn check_version(executable: &Path) -> Result<(), EnvironmentError> {
    let output = Command::new(executable)
        .arg("--version")
        .output()
        .map_err(|error| {
            input_error(
                2,
                "--fs-uae",
                &format!("cannot run '{} --version': {error}", executable.display()),
            )
        })?;
    if !output.status.success() {
        return Err(input_error(
            2,
            "--fs-uae",
            &format!("'{} --version' failed", executable.display()),
        ));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let first = stdout.lines().next().unwrap_or("").trim();
    let token = first.split_whitespace().next().unwrap_or("");
    let version = parse_version(token).ok_or_else(|| {
        input_error(
            2,
            "--fs-uae",
            &format!(
                "cannot read the FS-UAE version from '{}'",
                display::sanitize(first)
            ),
        )
    })?;
    if version < MINIMUM_FS_UAE {
        return Err(input_error(
            2,
            "--fs-uae",
            &format!("FS-UAE '{token}' is older than the required 3.2.0"),
        ));
    }
    Ok(())
}

/// Parses the leading `major.minor[.patch]` numbers of a version token.
///
/// A packaged suffix such as `3.2.35-2` keeps its numeric prefix; the suffix is
/// not part of the ordering decision.
fn parse_version(token: &str) -> Option<(u64, u64, u64)> {
    let mut parts = token.split('.');
    let major = leading_number(parts.next()?)?;
    let minor = leading_number(parts.next()?)?;
    let patch = parts.next().and_then(leading_number).unwrap_or(0);
    Some((major, minor, patch))
}

/// Reads the leading decimal digits of a version component.
fn leading_number(text: &str) -> Option<u64> {
    let digits: String = text.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() {
        None
    } else {
        digits.parse().ok()
    }
}

/// Resolves and size-checks the Kickstart image.
pub(super) fn resolve_rom(explicit: Option<&str>) -> Result<Rom, EnvironmentError> {
    let selected = explicit
        .map(OsString::from)
        .or_else(|| std::env::var_os("CLORRAINE_KICKSTART"))
        .ok_or_else(|| {
            input_error(
                2,
                "--kickstart",
                "a Kickstart 3.1 A1200 ROM is required; pass --kickstart PATH or set CLORRAINE_KICKSTART",
            )
        })?;
    let path = absolute(Path::new(&selected))
        .map_err(|error| input_error(2, "--kickstart", &error.to_string()))?;
    safe_path(&path, "--kickstart")?;
    let media = open_media(&path).map_err(|error| {
        input_error(
            2,
            "--kickstart",
            &format!(
                "'{}' is not a readable ordinary file: {error}",
                path.display()
            ),
        )
    })?;
    let size = media
        .metadata()
        .map_err(|error| input_error(2, "--kickstart", &error.to_string()))?
        .len();
    match size {
        ROM_512K | ROM_KS31_A1200 => {}
        ROM_256K => {
            return Err(input_error(
                2,
                "--kickstart",
                "a 256 KB Kickstart 1.3 ROM cannot boot the A1200 recipe;\n\
                 supply a 512 KB Kickstart 3.1 A1200 ROM",
            ));
        }
        other => {
            return Err(input_error(
                2,
                "--kickstart",
                &format!(
                    "'{}' is {other} bytes; expected a 512 KB Kickstart 3.1 A1200 ROM (524288 bytes)",
                    path.display()
                ),
            ));
        }
    }
    let copy_into_run = path.as_os_str().as_encoded_bytes().ends_with(b" ");
    Ok(Rom { path, media, copy_into_run })
}

/// Resolves the optional guest timeout from the environment.
pub(super) fn resolve_timeout() -> Result<Duration, EnvironmentError> {
    let Some(value) = std::env::var_os("CLORRAINE_EMULATOR_TIMEOUT_SECONDS") else {
        return Ok(Duration::from_secs(DEFAULT_TIMEOUT_SECONDS));
    };
    let Some(text) = value.to_str() else {
        return Err(input_error(
            2,
            "CLORRAINE_EMULATOR_TIMEOUT_SECONDS",
            "must be an integer between 1 and 3600",
        ));
    };
    match text.parse::<u64>() {
        Ok(seconds) if (1..=MAX_TIMEOUT_SECONDS).contains(&seconds) => {
            Ok(Duration::from_secs(seconds))
        }
        _ => Err(input_error(
            2,
            "CLORRAINE_EMULATOR_TIMEOUT_SECONDS",
            &format!(
                "'{}' is not an integer between 1 and 3600",
                display::sanitize(text)
            ),
        )),
    }
}

/// Rejects configuration-control bytes without trimming valid spaces.
fn safe_path(path: &Path, flag: &str) -> Result<(), EnvironmentError> {
    match path.to_str() {
        Some(text) if !text.chars().any(char::is_control) => Ok(()),
        _ => Err(input_error(2, flag, "path must be UTF-8 without control characters")),
    }
}

/// Requires ordinary ROM bytes, checking the descriptor as well as the path.
///
/// A nonblocking open prevents a raced replacement FIFO from hanging preflight.
fn open_media(path: &Path) -> io::Result<File> {
    if !fs::metadata(path)?.is_file() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "not an ordinary file"));
    }
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::OpenOptionsExt;
        const NONBLOCK: i32 = 0x800;
        options.custom_flags(NONBLOCK);
    }
    let file = options.open(path)?;
    if !file.metadata()?.is_file() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "not an ordinary file"));
    }
    Ok(file)
}


/// Keeps selection errors in the driver's established text/JSON status model.
fn input_error(code: i32, flag: &str, detail: &str) -> EnvironmentError {
    EnvironmentError {
        code,
        message: display::sanitize(&format!("clorraine: {flag}: {detail}\n")).into_owned(),
    }
}
