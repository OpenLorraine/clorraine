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
use super::super::platform::{self, HostOs};
use super::super::setup;
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

/// The largest FS-UAE executable whose version resource is read on Windows.
///
/// The FS-UAE 3.2.35 executable is about 66 MiB, so the bound leaves room for
/// later releases while still refusing to load an arbitrarily large file.
const MAX_EXECUTABLE_BYTES: u64 = 256 * 1024 * 1024;

/// Resolves the FS-UAE executable from the flag, environment, the path
/// remembered by `setup`, or `PATH`.
pub(super) fn resolve_executable(explicit: Option<&str>, path: &OsStr) -> Result<PathBuf, EnvironmentError> {
    let selected = match explicit
        .map(OsString::from)
        .or_else(|| std::env::var_os("CLORRAINE_FS_UAE"))
    {
        Some(selected) => Some(selected),
        None => remembered(|config| config.fs_uae)?,
    };
    let executable = match selected {
        Some(selected) => absolute(Path::new(&selected))
            .map_err(|error| input_error(2, "--fs-uae", &error.to_string()))?,
        None => {
            let found = std::env::split_paths(path)
                .map(|dir| dir.join(platform::executable_name(HostOs::current(), "fs-uae")))
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

/// Reads one emulator path remembered by `setup`.
///
/// A config that cannot be read stops the run with its own message rather
/// than silently falling through to a lower-priority source.
fn remembered(
    pick: impl FnOnce(setup::Config) -> Option<PathBuf>,
) -> Result<Option<OsString>, EnvironmentError> {
    let config = setup::stored_config()
        .map_err(|failure| EnvironmentError { code: failure.code, message: failure.message })?;
    Ok(pick(config).map(PathBuf::into_os_string))
}

/// Reads the FS-UAE version once and requires 3.2.0 or newer.
pub(super) fn check_version(executable: &Path) -> Result<(), EnvironmentError> {
    fs_uae_version_for(HostOs::current(), executable)
        .map(|_| ())
        .map_err(|detail| input_error(2, "--fs-uae", &detail))
}

/// The version of the FS-UAE at `executable`, when it is 3.2.0 or newer.
///
/// Unix runs `fs-uae --version` once and reads its first line. The Windows
/// build is a GUI program that prints nothing to a pipe, so there the version
/// comes from the executable's version resource and the file is never run;
/// a file over 256 MiB is refused rather than read.
///
/// # Errors
///
/// The detail text for a `--fs-uae` usage error: the version cannot be read,
/// or it is older than 3.2.0.
pub fn fs_uae_version_for(os: HostOs, executable: &Path) -> Result<(u64, u64, u64), String> {
    let (version, token) = match os {
        HostOs::Windows => {
            let shown = platform::display_path(os, &executable.display().to_string());
            let unreadable = || format!("cannot read the FS-UAE version from '{}'", display::sanitize(&shown));
            let bytes = read_limited(executable).ok_or_else(unreadable)?;
            let version = platform::pe_file_version(&bytes).ok_or_else(unreadable)?;
            (version, format!("{}.{}.{}", version.0, version.1, version.2))
        }
        HostOs::Unix => {
            let output = Command::new(executable)
                .arg("--version")
                .output()
                .map_err(|error| format!("cannot run '{} --version': {error}", executable.display()))?;
            if !output.status.success() {
                return Err(format!("'{} --version' failed", executable.display()));
            }
            let stdout = String::from_utf8_lossy(&output.stdout);
            let first = stdout.lines().next().unwrap_or("").trim();
            let token = first.split_whitespace().next().unwrap_or("");
            let version = parse_version(token).ok_or_else(|| {
                format!("cannot read the FS-UAE version from '{}'", display::sanitize(first))
            })?;
            (version, token.to_owned())
        }
    };
    if version < MINIMUM_FS_UAE {
        return Err(format!("FS-UAE '{}' is older than the required 3.2.0", display::sanitize(&token)));
    }
    Ok(version)
}

/// The bytes of `path` when it is a file of at most 256 MiB.
fn read_limited(path: &Path) -> Option<Vec<u8>> {
    let file = File::open(path).ok()?;
    if !file.metadata().ok()?.is_file() || file.metadata().ok()?.len() > MAX_EXECUTABLE_BYTES {
        return None;
    }
    let mut bytes = Vec::new();
    io::Read::read_to_end(&mut io::Read::take(file, MAX_EXECUTABLE_BYTES + 1), &mut bytes).ok()?;
    (bytes.len() as u64 <= MAX_EXECUTABLE_BYTES).then_some(bytes)
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

/// True when a Windows run would get an encrypted Cloanto ROM it cannot use.
///
/// Such a ROM starts with `AMIROMTYPE1`; FS-UAE decrypts it with the
/// `rom.key` in the same folder, and without the key the Amiga never boots, so
/// the run could only time out. Linux keeps accepting the ROM as before.
pub fn kickstart_key_missing_for(os: HostOs, rom: &Path, header: &[u8]) -> bool {
    matches!(os, HostOs::Windows)
        && header.starts_with(b"AMIROMTYPE1")
        && !rom.parent().is_some_and(|folder| folder.join("rom.key").is_file())
}

/// Resolves and size-checks the Kickstart image.
pub(super) fn resolve_rom(explicit: Option<&str>) -> Result<Rom, EnvironmentError> {
    resolve_rom_for(HostOs::current(), explicit)
}

/// [`resolve_rom`] for an explicit host family.
fn resolve_rom_for(os: HostOs, explicit: Option<&str>) -> Result<Rom, EnvironmentError> {
    let selected = match explicit
        .map(OsString::from)
        .or_else(|| std::env::var_os("CLORRAINE_KICKSTART"))
    {
        Some(selected) => Some(selected),
        None => remembered(|config| config.kickstart)?,
    };
    let selected = selected
        .ok_or_else(|| {
            input_error(
                2,
                "--kickstart",
                "a Kickstart 3.1 A1200 ROM is required; pass --kickstart PATH, set CLORRAINE_KICKSTART, or remember it with clorraine setup --kickstart PATH",
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
    // Only Windows reads the header, so Linux opens the image once, as before.
    let mut header = [0u8; 11];
    let read = if matches!(os, HostOs::Windows) {
        File::open(&path)
            .and_then(|mut file| io::Read::read(&mut file, &mut header))
            .unwrap_or(0)
    } else {
        0
    };
    if kickstart_key_missing_for(os, &path, &header[..read]) {
        return Err(input_error(
            2,
            "--kickstart",
            &format!(
                "'{}' is an encrypted Cloanto ROM; put its rom.key in the same folder",
                path.display()
            ),
        ));
    }
    let copy_into_run = path.as_os_str().as_encoded_bytes().ends_with(b" ");
    // FS-UAE looks for rom.key beside the ROM it is given, so a copy takes the key along.
    let key = path.parent().map(|folder| folder.join("rom.key")).filter(|key| key.is_file());
    Ok(Rom { path, media, copy_into_run, key })
}

/// Resolves the optional guest timeout from the environment.
pub(super) fn resolve_timeout() -> Result<Duration, EnvironmentError> {
    resolve_timeout_value(std::env::var_os("CLORRAINE_EMULATOR_TIMEOUT_SECONDS"))
}

/// Validates one optional timeout value.
///
/// The value is a parameter so the bounds are unit-testable without mutating
/// the process environment shared by parallel tests.
fn resolve_timeout_value(value: Option<OsString>) -> Result<Duration, EnvironmentError> {
    let Some(value) = value else {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A 524,299-byte encrypted Cloanto image in a fresh folder.
    fn encrypted_rom(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("clc-rom-key-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create the ROM folder");
        let mut bytes = vec![0u8; ROM_KS31_A1200 as usize];
        bytes[..11].copy_from_slice(b"AMIROMTYPE1");
        let rom = dir.join("amiga-os-310-a1200.rom");
        fs::write(&rom, bytes).expect("write the ROM");
        rom
    }

    #[test]
    fn windows_refuses_an_encrypted_rom_without_its_key() {
        let rom = encrypted_rom("missing");
        let result = resolve_rom_for(HostOs::Windows, rom.to_str());
        let _ = fs::remove_dir_all(rom.parent().expect("folder"));
        let error = result.err().expect("an encrypted ROM without rom.key is refused");
        assert_eq!(error.code, 2);
        assert!(error.message.contains("rom.key"), "{}", error.message);
    }

    #[test]
    fn an_encrypted_rom_with_its_key_is_accepted_and_carries_the_key() {
        let rom = encrypted_rom("present");
        let key = rom.with_file_name("rom.key");
        fs::write(&key, b"key").expect("write the key");
        let result = resolve_rom_for(HostOs::Windows, rom.to_str());
        let linux = resolve_rom_for(HostOs::Unix, rom.to_str()).is_ok();
        let _ = fs::remove_dir_all(rom.parent().expect("folder"));
        let accepted = result.unwrap_or_else(|error| panic!("{}", error.message));
        assert_eq!(accepted.key.as_deref(), Some(key.as_path()));
        assert!(linux, "Linux accepts the ROM as before");
    }

    #[test]
    fn the_default_timeout_is_ninety_seconds() {
        assert_eq!(
            resolve_timeout_value(None).expect("default timeout"),
            Duration::from_secs(90)
        );
    }

    #[test]
    fn timeout_values_are_bounded() {
        for value in ["1", "3600"] {
            assert!(
                resolve_timeout_value(Some(value.into())).is_ok(),
                "{value} is accepted"
            );
        }
        for value in ["0", "3601", "abc", ""] {
            assert!(
                resolve_timeout_value(Some(value.into())).is_err(),
                "{value} is rejected"
            );
        }
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
        message: format!("{}\n", display::sanitize(&format!("clorraine: {flag}: {detail}"))),
    }
}

/// True when the run will use the path `setup` remembered for `key`: no flag
/// and no environment variable names one.
pub(super) fn uses_remembered(explicit: Option<&str>, variable: &str, key: &str) -> bool {
    if explicit.is_some() || std::env::var_os(variable).is_some() {
        return false;
    }
    match setup::stored_config() {
        Ok(config) => match key {
            "fs-uae" => config.fs_uae.is_some(),
            _ => config.kickstart.is_some(),
        },
        Err(_) => false,
    }
}

/// Adds where a remembered path came from and how to change it.
pub(super) fn remembered_hint(mut error: EnvironmentError, key: &str) -> EnvironmentError {
    let config = setup::config_file()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|_| "the clorraine config".to_owned());
    error.message.push_str(&format!(
        "{}\n",
        display::sanitize(&format!(
            "  the {key} path was remembered in '{config}'; change it with: \
             clorraine setup --toolchain PATH --{key} PATH"
        ))
    ));
    error
}
