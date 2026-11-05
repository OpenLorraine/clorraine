//! Text and layers standard-library specification tests.
//!
//! Bundle 4 adds `std.amiga.text` (fonts, measurement, drawing) and
//! `std.amiga.layers` (layers and clipping). These cases pin the public
//! contract before the modules and the C shim exist: the full surface must
//! compile on both hosted-Amiga profiles, the host profile must reject the
//! Amiga-only modules, and the emitted C must cross-build with the 1.3 and 3.1
//! runtimes.
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
            "clc-text-{}-{tag}-{sequence}",
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

/// The full text/layers surface, exercised in one program.
const FULL_SURFACE: &str = r#"module tests.text.full;

import { Screen, openScreen } from std.amiga.intuition;
import {
    Font, openFont, openDiskFont, defaultFont, freeFont, textWidth,
    textHeight, drawText
} from std.amiga.text;
import {
    Layer, newLayer, freeLayer, layerMove, layerSize, layerFill, layerClear,
    layerText, layerBeginUpdate, layerEndUpdate
} from std.amiga.layers;
import { Bitmap, newBitmap, freeBitmap } from std.amiga.video;
import { String, stringOf, toCString } from std.text;
import { slice } from std.mem;
import { println } from std.io;

func main(): i32 {
    let title: String = stringOf("text");
    let screen: Screen? = openScreen(320, 256, 4, toCString(title));
    if (screen == null) {
        return 1;
    }
    let font: Font? = openFont(toCString(stringOf("topaz.font")), 8, 0);
    if (font == null) {
        font = defaultFont();
    }
    if (font == null) {
        return 2;
    }
    let disk: Font? = openDiskFont(toCString(stringOf("topaz.font")), 8, 0);
    if (disk != null) {
        freeFont(disk);
    }
    let text: u8[16] = [67, 108, 111, 114, 114, 97, 105, 110, 101, 0, 0, 0, 0, 0, 0, 0];
    let width: i32 = textWidth(font, slice(text, 0, 9));
    let height: i32 = textHeight(font);
    drawText(screen, font, 10, 20, 1, slice(text, 0, 9));

    let bitmap: Bitmap? = newBitmap(240, 180, 4);
    if (bitmap == null) {
        return 3;
    }
    let layer: Layer? = newLayer(bitmap, 40, 80, 199, 143);
    if (layer == null) {
        return 4;
    }
    let began: bool = layerBeginUpdate(layer);
    layerClear(layer, 0);
    layerFill(layer, 0, 0, 159, 63, 1);
    layerText(layer, font, 4, 16, 2, slice(text, 0, 9));
    layerEndUpdate(layer);
    let moved: bool = layerMove(layer, 4, 2);
    let resized: bool = layerSize(layer, -4, -2);
    println($"text {width} {height} {began} {moved} {resized}");
    freeLayer(layer);
    freeBitmap(bitmap);
    freeFont(font);
    return 0;
}
"#;

/// The full surface compiles for `hosted-amiga` with zero diagnostics.
#[test]
fn the_full_text_surface_compiles_for_hosted_amiga() {
    let (output, emitted) = compile_source("hosted", "hosted-amiga", FULL_SURFACE);
    assert_eq!(
        output.status.code(),
        Some(0),
        "the text surface must compile for hosted-amiga; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "the text surface must compile with zero diagnostics; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(emitted, "accepted source must emit C");
}

/// The full surface compiles for `hosted-amiga-13` with zero diagnostics.
#[test]
fn the_full_text_surface_compiles_for_the_a500_profile() {
    let (output, emitted) = compile_source("a500", "hosted-amiga-13", FULL_SURFACE);
    assert_eq!(
        output.status.code(),
        Some(0),
        "the text surface must compile for hosted-amiga-13; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "the text surface must compile with zero diagnostics; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(emitted, "accepted source must emit C");
}

/// The host profile rejects the Amiga-only modules with `E11200`.
#[test]
fn the_host_profile_rejects_text_and_layers() {
    for (tag, source) in [
        (
            "text",
            r#"module tests.text.host;

import { defaultFont } from std.amiga.text;

func main(): i32 {
    let font = defaultFont();
    return 0;
}
"#,
        ),
        (
            "layers",
            r#"module tests.layers.host;

import { newLayer } from std.amiga.layers;

func main(): i32 {
    return 0;
}
"#,
        ),
    ] {
        let (output, emitted) = compile_source(tag, "host", source);
        assert_eq!(
            output.status.code(),
            Some(1),
            "the host profile must reject the {tag} module; stderr:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("E11200"),
            "expected E11200 for {tag}, got:\n{stderr}"
        );
        assert!(!emitted, "a rejected {tag} program must not emit C");
    }
}

/// The full surface cross-builds with the Kickstart 1.3 runtime.
#[test]
fn the_full_text_surface_cross_builds_with_nix13() {
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
        "the 1.3 text surface must emit C; stderr:\n{}",
        String::from_utf8_lossy(&emit.stderr)
    );
    let binary = scratch.path().join("text");
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
        "the 1.3 text surface must build; stderr:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
}

/// The full surface cross-builds with the 3.1 runtime.
#[test]
fn the_full_text_surface_cross_builds_with_nix20() {
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
        "the 3.1 text surface must emit C; stderr:\n{}",
        String::from_utf8_lossy(&emit.stderr)
    );
    let binary = scratch.path().join("text");
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
        "the 3.1 text surface must build; stderr:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
}
