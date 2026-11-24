//! Toolchain setup, validation, and persisted selection.
//!
//! The user downloads an Amiga compiler tree once. `setup` verifies that the
//! tree is complete, that its compiler is the supported `16.2-rc24` build, and
//! that both libnix runtimes and the NDK headers can link a trivial program.
//! Only after those checks pass does it replace the stored toolchain path.
//!
//! Build and run commands use a lighter selection check: the root, compiler,
//! and `m68k-amigaos` tree must exist, but the expensive probes run only during
//! setup. A broken library then fails at the real link with the compiler's own
//! error.

use std::ffi::OsStr;
use std::fs::{self, OpenOptions};
use std::io::{self, Read, Write};
use std::thread;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use super::cli::Options;
use super::display;

/// The only toolchain version this release validates.
const SUPPORTED_VERSION: &str = "16.2-rc24";

/// The smallest glibc version that can run the supported upstream host tools.
const MINIMUM_GLIBC: (u32, u32) = (2, 38);

/// A complete, validated toolchain.
pub struct Toolchain {
    /// Canonical toolchain root.
    pub root: PathBuf,
    /// Absolute compiler inside the root.
    pub compiler: PathBuf,
    /// The first line reported by `--version`.
    pub version: String,
}

/// A setup failure with the driver's observable exit status.
#[derive(Debug)]
pub struct SetupError {
    /// Two for an invalid selection, one for a failed probe or host check.
    pub code: i32,
    /// Terminal-safe message ready to print.
    pub message: String,
}

/// How much validation a caller needs.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Validation {
    /// Validate layout, compiler, version, and both runtime probes.
    Full,
    /// Validate only the layout and compiler used by a build.
    Selection,
}

/// Runs `setup --toolchain PATH`.
///
/// The stored config is replaced only after the full validation succeeds.
pub fn run(options: &Options) -> Result<(), SetupError> {
    let selected = options
        .toolchain
        .as_deref()
        .ok_or_else(|| error(2, "--toolchain is required with 'setup'"))?;
    let toolchain = validate(Path::new(selected), Validation::Full)?;
    write_config(&toolchain.root)?;
    println!(
        "configured toolchain {}",
        display::sanitize(&toolchain.root.display().to_string())
    );
    println!("{}", display::sanitize(&toolchain.version));
    Ok(())
}

/// Validates a toolchain root and returns its selected compiler.
///
/// Relative roots are resolved against the caller's current directory. Every
/// returned path is canonical, so a later directory change or replacement
/// alias cannot select a different compiler.
pub fn validate(selected: &Path, mode: Validation) -> Result<Toolchain, SetupError> {
    let text = selected.to_str().ok_or_else(|| {
        error(
            2,
            format!(
                "--toolchain: '{}' is not valid UTF-8",
                display::sanitize(&selected.display().to_string())
            ),
        )
    })?;
    if text.chars().any(char::is_control) {
        return Err(error(2, "--toolchain: path must not contain control characters"));
    }
    let root = selected.canonicalize().map_err(|cause| {
        error(
            2,
            format!(
                "--toolchain: '{}': {cause}",
                display::sanitize(&selected.display().to_string())
            ),
        )
    })?;
    if !root.is_dir() {
        return Err(error(2, format!("--toolchain: '{}' is not a directory", root.display())));
    }
    let root_text = root
        .to_str()
        .ok_or_else(|| error(2, "--toolchain: canonical path must be UTF-8"))?;
    if root_text.chars().any(char::is_control) {
        return Err(error(2, "--toolchain: canonical path must not contain control characters"));
    }
    let amiga = contained_directory(&root.join("m68k-amigaos"), &root)
        .ok_or_else(|| error(2, format!("--toolchain: '{}' has no m68k-amigaos tree", root.display())))?;
    for required in ["libnix", "ndk-include"] {
        if contained_directory(&amiga.join(required), &root).is_none() {
            return Err(error(
                2,
                format!("--toolchain: '{}' has no m68k-amigaos/{required}", root.display()),
            ));
        }
    }
    let compiler = root.join("bin/m68k-amigaos-gcc");
    let compiler = compiler.canonicalize().map_err(|_| {
        error(
            2,
            format!("--toolchain: '{}' has no bin/m68k-amigaos-gcc", root.display()),
        )
    })?;
    if !compiler.starts_with(&root) {
        return Err(error(2, "--toolchain: the compiler escapes the selected root"));
    }
    let compiler_text = compiler
        .to_str()
        .ok_or_else(|| error(2, "--toolchain: canonical compiler path must be UTF-8"))?;
    if compiler_text.chars().any(char::is_control) {
        return Err(error(2, "--toolchain: compiler path must not contain control characters"));
    }
    if !is_executable(&compiler) {
        return Err(error(
            2,
            format!("--toolchain: '{}' is not executable", compiler.display()),
        ));
    }
    if mode == Validation::Selection {
        return Ok(Toolchain {
            root,
            compiler,
            version: String::new(),
        });
    }
    check_glibc()?;
    let version = compiler_version(&compiler)?;
    if !version.contains(SUPPORTED_VERSION) {
        return Err(error(
            1,
            format!(
                "clorraine: unsupported toolchain; expected AmigaPorts {SUPPORTED_VERSION}, got '{}'\n\
                 install the pinned toolchain and select its root again",
                display::sanitize(&version)
            ),
        ));
    }
    probe(&compiler)?;
    Ok(Toolchain { root, compiler, version })
}

