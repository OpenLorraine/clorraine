//! Audio/input/timer standard-library specification tests.
//!
//! Bundle 3 adds `std.amiga.audio` (audio.device, samples, four channels),
//! `std.amiga.input` (gameport joystick), and `std.amiga.timer` (timer.device
//! VBlank waits). These cases pin the public contract before the modules and
//! the C shim exist: the full surface must compile on both hosted-Amiga
//! profiles, the host profile must reject the Amiga-only modules, and the
//! emitted C must cross-build with the 1.3 and 3.1 runtimes.
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
            "clc-audio-{}-{tag}-{sequence}",
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

/// The full audio/input/timer surface, exercised in one program.
const FULL_SURFACE: &str = r#"module tests.audio.full;

import {
    Audio, Sample, openAudio, freeAudio, audioChannels, audioPlay,
    audioStop, audioControl, newSample, freeSample, sampleLength,
    samplePeriod, sampleVolume, sampleCycles
} from std.amiga.audio;
import {
    readJoystick, decodeJoystick, JOY_UP, JOY_DOWN, JOY_LEFT, JOY_RIGHT,
    JOY_FIRE
} from std.amiga.input;
import { Timer, openTimer, freeTimer, timerWait } from std.amiga.timer;
import { slice } from std.mem;
import { println } from std.io;
import { toHex } from std.fmt;

func main(): i32 {
    let data: u8[64] = [; 0];
    let i: i32 = 0;
    while (i < 64) {
        data[usize(i)] = u8(i * 3);
        i = i + 1;
    }
    let sample: Sample? = newSample(slice(data, 0, 64), 428, 64, 1);
    if (sample == null) {
        return 1;
    }
    let audio: Audio? = openAudio(4);
    if (audio == null) {
        return 2;
    }
    let mask: u8 = audioChannels(audio);
    let played: bool = audioPlay(audio, 0, sample);
    let controlled: bool = audioControl(audio, 0, 400, 32);
    audioStop(audio, 0);
    let length: u32 = sampleLength(sample);
    let period: u16 = samplePeriod(sample);
    let volume: u8 = sampleVolume(sample);
    let cycles: u16 = sampleCycles(sample);
    let timer: Timer? = openTimer();
    if (timer == null) {
        return 3;
    }
    let waited: bool = timerWait(timer, 1);
    let joy: u8 = readJoystick(0);
    let decoded: u8 = decodeJoystick(0x0002, false);
    let flags: u8 = JOY_UP | JOY_DOWN | JOY_LEFT | JOY_RIGHT | JOY_FIRE;
    println($"audio {toHex(mask)} {played} {controlled} {length} {period} {volume} {cycles} {toHex(joy)} {toHex(decoded)} {flags} {waited}");
    freeTimer(timer);
    freeAudio(audio);
    freeSample(sample);
    return 0;
}
"#;

/// The full surface compiles for `hosted-amiga` with zero diagnostics.
#[test]
fn the_full_audio_surface_compiles_for_hosted_amiga() {
    let (output, emitted) = compile_source("hosted", "hosted-amiga", FULL_SURFACE);
    assert_eq!(
        output.status.code(),
        Some(0),
        "the audio surface must compile for hosted-amiga; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "the audio surface must compile with zero diagnostics; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(emitted, "accepted source must emit C");
}

/// The full surface compiles for `hosted-amiga-13` with zero diagnostics.
#[test]
fn the_full_audio_surface_compiles_for_the_a500_profile() {
    let (output, emitted) = compile_source("a500", "hosted-amiga-13", FULL_SURFACE);
    assert_eq!(
        output.status.code(),
        Some(0),
        "the audio surface must compile for hosted-amiga-13; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "the audio surface must compile with zero diagnostics; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(emitted, "accepted source must emit C");
}

/// The host profile rejects the Amiga-only modules with `E11200`.
#[test]
fn the_host_profile_rejects_the_audio_module() {
    let source = r#"module tests.audio.host;

import { openAudio } from std.amiga.audio;

func main(): i32 {
    let audio = openAudio(1);
    return 0;
}
"#;
    let (output, emitted) = compile_source("host-reject", "host", source);
    assert_eq!(
        output.status.code(),
        Some(1),
        "the host profile must reject std.amiga.audio; stderr:\n{}",
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

/// The host profile rejects the input and timer modules with `E11200`.
#[test]
fn the_host_profile_rejects_input_and_timer() {
    for (tag, source) in [
        (
            "input",
            r#"module tests.input.host;

import { readJoystick } from std.amiga.input;

func main(): i32 {
    let value = readJoystick(0);
    return 0;
}
"#,
        ),
        (
            "timer",
            r#"module tests.timer.host;

import { openTimer } from std.amiga.timer;

func main(): i32 {
    let timer = openTimer();
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
fn the_full_audio_surface_cross_builds_with_nix13() {
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
        "the 1.3 audio surface must emit C; stderr:\n{}",
        String::from_utf8_lossy(&emit.stderr)
    );
    let binary = scratch.path().join("audio");
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
        "the 1.3 audio surface must build; stderr:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
}

/// The full surface cross-builds with the 3.1 runtime.
#[test]
fn the_full_audio_surface_cross_builds_with_nix20() {
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
        "the 3.1 audio surface must emit C; stderr:\n{}",
        String::from_utf8_lossy(&emit.stderr)
    );
    let binary = scratch.path().join("audio");
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
        "the 3.1 audio surface must build; stderr:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
}

/// The 1.3 profile still rejects `std.amiga.exec`.
#[test]
fn the_a500_profile_still_rejects_exec() {
    let source = r#"module tests.audio.exec;

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
