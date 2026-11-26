//! Toolchain setup, validation, and persisted selection.
//!
//! The user downloads an Amiga compiler tree once. `setup` verifies that the
//! tree is complete, that its compiler is the supported `16.2-rc24` build, and
//! that both libnix runtimes and the NDK headers can link a trivial program.
//! Only after those checks pass does it replace the stored toolchain path.
//!
//! `setup` can also remember the FS-UAE executable and the Kickstart image for
//! emulator runs. Each is validated before it is stored, and a later setup
//! that does not name them keeps the remembered values.
//!
//! Build and run commands use a lighter selection check: the root, compiler,
//! and `m68k-amigaos` tree must exist, but the expensive probes run only during
//! setup. A broken library then fails at the real link with the compiler's own
//! error.

use std::fs::{self, OpenOptions};
use std::io::{self, Read, Write};
use std::thread;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use super::cli::Options;
use super::display;
use super::emulator;
use super::platform::{self, executable_name, HostOs};

/// The only toolchain version this release validates.
const SUPPORTED_VERSION: &str = "16.2-rc24";

/// The smallest glibc version that can run the supported upstream host tools.
const MINIMUM_GLIBC: (u32, u32) = (2, 38);

/// A complete, validated toolchain.
pub struct Toolchain {
    /// The canonical toolchain root, in the spelling people write (no
    /// verbatim `\\?\` prefix on Windows).
    pub root: PathBuf,
    /// The absolute compiler inside the root, in the same spelling.
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

/// The settings the CLI remembers between commands.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Config {
    /// The selected toolchain root.
    pub toolchain: Option<PathBuf>,
    /// The Kickstart image for emulator runs.
    pub kickstart: Option<PathBuf>,
    /// The FS-UAE executable for emulator runs.
    pub fs_uae: Option<PathBuf>,
}

/// Runs `setup --toolchain PATH [--kickstart PATH] [--fs-uae PATH]`.
///
/// The stored config is replaced only after every named input validates. An
/// emulator path that is not named keeps its remembered value.
pub fn run(options: &Options) -> Result<(), SetupError> {
    let selected = options
        .toolchain
        .as_deref()
        .ok_or_else(|| error(2, "--toolchain is required with 'setup'"))?;
    let toolchain = validate(Path::new(selected), Validation::Full)?;
    let (fs_uae, kickstart) =
        emulator::validate_media(options.fs_uae.as_deref(), options.kickstart.as_deref())
            .map_err(|failure| SetupError { code: failure.code, message: failure.message })?;
    // An unreadable or malformed old config must not block the repair that
    // setup exists for; it is replaced as a whole below.
    let stored = stored_config().unwrap_or_default();
    let config = Config {
        toolchain: Some(toolchain.root.clone()),
        kickstart: kickstart.clone().or(stored.kickstart),
        fs_uae: fs_uae.clone().or(stored.fs_uae),
    };
    write_config(&config)?;
    println!(
        "configured toolchain {}",
        display::sanitize(&toolchain.root.display().to_string())
    );
    println!("{}", display::sanitize(&toolchain.version));
    if let Some(path) = &kickstart {
        println!("configured Kickstart {}", display::sanitize(&path.display().to_string()));
    }
    if let Some(path) = &fs_uae {
        println!("configured FS-UAE {}", display::sanitize(&path.display().to_string()));
    }
    Ok(())
}

/// Validates a toolchain root and returns its selected compiler.
///
/// Relative roots are resolved against the caller's current directory. Every
/// returned path is resolved and absolute, so a later directory change or
/// replacement alias cannot select a different compiler; on Windows it is
/// returned without the verbatim `\\?\` prefix that resolving adds.
pub fn validate(selected: &Path, mode: Validation) -> Result<Toolchain, SetupError> {
    validate_for(HostOs::current(), selected, mode)
}