/// Reads the stored toolchain, if any.
pub fn stored_toolchain() -> Result<Option<PathBuf>, SetupError> {
    let path = config_file()?;
    match fs::read_to_string(&path) {
        Ok(text) => parse_config(&text, &path),
        Err(cause) if cause.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(cause) => Err(error(
            2,
            format!(
                "clorraine: cannot read config '{}': {cause}",
                display::sanitize(&path.display().to_string())
            ),
        )),
    }
}

/// The config file owned by the CLI.
pub fn config_file() -> Result<PathBuf, SetupError> {
    Ok(config_dir(
        std::env::var_os("XDG_CONFIG_HOME").as_deref(),
        std::env::var_os("HOME").as_deref(),
    )?.join("config"))
}

/// Resolves the config directory from the two platform inputs.
///
/// A relative `XDG_CONFIG_HOME` is ignored, matching the XDG base-directory
/// rule. Without a usable home the command cannot own a config file.
fn config_dir(xdg: Option<&OsStr>, home: Option<&OsStr>) -> Result<PathBuf, SetupError> {
    if let Some(xdg) = xdg.map(Path::new).filter(|path| path.is_absolute()) {
        return Ok(xdg.join("clorraine"));
    }
    let home = home
        .map(Path::new)
        .filter(|path| path.is_absolute())
        .ok_or_else(|| error(2, "clorraine: cannot determine a config directory; set HOME or XDG_CONFIG_HOME"))?;
    Ok(home.join(".config/clorraine"))
}

/// Parses the one supported key while preserving spaces in the value.
fn parse_config(text: &str, path: &Path) -> Result<Option<PathBuf>, SetupError> {
    let mut selected = None;
    for line in text.lines() {
        let line = line.strip_suffix('\r').unwrap_or(line);
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(value) = line.strip_prefix("toolchain=") {
            if value.is_empty() {
                return Err(error(
                    2,
                    format!(
                        "clorraine: config '{}' has an empty toolchain value",
                        display::sanitize(&path.display().to_string())
                    ),
                ));
            }
            selected = Some(PathBuf::from(value));
        }
    }
    Ok(selected)
}

/// Writes the config atomically in the same directory.
fn write_config(toolchain: &Path) -> Result<(), SetupError> {
    write_config_at(&config_file()?, toolchain)
}

/// Writes one config file atomically; separated for a filesystem-level test.
fn write_config_at(path: &Path, toolchain: &Path) -> Result<(), SetupError> {
    let parent = path
        .parent()
        .ok_or_else(|| error(2, "clorraine: config path has no parent directory"))?;
    fs::create_dir_all(parent).map_err(|cause| {
        error(
            2,
            format!(
                "clorraine: cannot create config directory '{}': {cause}",
                display::sanitize(&parent.display().to_string())
            ),
        )
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(parent, fs::Permissions::from_mode(0o700));
    }
    let temporary = parent.join(format!("config.tmp.{}", std::process::id()));
    let _ = fs::remove_file(&temporary);
    let text = format!("toolchain={}\n", toolchain.display());
    let result = (|| -> io::Result<()> {
        let mut builder = OpenOptions::new();
        builder.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            builder.mode(0o644);
        }
        let mut file = builder.open(&temporary)?;
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, path)
    })();
    if let Err(cause) = result {
        let _ = fs::remove_file(&temporary);
        return Err(error(
            2,
            format!(
                "clorraine: cannot write config '{}': {cause}",
                display::sanitize(&path.display().to_string())
            ),
        ));
    }
    Ok(())
}

