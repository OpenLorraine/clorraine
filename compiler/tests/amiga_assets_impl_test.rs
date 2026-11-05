//! DOS and asset standard-library implementation tests.
//!
//! These cases inspect the C the compiler emits and pin internal guarantees
//! that the public specification cases do not need to name: the parser is pure
//! (no Amiga modules or `ffi`), application calls lower to the standard-library
//! wrappers, and a failed decode leaves the caller's buffer untouched.

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
            "clc-assets-impl-{}-{tag}-{sequence}",
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

/// Compiles `source` for `profile` and returns the emitted C.
fn emit(tag: &str, profile: &str, source: &str) -> String {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source).expect("write the snippet");
    let output_path = scratch.path().join("out.c");
    let run = run_clc(&[
        "--profile".to_owned(),
        profile.to_owned(),
        "-o".to_owned(),
        output_path.display().to_string(),
        source_path.display().to_string(),
    ]);
    assert_eq!(
        run.status.code(),
        Some(0),
        "{tag}: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&run.stderr)
    );
    std::fs::read_to_string(&output_path).expect("read the emitted C")
}

/// Asserts `haystack` contains `needle`.
fn assert_contains(tag: &str, haystack: &str, needle: &str) {
    assert!(
        haystack.contains(needle),
        "{tag}: emitted C does not contain {needle:?}"
    );
}

/// Compiles a host program, builds the emitted C, and runs it.
fn build_and_run_host(tag: &str, source: &str) -> Output {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source).expect("write the snippet");
    let c_path = scratch.path().join("out.c");
    let compile = run_clc(&[
        "--profile".to_owned(),
        "host".to_owned(),
        "-o".to_owned(),
        c_path.display().to_string(),
        source_path.display().to_string(),
    ]);
    assert_eq!(
        compile.status.code(),
        Some(0),
        "{tag}: the host profile rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&compile.stderr)
    );
    let program = scratch.path().join("prog");
    let built = Command::new("cc")
        .args([
            "-std=c99",
            "-O0",
            "-g",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-o",
        ])
        .arg(&program)
        .arg(&c_path)
        .output()
        .expect("failed to spawn the host C compiler");
    assert!(
        built.status.success(),
        "{tag}: the host C compiler rejected the emitted C; stderr:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
    Command::new(&program)
        .output()
        .unwrap_or_else(|err| panic!("{tag}: cannot run the program: {err}"))
}

/// The parser uses no Amiga module and no FFI declaration.
#[test]
fn the_parser_module_is_pure() {
    let source = include_str!("../../stdlib/std.iff.clo");
    assert!(
        !source.contains("std.amiga."),
        "std.iff must not import an Amiga module"
    );
    assert!(
        !source.contains("ffi "),
        "std.iff must not declare an FFI target"
    );
}

/// An application call lowers to the `std.iff` wrapper, not inline code.
#[test]
fn the_parser_call_lowers_to_the_std_wrapper() {
    let source = r#"module impl.assets.parser;

import { IlbmInfo, ilbmInfo } from std.iff;
import { slice } from std.mem;

func main(): i32 {
    let data: u8[12] = [70, 79, 82, 77, 0, 0, 0, 4, 73, 76, 66, 77];
    let info: IlbmInfo = ilbmInfo(slice(data, 0, 12));
    if (info.ok) {
        return 1;
    }
    return 0;
}
"#;
    let emitted = emit("parser-wrapper", "host", source);
    assert_contains("parser-wrapper", &emitted, "cl_std_iff_ilbmInfo(");
    assert_contains("parser-wrapper", &emitted, "ClIlbmInfo");
}