/// [`validate`] with the host family as a parameter.
///
/// The family picks the compiler's file name (`bin/m68k-amigaos-gcc.exe` on
/// Windows) and the path spelling: Windows paths are returned and reported
/// without the verbatim `\\?\` prefix that resolving them adds, so the
/// config file and every message show an ordinary `C:\…` path.
///
/// # Errors
///
/// A [`SetupError`] with exit code 2 for an invalid selection, or 1 when the
/// compiler fails its version check or runtime probes.
pub fn validate_for(os: HostOs, selected: &Path, mode: Validation) -> Result<Toolchain, SetupError> {
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
    // The containment checks compare canonical spellings; people see the
    // display spelling, which is also what the config stores.
    let shown_root = shown_path(os, root.clone());
    if !root.is_dir() {
        return Err(error(2, format!("--toolchain: '{}' is not a directory", shown_root.display())));
    }
    let root_text = root
        .to_str()
        .ok_or_else(|| error(2, "--toolchain: canonical path must be UTF-8"))?;
    if root_text.chars().any(char::is_control) {
        return Err(error(2, "--toolchain: canonical path must not contain control characters"));
    }
    let amiga = contained_directory(&root.join("m68k-amigaos"), &root)
        .ok_or_else(|| error(2, format!("--toolchain: '{}' has no m68k-amigaos tree", shown_root.display())))?;
    for required in ["libnix", "ndk-include"] {
        if contained_directory(&amiga.join(required), &root).is_none() {
            return Err(error(
                2,
                format!("--toolchain: '{}' has no m68k-amigaos/{required}", shown_root.display()),
            ));
        }
    }
    let compiler_name = executable_name(os, "m68k-amigaos-gcc");
    let compiler = root.join("bin").join(&compiler_name);
    let compiler = compiler.canonicalize().map_err(|_| {
        error(
            2,
            format!("--toolchain: '{}' has no bin/{compiler_name}", shown_root.display()),
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
            format!("--toolchain: '{}' is not executable", shown_path(os, compiler.clone()).display()),
        ));
    }
    if mode == Validation::Selection {
        return Ok(Toolchain {
            root: shown_root,
            compiler: shown_path(os, compiler),
            version: String::new(),
        });
    }
    if cfg!(target_os = "linux") {
        check_glibc()?;
    }
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
    Ok(Toolchain { root: shown_root, compiler: shown_path(os, compiler), version })
}

/// Reads the stored toolchain, if any.
pub fn stored_toolchain() -> Result<Option<PathBuf>, SetupError> {
    Ok(stored_config()?.toolchain)
}

/// Reads every remembered setting; a missing config is an empty one.
pub fn stored_config() -> Result<Config, SetupError> {
    let path = config_file()?;
    match fs::read_to_string(&path) {
        Ok(text) => parse_config(&text, &path),
        Err(cause) if cause.kind() == io::ErrorKind::NotFound => Ok(Config::default()),
        Err(cause) => Err(error(
            2,
            format!(
                "clorraine: cannot read config '{}': {cause}",
                display::sanitize(&path.display().to_string())
            ),
        )),
    }
}

/// The config file owned by the CLI: `XDG_CONFIG_HOME` or `HOME` on Unix,
/// `%APPDATA%` on Windows (see [`platform::config_file`]).
pub fn config_file() -> Result<PathBuf, SetupError> {
    platform::config_file(HostOs::current(), |name| std::env::var_os(name))
        .map_err(|message| error(2, message))
}

/// A resolved path in the spelling the host's users write (no verbatim
/// prefix on Windows).
fn shown_path(os: HostOs, path: PathBuf) -> PathBuf {
    match path.to_str() {
        Some(text) => PathBuf::from(platform::display_path(os, text)),
        None => path,
    }
}

/// Parses the supported keys while preserving spaces in the values. Unknown
/// keys are ignored so an older driver can read a newer config.
fn parse_config(text: &str, path: &Path) -> Result<Config, SetupError> {
    let mut config = Config::default();
    for line in text.lines() {
        let line = line.strip_suffix('\r').unwrap_or(line);
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let slot = match key {
            "toolchain" => &mut config.toolchain,
            "kickstart" => &mut config.kickstart,
            "fs-uae" => &mut config.fs_uae,
            _ => continue,
        };
        if value.is_empty() {
            return Err(error(
                2,
                format!(
                    "clorraine: config '{}' has an empty {key} value",
                    display::sanitize(&path.display().to_string())
                ),
            ));
        }
        *slot = Some(PathBuf::from(value));
    }
    Ok(config)
}

