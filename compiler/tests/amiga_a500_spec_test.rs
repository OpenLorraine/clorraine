//! A500/Kickstart 1.3 specification tests.
//!
//! The 1.3 profile is the lowest supported Amiga baseline. These cases pin the
//! profile surface, the module gate, and the `-mcrt=nix13` build before the
//! implementation exists. The host cases always run; the cross-build case skips
//! when the Amiga toolchain is absent.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

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

/// `std.amiga.exec` is a 2.0+ surface; the 1.3 profile rejects it.
#[test]
fn the_a500_profile_rejects_the_exec_module() {
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
        Some(1),
        "the 1.3 profile must reject std.amiga.exec; stderr:\n{}",
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