/// A decode rejected by the value checks returns before it clears or writes
/// the destination. A malformed body may clear or partially write and still
/// return false; the decoder guarantees no out-of-bounds access, not atomicity.
#[test]
fn a_validation_failure_leaves_the_destination_untouched() {
    let source = r#"module impl.assets.untouched;

import { IlbmInfo, ilbmDecode } from std.iff;
import { slice } from std.mem;
import { println } from std.io;

func main(): i32 {
    let data: u8[2] = [0, 0];
    let pixels: u8[4] = [170, 170, 170, 170];
    let failed: IlbmInfo = IlbmInfo { ok: false, width: 0, height: 0, depth: 0, compression: 0, colors: 0, cmap_start: 0, body_start: 0, body_len: 0 };
    if (ilbmDecode(slice(data, 0, 2), failed, borrow slice(pixels, 0, 4))) {
        return 1;
    }
    let shortInfo: IlbmInfo = IlbmInfo { ok: true, width: 2, height: 2, depth: 2, compression: 0, colors: 0, cmap_start: 0, body_start: 0, body_len: 1 };
    if (ilbmDecode(slice(data, 0, 2), shortInfo, borrow slice(pixels, 0, 1))) {
        return 2;
    }
    println($"untouched {pixels[0]} {pixels[1]} {pixels[2]} {pixels[3]}");
    return 0;
}
"#;
    let output = build_and_run_host("untouched", source);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "untouched 170 170 170 170\n"
    );
}

/// The emitted DOS wrappers call the shim and use the ABI handle spellings.
#[test]
fn the_dos_surface_lowers_to_the_expected_prototypes() {
    let source = r#"module impl.assets.dos;

import { File, createFile } from std.io;
import {
    Lock, lockPath, unlockPath, PathInfo, pathInfo, VolumeInfo, volumeInfo,
    makeDir
} from std.amiga.dos;
import { String, stringOf, toCString } from std.text;

func main(): i32 {
    let lock: Lock? = lockPath(toCString(stringOf("DF0:")));
    if (lock != null) {
        unlockPath(lock);
    }
    let info: PathInfo? = pathInfo(toCString(stringOf("DF0:")));
    let volume: VolumeInfo? = volumeInfo(toCString(stringOf("DF0:")));
    let made: bool = makeDir(toCString(stringOf("DF0:dir")));
    if (made) {
        return 1;
    }
    if (info == null || volume == null) {
        return 2;
    }
    return 0;
}
"#;
    let emitted = emit("dos-wrapper", "hosted-amiga-13", source);
    for needle in [
        "cl_std_amiga_dos_lockPath(",
        "cl_std_amiga_dos_pathInfo(",
        "cl_std_amiga_dos_volumeInfo(",
        "cl_std_amiga_dos_makeDir(",
        "BPTR cl_dos_lock(",
        "ClPathInfo cl_dos_pathinfo_new(",
        "ClVolumeInfo cl_dos_volumeinfo_new(",
        "cl_dos_pathinfo_free(",
        "cl_dos_volumeinfo_free(",
    ] {
        assert_contains("dos-wrapper", &emitted, needle);
    }
}

/// Owned DOS handles drop in reverse declaration order.
#[test]
fn dos_handles_drop_in_reverse_declaration_order() {
    let source = r#"module impl.assets.dos_drop;

import {
    Lock, lockPath, PathInfo, pathInfo, VolumeInfo, volumeInfo
} from std.amiga.dos;
import { String, stringOf, toCString } from std.text;

func main(): i32 {
    let lock: Lock? = lockPath(toCString(stringOf("DF0:")));
    let info: PathInfo? = pathInfo(toCString(stringOf("DF0:")));
    let volume: VolumeInfo? = volumeInfo(toCString(stringOf("DF0:")));
    if (lock == null || info == null || volume == null) {
        return 1;
    }
    return 0;
}
"#;
    let emitted = emit("dos-drop", "hosted-amiga-13", source);
    let start = emitted
        .find("static i32 cl_impl_assets_dos_drop_main(void)")
        .expect("main function");
    let body = &emitted[start..];
    let cleanup = body.find("__cl_c1:").expect("main cleanup ladder");
    let cleanup = &body[cleanup..];
    let cleanup_end = cleanup.find("goto __cl_c2;").unwrap_or(cleanup.len());
    let cleanup = &cleanup[..cleanup_end];
    let volume_free = cleanup.find("cl_dos_volumeinfo_free(").expect("volume drop");
    let pathinfo_free = cleanup.find("cl_dos_pathinfo_free(").expect("path drop");
    let lock_free = cleanup.find("cl_dos_unlock(").expect("lock drop");
    assert!(
        volume_free < pathinfo_free && pathinfo_free < lock_free,
        "drops must run volume, path, lock: {cleanup}"
    );
}

