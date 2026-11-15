//! A500/Kickstart 1.3 specification tests.
//!
//! The 1.3 profile is the lowest supported Amiga baseline. These cases pin the
//! profile surface, the module gate, and the `-mcrt=nix13` build before the
//! profile exists. They also pin the deterministic capture geometry of the
//! hello and four demo screenshot gates. The host cases always run; the
//! cross-build case skips when the Amiga toolchain is absent.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Read a repository file relative to the workspace root.
fn repo_text(relative: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
}

/// Split a `WxH+X+Y` crop geometry into its four fields.
fn parse_crop(script: &str, geometry: &str) -> (u32, u32, u32, u32) {
    let parts: Vec<&str> = geometry.split(['x', '+']).collect();
    assert_eq!(
        parts.len(),
        4,
        "{script}: crop {geometry} is not WxH+X+Y"
    );
    let parse = |field: &str| {
        field.parse::<u32>().unwrap_or_else(|_| {
            panic!("{script}: crop {geometry} has a non-numeric field {field:?}")
        })
    };
    (parse(parts[0]), parse(parts[1]), parse(parts[2]), parse(parts[3]))
}

/// Temporary directory that removes itself when the test finishes.
struct ScratchDir {
    path: PathBuf,
}

impl ScratchDir {
    fn new(tag: &str) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "clc-a500-{}-{tag}-{sequence}",
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

fn run_clc(args: &[String]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_clc"))
        .args(args)
        .output()
        .expect("failed to spawn the compiler binary")
}

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

fn tool_available(tool: &str) -> bool {
    Command::new(tool).arg("--version").output().is_ok()
}

/// The five A500 screenshot gates below pin the FS-UAE window, check the
/// captured size before cropping, and crop only inside that window.
///
/// The capture is the gate's oracle, so a host display scale, window default,
/// or aspect-correction change must fail with the observed size instead of a
/// large image comparison mismatch.
#[test]
fn the_a500_screenshot_gates_pin_their_capture_geometry() {
    let gates: [(&str, u32, u32); 5] = [
        ("docker/scripts/a500-smoke.sh", 640, 256),
        ("docker/scripts/a500-boing-smoke.sh", 960, 540),
        ("docker/scripts/a500-sprite-smoke.sh", 960, 540),
        ("docker/scripts/a500-paddle-smoke.sh", 960, 540),
        ("docker/scripts/a500-text-smoke.sh", 960, 540),
    ];
    for (script, width, height) in gates {
        let text = repo_text(script);
        for flag in [
            format!("--window-width={width}"),
            format!("--window-height={height}"),
            "--window-border=0".to_owned(),
        ] {
            assert!(
                text.contains(&flag),
                "{script} must pin the FS-UAE window with {flag:?}"
            );
        }
        let expected_size = format!("{width}x{height}");
        assert!(
            text.contains(&format!("\"${{size}}\" != \"{expected_size}\"")),
            "{script} must assert a {expected_size} capture"
        );
        // The size check must run inside the capture poll loop and before the
        // first crop, so a wrong-sized window fails before any comparison.
        let poll = text
            .find("while [ \"${step}\"")
            .unwrap_or_else(|| panic!("{script}: no capture poll loop"));
        let size_check = text
            .find("identify -format '%wx%h'")
            .unwrap_or_else(|| panic!("{script}: no capture size check"));
        let first_crop = text
            .find("-crop ")
            .unwrap_or_else(|| panic!("{script}: no crop"));
        assert!(
            poll < size_check,
            "{script}: the capture size check must be inside the poll loop"
        );
        assert!(
            size_check < first_crop,
            "{script}: the capture size check must precede the first crop"
        );
        let tokens: Vec<&str> = text.split_whitespace().collect();
        let mut crops = 0;
        for (index, token) in tokens.iter().enumerate() {
            if *token != "-crop" {
                continue;
            }
            let geometry = tokens
                .get(index + 1)
                .unwrap_or_else(|| panic!("{script}: -crop without a geometry"));
            let (crop_width, crop_height, x, y) = parse_crop(script, geometry);
            assert!(
                x + crop_width <= width && y + crop_height <= height,
                "{script}: crop {geometry} leaves the {expected_size} window"
            );
            crops += 1;
        }
        assert!(crops > 0, "{script} must crop at least one image");
    }
}

/// The drawing surface exists in Kickstart 1.3, so the 1.3 profile accepts it.
#[test]
fn the_a500_profile_accepts_the_graphics_surface() {
    let source = r#"module tests.a500.graphics;

import { Screen, openScreen } from std.amiga.intuition;
import { setColor, fill, draw, clear } from std.amiga.graphics;
import { String, stringOf, toCString } from std.text;

func main(): i32 {
    let title: String = stringOf("a500");
    let screen: Screen? = openScreen(320, 200, 4, toCString(title));
    if (screen == null) {
        return 1;
    }
    setColor(screen, 1, 255, 0, 0);
    clear(screen, 0);
    fill(screen, 0, 0, 15, 15, 1);
    draw(screen, 0, 0, 319, 199, 1);
    return 0;
}
"#;
    let (output, emitted) = compile_source("graphics", "hosted-amiga-13", source);
    assert_eq!(
        output.status.code(),
        Some(0),
        "the 1.3 profile must accept graphics; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(emitted, "accepted source must emit C");
}

/// `std.amiga.exec` is part of the classic 1.3 Exec surface; the A500 profile
/// accepts it and emits C.
#[test]
fn the_a500_profile_accepts_the_exec_module() {
    let source = r#"module tests.a500.exec;

import { MsgPort, createMsgPort, deleteMsgPort } from std.amiga.exec;

func main(): i32 {
    let port: MsgPort? = createMsgPort();
    if (port == null) {
        return 1;
    }
    deleteMsgPort(port);
    return 0;
}
"#;
    let (output, emitted) = compile_source("exec", "hosted-amiga-13", source);
    assert_eq!(
        output.status.code(),
        Some(0),
        "the 1.3 profile must accept std.amiga.exec; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "the 1.3 exec surface must compile with zero diagnostics; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(emitted, "accepted source must emit C");
}

/// The 3.1 profile keeps the exec surface.
#[test]
fn the_31_profile_still_accepts_the_exec_module() {
    let source = r#"module tests.a500.exec31;

import { MsgPort, createMsgPort, deleteMsgPort } from std.amiga.exec;

func main(): i32 {
    let port: MsgPort? = createMsgPort();
    if (port == null) {
        return 1;
    }
    deleteMsgPort(port);
    return 0;
}
"#;
    let (output, emitted) = compile_source("exec31", "hosted-amiga", source);
    assert_eq!(
        output.status.code(),
        Some(0),
        "the 3.1 profile must keep std.amiga.exec; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(emitted, "accepted source must emit C");
}

/// The 1.3 hello program cross-builds with the `nix13` runtime.
#[test]
fn the_a500_hello_cross_builds_with_nix13() {
    if !tool_available("m68k-amigaos-gcc") {
        eprintln!("skip: m68k-amigaos-gcc is not on PATH");
        return;
    }
    let scratch = ScratchDir::new("build");
    let output_path = scratch.path().join("hello.c");
    let program = Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples/hello/hello.clo");
    let emit = run_clc(&[
        "--profile".to_owned(),
        "hosted-amiga-13".to_owned(),
        "-o".to_owned(),
        output_path.display().to_string(),
        program.display().to_string(),
    ]);
    assert_eq!(
        emit.status.code(),
        Some(0),
        "the 1.3 hello must emit C; stderr:\n{}",
        String::from_utf8_lossy(&emit.stderr)
    );
    let binary = scratch.path().join("hello");
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
        .arg(&output_path)
        .args(["-lamiga", "-lm"])
        .output()
        .expect("failed to spawn m68k-amigaos-gcc");
    assert!(
        build.status.success(),
        "the 1.3 hello must build with nix13; stderr:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
}
