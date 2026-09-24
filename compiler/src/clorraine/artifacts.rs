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
use std::sync::atomic::{AtomicUsize, Ordering};

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

/// The host-run artifact path of the freestanding profile.
///
/// The freestanding build produces the m68k artifact the target would run. The
/// `run` command separately compiles the same translation unit for the host
/// with the no-libc runtime, and names it `<stem>.host` so it sits beside the
/// m68k artifact without colliding with it.
pub fn host_bin_path(profile: Profile, stem: &str) -> PathBuf {
    program_dir(profile, stem).join(format!("{stem}.host"))
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

/// Creates a fresh scratch C file and returns its directory and path.
///
/// `emit` writes the translation unit to a temporary file before printing or
/// copying it. That file must not live at a predictable path: a name another
/// process pre-planted (for example as a symbolic link) would otherwise be
/// written through. The file is therefore created inside a new directory under
/// the gitignored build tree, with a name that has not existed before, and
/// opened with `create_new` so an existing entry is never followed. The caller
/// removes the returned directory once it has read the C.
pub fn create_scratch_c() -> std::io::Result<(PathBuf, PathBuf)> {
    let dir = create_emit_dir()?;
    match reserve_scratch_file(&dir) {
        Ok(file) => Ok((dir, file)),
        Err(error) => {
            // Do not leave a half-made scratch directory behind.
            let _ = std::fs::remove_dir_all(&dir);
            Err(error)
        }
    }
}

/// Creates a unique, owner-only directory for one `emit` scratch file.
///
/// The name combines this process id with a counter; a collision (an existing
/// entry, including a pre-planted one) makes the loop try the next value.
fn create_emit_dir() -> std::io::Result<PathBuf> {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let base = root();
    std::fs::create_dir_all(&base)?;
    loop {
        let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir = base.join(format!(".emit-{}-{sequence}", std::process::id()));
        match create_private_dir(&dir) {
            Ok(()) => return Ok(dir),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
}

/// Creates `dir` with owner-only permissions where the platform supports it.
#[cfg(unix)]
fn create_private_dir(dir: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::DirBuilderExt;
    std::fs::DirBuilder::new().mode(0o700).create(dir)
}

/// Creates `dir` with the platform's default permissions off Unix.
#[cfg(not(unix))]
fn create_private_dir(dir: &Path) -> std::io::Result<()> {
    std::fs::create_dir(dir)
}

/// Opens a new file inside `dir` and returns its path.
///
/// `create_new` refuses to open an existing path, so neither a regular file
/// nor a symbolic link can be written through. A collision tries the next
/// name.
fn reserve_scratch_file(dir: &Path) -> std::io::Result<PathBuf> {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    loop {
        let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = dir.join(format!("emitted-{sequence}.c"));
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(_) => return Ok(path),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
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
