//! Intuition window and event-loop specification tests.
//!
//! Bundle 6 adds the Intuition window lifecycle, inner-window drawing and
//! text, and the IDCMP event loop. The host has no Intuition, so these cases
//! prove the public surface compiles on both hosted-Amiga profiles, the host
//! profile rejects the Amiga-only module, and the emitted C cross-builds with
//! the 1.3 and 2.0 runtimes. The interactive event behavior is proven by the
//! A500 GUI smoke; the cases live in the plan's testing strategy as ST-4 and
//! ST-5.
//!
//! The red phase is the unresolved-export compile failure while the module and
//! shim are still absent.

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
            "clc-intuition-{}-{tag}-{sequence}",
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

/// The full window and event surface, exercised in one compile-only program.
const FULL_WINDOW_SURFACE: &str = r#"module tests.intuition.window;

import {
    Screen, openScreen, Window, openWindow, closeWindow,
    windowWidth, windowHeight, WindowEvent, waitEvent, pollEvent,
    EVENT_NONE, EVENT_CLOSE, EVENT_GADGET, EVENT_MENU, EVENT_MOUSE,
    EVENT_KEY, EVENT_ACTIVE, EVENT_INACTIVE
} from std.amiga.intuition;
import {
    setColor, windowClear, windowFill, windowDraw
} from std.amiga.graphics;
import { Font, openFont, defaultFont, windowText } from std.amiga.text;
import { String, stringOf, toCString, bytes } from std.text;

func run(screen: Screen): i32 {
    let window: Window? = openWindow(screen, 24, 24, 272, 136,
                                      toCString(stringOf("Window")));
    if (window == null) {
        return 1;
    }
    let width: i32 = windowWidth(window);
    let height: i32 = windowHeight(window);
    windowClear(window, 0);
    windowFill(window, 0, 0, width - 1, 8, 1);
    windowDraw(window, 0, 9, width - 1, 9, 2);

    let font: Font? = openFont(toCString(stringOf("topaz.font")), 8, 0);
    if (font == null) {
        font = defaultFont();
    }
    if (font == null) {
        return 2;
    }
    windowText(window, font, 2, 8, 1, bytes(stringOf("Clorraine")));

    let polled: WindowEvent = pollEvent(window);
    if (polled.kind == EVENT_NONE) {
        let event: WindowEvent = waitEvent(window);
        let kind: i32 = event.kind;
        let code: i32 = event.code;
        let qualifier: u16 = event.qualifier;
        let mx: i32 = event.mouse_x;
        let my: i32 = event.mouse_y;
        let gadget: i32 = event.gadget;
        let menu: i32 = event.menu;
        if (kind == EVENT_CLOSE || kind == EVENT_GADGET || kind == EVENT_MENU
            || kind == EVENT_MOUSE || kind == EVENT_KEY
            || kind == EVENT_ACTIVE || kind == EVENT_INACTIVE) {
            closeWindow(window);
            return code + i32(qualifier) + mx + my + gadget + menu;
        }
    }
    closeWindow(window);
    return height;
}

func main(): i32 {
    let screenTitle: String = stringOf("Intuition window");
    let screen: Screen? = openScreen(320, 200, 4, toCString(screenTitle));
    if (screen == null) {
        return 1;
    }
    setColor(screen, 0, 8, 8, 24);
    setColor(screen, 1, 220, 220, 220);
    setColor(screen, 2, 255, 128, 0);
    setColor(screen, 17, 255, 255, 255);
    setColor(screen, 18, 0, 0, 0);
    return run(screen);
}
"#;

/// The full gadget and menu surface, exercised in one compile-only program.
const FULL_GADGET_SURFACE: &str = r#"module tests.intuition.gadgets;

import {
    Screen, openScreen, Window, openWindow, closeWindow,
    addButton, addString, windowString, addMenu, addMenuItem
} from std.amiga.intuition;
import { String, stringOf, toCString } from std.text;
import { slice } from std.mem;

func main(): i32 {
    let screenTitle: String = stringOf("Gadgets");
    let screen: Screen? = openScreen(320, 200, 4, toCString(screenTitle));
    if (screen == null) { return 1; }
    let window: Window? = openWindow(screen, 10, 10, 280, 140,
                                      toCString(stringOf("Gadgets")));
    if (window == null) { return 2; }
    let button: bool = addButton(window, 8, 28, 80, 18,
                                 toCString(stringOf("Press")), 1);
    if (!button) { return 3; }
    let field: bool = addString(window, 8, 56, 160, 18,
                                toCString(stringOf("Name:")),
                                toCString(stringOf("")), 32, 2);
    if (!field) { return 4; }
    let buf: u8[64] = [; 0];
    let count: i32 = windowString(window, 2, borrow slice(buf, 0, 64));
    if (count < 0) { return 5; }
    let file: i32 = addMenu(window, toCString(stringOf("File")));
    if (file < 0) { return 6; }
    let item: bool = addMenuItem(window, file, toCString(stringOf("Quit")), 11);
    if (!item) { return 7; }
    let help: i32 = addMenu(window, toCString(stringOf("Help")));
    if (help != 1) { return 8; }
    let about: bool = addMenuItem(window, help, toCString(stringOf("About")), 12);
    if (!about) { return 9; }
    closeWindow(window);
    return count + file + help;
}
"#;

