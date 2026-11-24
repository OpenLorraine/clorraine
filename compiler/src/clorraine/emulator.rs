//! Verified native FS-UAE runs for the `clorraine` developer driver.
//!
//! The runner stages a private 0700 directory that contains a compiled guest
//! wrapper, the copied program, and a `CNEXT:` startup script. FS-UAE owns that
//! directory as a hard drive and runs the wrapper, which records the program's
//! return code and writes a status marker only for zero. The host polls both
//! markers, reads the guest output, terminates the emulator with a bounded
//! grace period, and removes the private run only after the result is safe.
//!
//! Selection errors (missing or old FS-UAE, a wrong ROM, an invalid timeout) are
//! reported before anything is compiled and exit 2. Post-spawn failures exit 1
//! and retain the private directory so the guest files remain inspectable.

use std::ffi::{OsStr, OsString};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use super::artifacts;
use super::cli::Options;
use super::display;
use super::environment::{self, EnvironmentError};
use super::run::RunResult;
use crate::diag::{format_error, DiagnosticFormat};

/// Guest wrapper source compiled into every private run.
///
/// The release archive contains only the CLI binaries, so the source cannot be
/// read from a checkout at run time. Embedding it keeps the runner independent
/// of the install layout.
const WRAPPER_SOURCE: &str = include_str!("../../runtime/emulator_runner.c");

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

/// How often the host checks the guest markers.
const POLL_INTERVAL: Duration = Duration::from_millis(250);

/// How long a partial marker may stay incomplete before it is a failure.
const PARTIAL_GRACE: Duration = Duration::from_secs(1);

/// How long a killed emulator may take to be reaped before the run is retained.
const KILL_GRACE: Duration = Duration::from_secs(5);

/// The selected emulator inputs, validated before a build starts.
pub(super) struct Emulator {
    /// Absolute FS-UAE executable.
    executable: PathBuf,
    /// Validated Kickstart image.
    rom: Rom,
    /// Maximum time from spawn to a complete guest result.
    timeout: Duration,
}

/// A validated Kickstart image and its open handle.
struct Rom {
    /// Absolute path used by the emulator configuration.
    path: PathBuf,
    /// Open media used for the exceptional space-suffixed copy.
    media: File,
    /// True when the path ends in a space and needs a private copy.
    copy_into_run: bool,
}

/// The staged files and path options of one private run.
struct Setup {
    /// The FS-UAE configuration file.
    config: PathBuf,
    /// Literal `--name=path` options for the configuration.
    paths: Vec<(&'static str, PathBuf)>,
}

/// One privately created run directory.
///
/// The directory is removed by [`Drop`] only while `cleanup_safe` is true. A
/// spawned emulator flips the flag to false so an early return can never delete
/// files the guest may still be writing.
struct Storage {
    /// Absolute private run directory.
    root: PathBuf,
    /// True while removal is known to be safe.
    cleanup_safe: bool,
}

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
enum Outcome {
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

impl Emulator {
    /// Resolves and validates the executable, ROM, and timeout.
    ///
    /// The `PATH` value is a parameter so the caller's search path can be
    /// exercised without changing the process environment. An explicit
    /// `--fs-uae` value never falls back to the environment or `PATH`.
    pub(super) fn prepare(options: &Options, path: &OsStr) -> Result<Self, EnvironmentError> {
        let executable = resolve_executable(options.fs_uae.as_deref(), path)?;
        check_version(&executable)?;
        let rom = resolve_rom(options.kickstart.as_deref())?;
        let timeout = resolve_timeout()?;
        Ok(Self { executable, rom, timeout })
    }

