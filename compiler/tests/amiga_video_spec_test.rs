//! Native-video standard-library specification tests.
//!
//! Bundle 1 adds `std.amiga.video` (chip memory, bitmaps, drawing, blitter,
//! copper, display) and `std.math` (fixed-point helpers). These cases pin the
//! public contract before the modules and the C shim exist: the full surface
//! must compile on both hosted-Amiga profiles, the host profile must reject the
//! Amiga-only module, and the emitted C must cross-build with the 1.3 and 3.1
//! C runtimes under strict warnings.
//!
//! The red phase is the unresolved-module compile failure while the sources
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
            "clc-video-{}-{tag}-{sequence}",
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

/// True when a tool is available on PATH.
fn tool_available(tool: &str) -> bool {
    Command::new(tool).arg("--version").output().is_ok()
}

/// The full native-video surface, exercised in one program.
///
/// The program allocates chip memory, creates three bitmaps (including a
/// single-plane mask), draws with every primitive, performs a copy and a
/// masked copy, builds a copper list, opens and flips a display, reads plane
/// bits, calls the math helpers, and releases everything in the documented
/// order (display first, then the bitmaps).
const FULL_SURFACE: &str = r#"module tests.video.full;

import {
    ChipBuffer, Bitmap, Copper, Display,
    allocChip, freeChip, chipSize, chipRead8, chipWrite8,
    chipRead16, chipWrite16, chipRead32, chipWrite32,
    newBitmap, freeBitmap, bitmapWidth, bitmapHeight, bitmapDepth,
    bitmapBytesPerRow, bitmapClear, bitmapPlot, bitmapLine, bitmapFill,
    bitmapPlaneBit, bitmapSetPlaneBit, bitmapBlit, bitmapBlitMasked,
    bitmapChecksum, newCopper, copperWait, copperMove, copperColor,
    copperEnd, openDisplay, freeDisplay, displaySetColor, displaySetCopper,
    displayFlip, displayFailed, displayIndex, waitVBlank
} from std.amiga.video;
import { sin, cos, sqrt } from std.math;
import { println } from std.io;
import { toHex } from std.fmt;

func main(): i32 {
    let chip: ChipBuffer? = allocChip(64);
    if (chip == null) {
        return 1;
    }
    chipWrite8(chip, 0, 42);
    chipWrite16(chip, 2, 0x1234);
    chipWrite32(chip, 4, 0x12345678);
    let a: u8 = chipRead8(chip, 0);
    let b: u16 = chipRead16(chip, 2);
    let c: u32 = chipRead32(chip, 4);
    if (chipSize(chip) != 64) {
        return 2;
    }
    freeChip(chip);

    let front: Bitmap? = newBitmap(320, 256, 4);
    if (front == null) {
        return 3;
    }
    let back: Bitmap? = newBitmap(320, 256, 4);
    if (back == null) {
        return 4;
    }
    let mask: Bitmap? = newBitmap(64, 64, 1);
    if (mask == null) {
        return 5;
    }
    let sprite: Bitmap? = newBitmap(64, 64, 4);
    if (sprite == null) {
        return 9;
    }

    bitmapClear(front, 0);
    bitmapFill(front, 0, 0, 319, 255, 1);
    bitmapPlot(front, 10, 10, 2);
    bitmapLine(front, 0, 0, 319, 255, 3);
    bitmapSetPlaneBit(front, 0, 1, 1, true);
    let bit: bool = bitmapPlaneBit(front, 0, 1, 1);
    bitmapBlit(front, back, 0, 0, 0, 0, 320, 256);
    bitmapClear(sprite, 0);
    bitmapFill(sprite, 0, 0, 63, 63, 4);
    bitmapBlitMasked(front, sprite, mask, 0, 0, 16, 16, 64, 64);
    let sum: u32 = bitmapChecksum(front);
    let w: i32 = bitmapWidth(front);
    let h: i32 = bitmapHeight(front);
    let d: u8 = bitmapDepth(front);
    let bpr: u32 = bitmapBytesPerRow(front);

    let copper: Copper? = newCopper();
    if (copper == null) {
        return 6;
    }
    copperWait(copper, 0);
    copperColor(copper, 1, 255, 0, 0);
    copperMove(copper, 0x0180, 0x0F00);
    copperEnd(copper);

    let display: Display? = openDisplay(front, back);
    if (display == null) {
        return 7;
    }
    displaySetColor(display, 0, 0, 0, 0);
    displaySetColor(display, 1, 255, 0, 0);
    if (!displaySetCopper(display, copper)) {
        return 8;
    }
    displayFlip(display);
    if (displayFailed(display)) {
        return 10;
    }
    waitVBlank();
    displayFlip(display);

    let idx: u8 = displayIndex(display);
    let s: i32 = sin(64);
    let co: i32 = cos(0);
    let root: i32 = sqrt(16);
    println($"video {toHex(sum)} {w} {h} {i32(d)} {i32(bpr)} {i32(idx)} {s} {co} {root} {bit} {i32(a)} {i32(b)} {i32(c)}");
    freeDisplay(display);
    freeBitmap(mask);
    freeBitmap(sprite);
    freeBitmap(back);
    freeBitmap(front);
    return 0;
}
"#;

