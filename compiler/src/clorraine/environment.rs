//! Environment checks for the `clorraine` developer driver.
//!
//! Native SDK selection is resolved once from explicit options, environment or
//! the canonical installed executable. A selected invalid installation never
//! falls back to another location. Without a native selection, legacy commands
//! keep the pinned container marker and PATH checks.
//!
//! Host and emit commands ignore unused SDK settings. Receipt validation does
//! not run the tools it validates or replace the installer's archive audit.

use std::path::{Path, PathBuf};

use super::cli::{Command, Options};
use super::{display, native_metadata};
use crate::profile::Profile;

/// The file the SDK image always contains; its presence marks the container.
pub const SDK_MARKER: &str = "/opt/amiga/BUILD-PROVENANCE.txt";

/// Native paths pinned beneath one canonical assembly for this invocation.
pub struct NativeSdk {
    /// Absolute compiler spelling within the selected, immutable SDK.
    pub compiler: PathBuf,
    /// Paired venv-spelled interpreter, present only when a vamos run needs it.
    pub python: Option<PathBuf>,
}

/// An environment failure with the driver's established observable status.
pub struct EnvironmentError {
    /// Two for invalid selection, one for a missing required executable/config.
    pub code: i32,
    /// Terminal-sanitised message suitable for text or NDJSON diagnostics.
    pub message: String,
}

/// Selects and validates a native assembly without consulting lower priorities.
///
/// `None` means the caller retains its existing host/legacy checks. Explicit
/// options override environment, which overrides adjacent installed discovery.
/// Returned paths never traverse the mutable `current` alias again.
pub fn resolve_sdk(options: &Options) -> Result<Option<NativeSdk>, EnvironmentError> {
    if options.profile == Profile::Host || options.command == Command::Emit {
        return Ok(None);
    }
    if let Some(root) = &options.sdk_root {
        return load_sdk(Path::new(root), options).map(Some);
    }
    if let Some(root) = std::env::var_os("CLORRAINE_SDK") {
        return load_sdk(Path::new(&root), options).map(Some);
    }
    let executable = std::env::current_exe()
        .and_then(|path| path.canonicalize())
        .map_err(|error| invalid_sdk(Path::new("installed executable"), &error.to_string()))?;
    if let Some(bin) = executable.parent().filter(|path| path.file_name().is_some_and(|name| name == "bin")) {
        if let Some(root) = bin.parent() {
            let candidate = ["installation.json", "sdk"].iter().any(|name| {
                // Unlike exists(), symlink_metadata also notices dangling
                // entries. Permission errors cannot safely mean "no SDK".
                match std::fs::symlink_metadata(root.join(name)) {
                    Ok(_) => true,
                    Err(error) => error.kind() != std::io::ErrorKind::NotFound,
                }
            });
            if candidate {
                return load_sdk(&root.join("sdk/amiga"), options).map(Some);
            }
        }
    }
    Ok(None)
}

/// Resolves the assembly prefix before following any SDK leaf directory.
///
/// Canonicalising sdk/amiga first would silently adopt another installation
/// when that leaf is an escaping alias. Only the enclosing root/current alias
/// is resolved; real SDK directories and internal tool containment follow.
fn load_sdk(selected: &Path, options: &Options) -> Result<NativeSdk, EnvironmentError> {
    if !cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        return Err(invalid_sdk(selected, "native SDKs require Linux x86_64"));
    }
    if selected.file_name().is_none_or(|name| name != "amiga")
        || selected.parent().and_then(Path::file_name).is_none_or(|name| name != "sdk")
    {
        return Err(invalid_sdk(selected, "select the installation's sdk/amiga directory with --sdk-root or CLORRAINE_SDK"));
    }
    let root = selected.parent().and_then(Path::parent)
        .ok_or_else(|| invalid_sdk(selected, "missing installation root"))?;
    let root = if root.as_os_str().is_empty() { Path::new(".") } else { root };
    let root = root.canonicalize().map_err(|error| invalid_sdk(selected, &error.to_string()))?;
    real_directory(&root.join("sdk"), selected)?;
    let sdk = root.join("sdk/amiga");
    real_directory(&sdk, selected)?;
    let descriptor = root.join("installation.json");
    let metadata = native_metadata::installation(&descriptor)
        .map_err(|error| invalid_sdk(selected, &format!("installation.json: {error}")))?;
    native_metadata::preparation(&sdk.join(".ndk-prepared.json"), &metadata)
        .map_err(|error| invalid_sdk(selected, &format!(".ndk-prepared.json: {error}")))?;
    let compiler = sdk.join("bin/m68k-amigaos-gcc");
    contained_tool(&compiler, &sdk, selected, "m68k-amigaos-gcc", false)?;
    let python = if options.command == Command::Run && !options.emulator
        && matches!(options.profile, Profile::HostedAmiga | Profile::HostedAmiga13)
    {
        Some(companion_python(&root, selected, &metadata)?)
    } else {
        None
    };
    Ok(NativeSdk { compiler, python })
}