    /// Stages, runs, and cleans up one verified guest run.
    pub(super) fn run(
        self,
        compiler: &Path,
        program: &Path,
        capture: Option<&Path>,
        format: DiagnosticFormat,
    ) -> RunResult {
        let mut storage = match Storage::create(program) {
            Ok(storage) => storage,
            Err(error) => {
                report(format, &format!("clorraine: {error}"));
                return failed();
            }
        };
        let setup = match self.stage(&storage.root, program, compiler) {
            Ok(setup) => setup,
            Err(error) => {
                // Nothing has run yet, so the private directory is disposable.
                report(format, &format!("clorraine: {error}"));
                return failed();
            }
        };
        storage.cleanup_safe = false;
        let mut child = match self.spawn(&storage.root, &setup) {
            Ok(child) => child,
            Err(error) => {
                // A failed spawn means no emulator owns the directory.
                storage.cleanup_safe = true;
                report(format, &format!("clorraine: {error}"));
                return failed();
            }
        };
        let outcome = poll(&mut child, &storage.root.join("drive"), self.timeout);
        let terminated = terminate(&mut child);
        if !terminated {
            let result = outcome_result(&outcome);
            return fail_retained(
                &storage,
                format,
                "emulator termination is unconfirmed",
                result,
            );
        }
        match outcome {
            Outcome::Success(result) => {
                if let Some(path) = capture {
                    if let Err(error) = fs::write(path, &result) {
                        let reason = format!(
                            "cannot write '{}': {error}",
                            display::sanitize(&path.display().to_string())
                        );
                        return fail_retained(&storage, format, &reason, None);
                    }
                } else {
                    print_result(&result);
                }
                if let Err(error) = storage.cleanup() {
                    report(format, &retained_message(&storage.root, &error));
                    return failed();
                }
                RunResult { code: 0, captured: capture.is_some() }
            }
            Outcome::GuestFailure(code, result) => fail_retained(
                &storage,
                format,
                &format!("guest program exited with code {code}"),
                Some(&result),
            ),
            Outcome::Incomplete(result) => fail_retained(
                &storage,
                format,
                "guest program did not report success",
                Some(&result),
            ),
            Outcome::TimedOut(result) => fail_retained(
                &storage,
                format,
                &format!(
                    "guest run timed out after {} seconds",
                    self.timeout.as_secs()
                ),
                Some(&result),
            ),
            Outcome::ReadError(error) => fail_retained(&storage, format, &error, None),
        }
    }

    /// Writes the guest drive, wrapper source, and emulator configuration.
    ///
    /// The wrapper is compiled before the configuration is used, so a compile
    /// failure never starts FS-UAE.
    fn stage(&self, root: &Path, program: &Path, compiler: &Path) -> Result<Setup, String> {
        self.stage_files(root, program)?;
        compile_wrapper(
            compiler,
            &root.join("emulator_runner.c"),
            &root.join("drive/runner"),
        )?;
        self.setup(root)
    }

    /// Creates the private directory layout and copies the guest program.
    fn stage_files(&self, root: &Path, program: &Path) -> Result<(), String> {
        let prepare = || -> io::Result<()> {
            for directory in [
                "drive/s",
                "base",
                "cache",
                "state",
                "logs",
                "tmp",
                "home/.config",
                "home/.local/share",
                "kickstarts",
            ] {
                fs::create_dir_all(root.join(directory))?;
            }
            fs::copy(program, root.join("drive/program"))?;
            fs::write(root.join("drive/s/startup-sequence"), b"CNEXT:runner\n")?;
            fs::write(root.join("emulator_runner.c"), WRAPPER_SOURCE.as_bytes())?;
            Ok(())
        };
        prepare().map_err(|error| format!("cannot stage the private FS-UAE run: {error}"))
    }

    /// Builds the configuration and its literal path options.
    fn setup(&self, root: &Path) -> Result<Setup, String> {
        let rom = if self.rom.copy_into_run {
            let media = root.join("media");
            fs::create_dir(&media)
                .map_err(|error| format!("cannot create the ROM copy directory: {error}"))?;
            let copy = media.join("kickstart.rom");
            let mut source = self
                .rom
                .media
                .try_clone()
                .map_err(|error| format!("cannot reopen the Kickstart image: {error}"))?;
            let mut destination = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&copy)
                .map_err(|error| format!("cannot copy the Kickstart image: {error}"))?;
            io::copy(&mut source, &mut destination)
                .map_err(|error| format!("cannot copy the Kickstart image: {error}"))?;
            copy
        } else {
            self.rom.path.clone()
        };
        let config = root.join("run.fs-uae");
        fs::write(
            &config,
            b"[fs-uae]\nend_config=1\namiga_model=A1200\n\
              hard_drive_0_label=CNEXT\nhard_drive_0_priority=0\n\
              floppy_drive_0=\nfloppy_drive_1=\ncdrom_drive_0=\n",
        )
        .map_err(|error| format!("cannot write the FS-UAE configuration: {error}"))?;
        let paths = vec![
            ("base_dir", root.join("base")),
            ("cache_dir", root.join("cache")),
            ("save_states_dir", root.join("state")),
            ("logs_dir", root.join("logs")),
            ("kickstarts_dir", root.join("kickstarts")),
            ("hard_drive_0", root.join("drive")),
            ("kickstart_file", rom),
        ];
        Ok(Setup { config, paths })
    }

