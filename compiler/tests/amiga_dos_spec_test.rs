//! DOS standard-library specification tests.
//!
//! Bundle 5 extends `std.amiga.dos` with file seek/position/length, locks,
//! path information, directory and file mutation, current/parent directory,
//! and volume information. These cases pin the public surface on both
//! hosted-Amiga profiles, prove the host profile rejects it, and cross-build
//! the emitted C with the 1.3 and 3.1 runtimes.
//!
//! The red phase is the unresolved-export compile failure while the wrappers
//! are still absent.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Temporary directory that removes itself when the test finishes.
struct ScratchDir {
    path: PathBuf,
}

impl ScratchDir {
    /// Creates a unique directory for one test process and tag.
    fn new(tag: &str) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "clc-dos-{}-{tag}-{sequence}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path)
            .unwrap_or_else(|err| panic!("cannot create {}: {err}", path.display()));
        Self { path }
    }

    /// Path of the scratch directory.
    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// Runs the built compiler with the given arguments.
fn run_clc(args: &[String]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_clc"))
        .args(args)
        .output()
        .expect("failed to spawn the compiler binary")
}

/// True when a tool is available on PATH.
fn tool_available(tool: &str) -> bool {
    Command::new(tool).arg("--version").output().is_ok()
}

/// Writes `source` to a scratch file and compiles it for `profile`.
fn compile_source(tag: &str, profile: &str, source: &str) -> (Output, bool) {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source).expect("write the snippet");
    let output_path = scratch.path().join("out.c");
    let output = run_clc(&[
        "--profile".to_owned(),
        profile.to_owned(),
        "-o".to_owned(),
        output_path.display().to_string(),
        source_path.display().to_string(),
    ]);
    let emitted = output_path.exists();
    (output, emitted)
}

/// The full DOS surface, exercised in one compile-only program.
const FULL_SURFACE: &str = r#"module tests.dos.full;

import {
    File, openFile, createFile, readFile, writeBytes, MODE_OLDFILE
} from std.io;
import {
    SEEK_SET, seekFile, filePosition, fileLength,
    Lock, lockPath, unlockPath,
    PathInfo, pathInfo, pathInfoIsDir, pathInfoSize, pathInfoName,
    VolumeInfo, volumeInfo, volumeName, volumeFreeBytes, volumeUsedBytes,
    makeDir, deleteFile, renameFile, setCurrentDir, currentDirName,
    parentDirName
} from std.amiga.dos;
import { String, stringOf, toCString } from std.text;
import { slice } from std.mem;
import { println } from std.io;

func main(): i32 {
    let path: String = stringOf("DF0:probe");
    let file: File? = createFile(toCString(path));
    if (file == null) {
        return 1;
    }
    let data: u8[4] = [1, 2, 3, 4];
    let written: i32 = writeBytes(file, slice(data, 0, 4));
    let length: i32 = fileLength(file);
    let moved: bool = seekFile(file, 1, SEEK_SET);
    let position: i32 = filePosition(file);
    let one: u8[1] = [; 0];
    let got: i32 = readFile(file, borrow slice(one, 0, 1));

    let lock: Lock? = lockPath(toCString(stringOf("DF0:")));
    if (lock != null) {
        unlockPath(lock);
    }

    let info: PathInfo? = pathInfo(toCString(path));
    if (info != null) {
        let isDir: bool = pathInfoIsDir(info);
        let size: i32 = pathInfoSize(info);
        let name: u8[32] = [; 0];
        let nameLen: i32 = pathInfoName(info, borrow slice(name, 0, 32));
    }

    let volume: VolumeInfo? = volumeInfo(toCString(stringOf("DF0:")));
    if (volume != null) {
        let name: u8[32] = [; 0];
        let nameLen: i32 = volumeName(volume, borrow slice(name, 0, 32));
        let free: i32 = volumeFreeBytes(volume);
        let used: i32 = volumeUsedBytes(volume);
    }

    let made: bool = makeDir(toCString(stringOf("DF0:dir")));
    let renamed: bool = renameFile(toCString(stringOf("DF0:a")), toCString(stringOf("DF0:b")));
    let deleted: bool = deleteFile(toCString(stringOf("DF0:b")));
    let changed: bool = setCurrentDir(toCString(stringOf("DF0:")));
    let cwd: u8[64] = [; 0];
    let cwdLen: i32 = currentDirName(borrow slice(cwd, 0, 64));
    let parent: u8[64] = [; 0];
    let parentLen: i32 = parentDirName(borrow slice(parent, 0, 64));

    let reopened: File? = openFile(toCString(path), MODE_OLDFILE);
    if (reopened == null) {
        return 2;
    }

    println($"dos {written} {length} {moved} {position} {got} {made} {renamed} {deleted} {changed} {cwdLen} {parentLen}");
    return 0;
}
"#;

