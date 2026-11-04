//! Sprite standard-library specification tests.
//!
//! Bundle 2 adds `std.amiga.sprite` for the eight OCS hardware sprite channels,
//! attached 16-color pairs, hardware collision, and animation objects. These
//! cases pin the public contract before the module and the C shim exist: the
//! full surface must compile on both hosted-Amiga profiles, the host profile
//! must reject the Amiga-only module, and the emitted C must cross-build with
//! the 1.3 and 3.1 runtimes.
//!
//! The red phase is the unresolved-module compile failure while the source is
//! still absent.

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
            "clc-sprite-{}-{tag}-{sequence}",
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

/// The full sprite surface, exercised in one program.
const FULL_SURFACE: &str = r#"module tests.sprite.full;

import {
    Sprite, SpriteAnimation,
    newSprite, freeSprite, spriteSetPixels, spriteMove, spriteShow,
    spriteHide, spriteAttach, spriteChannel, spriteHeight, spriteX,
    spriteY, spriteVisible, enableCollisions, collisions,
    newAnimation, freeAnimation, animationSetFrame, animationShowFrame,
    animationNext, animationMove, animationShow, animationHide,
    animationFrame
} from std.amiga.sprite;
import { slice } from std.mem;
import { println } from std.io;
import { toHex } from std.fmt;

func main(): i32 {
    let even: Sprite? = newSprite(2, 32);
    if (even == null) {
        return 1;
    }
    let odd: Sprite? = newSprite(3, 32);
    if (odd == null) {
        return 2;
    }
    let pixels: u8[512] = [; 0];
    let i: i32 = 0;
    while (i < 512) {
        pixels[usize(i)] = u8(i % 4);
        i = i + 1;
    }
    spriteSetPixels(even, slice(pixels, 0, 512));
    spriteSetPixels(odd, slice(pixels, 0, 512));
    spriteMove(even, 100, 80);
    spriteMove(odd, 100, 80);
    if (!spriteAttach(even, odd)) {
        return 3;
    }
    spriteShow(even);
    spriteShow(odd);
    let channel: u8 = spriteChannel(even);
    let height: u8 = spriteHeight(even);
    let x: i32 = spriteX(even);
    let y: i32 = spriteY(even);
    let visible: bool = spriteVisible(even);
    enableCollisions();
    let bits: u32 = collisions();
    spriteHide(even);
    spriteHide(odd);

    let anim: SpriteAnimation? = newAnimation(4, 16, 4);
    if (anim == null) {
        return 4;
    }
    let small: u8[256] = [; 0];
    let j: i32 = 0;
    while (j < 256) {
        small[usize(j)] = u8(j % 3);
        j = j + 1;
    }
    animationSetFrame(anim, 0, slice(small, 0, 256));
    animationSetFrame(anim, 1, slice(small, 0, 256));
    animationShowFrame(anim, 0);
    animationNext(anim);
    animationMove(anim, 50, 50);
    animationShow(anim);
    animationHide(anim);
    let frame: u8 = animationFrame(anim);
    println($"sprite {channel} {height} {x} {y} {visible} {toHex(bits)} {frame}");
    freeAnimation(anim);
    freeSprite(odd);
    freeSprite(even);
    return 0;
}
"#;

/// The full surface compiles for `hosted-amiga` with zero diagnostics.
#[test]
fn the_full_sprite_surface_compiles_for_hosted_amiga() {
    let (output, emitted) = compile_source("hosted", "hosted-amiga", FULL_SURFACE);
    assert_eq!(
        output.status.code(),
        Some(0),
        "the sprite surface must compile for hosted-amiga; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "the sprite surface must compile with zero diagnostics; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(emitted, "accepted source must emit C");
}

/// The full surface compiles for `hosted-amiga-13` with zero diagnostics.
#[test]
fn the_full_sprite_surface_compiles_for_the_a500_profile() {
    let (output, emitted) = compile_source("a500", "hosted-amiga-13", FULL_SURFACE);
    assert_eq!(
        output.status.code(),
        Some(0),
        "the sprite surface must compile for hosted-amiga-13; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "the sprite surface must compile with zero diagnostics; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(emitted, "accepted source must emit C");
}

/// The screen-attachment surface compiles on both hosted profiles.
#[test]
fn the_screen_attachment_surface_compiles() {
    let source = r#"module tests.sprite.screen;

import { Screen, openScreen } from std.amiga.intuition;
import { String, stringOf, toCString } from std.text;
import {
    Sprite, SpriteAnimation, newSprite, newAnimation, freeSprite,
    freeAnimation, spriteAttachScreen, spriteDetach,
    animationAttachScreen, animationDetach
} from std.amiga.sprite;

func main(): i32 {
    let title: String = stringOf("sprites");
    let screen: Screen? = openScreen(320, 256, 4, toCString(title));
    if (screen == null) {
        return 1;
    }
    let sprite: Sprite? = newSprite(2, 16);
    if (sprite == null) {
        return 2;
    }
    if (!spriteAttachScreen(sprite, screen)) {
        return 3;
    }
    spriteDetach(sprite);
    let anim: SpriteAnimation? = newAnimation(4, 16, 4);
    if (anim == null) {
        return 4;
    }
    if (!animationAttachScreen(anim, screen)) {
        return 5;
    }
    animationDetach(anim);
    freeAnimation(anim);
    freeSprite(sprite);
    return 0;
}
"#;
    for profile in ["hosted-amiga", "hosted-amiga-13"] {
        let (output, emitted) = compile_source("screen-attach", profile, source);
        assert_eq!(
            output.status.code(),
            Some(0),
            "the sprite screen surface must compile for {profile}; stderr:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(emitted, "accepted source must emit C");
    }
}

/// The host profile rejects the Amiga-only module with `E11200`.
#[test]
fn the_host_profile_rejects_the_sprite_module() {
    let source = r#"module tests.sprite.host;

import { newSprite } from std.amiga.sprite;

func main(): i32 {
    let sprite = newSprite(0, 16);
    return 0;
}
"#;
    let (output, emitted) = compile_source("host-reject", "host", source);
    assert_eq!(
        output.status.code(),
        Some(1),
        "the host profile must reject std.amiga.sprite; stderr:\n{}",
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
fn the_full_sprite_surface_cross_builds_with_nix13() {
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
        "the 1.3 sprite surface must emit C; stderr:\n{}",
        String::from_utf8_lossy(&emit.stderr)
    );
    let binary = scratch.path().join("sprite");
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
        "the 1.3 sprite surface must build; stderr:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
}

/// The full surface cross-builds with the 3.1 runtime.
#[test]
fn the_full_sprite_surface_cross_builds_with_nix20() {
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
        "the 3.1 sprite surface must emit C; stderr:\n{}",
        String::from_utf8_lossy(&emit.stderr)
    );
    let binary = scratch.path().join("sprite");
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
        "the 3.1 sprite surface must build; stderr:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
}

/// The 1.3 profile still rejects `std.amiga.exec`.
#[test]
fn the_a500_profile_still_rejects_exec() {
    let source = r#"module tests.sprite.exec;

import { createMsgPort } from std.amiga.exec;

func main(): i32 {
    return 0;
}
"#;
    let (output, emitted) = compile_source("exec-still-gated", "hosted-amiga-13", source);
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("E11200"), "expected E11200, got:\n{stderr}");
    assert!(!emitted);
}