/// Requires real SDK roots, rather than aliases to a different assembly.
fn real_directory(path: &Path, selected: &Path) -> Result<(), EnvironmentError> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() => Ok(()),
        _ => Err(invalid_sdk(selected, &format!("{} must be a real directory", path.display()))),
    }
}

/// Validates only the command-required paired venv, retaining its Python name.
fn companion_python(
    root: &Path,
    selected: &Path,
    metadata: &native_metadata::Installation,
) -> Result<PathBuf, EnvironmentError> {
    let venv = root.join("sdk/vamos");
    match std::fs::symlink_metadata(&venv) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(missing_native_tool(selected, "vamos"));
        }
        _ => real_directory(&venv, selected)?,
    }
    let config = venv.join("pyvenv.cfg");
    if std::fs::symlink_metadata(&config).is_err_and(|error| {
        error.kind() == std::io::ErrorKind::NotFound
    }) {
        return Err(missing_native_tool(selected, "pyvenv.cfg"));
    }
    native_metadata::ordinary_text(&config)
        .map_err(|error| invalid_sdk(selected, &format!("pyvenv.cfg: {error}")))?;
    native_metadata::wheels(&venv.join("wheels.json"), metadata)
        .map_err(|error| invalid_sdk(selected, &format!("wheels.json: {error}")))?;
    let bin = venv.join("bin");
    let canonical_bin = bin.canonicalize().map_err(|_| missing_native_tool(selected, "python"))?;
    if !canonical_bin.starts_with(&venv) {
        return Err(invalid_sdk(selected, "vamos bin directory escapes the paired environment"));
    }
    let python = bin.join("python");
    contained_tool(&python, &venv, selected, "python", true)?;
    Ok(python)
}

/// Allows internal compiler aliases, and the venv's system-interpreter link.
///
/// The returned invocation still uses the original absolute spelling: using
/// the final Python symlink target would discard the venv's prefix selection.
fn contained_tool(
    path: &Path,
    owner: &Path,
    selected: &Path,
    name: &str,
    system_interpreter: bool,
) -> Result<(), EnvironmentError> {
    let canonical = path.canonicalize().map_err(|_| missing_native_tool(selected, name))?;
    if !system_interpreter && !canonical.starts_with(owner) {
        return Err(invalid_sdk(selected, &format!("{name} escapes the selected SDK")));
    }
    if !is_executable_file(path) {
        return Err(missing_native_tool(selected, name));
    }
    Ok(())
}

/// Names the selected location without leaking terminal control bytes.
fn invalid_sdk(selected: &Path, detail: &str) -> EnvironmentError {
    EnvironmentError {
        code: 2,
        message: display::sanitize(&format!(
            "clorraine: invalid selected SDK '{}': {detail}\n\
             select a complete native sdk/amiga with --sdk-root or CLORRAINE_SDK",
            selected.display(),
        )).into_owned(),
    }
}

/// A well-formed assembly with an absent executable/config remains exit one.
fn missing_native_tool(selected: &Path, name: &str) -> EnvironmentError {
    EnvironmentError {
        code: 1,
        message: display::sanitize(&format!(
            "clorraine: native SDK '{}': required tool/config '{name}' is missing or not executable\n\
             restore the installation or select a complete SDK with --sdk-root / CLORRAINE_SDK",
            selected.display(),
        )).into_owned(),
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
pub fn missing_tool_in(tools: &[&str], path: &std::ffi::OsStr) -> Option<String> {
    let dirs: Vec<PathBuf> = std::env::split_paths(path).collect();
    tools
        .iter()
        .find(|tool| !tool_on_path(&dirs, tool))
        .map(|tool| (*tool).to_owned())
}

/// True when `tool` names an executable file in one of the search directories.
fn tool_on_path(dirs: &[PathBuf], tool: &str) -> bool {
    dirs.iter().any(|dir| is_executable_file(&dir.join(tool)))
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

/// The guidance printed when an Amiga or freestanding command runs outside the
/// SDK container.
pub fn sdk_guidance() -> String {
    "clorraine: this command needs the pinned SDK container.\n\
     From the repository root, run: make run FILE=FILE.clo\n\
     or open a shell (make -C docker shell), then: ./target/debug/clorraine run FILE.clo"
        .to_owned()
}

/// The error printed when a required tool is missing.
pub fn missing_tool_message(tool: &str) -> String {
    format!(
        "clorraine: required tool '{tool}' was not found on PATH\n\
         install the SDK toolchain or run inside the container (make -C docker shell)"
    )
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use super::*;

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
    fn an_empty_search_path_finds_every_tool_missing() {
        let empty = OsString::new();
        assert_eq!(
            missing_tool_in(&["cc", "m68k-amigaos-gcc"], &empty),
            Some("cc".to_owned())
        );
    }
}