/// The full window/event surface compiles for `hosted-amiga` with zero
/// diagnostics.
#[test]
fn the_full_window_event_surface_compiles_for_hosted_amiga() {
    let (output, emitted) = compile_source("hosted", "hosted-amiga", FULL_WINDOW_SURFACE);
    assert_eq!(
        output.status.code(),
        Some(0),
        "the window/event surface must compile for hosted-amiga; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "the window/event surface must compile with zero diagnostics; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(emitted, "accepted source must emit C");
}

/// The full window/event surface compiles for `hosted-amiga-13` with zero
/// diagnostics.
#[test]
fn the_full_window_event_surface_compiles_for_the_a500_profile() {
    let (output, emitted) = compile_source("a500", "hosted-amiga-13", FULL_WINDOW_SURFACE);
    assert_eq!(
        output.status.code(),
        Some(0),
        "the window/event surface must compile for hosted-amiga-13; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "the window/event surface must compile with zero diagnostics; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(emitted, "accepted source must emit C");
}

/// The host profile rejects the full Amiga-only surface with `E11200`.
#[test]
fn the_host_profile_rejects_intuition() {
    let (output, emitted) = compile_source("host-reject", "host", FULL_WINDOW_SURFACE);
    assert_eq!(
        output.status.code(),
        Some(1),
        "the host profile must reject the Intuition surface; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("E11200"),
        "expected E11200 for the Intuition surface, got:\n{stderr}"
    );
    assert!(!emitted, "a rejected program must not emit C");
}

/// The full window/event surface cross-builds with the Kickstart 1.3 runtime.
#[test]
fn the_full_window_event_surface_cross_builds_with_nix13() {
    if !tool_available("m68k-amigaos-gcc") {
        eprintln!("skip: m68k-amigaos-gcc is not on PATH");
        return;
    }
    let scratch = ScratchDir::new("build13");
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, FULL_WINDOW_SURFACE).expect("write the snippet");
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
        "the 1.3 window surface must emit C; stderr:\n{}",
        String::from_utf8_lossy(&emit.stderr)
    );
    let binary = scratch.path().join("intuition");
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
        "the 1.3 window surface must build; stderr:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
}

/// The full window/event surface cross-builds with the 2.0+ runtime.
#[test]
fn the_full_window_event_surface_cross_builds_with_nix20() {
    if !tool_available("m68k-amigaos-gcc") {
        eprintln!("skip: m68k-amigaos-gcc is not on PATH");
        return;
    }
    let scratch = ScratchDir::new("build20");
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, FULL_WINDOW_SURFACE).expect("write the snippet");
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
        "the 2.0+ window surface must emit C; stderr:\n{}",
        String::from_utf8_lossy(&emit.stderr)
    );
    let binary = scratch.path().join("intuition");
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
        "the 2.0+ window surface must build; stderr:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
}

/// The full gadget/menu surface compiles for `hosted-amiga` with zero
/// diagnostics.
#[test]
fn the_full_gadget_surface_compiles_for_hosted_amiga() {
    let (output, emitted) = compile_source("gadgets-hosted", "hosted-amiga", FULL_GADGET_SURFACE);
    assert_eq!(
        output.status.code(),
        Some(0),
        "the gadget surface must compile for hosted-amiga; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "the gadget surface must compile with zero diagnostics; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(emitted, "accepted source must emit C");
}

/// The full gadget/menu surface compiles for `hosted-amiga-13` with zero
/// diagnostics.
#[test]
fn the_full_gadget_surface_compiles_for_the_a500_profile() {
    let (output, emitted) = compile_source("gadgets-a500", "hosted-amiga-13", FULL_GADGET_SURFACE);
    assert_eq!(
        output.status.code(),
        Some(0),
        "the gadget surface must compile for hosted-amiga-13; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "the gadget surface must compile with zero diagnostics; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(emitted, "accepted source must emit C");
}

/// The full gadget/menu surface cross-builds with the Kickstart 1.3 runtime.
#[test]
fn the_full_gadget_surface_cross_builds_with_nix13() {
    if !tool_available("m68k-amigaos-gcc") {
        eprintln!("skip: m68k-amigaos-gcc is not on PATH");
        return;
    }
    let scratch = ScratchDir::new("gadgets-build13");
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, FULL_GADGET_SURFACE).expect("write the snippet");
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
        "the 1.3 gadget surface must emit C; stderr:\n{}",
        String::from_utf8_lossy(&emit.stderr)
    );
    let binary = scratch.path().join("gadgets");
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
        "the 1.3 gadget surface must build; stderr:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
}

