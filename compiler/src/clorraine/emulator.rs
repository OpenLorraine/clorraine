//! Private native FS-UAE runs, independent of resettable build artifacts.
//!
//! Only the copied program and fixed startup script are guest-mounted. Paths
//! stay in literal process arguments because FS-UAE's INI reader treats some
//! punctuation as comments. A stopped child permits cleanup; an unconfirmed
//! live child deliberately leaves its private storage intact.

use std::ffi::{OsStr, OsString};
use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::artifacts;
use super::cli::Options;
use super::environment::{self, EnvironmentError};
use super::display;

/// Inputs selected and checked before the compiler is started.
pub(super) struct Emulator {
    /// Absolute executable spelling, never searched again after compilation.
    executable: PathBuf,
    /// Literal absolute ROM spelling for ordinary filenames.
    rom: PathBuf,
    /// Open selected bytes, retained for the exceptional private media copy.
    media: File,
}

impl Emulator {
    /// Resolves CLI/environment/PATH precedence without fallback from a selected
    /// invalid executable. Kickstart has no scan or implicit filesystem default.
    pub(super) fn resolve(options: &Options, path: &OsStr) -> Result<Self, EnvironmentError> {
        let selected = options.fs_uae.as_ref().map(OsString::from)
            .or_else(|| std::env::var_os("CLORRAINE_FS_UAE"));
        let executable = match selected {
            Some(selected) => absolute(Path::new(&selected))
                .map_err(|error| input_error(1, "--fs-uae", &error.to_string()))?,
            None => {
                let found = std::env::split_paths(path)
                    .map(|dir| dir.join("fs-uae"))
                    .find(|file| environment::is_executable_file(file))
                    .ok_or_else(|| input_error(1, "--fs-uae",
                        "fs-uae was not found on PATH; install FS-UAE or set CLORRAINE_FS_UAE"))?;
                absolute(&found).map_err(|error| input_error(1, "--fs-uae", &error.to_string()))?
            }
        };
        safe_path(&executable, "--fs-uae")?;
        if !environment::is_executable_file(&executable) {
            return Err(input_error(1, "--fs-uae", &format!(
                "FS-UAE '{}' is missing or not executable; set CLORRAINE_FS_UAE",
                executable.display(),
            )));
        }
        let selected = options.kickstart.as_ref().map(OsString::from)
            .or_else(|| std::env::var_os("CLORRAINE_KICKSTART"))
            .ok_or_else(|| input_error(1, "--kickstart",
                "Kickstart is required; set CLORRAINE_KICKSTART to your local ROM"))?;
        let rom = absolute(Path::new(&selected))
            .map_err(|error| input_error(1, "--kickstart", &error.to_string()))?;
        safe_path(&rom, "--kickstart")?;
        let media = open_media(&rom).map_err(|error| input_error(1, "--kickstart",
            &format!("Kickstart '{}' is not a readable ordinary file: {error}; set CLORRAINE_KICKSTART",
                rom.display())))?;
        Ok(Self { executable, rom, media })
    }

    /// Copies a completed build, waits for the emulator and returns its process
    /// code. A code-less termination is an emulator error, not a guest result.
    pub(super) fn run(self, program: &Path) -> Result<i32, String> {
        let mut storage = Storage::create(program)?;
        let setup = self.stage(&storage.root, program)?;
        let mut command = Command::new(&self.executable);
        command.arg(&setup.config).current_dir(&storage.root);
        for (name, path) in &setup.paths {
            command.arg(path_option(name, path));
        }
        // Initial logging happens before FS-UAE reads its configuration.
        // Isolating XDG_DATA_HOME protects even that first log file.
        for (name, relative) in [
            ("HOME", "home"), ("XDG_CONFIG_HOME", "home/.config"),
            ("XDG_DATA_HOME", "home/.local/share"), ("XDG_CACHE_HOME", "cache"),
            ("XDG_STATE_HOME", "state"), ("FS_UAE_BASE_DIR", "base"),
        ] {
            command.env(name, storage.root.join(relative));
        }
        // Changing HOME must not change which X server credentials are used.
        // Resolve the caller's default/relative authority before the child CWD.
        if let Some(authority) = caller_authority() {
            command.env("XAUTHORITY", authority);
        }
        // Do not run a nested child under a temporary parent it might mutate.
        command.env("TMPDIR", storage.root.join("tmp"));
        storage.cleanup_safe = false;
        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(error) => {
                storage.cleanup_safe = true;
                return Err(format!("cannot start FS-UAE emulator '{}': {error}",
                    self.executable.display()));
            }
        };
        let result = match child.wait() {
            Ok(status) => {
                storage.cleanup_safe = true;
                status.code().ok_or_else(||
                    "FS-UAE emulator terminated without a process exit code".to_owned())
            }
            Err(error) => {
                // A failed wait is not evidence of termination. Confirm it
                // after killing our child before removing anything it mounted.
                let _ = child.kill();
                if child.wait().is_ok() {
                    storage.cleanup_safe = true;
                }
                Err(format!("cannot wait for FS-UAE emulator: {error}"))
            }
        };
        if !storage.cleanup_safe {
            return Err(format!("{}; emulator termination is unconfirmed; private run retained at '{}'",
                result.unwrap_err(), storage.root.display()));
        }
        storage.cleanup()?;
        result
    }

    /// Builds only static guest statements and keeps all host paths outside
    /// the guest script. The media copy is limited to a terminal ASCII space.
    fn stage(&self, root: &Path, program: &Path) -> Result<Setup, String> {
        let prepare = || -> io::Result<Setup> {
            for directory in [
                "drive/s", "base", "cache", "state", "logs", "tmp",
                "home/.config", "home/.local/share", "kickstarts",
            ] {
                fs::create_dir_all(root.join(directory))?;
            }
            fs::copy(program, root.join("drive/program"))?;
            fs::write(root.join("drive/s/startup-sequence"), b"program\n")?;
            let rom = if self.rom.as_os_str().as_encoded_bytes().ends_with(b" ") {
                fs::create_dir(root.join("media"))?;
                let copy = root.join("media/kickstart.rom");
                let mut source = self.media.try_clone()?;
                let mut destination = OpenOptions::new().write(true).create_new(true).open(&copy)?;
                io::copy(&mut source, &mut destination)?;
                copy
            } else {
                self.rom.clone()
            };
            let config = root.join("run.fs-uae");
            // end_config belongs in the file, not early CLI parsing: setting
            // it early would suppress this very file along with inherited ones.
            fs::write(&config, b"[fs-uae]\nend_config=1\namiga_model=A1200\n\
                hard_drive_0_label=CNEXT\nhard_drive_0_priority=0\n\
                floppy_drive_0=\nfloppy_drive_1=\ncdrom_drive_0=\n")?;
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
        };
        prepare().map_err(|error| format!("cannot stage FS-UAE emulator run: {error}"))
    }
}