/// Checks the host glibc when `getconf` can report it.
///
/// The upstream compiler is built for glibc 2.38. If `getconf` is unavailable
/// the probe below is still authoritative; if it reports a newer format the
/// command stays out of the way rather than inventing a version.
fn check_glibc() -> Result<(), SetupError> {
    let mut command = Command::new("getconf");
    command.arg("GNU_LIBC_VERSION");
    let Ok(output) = run_limited(command, Duration::from_secs(10)) else {
        return Ok(());
    };
    if !output.status.success() {
        return Ok(());
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let Some(version) = text.split_whitespace().find_map(|part| part.strip_prefix("2.")) else {
        return Ok(());
    };
    let Some(minor) = version.split('.').next().and_then(|part| part.parse::<u32>().ok()) else {
        return Ok(());
    };
    if (2, minor) < MINIMUM_GLIBC {
        return Err(error(
            1,
            format!(
                "clorraine: the Amiga toolchain needs glibc {}.{}+ (Ubuntu 24.04+); this host reports 2.{minor}",
                MINIMUM_GLIBC.0, MINIMUM_GLIBC.1
            ),
        ));
    }
    Ok(())
}

/// Returns the compiler's first `--version` line.
fn compiler_version(compiler: &Path) -> Result<String, SetupError> {
    let mut command = Command::new(compiler);
    command.arg("--version");
    let output = run_limited(command, Duration::from_secs(10)).map_err(|cause| {
        error(
            1,
            format!(
                "clorraine: cannot run toolchain compiler '{}': {cause}",
                display::sanitize(&compiler.display().to_string())
            ),
        )
    })?;
    if !output.status.success() {
        return Err(error(
            1,
            format!(
                "clorraine: toolchain compiler '{}' rejected --version: {}",
                display::sanitize(&compiler.display().to_string()),
                display::sanitize(&first_line(&output.stderr))
            ),
        ));
    }
    let line = first_line(&output.stdout);
    if line.is_empty() {
        return Err(error(1, "clorraine: toolchain compiler printed no version"));
    }
    Ok(line)
}

/// Compiles one C probe with both supported libnix runtimes.
fn probe(compiler: &Path) -> Result<(), SetupError> {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let parent = std::env::temp_dir();
    let root = parent.join(format!(
        "clorraine-toolchain-probe-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&root).map_err(|cause| {
        error(1, format!("clorraine: cannot create toolchain probe directory: {cause}"))
    })?;
    let source = root.join("probe.c");
    if let Err(cause) = fs::write(
        &source,
        "#include <proto/exec.h>\n#include <proto/dos.h>\nint main(void) { return 0; }\n",
    ) {
        let _ = fs::remove_dir_all(&root);
        return Err(error(1, format!("clorraine: cannot write toolchain probe: {cause}")));
    }
    for runtime in ["nix20", "nix13"] {
        let output = root.join(format!("probe-{runtime}"));
        let mut command = Command::new(compiler);
        command
            .arg("-std=c99")
            .arg(format!("-mcrt={runtime}"))
            .arg("-m68000")
            .arg("-o")
            .arg(&output)
            .arg(&source)
            .args(["-lamiga", "-lm"]);
        let result = match run_limited(command, Duration::from_secs(30)) {
            Ok(result) => result,
            Err(cause) => {
                let _ = fs::remove_dir_all(&root);
                return Err(error(
                    1,
                    format!("clorraine: toolchain {runtime} probe could not run: {cause}"),
                ));
            }
        };
        if !result.status.success() {
            let _ = fs::remove_dir_all(&root);
            return Err(error(
                1,
                format!(
                    "clorraine: toolchain {runtime} probe failed: {}",
                    display::sanitize(&first_line(&result.stderr))
                ),
            ));
        }
    }
    let _ = fs::remove_dir_all(&root);
    Ok(())
}

/// Runs a child with a bounded lifetime and captures both streams.
fn run_limited(mut command: Command, timeout: Duration) -> Result<Output, String> {
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|cause| format!("could not start: {cause}"))?;
    // Drain both pipes while the child runs. Waiting first would deadlock a
    // compiler that fills the pipe buffer with warnings or errors.
    let stdout = child.stdout.take().map(|mut pipe| {
        thread::spawn(move || {
            let mut bytes = Vec::new();
            let _ = pipe.read_to_end(&mut bytes);
            bytes
        })
    });
    let stderr = child.stderr.take().map(|mut pipe| {
        thread::spawn(move || {
            let mut bytes = Vec::new();
            let _ = pipe.read_to_end(&mut bytes);
            bytes
        })
    });
    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                // Do not join a reader that a surviving grandchild could keep
                // open; dropping the handle detaches it and the setup process
                // still returns the timeout.
                return Err("timed out".to_owned());
            }
            Ok(None) => thread::sleep(Duration::from_millis(20)),
            Err(cause) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("could not wait: {cause}"));
            }
        }
    };
    let stdout = stdout.map(|reader| reader.join().unwrap_or_default()).unwrap_or_default();
    let stderr = stderr.map(|reader| reader.join().unwrap_or_default()).unwrap_or_default();
    Ok(Output { status, stdout, stderr })
}

