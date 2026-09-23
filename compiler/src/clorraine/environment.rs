//! Environment checks for the `clorraine` developer driver.
//!
//! The driver is a tool of the pinned SDK container: it needs the cross C
//! compiler and the `vamos` runner that the container image installs. Two
//! checks keep the failure mode clear:
//!
//! - [`sdk_present`] proves the process is inside the SDK image by looking for
//!   a marker file the image always ships. Outside the image the driver can
//!   only advise the developer how to get back in.
//! - [`missing_tool`] names the first required tool absent from `PATH`, so a
//!   broken image reports the exact tool instead of a generic spawn failure.
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

/// The first required tool that is not on `PATH`, if any.
///
/// The search is a plain filename lookup in the `PATH` directories. The
/// caller reports the returned name and exits 1.
pub fn missing_tool(tools: &[&str]) -> Option<String> {
    let path = std::env::var_os("PATH").unwrap_or_default();
    missing_tool_in(tools, &path)
}

/// Like [`missing_tool`] but searches the given `PATH` value.
///
/// Taking the `PATH` as an argument keeps the check testable: a test can pass
/// an empty or synthetic search path without mutating the process environment.
pub fn missing_tool_in(tools: &[&str], path: &std::ffi::OsStr) -> Option<String> {
    let dirs: Vec<PathBuf> = std::env::split_paths(path).collect();
    tools
        .iter()
        .find(|tool| !tool_on_path(&dirs, tool))
        .map(|tool| (*tool).to_owned())
}

/// True when `tool` names a file in one of the search directories.
fn tool_on_path(dirs: &[PathBuf], tool: &str) -> bool {
    dirs.iter().any(|dir| dir.join(tool).is_file())
}

/// The guidance printed when the driver runs outside the SDK container.
pub fn sdk_guidance() -> String {
    "clorraine: this driver must run inside the pinned SDK container.\n\
     Run it via: make -C docker exec CMD=\"clorraine run FILE.clo\"\n\
     or open a shell with: make -C docker shell"
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

    #[test]
    fn a_tool_is_missing_unless_a_search_directory_contains_it() {
        let dir = std::env::temp_dir().join(format!("clorraine-env-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create the search directory");
        std::fs::write(dir.join("present-tool"), b"").expect("write the tool file");

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