/// Explicit config plus literal path controls parsed before normal config files.
struct Setup {
    config: PathBuf,
    paths: Vec<(&'static str, PathBuf)>,
}

/// Owns one exclusively created directory and nothing else.
///
/// There is no stale-state scan. On abrupt parent termination Rust destructors
/// do not run, so a surviving emulator keeps its files rather than losing them.
struct Storage {
    root: PathBuf,
    cleanup_safe: bool,
}

impl Storage {
    /// Creates private storage outside every program/profile artifact directory.
    fn create(program: &Path) -> Result<Self, String> {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let parent = std::env::temp_dir().canonicalize()
            .map_err(|error| format!("cannot resolve FS-UAE temporary directory: {error}"))?;
        safe_path(&parent, "TMPDIR").map_err(|error| error.message)?;
        // FS-UAE passes the drive path into comma-delimited UAE mount syntax.
        // A literal argv element cannot stop that later parser truncating it.
        if parent.as_os_str().as_encoded_bytes().contains(&b',') {
            return Err("FS-UAE TMPDIR must use a safe directory without commas".to_owned());
        }
        let build = artifacts::root().canonicalize()
            .map_err(|error| format!("cannot resolve FS-UAE build artifact tree: {error}"))?;
        let program_dir = program.parent().unwrap_or_else(|| Path::new(".")).canonicalize()
            .map_err(|error| format!("cannot resolve FS-UAE program build directory: {error}"))?;
        // Another source or profile can be rebuilt while this child is alive,
        // so protecting only this invocation's binary directory is insufficient.
        // A program-directory alias can also point outside the artifact root,
        // so the canonical current-program check remains necessary alongside it.
        if parent.starts_with(&build) || parent.starts_with(&program_dir) {
            return Err("FS-UAE TMPDIR must be outside the resettable build/clorraine artifact tree and current program build directory".to_owned());
        }
        for _ in 0..128 {
            let root = parent.join(format!("clorraine-emulator-{}-{}",
                std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
            let mut builder = fs::DirBuilder::new();
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                builder.mode(0o700);
            }
            match builder.create(&root) {
                Ok(()) => return Ok(Self { root, cleanup_safe: true }),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(format!("cannot create private FS-UAE run: {error}")),
            }
        }
        Err("cannot create a unique private FS-UAE run directory".to_owned())
    }

    /// Removes only this run after confirmed child termination.
    fn cleanup(&mut self) -> Result<(), String> {
        fs::remove_dir_all(&self.root)
            .map_err(|error| format!("cannot clean stopped FS-UAE run '{}': {error}", self.root.display()))?;
        self.cleanup_safe = false;
        Ok(())
    }
}

impl Drop for Storage {
    fn drop(&mut self) {
        if self.cleanup_safe {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}

/// Makes relative caller paths independent of the child's private working dir.
fn absolute(path: &Path) -> io::Result<PathBuf> {
    if path.is_absolute() { Ok(path.to_owned()) } else { Ok(std::env::current_dir()?.join(path)) }
}

/// Rejects configuration-control bytes without trimming valid spaces or
/// substituting a display-escaped string for the real filesystem path.
fn safe_path(path: &Path, flag: &str) -> Result<(), EnvironmentError> {
    match path.to_str() {
        Some(text) if !text.chars().any(char::is_control) => Ok(()),
        _ => Err(input_error(2, flag, "path must be UTF-8 without control characters")),
    }
}

/// Requires ordinary ROM bytes, checking the descriptor as well as the path.
/// Nonblocking open prevents a raced replacement FIFO from hanging preflight.
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

/// Builds a single argv element, not shell or INI syntax.
fn path_option(name: &str, path: &Path) -> OsString {
    let mut argument = OsString::from(format!("--{name}="));
    argument.push(path);
    argument
}

/// Preserves X authentication while isolating the emulator's HOME.
fn caller_authority() -> Option<PathBuf> {
    std::env::var_os("XAUTHORITY").map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".Xauthority")))
        .and_then(|path| absolute(&path).ok())
}

/// Keeps all preflight errors in the driver's existing text/JSON status model.
fn input_error(code: i32, flag: &str, detail: &str) -> EnvironmentError {
    EnvironmentError {
        code,
        message: display::sanitize(&format!("clorraine: {flag}: {detail}")).into_owned(),
    }
}