/// The full surface compiles for `hosted-amiga` with zero diagnostics.
#[test]
fn the_full_surface_compiles_for_hosted_amiga() {
    let (output, emitted) = compile_source("hosted", "hosted-amiga", FULL_SURFACE);
    assert_eq!(
        output.status.code(),
        Some(0),
        "the native-video surface must compile for hosted-amiga; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "the native-video surface must compile with zero diagnostics; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(emitted, "accepted source must emit C");
}

/// The full surface compiles for `hosted-amiga-13` with zero diagnostics.
#[test]
fn the_full_surface_compiles_for_the_a500_profile() {
    let (output, emitted) = compile_source("a500", "hosted-amiga-13", FULL_SURFACE);
    assert_eq!(
        output.status.code(),
        Some(0),
        "the native-video surface must compile for hosted-amiga-13; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "the native-video surface must compile with zero diagnostics; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(emitted, "accepted source must emit C");
}

/// The host profile rejects the Amiga-only module with `E11200`.
#[test]
fn the_host_profile_rejects_the_native_video_module() {
    let source = r#"module tests.video.host;

import { newBitmap } from std.amiga.video;

func main(): i32 {
    let bitmap = newBitmap(16, 16, 4);
    return 0;
}
"#;
    let (output, emitted) = compile_source("host-reject", "host", source);
    assert_eq!(
        output.status.code(),
        Some(1),
        "the host profile must reject std.amiga.video; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("E11200"),
        "expected E11200, got:\n{stderr}"
    );
    assert!(
        !emitted,
        "a rejected program must not write a translation unit"
    );
}

/// The full surface cross-builds with the Kickstart 1.3 runtime.
#[test]
fn the_full_surface_cross_builds_with_nix13() {
    if !tool_available("m68k-amigaos-gcc") {
        eprintln!("skip: m68k-amigaos-gcc is not on PATH");
        return;
    }
    let scratch = ScratchDir::new("build13");
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, FULL_SURFACE).expect("write the snippet");
    let c_path = scratch.path().join("out.c");
    let emit = run_clc(&[
        "--profile".to_owned(),
        "hosted-amiga-13".to_owned(),
        "-o".to_owned(),
        c_path.display().to_string(),
        source_path.display().to_string(),
    ]);
    assert_eq!(
        emit.status.code(),
        Some(0),
        "the 1.3 surface must emit C; stderr:\n{}",
        String::from_utf8_lossy(&emit.stderr)
    );
    let binary = scratch.path().join("video");
    let build = Command::new("m68k-amigaos-gcc")
        .args([
            "-std=c99",
            "-mcrt=nix13",
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
        "the 1.3 surface must build; stderr:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
}

/// The full surface cross-builds with the 3.1 runtime.
#[test]
fn the_full_surface_cross_builds_with_nix20() {
    if !tool_available("m68k-amigaos-gcc") {
        eprintln!("skip: m68k-amigaos-gcc is not on PATH");
        return;
    }
    let scratch = ScratchDir::new("build31");
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, FULL_SURFACE).expect("write the snippet");
    let c_path = scratch.path().join("out.c");
    let emit = run_clc(&[
        "--profile".to_owned(),
        "hosted-amiga".to_owned(),
        "-o".to_owned(),
        c_path.display().to_string(),
        source_path.display().to_string(),
    ]);
    assert_eq!(
        emit.status.code(),
        Some(0),
        "the 3.1 surface must emit C; stderr:\n{}",
        String::from_utf8_lossy(&emit.stderr)
    );
    let binary = scratch.path().join("video");
    let build = Command::new("m68k-amigaos-gcc")
        .args([
            "-std=c99",
            "-mcrt=nix20",
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
        "the 3.1 surface must build; stderr:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
}



/// `std.math` runs on the host with the pinned quarter-turn values.
///
/// This is the only new module that is not Amiga-specific, so its arithmetic
/// can be observed directly: the fixed-point sine peaks at ±256, the integer
/// square root floors, and negative input returns zero.
#[test]
fn the_host_math_helpers_produce_the_pinned_values() {
    let source = r#"module tests.video.math;

import { sin, cos, sqrt } from std.math;
import { println } from std.io;

func main(): i32 {
    println($"math {sin(0)} {sin(64)} {sin(128)} {sin(192)} {cos(0)} {sqrt(0)} {sqrt(15)} {sqrt(16)} {sqrt(1000000)} {sqrt(-4)} {sqrt(2147483647)} {cos(2147483647)}");
    return 0;
}
"#;
    let scratch = ScratchDir::new("math-run");
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
        "the host math program must compile; stderr:\n{}",
        String::from_utf8_lossy(&compile.stderr)
    );
    let binary = scratch.path().join("math");
    let build = Command::new("cc")
        .args([
            "-std=c99",
            "-O0",
            "-g",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-o",
        ])
        .arg(&binary)
        .arg(&c_path)
        .output()
        .expect("run the host C compiler");
    assert!(
        build.status.success(),
        "the host math program must build; stderr:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let run = Command::new(&binary)
        .output()
        .expect("run the host math program");
    assert!(run.status.success(), "the host math program must exit 0");
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert_eq!(
        stdout.trim(),
        "math 0 256 0 -256 256 0 3 4 1000 0 46340 256",
        "unexpected fixed-point math results:\n{stdout}"
    );
}
