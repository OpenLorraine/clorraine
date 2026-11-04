//! Environment checks for the `clorraine` developer driver.
//!
//! The driver is a tool of the pinned SDK container: Amiga and freestanding
//! commands need the cross C compiler and the `vamos` runner that the
//! container image installs. Two checks keep the failure mode clear:
//!
//! - [`sdk_present`] proves the process is inside the SDK image by looking for
//!   a marker file the image always ships. Outside the image, an Amiga or
//!   freestanding command stops with guidance; the host profile and `emit`
//!   continue because they use only the host compiler.
//! - [`missing_tool_in`] names the first required tool absent from `PATH`, so
//!   a broken image reports the exact tool instead of a generic spawn failure.
//!
//! Both take their inputs as parameters (a marker path, a tool list) so unit
//! tests can exercise them without touching the host environment.

use std::path::{Path, PathBuf};

/// The file the SDK image always contains; its presence marks the container.
pub const SDK_MARKER: &str = "/opt/amiga/BUILD-PROVENANCE.txt";

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
fn is_executable_file(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .map(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

/// Non-Unix fallback: a regular file stands in for an executable one.
#[cfg(not(unix))]
fn is_executable_file(path: &Path) -> bool {
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