/// The full gadget/menu surface cross-builds with the 2.0+ runtime.
#[test]
fn the_full_gadget_surface_cross_builds_with_nix20() {
    if !tool_available("m68k-amigaos-gcc") {
        eprintln!("skip: m68k-amigaos-gcc is not on PATH");
        return;
    }
    let scratch = ScratchDir::new("gadgets-build20");
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, FULL_GADGET_SURFACE).expect("write the snippet");
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
        "the 2.0+ gadget surface must emit C; stderr:\n{}",
        String::from_utf8_lossy(&emit.stderr)
    );
    let binary = scratch.path().join("gadgets");
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
        "the 2.0+ gadget surface must build; stderr:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
}

/// The full requester, alert, and pointer surface, exercised in one
/// compile-only program.
const FULL_REQUESTER_SURFACE: &str = r#"module tests.intuition.requesters;

import {
    Screen, openScreen, Window, openWindow, closeWindow,
    askYesNo, showAlert, setPointer
} from std.amiga.intuition;
import { setColor } from std.amiga.graphics;
import { String, stringOf, toCString } from std.text;

func main(): i32 {
    let screenTitle: String = stringOf("Requesters");
    let screen: Screen? = openScreen(320, 200, 4, toCString(screenTitle));
    if (screen == null) { return 1; }
    setColor(screen, 17, 255, 255, 255);
    setColor(screen, 18, 0, 0, 0);
    let window: Window? = openWindow(screen, 10, 10, 280, 140,
                                      toCString(stringOf("Requesters")));
    if (window == null) { return 2; }
    setPointer(window, true);
    let answer: bool = askYesNo(window, toCString(stringOf("Continue?")));
    if (answer) {
        showAlert(toCString(stringOf("Done")));
    }
    setPointer(window, false);
    closeWindow(window);
    return 0;
}
"#;

/// The full requester/alert/pointer surface compiles for `hosted-amiga`.
#[test]
fn the_full_requester_surface_compiles_for_hosted_amiga() {
    let (output, emitted) = compile_source(
        "requesters-hosted",
        "hosted-amiga",
        FULL_REQUESTER_SURFACE,
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "the requester surface must compile for hosted-amiga; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "the requester surface must compile with zero diagnostics; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(emitted, "accepted source must emit C");
}

/// The full requester/alert/pointer surface compiles for `hosted-amiga-13`.
#[test]
fn the_full_requester_surface_compiles_for_the_a500_profile() {
    let (output, emitted) = compile_source(
        "requesters-a500",
        "hosted-amiga-13",
        FULL_REQUESTER_SURFACE,
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "the requester surface must compile for hosted-amiga-13; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "the requester surface must compile with zero diagnostics; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(emitted, "accepted source must emit C");
}

/// The full requester/alert/pointer surface cross-builds with the 1.3 runtime.
#[test]
fn the_full_requester_surface_cross_builds_with_nix13() {
    if !tool_available("m68k-amigaos-gcc") {
        eprintln!("skip: m68k-amigaos-gcc is not on PATH");
        return;
    }
    let scratch = ScratchDir::new("requesters-build13");
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, FULL_REQUESTER_SURFACE).expect("write the snippet");
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
        "the 1.3 requester surface must emit C; stderr:\n{}",
        String::from_utf8_lossy(&emit.stderr)
    );
    let binary = scratch.path().join("requesters");
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
        "the 1.3 requester surface must build; stderr:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
}

/// The full requester/alert/pointer surface cross-builds with the 2.0+ runtime.
#[test]
fn the_full_requester_surface_cross_builds_with_nix20() {
    if !tool_available("m68k-amigaos-gcc") {
        eprintln!("skip: m68k-amigaos-gcc is not on PATH");
        return;
    }
    let scratch = ScratchDir::new("requesters-build20");
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, FULL_REQUESTER_SURFACE).expect("write the snippet");
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
        "the 2.0+ requester surface must emit C; stderr:\n{}",
        String::from_utf8_lossy(&emit.stderr)
    );
    let binary = scratch.path().join("requesters");
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
        "the 2.0+ requester surface must build; stderr:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
}