    /// Spawns FS-UAE as a direct child with isolated directories and streams.
    fn spawn(&self, root: &Path, setup: &Setup) -> Result<Child, String> {
        let out = File::create(root.join("logs/fs-uae.out"))
            .map_err(|error| format!("cannot create the emulator log: {error}"))?;
        let err = File::create(root.join("logs/fs-uae.err"))
            .map_err(|error| format!("cannot create the emulator log: {error}"))?;
        let mut command = Command::new(&self.executable);
        command.arg(&setup.config).current_dir(root);
        for (name, path) in &setup.paths {
            command.arg(path_option(name, path));
        }
        // Isolating the emulator's home keeps its settings and logs out of the
        // user's real configuration.
        for (name, relative) in [
            ("HOME", "home"),
            ("XDG_CONFIG_HOME", "home/.config"),
            ("XDG_DATA_HOME", "home/.local/share"),
            ("XDG_CACHE_HOME", "cache"),
            ("XDG_STATE_HOME", "state"),
            ("FS_UAE_BASE_DIR", "base"),
        ] {
            command.env(name, root.join(relative));
        }
        // Changing HOME must not change which X server credentials are used.
        if let Some(authority) = caller_authority() {
            command.env("XAUTHORITY", authority);
        }
        command.env("TMPDIR", root.join("tmp"));
        command
            .stdin(Stdio::null())
            .stdout(Stdio::from(out))
            .stderr(Stdio::from(err));
        command
            .spawn()
            .map_err(|error| format!("cannot start FS-UAE '{}': {error}", self.executable.display()))
    }
}

/// Compiles the embedded guest wrapper with the selected rc24 compiler.
///
/// The command line is pinned: a different runtime or warning set would make a
/// green guest run mean something different from the accepted recipe.
fn compile_wrapper(compiler: &Path, source: &Path, runner: &Path) -> Result<(), String> {
    let output = Command::new(compiler)
        .args([
            "-std=c99",
            "-mcrt=nix20",
            "-m68000",
            "-fwrapv",
            "-O2",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-o",
        ])
        .arg(runner)
        .arg(source)
        .args(["-lamiga", "-lm"])
        .output()
        .map_err(|error| {
            format!(
                "guest runner build failed: cannot run '{}': {error}",
                compiler.display()
            )
        })?;
    if output.status.success() {
        return Ok(());
    }
    let first_error = String::from_utf8_lossy(&output.stderr)
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("(the compiler produced no error output)")
        .to_owned();
    Err(format!("guest runner build failed: {first_error}"))
}

impl Storage {
    /// Creates a private 0700 directory outside every resettable build path.
    fn create(program: &Path) -> Result<Self, String> {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let parent = std::env::temp_dir()
            .canonicalize()
            .map_err(|error| format!("cannot resolve TMPDIR: {error}"))?;
        match parent.to_str() {
            Some(text) if !text.chars().any(char::is_control) => {}
            _ => return Err("TMPDIR must be UTF-8 without control characters".to_owned()),
        }
        // FS-UAE passes the drive path into comma-delimited UAE mount syntax;
        // a literal argv element cannot stop that later parser truncating it.
        if parent.as_os_str().as_encoded_bytes().contains(&b',') {
            return Err("FS-UAE TMPDIR must use a safe directory without commas".to_owned());
        }
        let build = artifacts::root()
            .canonicalize()
            .map_err(|error| format!("cannot resolve the build artifact tree: {error}"))?;
        let program_dir = program
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .canonicalize()
            .map_err(|error| format!("cannot resolve the program directory: {error}"))?;
        if parent.starts_with(&build) || parent.starts_with(&program_dir) {
            return Err(
                "FS-UAE TMPDIR must be outside the resettable build/clorraine tree and current program directory"
                    .to_owned(),
            );
        }
        for _ in 0..128 {
            let root = parent.join(format!(
                "clorraine-emulator-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            let mut builder = fs::DirBuilder::new();
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                builder.mode(0o700);
            }
            match builder.create(&root) {
                Ok(()) => return Ok(Self { root, cleanup_safe: true }),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(format!("cannot create the private run: {error}")),
            }
        }
        Err("cannot create a unique private run directory".to_owned())
    }

    /// Removes the private run after confirmed termination.
    fn cleanup(&mut self) -> Result<(), String> {
        // Never retry through Drop: a reported retained directory must stay.
        self.cleanup_safe = false;
        fs::remove_dir_all(&self.root).map_err(|error| {
            format!(
                "cannot remove '{}': {error}",
                display::sanitize(&self.root.display().to_string())
            )
        })
    }
}

impl Drop for Storage {
    fn drop(&mut self) {
        if self.cleanup_safe {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}

/// Resolves the FS-UAE executable from the flag, environment, or `PATH`.
fn resolve_executable(explicit: Option<&str>, path: &OsStr) -> Result<PathBuf, EnvironmentError> {
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
fn check_version(executable: &Path) -> Result<(), EnvironmentError> {
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
fn resolve_rom(explicit: Option<&str>) -> Result<Rom, EnvironmentError> {
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
fn resolve_timeout() -> Result<Duration, EnvironmentError> {
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

/// Polls the rc and status markers until a complete result or a failure.
fn poll(child: &mut Child, drive: &Path, timeout: Duration) -> Outcome {
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
        Err(error) => Err(format!(
            "cannot read '{}': {error}",
            display::sanitize(&path.display().to_string())
        )),
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
        Err(error) => Err(format!(
            "cannot read '{}': {error}",
            display::sanitize(&path.display().to_string())
        )),
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
fn terminate(child: &mut Child) -> bool {
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

/// Ends the run as a retained failure and prints the readable guest output.
fn fail_retained(
    storage: &Storage,
    format: DiagnosticFormat,
    reason: &str,
    result: Option<&[u8]>,
) -> RunResult {
    if let Some(result) = result {
        print_result(result);
    }
    report(format, &retained_message(&storage.root, reason));
    failed()
}

/// The one-line retained-run diagnostic.
fn retained_message(root: &Path, reason: &str) -> String {
    format!(
        "clorraine: {reason}; private run retained at '{}'",
        display::sanitize(&root.display().to_string())
    )
}

/// Writes guest result bytes to stdout.
fn print_result(result: &[u8]) {
    let mut stdout = io::stdout();
    let _ = stdout.write_all(result);
    let _ = stdout.flush();
}

/// Writes one driver diagnostic in the selected format.
fn report(format: DiagnosticFormat, message: &str) {
    eprint!("{}", format_error(format, message));
}

/// The failed result shape used by every runtime error.
fn failed() -> RunResult {
    RunResult { code: 1, captured: false }
}

/// The result bytes carried by an outcome, when it has any.
fn outcome_result(outcome: &Outcome) -> Option<&[u8]> {
    match outcome {
        Outcome::Success(result)
        | Outcome::GuestFailure(_, result)
        | Outcome::Incomplete(result)
        | Outcome::TimedOut(result) => Some(result),
        Outcome::ReadError(_) => None,
    }
}

/// Makes a relative caller path independent of the child's working directory.
fn absolute(path: &Path) -> io::Result<PathBuf> {
    if path.is_absolute() {
        Ok(path.to_owned())
    } else {
        Ok(std::env::current_dir()?.join(path))
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

/// Builds one literal `--name=path` argument, not shell or INI syntax.
fn path_option(name: &str, path: &Path) -> OsString {
    let mut argument = OsString::from(format!("--{name}="));
    argument.push(path);
    argument
}

/// Preserves X authentication while isolating the emulator's HOME.
fn caller_authority() -> Option<PathBuf> {
    std::env::var_os("XAUTHORITY")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".Xauthority")))
        .and_then(|path| absolute(&path).ok())
}

/// Keeps selection errors in the driver's established text/JSON status model.
fn input_error(code: i32, flag: &str, detail: &str) -> EnvironmentError {
    EnvironmentError {
        code,
        message: display::sanitize(&format!("clorraine: {flag}: {detail}\n")).into_owned(),
    }
}
