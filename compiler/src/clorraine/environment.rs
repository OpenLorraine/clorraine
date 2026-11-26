//! Toolchain selection and required-tool checks for the developer driver.
//!
//! An explicit `--toolchain`, `CLORRAINE_TOOLCHAIN`, or stored config selects
//! one upstream compiler tree. The selection never falls back after an invalid
//! value. The pinned SDK container keeps its legacy `PATH` lookup and host
//! handoff only when its marker is present and no native toolchain was chosen.
//!
//! Host and `emit` commands ignore toolchain settings. The selected compiler is
//! returned as an absolute path so a later directory change cannot redirect it.

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use super::cli::{Command, Options};
use super::display;
use super::platform::{self, HostOs};
use super::setup::{self, Validation};
use super::SETUP_GUIDANCE;

/// The file the SDK image always contains; its presence marks the container.
pub const SDK_MARKER: &str = "/opt/amiga/BUILD-PROVENANCE.txt";

/// A native toolchain selected for one invocation.
#[derive(Debug)]
pub struct NativeSdk {
    /// Absolute compiler spelling within the selected immutable tree.
    pub compiler: PathBuf,
}

/// An environment failure with the driver's established observable status.
#[derive(Debug)]
pub struct EnvironmentError {
    /// Two for invalid selection, one for a missing required executable/config.
    pub code: i32,
    /// Terminal-sanitised message suitable for text or NDJSON diagnostics.
    pub message: String,
}

/// Resolves a native toolchain without consulting lower priorities after a
/// selection fails.
///
/// `None` means the caller keeps the host or legacy SDK-container path. An
/// absent selection outside the marked container is an actionable error, not a
/// silent fallback.
pub fn resolve_sdk(options: &Options, marker: &str) -> Result<Option<NativeSdk>, EnvironmentError> {
    if options.profile == crate::profile::Profile::Host || options.command == Command::Emit {
        return Ok(None);
    }
    if let Some(root) = &options.toolchain {
        return selected(Path::new(root));
    }
    if let Some(root) = std::env::var_os("CLORRAINE_TOOLCHAIN") {
        return selected(Path::new(&root));
    }
    resolve_from_stored(options, marker, setup::stored_toolchain())
}

/// Applies the stored selection when no explicit value was supplied.
///
/// The stored result and marker are parameters so the legacy fallback can be
/// tested without changing the process environment or the real config file.
fn resolve_from_stored(
    options: &Options,
    marker: &str,
    stored: Result<Option<PathBuf>, setup::SetupError>,
) -> Result<Option<NativeSdk>, EnvironmentError> {
    if options.profile == crate::profile::Profile::Host || options.command == Command::Emit {
        return Ok(None);
    }
    match stored {
        Ok(Some(root)) => selected(&root),
        Ok(None) => {
            if sdk_present(marker) {
                return Ok(None);
            }
            Err(EnvironmentError {
                code: 2,
                message: format!(
                    "clorraine: no toolchain selected; run '{}' or pass --toolchain PATH\n",
                    SETUP_GUIDANCE
                ),
            })
        }
        Err(error) => Err(from_setup(error)),
    }
}

/// Validates one selected root with the cheap build-time checks.
fn selected(root: &Path) -> Result<Option<NativeSdk>, EnvironmentError> {
    let toolchain = setup::validate(root, Validation::Selection).map_err(from_setup)?;
    Ok(Some(NativeSdk {
        compiler: toolchain.compiler,
    }))
}

/// Converts a setup diagnostic into the driver's environment status.
fn from_setup(error: setup::SetupError) -> EnvironmentError {
    EnvironmentError {
        code: error.code,
        message: error.message,
    }
}

/// True when the SDK image marker exists.
///
/// The marker is a regular file, so a bare directory with the right name does
/// not count.
pub fn sdk_present(marker: &str) -> bool {
    Path::new(marker).is_file()
}

/// The first required tool that is not an executable on `PATH`, if any.
///
/// The search takes the `PATH` value as an argument so it stays testable: a
/// test can pass an empty or synthetic search path without mutating the
/// process environment. The caller reports the returned name and exits 1.
pub fn missing_tool_in(tools: &[&str], path: &OsStr) -> Option<String> {
    let dirs: Vec<PathBuf> = std::env::split_paths(path).collect();
    tools
        .iter()
        .find(|tool| !tool_on_path(&dirs, tool))
        .map(|tool| (*tool).to_owned())
}

