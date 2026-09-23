//! Artifact paths for the `clorraine` developer driver.
//!
//! A build lays its files out under `build/clorraine/<stem>/<profile>/`, where
//! `<stem>` is the first source file's file stem. The paths are relative to the
//! process working directory, so a developer who runs the driver from the
//! repository root finds the artifacts under `build/` there.
//!
//! The program directory is removed and recreated at the start of each build,
//! so a stale binary from an earlier run can never be mistaken for a fresh one.

use std::path::{Path, PathBuf};

use crate::profile::Profile;

/// The root of the driver's build output, relative to the working directory.
pub fn root() -> PathBuf {
    PathBuf::from("build").join("clorraine")
}

/// The directory holding one program's artifacts for one profile.
pub fn program_dir(profile: Profile, stem: &str) -> PathBuf {
    root().join(stem).join(profile.name())
}

/// The emitted C translation unit path.
pub fn c_path(profile: Profile, stem: &str) -> PathBuf {
    program_dir(profile, stem).join(format!("{stem}.c"))
}

/// The built program path.
pub fn bin_path(profile: Profile, stem: &str) -> PathBuf {
    program_dir(profile, stem).join(stem)
}

/// The captured stdout path used by `--capture`.
pub fn out_path(profile: Profile, stem: &str) -> PathBuf {
    program_dir(profile, stem).join("out.txt")
}

/// Removes any previous program directory and creates it empty.
///
/// Recreating the directory is what makes repeated builds deterministic: the
/// old binary and captured output are gone before the new build starts.
pub fn reset_program_dir(profile: Profile, stem: &str) -> std::io::Result<PathBuf> {
    let dir = program_dir(profile, stem);
    if dir.exists() {
        std::fs::remove_dir_all(&dir)?;
    }
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// The file stem of the first source file, if there is one.
///
/// The first file's stem names the whole program's artifact directory, so all
/// files of a multi-file program land together.
pub fn program_stem(files: &[String]) -> Option<String> {
    let first = files.first()?;
    let stem = Path::new(first).file_stem()?.to_string_lossy().into_owned();
    if stem.is_empty() {
        None
    } else {
        Some(stem)
    }
}
