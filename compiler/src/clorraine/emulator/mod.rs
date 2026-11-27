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

mod diagnostics;
mod observe;
mod select;
mod storage;

use std::ffi::OsStr;
use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use super::cli::Options;
use super::display;
use super::environment::EnvironmentError;
use super::platform::{self, HostOs};
use super::run::RunResult;
use crate::diag::DiagnosticFormat;

use observe::Outcome;
pub use select::{fs_uae_version_for, kickstart_key_missing_for};
use storage::Storage;

/// Guest wrapper source compiled into every private run.
///
/// The release archive contains only the CLI binaries, so the source cannot be
/// read from a checkout at run time. Embedding it keeps the runner independent
/// of the install layout.
const WRAPPER_SOURCE: &str = include_str!("../../../runtime/emulator_runner.c");

/// Validates the emulator paths `setup` is asked to remember.
///
/// Each named path gets the same checks as an emulator run: FS-UAE must be an
/// executable 3.2.0 or newer, and the Kickstart must be a readable 512 KB
/// image. Returns the absolute paths to store.
pub(super) fn validate_media(
    fs_uae: Option<&str>,
    kickstart: Option<&str>,
) -> Result<(Option<PathBuf>, Option<PathBuf>), EnvironmentError> {
    let executable = match fs_uae {
        Some(path) => {
            let executable = select::resolve_executable(Some(path), OsStr::new(""))?;
            select::check_version(&executable)?;
            Some(executable)
        }
        None => None,
    };
    let rom = match kickstart {
        Some(path) => Some(select::resolve_rom(Some(path))?.path),
        None => None,
    };
    Ok((executable, rom))
}

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

impl Emulator {
    /// Resolves and validates the executable, ROM, and timeout.
    ///
    /// The `PATH` value is a parameter so the caller's search path can be
    /// exercised without changing the process environment. An explicit
    /// `--fs-uae` value never falls back to the environment or `PATH`.
    pub(super) fn prepare(options: &Options, path: &OsStr) -> Result<Self, EnvironmentError> {
        let fs_uae = options.fs_uae.as_deref();
        let kickstart = options.kickstart.as_deref();
        let hint = |remembered: bool, key: &'static str| {
            move |error: EnvironmentError| {
                if remembered { select::remembered_hint(error, key) } else { error }
            }
        };
        let remembered = select::uses_remembered(fs_uae, "CLORRAINE_FS_UAE", "fs-uae");
        let executable = select::resolve_executable(fs_uae, path)
            .map_err(hint(remembered, "fs-uae"))?;
        select::check_version(&executable).map_err(hint(remembered, "fs-uae"))?;
        let remembered = select::uses_remembered(kickstart, "CLORRAINE_KICKSTART", "kickstart");
        let rom = select::resolve_rom(kickstart).map_err(hint(remembered, "kickstart"))?;
        let timeout = select::resolve_timeout()?;
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
                diagnostics::report(format, &format!("clorraine: {error}"));
                return diagnostics::failed();
            }
        };
        let setup = match self.stage(&storage.root, program, compiler) {
            Ok(setup) => setup,
            Err(error) => {
                // Nothing has run yet, so the private directory is disposable.
                diagnostics::report(format, &format!("clorraine: {error}"));
                return diagnostics::failed();
            }
        };
        storage.cleanup_safe = false;
        let mut child = match self.spawn(&storage.root, &setup) {
            Ok(child) => child,
            Err(error) => {
                // A failed spawn means no emulator owns the directory.
                storage.cleanup_safe = true;
                diagnostics::report(format, &format!("clorraine: {error}"));
                return diagnostics::failed();
            }
        };
        let outcome = observe::poll(&mut child, &storage.root.join("drive"), self.timeout);
        let terminated = observe::terminate(&mut child);
        if !terminated {
            let result = observe::outcome_result(&outcome);
            return diagnostics::fail_retained(
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
                        return diagnostics::fail_retained(&storage, format, &reason, Some(&result));
                    }
                } else {
                    diagnostics::print_result(&result);
                }
                if let Err(error) = storage.cleanup() {
                    diagnostics::report(format, &diagnostics::retained_message(&storage.root, &error));
                    return diagnostics::failed();
                }
                RunResult { code: 0, captured: capture.is_some() }
            }
            Outcome::GuestFailure(code, result) => diagnostics::fail_retained(
                &storage,
                format,
                &format!("guest program exited with code {code}"),
                Some(&result),
            ),
            Outcome::Incomplete(result) => diagnostics::fail_retained(
                &storage,
                format,
                "guest program did not report success",
                Some(&result),
            ),
            Outcome::TimedOut(result) => diagnostics::fail_retained(
                &storage,
                format,
                &format!(
                    "guest run timed out after {} seconds",
                    self.timeout.as_secs()
                ),
                Some(&result),
            ),
            Outcome::ReadError(error) => diagnostics::fail_retained(&storage, format, &error, None),
        }
    }

    /// Writes the guest drive, wrapper source, and emulator configuration.
    ///
    /// The wrapper is compiled before the configuration is used, so a compile
    /// failure never starts FS-UAE.
    fn stage(&self, root: &Path, program: &Path, compiler: &Path) -> Result<Setup, String> {
        self.stage_files(root, program)?;
        storage::compile_wrapper(
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
            command.arg(storage::path_option(name, path));
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
        // Windows has no X server, so only Unix passes the credentials on.
        if HostOs::current() == HostOs::Unix {
            if let Some(authority) = storage::caller_authority() {
                command.env("XAUTHORITY", authority);
            }
        }
        command.env("TMPDIR", root.join("tmp"));
        // Windows programs find their temporary folder through TEMP and TMP.
        // They get the plain `C:\...` spelling: a verbatim `\\?\` path stops
        // working as soon as a library appends a forward-slash component.
        if HostOs::current() == HostOs::Windows {
            let tmp = PathBuf::from(platform::display_path(HostOs::Windows, &root.join("tmp").display().to_string()));
            command.env("TEMP", &tmp);
            command.env("TMP", &tmp);
        }
        command
            .stdin(Stdio::null())
            .stdout(Stdio::from(out))
            .stderr(Stdio::from(err));
        command
            .spawn()
            .map_err(|error| format!("cannot start FS-UAE '{}': {error}", self.executable.display()))
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