/// Both hosted-Amiga profiles accept the full surface.
#[test]
fn the_full_dos_surface_compiles_on_both_hosted_profiles() {
    for profile in ["hosted-amiga", "hosted-amiga-13"] {
        let (output, emitted) = compile_source(profile, profile, FULL_SURFACE);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{profile}: the DOS surface must compile; stderr:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(emitted, "{profile}: the DOS surface must emit C");
    }
}

/// Short name buffers are allowed at the call site; the shim bounds the copy.
#[test]
fn short_name_buffers_compile() {
    let source = r#"module tests.dos.short;

import { PathInfo, pathInfo, pathInfoName } from std.amiga.dos;
import { String, stringOf, toCString } from std.text;
import { slice } from std.mem;
import { println } from std.io;

func main(): i32 {
    let info: PathInfo? = pathInfo(toCString(stringOf("DF0:")));
    if (info == null) {
        return 1;
    }
    let one: u8[1] = [; 0];
    let copied: i32 = pathInfoName(info, borrow slice(one, 0, 1));
    let none: i32 = pathInfoName(info, borrow slice(one, 0, 0));
    println($"short {copied} {none}");
    return 0;
}
"#;
    let (output, emitted) = compile_source("short", "hosted-amiga-13", source);
    assert_eq!(
        output.status.code(),
        Some(0),
        "short buffers must compile; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(emitted);
}

/// The host profile rejects the Amiga-only DOS module with E11200.
#[test]
fn the_host_profile_rejects_the_dos_module() {
    let source = r#"module tests.host.dos;

import { delayTicks } from std.amiga.dos;

func main(): i32 {
    delayTicks(u32(1));
    return 0;
}
"#;
    let (output, emitted) = compile_source("host-dos", "host", source);
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("E11200"), "expected E11200, got:\n{stderr}");
    assert!(!emitted, "a rejected program must not emit C");
}

/// The full surface cross-builds with the Kickstart 1.3 runtime.
#[test]
fn the_full_dos_surface_cross_builds_with_nix13() {
    if !tool_available("m68k-amigaos-gcc") {
        eprintln!("skip: m68k-amigaos-gcc is not on PATH");
        return;
    }
    cross_build("nix13", "hosted-amiga-13", FULL_SURFACE, "-mcrt=nix13");
}

/// The full surface cross-builds with the 3.1 runtime.
#[test]
fn the_full_dos_surface_cross_builds_with_nix20() {
    if !tool_available("m68k-amigaos-gcc") {
        eprintln!("skip: m68k-amigaos-gcc is not on PATH");
        return;
    }
    cross_build("nix20", "hosted-amiga", FULL_SURFACE, "-mcrt=nix20");
}

/// Emits `source` and builds it with the Amiga compiler and runtime.
fn cross_build(tag: &str, profile: &str, source: &str, runtime: &str) {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source).expect("write the snippet");
    let c_path = scratch.path().join("out.c");
    let emit = run_clc(&[
        "--profile".to_owned(),
        profile.to_owned(),
        "-o".to_owned(),
        c_path.display().to_string(),
        source_path.display().to_string(),
    ]);
    assert_eq!(
        emit.status.code(),
        Some(0),
        "{tag}: the DOS surface must emit C; stderr:\n{}",
        String::from_utf8_lossy(&emit.stderr)
    );
    let binary = scratch.path().join("dos");
    let build = Command::new("m68k-amigaos-gcc")
        .args([
            "-std=c99",
            runtime,
            "-m68000",
            "-fwrapv",
            "-O0",
            "-g",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-o",
        ])
        .arg(&binary)
        .arg(&c_path)
        .args(["-lamiga", "-lm"])
        .output()
        .expect("run m68k-amigaos-gcc");
    assert!(
        build.status.success(),
        "{tag}: the DOS surface must build; stderr:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
}