/// The first non-empty line of a byte stream, trimmed.
fn first_line(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes)
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("")
        .to_owned()
}

/// Resolves a required directory and proves it stays inside the root.
fn contained_directory(path: &Path, root: &Path) -> Option<PathBuf> {
    let resolved = path.canonicalize().ok()?;
    (resolved.is_dir() && resolved.starts_with(root)).then_some(resolved)
}

/// True for a regular executable file.
fn is_executable(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::metadata(path)
            .map(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        path.is_file()
    }
}

/// Builds a setup error with the driver's `clorraine:` prefix.
///
/// Dynamic paths and compiler lines are sanitised at construction; the sentence
/// may contain the one intentional newline after the primary message.
fn error(code: i32, message: impl Into<String>) -> SetupError {
    let message = message.into();
    let message = if message.starts_with("clorraine:") {
        message
    } else {
        format!("clorraine: {message}")
    };
    SetupError { code, message: format!("{message}\n") }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_dir_prefers_an_absolute_xdg_home() {
        let path = config_dir(Some(OsStr::new("/xdg")), Some(OsStr::new("/home/user")))
            .expect("absolute xdg");
        assert_eq!(path, Path::new("/xdg/clorraine"));
    }

    #[test]
    fn config_dir_ignores_a_relative_xdg_home() {
        let path = config_dir(Some(OsStr::new("relative")), Some(OsStr::new("/home/user")))
            .expect("home fallback");
        assert_eq!(path, Path::new("/home/user/.config/clorraine"));
    }

    #[test]
    fn config_parser_preserves_spaces_and_equals_in_the_path() {
        let path = Path::new("/config");
        let selected = parse_config("toolchain=/a path/with=equals\n", path)
            .expect("parse")
            .expect("selection");
        assert_eq!(selected, Path::new("/a path/with=equals"));
    }

    #[test]
    fn config_parser_allows_an_empty_file_but_rejects_an_empty_value() {
        assert!(parse_config("# comment\n", Path::new("/config")).expect("empty").is_none());
        assert!(parse_config("toolchain=\n", Path::new("/config")).is_err());
    }

    #[test]
    fn config_writer_replaces_the_selection_atomically() {
        let dir = std::env::temp_dir().join(format!("clorraine-config-{}", std::process::id()));
        let path = dir.join("config");
        write_config_at(&path, Path::new("/a path/one")).expect("first write");
        assert_eq!(fs::read_to_string(&path).unwrap(), "toolchain=/a path/one\n");
        write_config_at(&path, Path::new("/a path/two")).expect("second write");
        assert_eq!(fs::read_to_string(&path).unwrap(), "toolchain=/a path/two\n");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_timed_out_child_returns_without_waiting_for_pipe_holders() {
        let mut command = Command::new("/bin/sh");
        command.args(["-c", "sleep 2"]);
        let started = Instant::now();
        let error = run_limited(command, Duration::from_millis(100)).expect_err("timeout");
        assert_eq!(error, "timed out");
        assert!(started.elapsed() < Duration::from_secs(1), "timeout must be bounded");
    }

    #[test]
    fn a_failed_config_rename_removes_the_temporary_file() {
        let dir = std::env::temp_dir().join(format!("clorraine-config-fail-{}", std::process::id()));
        let target = dir.join("config");
        fs::create_dir_all(&target).expect("create target directory");
        assert!(write_config_at(&target, Path::new("/a/toolchain")).is_err());
        let names: Vec<String> = fs::read_dir(&dir)
            .expect("read temp dir")
            .map(|entry| entry.expect("entry").file_name().to_string_lossy().into_owned())
            .collect();
        assert!(!names.iter().any(|name| name.starts_with("config.tmp.")), "temp file remains: {names:?}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn first_line_skips_blank_lines() {
        assert_eq!(first_line(b"\n  hello\r\nworld\n"), "hello");
        assert_eq!(first_line(b""), "");
    }
}
