//! Private run storage and the guest wrapper compile for the verified runner.
//!
//! The directory lives under the user's temporary root, is created owner-only,
//! and is removed only after the emulator has been reaped and the result read.

use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::super::artifacts;
use super::super::display;
use super::super::platform::{self, HostOs};
use super::absolute;

/// The variable that names the user's temporary folder on this host, for
/// messages about it.
fn temp_variable() -> &'static str {
    match HostOs::current() {
        HostOs::Unix => "TMPDIR",
        HostOs::Windows => "TEMP",
    }
}

/// A path for a message: terminal-safe and without a Windows verbatim prefix.
pub(super) fn shown(path: &Path) -> String {
    display::sanitize(&platform::display_path(HostOs::current(), &path.display().to_string())).into_owned()
}

/// One privately created run directory.
///
/// The directory is removed by [`Drop`] only while `cleanup_safe` is true. A
/// spawned emulator flips the flag to false so an early return can never delete
/// files the guest may still be writing.
pub(super) struct Storage {
    /// Absolute private run directory.
    pub(super) root: PathBuf,
    /// True while removal is known to be safe.
    pub(super) cleanup_safe: bool,
}

/// Compiles the embedded guest wrapper with the selected rc24 compiler.
///
/// The command line is pinned: a different runtime or warning set would make a
/// green guest run mean something different from the accepted recipe.
pub(super) fn compile_wrapper(compiler: &Path, source: &Path, runner: &Path) -> Result<(), String> {
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
    pub(super) fn create(program: &Path) -> Result<Self, String> {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let parent = std::env::temp_dir()
            .canonicalize()
            .map_err(|error| format!("cannot resolve {}: {error}", temp_variable()))?;
        match parent.to_str() {
            Some(text) if !text.chars().any(char::is_control) => {}
            _ => return Err(format!("{} must be UTF-8 without control characters", temp_variable())),
        }
        // FS-UAE passes the drive path into comma-delimited UAE mount syntax;
        // a literal argv element cannot stop that later parser truncating it.
        if parent.as_os_str().as_encoded_bytes().contains(&b',') {
            return Err(format!("FS-UAE {} must use a safe directory without commas", temp_variable()));
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
            return Err(format!(
                "FS-UAE {} must be outside the resettable build/clorraine tree and current program directory",
                temp_variable()
            ));
        }
        for _ in 0..128 {
            let root = parent.join(format!(
                "clorraine-emulator-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            // Unix creates the run owner-only; on Windows the folder inherits
            // the per-user access list of the user's temporary folder.
            #[cfg(unix)]
            let builder = {
                use std::os::unix::fs::DirBuilderExt;
                let mut builder = fs::DirBuilder::new();
                builder.mode(0o700);
                builder
            };
            #[cfg(not(unix))]
            let builder = fs::DirBuilder::new();
            match builder.create(&root) {
                Ok(()) => return Ok(Self { root, cleanup_safe: true }),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(format!("cannot create the private run: {error}")),
            }
        }
        Err("cannot create a unique private run directory".to_owned())
    }

    /// Removes the private run after confirmed termination.
    pub(super) fn cleanup(&mut self) -> Result<(), String> {
        // Never retry through Drop: a reported retained directory must stay.
        self.cleanup_safe = false;
        fs::remove_dir_all(&self.root)
            .map_err(|error| format!("cannot remove '{}': {error}", shown(&self.root)))
    }
}

impl Drop for Storage {
    fn drop(&mut self) {
        if self.cleanup_safe {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}

/// Builds one literal `--name=path` argument, not shell or INI syntax.
pub(super) fn path_option(name: &str, path: &Path) -> OsString {
    let mut argument = OsString::from(format!("--{name}="));
    argument.push(path);
    argument
}

/// Preserves X authentication while isolating the emulator's HOME.
pub(super) fn caller_authority() -> Option<PathBuf> {
    std::env::var_os("XAUTHORITY")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".Xauthority")))
        .and_then(|path| absolute(&path).ok())
}