/// True when `tool` names an executable file in one of the search directories.
///
/// On Windows the file is `<tool>.exe`, the name the shell would run.
fn tool_on_path(dirs: &[PathBuf], tool: &str) -> bool {
    let name = platform::executable_name(HostOs::current(), tool);
    dirs.iter().any(|dir| is_executable_file(&dir.join(&name)))
}

/// Chooses the host C compiler for one invocation.
///
/// A `CLORRAINE_CC` value (`override_cc`) is the only candidate: a path must
/// name an executable file, and a bare name must be found on `PATH`. Without
/// it, the first of [`platform::host_compilers`] found on `PATH` is used.
/// The returned program is spawned as given, so a bare name keeps the usual
/// `PATH` lookup.
///
/// # Errors
///
/// An exit-1 [`EnvironmentError`] naming the missing override, or the host's
/// missing-compiler hint when no candidate is found.
pub fn resolve_host_compiler(
    os: HostOs,
    override_cc: Option<&OsStr>,
    path: &OsStr,
) -> Result<OsString, EnvironmentError> {
    let dirs: Vec<PathBuf> = std::env::split_paths(path).collect();
    let found = |name: &OsStr| -> bool {
        let text = name.to_string_lossy();
        if platform::is_path_like(os, &text) {
            let given = Path::new(name);
            is_executable_file(given)
                || (os == HostOs::Windows && is_executable_file(Path::new(&platform::executable_name(os, &text))))
        } else {
            let file = platform::executable_name(os, &text);
            dirs.iter().any(|dir| is_executable_file(&dir.join(&file)))
        }
    };
    if let Some(chosen) = override_cc {
        if found(chosen) {
            return Ok(chosen.to_owned());
        }
        return Err(EnvironmentError {
            code: 1,
            message: format!(
                "clorraine: CLORRAINE_CC '{}' was not found",
                display::sanitize(&chosen.to_string_lossy())
            ),
        });
    }
    if let Some(name) = platform::host_compilers(os, None).into_iter().find(|name| found(name)) {
        return Ok(name);
    }
    let message = match os {
        HostOs::Unix => missing_tool_message("cc"),
        HostOs::Windows => format!(
            "clorraine: no C compiler (gcc, cc, or clang) was found on PATH\n{}",
            platform::missing_compiler_hint(os)
        ),
    };
    Err(EnvironmentError { code: 1, message })
}

/// The error printed when a required tool is missing.
pub fn missing_tool_message(tool: &str) -> String {
    let hint = match tool {
        "cc" => platform::missing_compiler_hint(HostOs::Unix).to_owned(),
        "vamos" => "install amitools (vamos) or use --emulator".to_owned(),
        _ => format!("install the toolchain or run '{SETUP_GUIDANCE}'"),
    };
    format!("clorraine: required tool '{tool}' was not found on PATH\n{hint}")
}