/// The C name copy is bounded by the caller buffer length.
#[test]
fn the_dos_name_copy_is_bounded() {
    let shim = include_str!("../runtime/shim_hosted_amiga.c");
    assert!(shim.contains("static LONG cl_dos_copy_name("));
    assert!(shim.contains("if (length > capacity)"));
    assert!(shim.contains("return cl_dos_copy_name(buffer, capacity,"));
    let dos = include_str!("../../stdlib/std.amiga.dos.clo");
    assert!(dos.contains("raw u8(buf.ptr), u32(len(buf))"));
}

/// True when a tool is available on PATH.
fn tool_available(tool: &str) -> bool {
    Command::new(tool).arg("--version").output().is_ok()
}

/// The fixture generator reproduces the committed bytes exactly.
#[test]
fn the_fixture_generator_reproduces_the_committed_bytes() {
    if !tool_available("python3") {
        eprintln!("skip: python3 is not on PATH");
        return;
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root");
    let script = root.join("tools/amiga-sdk/make_ilbm_fixture.py");
    let committed = root.join("examples/amiga/assets/clorraine.ilbm");
    let scratch = ScratchDir::new("fixture-gen");
    let generated = scratch.path().join("generated.ilbm");
    let run = Command::new("python3")
        .arg(&script)
        .arg("--output")
        .arg(&generated)
        .current_dir(root)
        .output()
        .expect("run the fixture generator");
    assert!(
        run.status.success(),
        "the fixture generator failed; stderr:\n{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let expected = std::fs::read(&committed).expect("read the committed fixture");
    let actual = std::fs::read(&generated).expect("read the generated fixture");
    assert_eq!(actual, expected, "the generator must reproduce the fixture");
}

/// The loader lowers to the DOS, parser, and bitmap primitives.
#[test]
fn the_loader_lowers_to_the_expected_primitives() {
    let source = r#"module impl.assets.loader;

import { loadIlbm, IlbmImage } from std.amiga.ilbm;
import { String, stringOf, toCString } from std.text;
import { slice } from std.mem;

func main(): i32 {
    let palette: u8[48] = [; 0];
    let image: IlbmImage = loadIlbm(
        toCString(stringOf("DF0:image.ilbm")), borrow slice(palette, 0, 48));
    if (!image.ok) {
        return 1;
    }
    return 0;
}
"#;
    let emitted = emit("loader-wrapper", "hosted-amiga-13", source);
    for needle in [
        "cl_std_amiga_ilbm_loadIlbm(",
        "cl_std_amiga_dos_fileLength(",
        "cl_std_iff_ilbmInfo(",
        "cl_std_iff_ilbmDecode(",
        "cl_video_bitmap_indices(",
        "ClIlbmImage",
    ] {
        assert_contains("loader-wrapper", &emitted, needle);
    }
}

/// The owned image drops its bitmap behind a null guard.
#[test]
fn the_loaded_image_drops_its_bitmap() {
    let source = r#"module impl.assets.loader_drop;

import { loadIlbm, IlbmImage } from std.amiga.ilbm;
import { String, stringOf, toCString } from std.text;
import { slice } from std.mem;

func main(): i32 {
    let palette: u8[48] = [; 0];
    let image: IlbmImage = loadIlbm(
        toCString(stringOf("DF0:image.ilbm")), borrow slice(palette, 0, 48));
    if (!image.ok) {
        return 1;
    }
    return 0;
}
"#;
    let emitted = emit("loader-drop", "hosted-amiga-13", source);
    assert_contains("loader-drop", &emitted, "cl_video_bitmap_free(");
    assert_contains("loader-drop", &emitted, "ClIlbmImage");
}
