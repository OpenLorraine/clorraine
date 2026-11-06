//! Specification tests for the Picasso96 RTG surface.
//!
//! The host cases always run; the strict cross-build skips itself when the
//! m68k toolchain is absent. These tests derive from the RTG requirements and
//! the published appendix, not from the shim.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

struct ScratchDir {
    path: PathBuf,
}

impl ScratchDir {
    fn new(tag: &str) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "clc-rtg-{}-{tag}-{sequence}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path)
            .unwrap_or_else(|error| panic!("cannot create {}: {error}", path.display()));
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

fn write_fixture(scratch: &ScratchDir, name: &str, source: &str) -> PathBuf {
    let path = scratch.path().join(name);
    std::fs::write(&path, source)
        .unwrap_or_else(|error| panic!("cannot write {name}: {error}"));
    path
}

fn run_clc(cwd: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_clc"))
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("failed to spawn the compiler binary")
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn assert_exit(tag: &str, output: &Output, expected: i32) {
    assert_eq!(
        output.status.code(),
        Some(expected),
        "{tag}: expected exit {expected}; stderr:\n{}",
        stderr(output)
    );
}

fn tool_available(tool: &str) -> bool {
    Command::new(tool).arg("--version").output().is_ok()
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn cross_build(tag: &str, profile: &str, crt: &str, source: &str) {
    if !tool_available("m68k-amigaos-gcc") {
        eprintln!("skip: m68k-amigaos-gcc is not on PATH");
        return;
    }
    let scratch = ScratchDir::new(tag);
    let source_path = write_fixture(&scratch, "main.clo", source);
    let c_path = scratch.path().join("out.c");
    let emit = run_clc(
        scratch.path(),
        &[
            "--profile",
            profile,
            "-o",
            c_path.to_str().expect("the output path is UTF-8"),
            source_path.to_str().expect("the source path is UTF-8"),
        ],
    );
    assert_exit(tag, &emit, 0);
    let binary = scratch.path().join("probe");
    let build = Command::new("m68k-amigaos-gcc")
        .args([
            "-std=c99",
            &format!("-mcrt={crt}"),
            "-m68000",
            "-fwrapv",
            "-O0",
            "-g",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-o",
            binary.to_str().expect("the output path is UTF-8"),
            c_path.to_str().expect("the C path is UTF-8"),
            "-lamiga",
            "-lm",
        ])
        .output()
        .expect("run m68k-amigaos-gcc");
    assert!(
        build.status.success(),
        "{tag}: the surface must build with {crt}; stderr:\n{}",
        stderr(&build)
    );
}

fn appendix_section(name: &str) -> String {
    let appendix = std::fs::read_to_string(repo_root().join("spec/appendix-stdlib.md"))
        .expect("the stdlib appendix is readable");
    let start = appendix
        .find(name)
        .unwrap_or_else(|| panic!("the appendix has {name}"));
    let section = &appendix[start..];
    let end = section.find("\n### ").unwrap_or(section.len());
    section[..end].to_owned()
}

const RTG_SOURCE: &str = r#"module tests.rtg.surface;

import { RtgScreen, RtgBitmap, openRtgScreen, rtgScreenWidth,
         rtgScreenHeight, rtgScreenDepth, rtgScreenFormat, rtgScreenMode,
         allocRtgBitmap, rtgBitmapWidth, rtgBitmapHeight, rtgBitmapDepth,
         rtgBitmapFormat, rtgBitmapBytesPerRow, rtgScreenWritePixel,
         rtgScreenReadPixel, rtgScreenFillRect, rtgBitmapWritePixel,
         rtgBitmapReadPixel, rtgBitmapFillRect, rtgScreenBlitBitmap,
         rtgScreenReadPixels, rtgBitmapReadPixels } from std.amiga.picasso96;
import { Buffer, bufferNew, bufferBytes } from std.mem;

func main(): i32 {
    let screenCreated: RtgScreen? = openRtgScreen(640, 480, 32, "rtg");
    if (screenCreated == null) { return 1; }
    let screen: RtgScreen = screenCreated;
    if (rtgScreenWidth(screen) != 640) { return 2; }
    if (rtgScreenHeight(screen) != 480) { return 3; }
    if (rtgScreenDepth(screen) != 32) { return 4; }
    if (rtgScreenMode(screen) == 0) { return 5; }
    if (rtgScreenFormat(screen) == 0) { return 6; }

    if (!rtgScreenWritePixel(screen, 0, 0, 0x00FF0000)) { return 7; }
    if (rtgScreenReadPixel(screen, 0, 0) != 0x00FF0000) { return 8; }
    if (!rtgScreenFillRect(screen, 10, 10, 20, 20, 0x0000FF00)) { return 9; }

    let bitmapCreated: RtgBitmap? = allocRtgBitmap(32, 32, 32);
    if (bitmapCreated == null) { return 10; }
    let bitmap: RtgBitmap = bitmapCreated;
    if (rtgBitmapWidth(bitmap) != 32) { return 11; }
    if (rtgBitmapHeight(bitmap) != 32) { return 12; }
    if (rtgBitmapDepth(bitmap) != 32) { return 13; }
    if (rtgBitmapFormat(bitmap) == 0) { return 14; }
    if (rtgBitmapBytesPerRow(bitmap) <= 0) { return 15; }
    if (!rtgBitmapFillRect(bitmap, 0, 0, 31, 31, 0x000000FF)) { return 16; }
    if (!rtgBitmapWritePixel(bitmap, 0, 0, 0x00FFFF00)) { return 17; }
    if (rtgBitmapReadPixel(bitmap, 0, 0) != 0x00FFFF00) { return 18; }
    if (!rtgScreenBlitBitmap(screen, bitmap, 10, 10, 0, 0, 32, 32)) { return 19; }

    let screenBuffer: Buffer? = bufferNew(usize(64 * 64 * 3));
    if (screenBuffer == null) { return 20; }
    let screenBytes: Buffer = screenBuffer;
    let screenView: u8[] = bufferBytes(screenBytes);
    if (rtgScreenReadPixels(screen, 0, 0, 64, 64, screenView) != 64 * 64 * 3) {
        return 21;
    }

    let bitmapBuffer: Buffer? = bufferNew(usize(32 * 32 * 3));
    if (bitmapBuffer == null) { return 22; }
    let bitmapBytes: Buffer = bitmapBuffer;
    let bitmapView: u8[] = bufferBytes(bitmapBytes);
    if (rtgBitmapReadPixels(bitmap, 0, 0, 32, 32, bitmapView) != 32 * 32 * 3) {
        return 23;
    }
    return 0;
}
"#;

const HEADER_NAME_SOURCE: &str = r#"module tests.rtg.names;

enum RGBFB : u8 {
    NONE = 1,
    CLUT = 2,
}

enum P96BMA : u8 {
    WIDTH = 3,
}

func main(): i32 {
    return i32(RGBFB.NONE) + i32(RGBFB.CLUT) + i32(P96BMA.WIDTH);
}
"#;

const DROP_SOURCE: &str = r#"module tests.rtg.drop;

import { RtgScreen, RtgBitmap, openRtgScreen, allocRtgBitmap }
    from std.amiga.picasso96;

func main(): i32 {
    let screenCreated: RtgScreen? = openRtgScreen(64, 64, 32, "rtg");
    if (screenCreated == null) { return 1; }
    let screen: RtgScreen = screenCreated;
    let bitmapCreated: RtgBitmap? = allocRtgBitmap(8, 8, 32);
    if (bitmapCreated == null) { return 2; }
    let bitmap: RtgBitmap = bitmapCreated;
    return 0;
}
"#;

#[test]
fn the_rtg_module_is_registered_with_all_its_exports() {
    let module = clc::stdlib::lookup("std.amiga.picasso96")
        .expect("the RTG module is registered");
    assert!(module.exports.contains(&"RtgScreen"));
    assert!(module.exports.contains(&"RtgBitmap"));
    assert!(module.exports.contains(&"rtgBoards"));
    assert!(module.exports.contains(&"rtgScreenReadPixels"));
    assert!(clc::stdlib::source("std.amiga.picasso96").is_some());
}

#[test]
fn the_rtg_surface_is_rejected_on_unsupported_profiles() {
    for profile in ["host", "hosted-amiga-13", "freestanding"] {
        let scratch = ScratchDir::new(&format!("reject-{profile}"));
        let source_path = write_fixture(&scratch, "main.clo", RTG_SOURCE);
        let output_path = scratch.path().join("out.c");
        let output = run_clc(
            scratch.path(),
            &[
                "--profile",
                profile,
                "-o",
                output_path.to_str().expect("the output path is UTF-8"),
                source_path.to_str().expect("the source path is UTF-8"),
            ],
        );
        assert_exit(&format!("rtg-reject-{profile}"), &output, 1);
        assert!(
            stderr(&output).contains("E11200"),
            "expected E11200 on {profile}, got:\n{}",
            stderr(&output)
        );
        assert!(
            !output_path.exists(),
            "a rejected {profile} program must not emit C"
        );
    }
}

#[test]
fn the_rtg_surface_builds_on_hosted_amiga() {
    let scratch = ScratchDir::new("surface");
    let source_path = write_fixture(&scratch, "main.clo", RTG_SOURCE);
    let output_path = scratch.path().join("out.c");
    let output = run_clc(
        scratch.path(),
        &[
            "--profile",
            "hosted-amiga",
            "-o",
            output_path.to_str().expect("the output path is UTF-8"),
            source_path.to_str().expect("the source path is UTF-8"),
        ],
    );
    assert_exit("rtg-surface", &output, 0);
    assert!(
        stderr(&output).is_empty(),
        "the RTG surface must compile with zero diagnostics; stderr:\n{}",
        stderr(&output)
    );
    let c = std::fs::read_to_string(&output_path).expect("the emitted C is readable");
    for call in [
        "cl_rtg_boards",
        "cl_rtg_mode",
        "cl_rtg_open_screen",
        "cl_rtg_alloc_bitmap",
        "cl_rtg_screen_write_pixel",
        "cl_rtg_screen_read_pixels",
        "cl_rtg_bitmap_read_pixels",
        "cl_rtg_screen_blit_bitmap",
    ] {
        assert!(c.contains(call), "the emitted C is missing {call}");
    }
}

#[test]
fn the_rtg_surface_cross_builds_with_nix20() {
    cross_build("rtg-build20", "hosted-amiga", "nix20", RTG_SOURCE);
}

#[test]
fn the_rtg_drops_are_generated_once() {
    let scratch = ScratchDir::new("drop");
    let source_path = write_fixture(&scratch, "main.clo", DROP_SOURCE);
    let output_path = scratch.path().join("out.c");
    let output = run_clc(
        scratch.path(),
        &[
            "--profile",
            "hosted-amiga",
            "-o",
            output_path.to_str().expect("the output path is UTF-8"),
            source_path.to_str().expect("the source path is UTF-8"),
        ],
    );
    assert_exit("rtg-drop", &output, 0);
    let c = std::fs::read_to_string(&output_path).expect("the emitted C is readable");
    assert!(c.contains("cl_rtg_close_screen(screen)"));
    assert!(c.contains("cl_rtg_free_bitmap(bitmap)"));
}

#[test]
fn the_rtg_header_does_not_reserve_user_enum_names() {
    cross_build("rtg-header-names", "hosted-amiga", "nix20", HEADER_NAME_SOURCE);
}

#[test]
fn the_rtg_exports_match_the_stdlib_appendix() {
    let text = appendix_section("### 14.23");
    let module = clc::stdlib::lookup("std.amiga.picasso96")
        .expect("the RTG module is registered");
    for name in module.exports {
        assert!(
            text.split(|c: char| !c.is_alphanumeric() && c != '_')
                .any(|word| word == *name),
            "the appendix section 14.23 is missing {name}"
        );
    }
}