/// True when `path` is a file the current user may execute.
///
/// A plain file on `PATH` is not enough: a shell runs it only when the execute
/// bit is set, so a present-but-not-executable tool must be reported missing
/// instead of failing later with a confusing spawn error. Symbolic links are
/// followed, matching how a shell resolves a `PATH` entry.
#[cfg(unix)]
pub(super) fn is_executable_file(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .map(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

/// Non-Unix fallback: a regular file stands in for an executable one.
#[cfg(not(unix))]
pub(super) fn is_executable_file(path: &Path) -> bool {
    path.is_file()
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use super::*;

    /// A search directory holding executable files named `names`.
    #[cfg(unix)]
    fn tools_dir(tag: &str, names: &[&str]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("clorraine-cc-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create the search directory");
        for name in names {
            let file = dir.join(name);
            std::fs::write(&file, b"").expect("write the tool file");
            make_executable(&file);
        }
        dir
    }

    #[cfg(unix)]
    #[test]
    fn the_host_compiler_is_the_first_windows_candidate_on_path() {
        let dir = tools_dir("order", &["cc.exe", "clang.exe"]);
        let found = resolve_host_compiler(HostOs::Windows, None, dir.as_os_str());
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(found.expect("a compiler"), OsString::from("cc"));
    }

    #[cfg(unix)]
    #[test]
    fn a_bare_override_is_looked_up_on_path_and_a_path_override_by_file() {
        let dir = tools_dir("override", &["mycc", "gcc.exe"]);
        let bare = resolve_host_compiler(HostOs::Unix, Some(OsStr::new("mycc")), dir.as_os_str());
        let path = dir.join("mycc");
        let by_path = resolve_host_compiler(HostOs::Unix, Some(path.as_os_str()), OsStr::new(""));
        let windows_stem = dir.join("gcc");
        let windows = resolve_host_compiler(HostOs::Windows, Some(windows_stem.as_os_str()), OsStr::new(""));
        let missing = resolve_host_compiler(HostOs::Unix, Some(OsStr::new("mycc")), OsStr::new(""));
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(bare.expect("found on PATH"), OsString::from("mycc"));
        assert_eq!(by_path.expect("found by path"), path.into_os_string());
        assert_eq!(windows.expect("found with .exe"), windows_stem.into_os_string());
        let error = missing.expect_err("not on an empty PATH");
        assert_eq!((error.code, error.message.as_str()), (1, "clorraine: CLORRAINE_CC 'mycc' was not found"));
    }

    #[test]
    fn a_missing_host_compiler_keeps_the_unix_message_and_names_gcc_on_windows() {
        let unix = resolve_host_compiler(HostOs::Unix, None, OsStr::new("")).expect_err("none");
        assert_eq!(unix.code, 1);
        assert_eq!(unix.message, missing_tool_message("cc"));
        assert_eq!(
            unix.message,
            "clorraine: required tool 'cc' was not found on PATH\ninstall a C compiler (cc)"
        );
        let windows = resolve_host_compiler(HostOs::Windows, None, OsStr::new("")).expect_err("none");
        assert_eq!(windows.code, 1);
        assert!(windows.message.contains("gcc, cc, or clang"), "{}", windows.message);
        assert!(windows.message.ends_with(platform::missing_compiler_hint(HostOs::Windows)));
    }

    #[test]
    fn the_marker_counts_only_when_it_is_a_file() {
        let existing = concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml");
        assert!(sdk_present(existing));
        assert!(!sdk_present("/no/such/clorraine/sdk-marker.txt"));
    }

    /// Marks a file executable so a test can stand in for a real tool.
    #[cfg(unix)]
    fn make_executable(path: &Path) {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = std::fs::metadata(path)
            .expect("the tool file exists")
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(path, permissions).expect("set the execute bit");
    }

    #[test]
    fn a_tool_is_missing_unless_an_executable_is_on_the_search_path() {
        let dir = std::env::temp_dir().join(format!("clorraine-env-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create the search directory");
        let executable = dir.join("present-tool");
        std::fs::write(&executable, b"").expect("write the tool file");
        #[cfg(unix)]
        make_executable(&executable);
        std::fs::write(dir.join("not-executable-tool"), b"").expect("write the plain file");

        let path = OsString::from(dir.as_os_str());
        assert_eq!(missing_tool_in(&["present-tool"], &path), None);
        assert_eq!(
            missing_tool_in(&["absent-tool"], &path),
            Some("absent-tool".to_owned())
        );
        // The first absent tool wins when several are checked.
        assert_eq!(
            missing_tool_in(&["present-tool", "absent-tool"], &path),
            Some("absent-tool".to_owned())
        );
        // A present file without the execute bit is still missing.
        #[cfg(unix)]
        assert_eq!(
            missing_tool_in(&["not-executable-tool"], &path),
            Some("not-executable-tool".to_owned())
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_marked_container_without_a_selection_keeps_the_legacy_path() {
        let options = super::super::cli::parse(&[
            "run".to_owned(), "--profile".to_owned(), "hosted-amiga".to_owned(),
            "hello.clo".to_owned(),
        ]).expect("parse run");
        let options = match options {
            super::super::cli::Action::Invoke(options) => options,
            other => panic!("expected an invocation, got {other:?}"),
        };
        let marker = concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml");
        assert!(resolve_from_stored(&options, marker, Ok(None)).unwrap().is_none());
    }

    #[test]
    fn an_unmarked_host_without_a_selection_is_told_to_run_setup() {
        let options = super::super::cli::parse(&[
            "run".to_owned(), "--profile".to_owned(), "hosted-amiga".to_owned(),
            "hello.clo".to_owned(),
        ]).expect("parse run");
        let options = match options {
            super::super::cli::Action::Invoke(options) => options,
            other => panic!("expected an invocation, got {other:?}"),
        };
        let error = resolve_from_stored(&options, "/no/such/sdk-marker", Ok(None))
            .expect_err("a missing marker must stop the selection");
        assert_eq!(error.code, 2);
        assert!(error.message.contains("setup --toolchain"), "message: {}", error.message);
    }

    #[test]
    fn an_empty_search_path_finds_every_tool_missing() {
        let empty = OsString::new();
        assert_eq!(
            missing_tool_in(&["cc", "m68k-amigaos-gcc"], &empty),
            Some("cc".to_owned())
        );
    }
}