/// Renders the config file text, one `key=value` line per remembered setting.
fn render_config(config: &Config) -> String {
    let mut text = String::new();
    for (key, value) in [
        ("toolchain", &config.toolchain),
        ("kickstart", &config.kickstart),
        ("fs-uae", &config.fs_uae),
    ] {
        if let Some(value) = value {
            text.push_str(&format!("{key}={}\n", value.display()));
        }
    }
    text
}

/// Writes the config atomically in the same directory.
fn write_config(config: &Config) -> Result<(), SetupError> {
    write_config_at(&config_file()?, config)
}

/// Writes one config file atomically; separated for a filesystem-level test.
fn write_config_at(path: &Path, config: &Config) -> Result<(), SetupError> {
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
    let text = render_config(config);
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

    /// A config that selects only `toolchain`.
    fn selection(toolchain: &str) -> Config {
        Config { toolchain: Some(PathBuf::from(toolchain)), ..Config::default() }
    }

    #[test]
    fn the_emulator_paths_round_trip_and_unknown_keys_are_ignored() {
        let config = Config {
            toolchain: Some(PathBuf::from("/t")),
            kickstart: Some(PathBuf::from("/roms/kick 31.rom")),
            fs_uae: Some(PathBuf::from("/usr/bin/fs-uae")),
        };
        let text = render_config(&config);
        assert_eq!(text, "toolchain=/t\nkickstart=/roms/kick 31.rom\nfs-uae=/usr/bin/fs-uae\n");
        let parsed = parse_config(&format!("{text}future=1\n"), Path::new("/config")).expect("parse");
        assert_eq!(parsed, config);
    }

    /// The Unix config rule over a fixed `XDG_CONFIG_HOME` and `HOME`.
    fn unix_config(xdg: &str, home: &str) -> PathBuf {
        let (xdg, home) = (xdg.to_owned(), home.to_owned());
        platform::config_file(HostOs::Unix, move |name| match name {
            "XDG_CONFIG_HOME" => Some(xdg.clone().into()),
            "HOME" => Some(home.clone().into()),
            _ => None,
        })
        .expect("a config file")
    }

    #[test]
    fn config_dir_prefers_an_absolute_xdg_home() {
        assert_eq!(unix_config("/xdg", "/home/user"), Path::new("/xdg/clorraine/config"));
    }

    #[test]
    fn config_dir_ignores_a_relative_xdg_home() {
        assert_eq!(unix_config("relative", "/home/user"), Path::new("/home/user/.config/clorraine/config"));
    }

    #[test]
    fn config_parser_preserves_spaces_and_equals_in_the_path() {
        let path = Path::new("/config");
        let selected = parse_config("toolchain=/a path/with=equals\n", path)
            .expect("parse")
            .toolchain
            .expect("selection");
        assert_eq!(selected, Path::new("/a path/with=equals"));
    }

    #[test]
    fn config_parser_allows_an_empty_file_but_rejects_an_empty_value() {
        assert_eq!(parse_config("# comment\n", Path::new("/config")).expect("empty"), Config::default());
        assert!(parse_config("toolchain=\n", Path::new("/config")).is_err());
        assert!(parse_config("kickstart=\n", Path::new("/config")).is_err());
    }

    #[test]
    fn config_writer_replaces_the_selection_atomically() {
        let dir = std::env::temp_dir().join(format!("clorraine-config-{}", std::process::id()));
        let path = dir.join("config");
        write_config_at(&path, &selection("/a path/one")).expect("first write");
        assert_eq!(fs::read_to_string(&path).unwrap(), "toolchain=/a path/one\n");
        write_config_at(&path, &selection("/a path/two")).expect("second write");
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
        assert!(write_config_at(&target, &selection("/a/toolchain")).is_err());
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
