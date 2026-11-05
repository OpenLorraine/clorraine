//! Specification tests for the Workbench, icon, and expansion surface.
//!
//! Phase 1 covers the public message port that the Workbench AppMessage
//! fixture needs. The icon, Workbench, and expansion cases are added by their
//! phases; the host cases always run, and the cross-build cases skip themselves
//! when the m68k toolchain is absent.

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
            "clc-workbench-{}-{tag}-{sequence}",
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

/// Emits `source` for `profile` and cross-builds it with the named libnix
/// runtime. The host-only CI has no m68k compiler, so the case skips itself.
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
    let args = vec![
        "-std=c99".to_owned(),
        format!("-mcrt={crt}"),
        "-m68000".to_owned(),
        "-fwrapv".to_owned(),
        "-O0".to_owned(),
        "-g".to_owned(),
        "-Wall".to_owned(),
        "-Wextra".to_owned(),
        "-Werror".to_owned(),
        "-o".to_owned(),
        binary.display().to_string(),
        c_path.display().to_string(),
        "-lamiga".to_owned(),
        "-lm".to_owned(),
    ];
    let build = Command::new("m68k-amigaos-gcc")
        .args(&args)
        .output()
        .expect("run m68k-amigaos-gcc");
    assert!(
        build.status.success(),
        "{tag}: the surface must build with {crt}; stderr:\n{}",
        stderr(&build)
    );
}

/// The public port keeps its name `String` alive until the port is deleted.
const PUBLIC_PORT_SOURCE: &str = "\
module tests.wb.publicport;

import { MsgPort, createPublicPort, deleteMsgPort } from std.amiga.exec;
import { String, stringOf, toCString } from std.text;

func main(): i32 {
    let name: String = stringOf(\"ClorraineWbTest\");
    let port: MsgPort? = createPublicPort(toCString(name));
    if (port == null) {
        return 1;
    }
    deleteMsgPort(port);
    return 0;
}
";

// The public port surface compiles on both hosted profiles and names its shim
// call.
#[test]
fn the_public_port_builds_on_both_hosted_profiles() {
    for profile in ["hosted-amiga", "hosted-amiga-13"] {
        let scratch = ScratchDir::new("public-port");
        let source_path = write_fixture(&scratch, "main.clo", PUBLIC_PORT_SOURCE);
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
        assert_exit(profile, &output, 0);
        assert!(
            stderr(&output).is_empty(),
            "{profile}: the public port must compile with zero diagnostics; stderr:\n{}",
            stderr(&output)
        );
        let c = std::fs::read_to_string(&output_path).expect("the emitted C is readable");
        assert!(
            c.contains("cl_create_public_port"),
            "{profile}: the emitted C is missing cl_create_public_port"
        );
    }
}

// The host profile rejects the Amiga-only module that owns the public port.
#[test]
fn the_host_profile_rejects_the_public_port() {
    let scratch = ScratchDir::new("public-port-host");
    let source_path = write_fixture(&scratch, "main.clo", PUBLIC_PORT_SOURCE);
    let output_path = scratch.path().join("out.c");
    let output = run_clc(
        scratch.path(),
        &[
            "--profile",
            "host",
            "-o",
            output_path.to_str().expect("the output path is UTF-8"),
            source_path.to_str().expect("the source path is UTF-8"),
        ],
    );
    assert_exit("host", &output, 1);
    assert!(
        stderr(&output).contains("E11200"),
        "expected E11200, got:\n{}",
        stderr(&output)
    );
    assert!(!output_path.exists(), "a rejected program must not emit C");
}

// The public port is a 1.3-safe `CreatePort` call, so it cross-builds with both
// runtimes.
#[test]
fn the_public_port_cross_builds_with_nix13_and_nix20() {
    cross_build("public-port-13", "hosted-amiga-13", "nix13", PUBLIC_PORT_SOURCE);
    cross_build("public-port-20", "hosted-amiga", "nix20", PUBLIC_PORT_SOURCE);
}

// The stdlib appendix §13.2 documents the public port.
#[test]
fn the_exec_appendix_documents_the_public_port() {
    let appendix = std::fs::read_to_string(repo_root().join("spec/appendix-stdlib.md"))
        .expect("the stdlib appendix is readable");
    assert!(
        appendix.contains("createPublicPort"),
        "the appendix is missing createPublicPort"
    );
}

/// The icon surface exercises a default icon, a write/reload round-trip, the
/// type accessors, and the explicit free.
const ICON_SOURCE: &str = "\
module tests.wb.icon;

import { DiskObject, ICON_TOOL, ICON_APPICON, getDefDiskObject,
         getDiskObject, putDiskObject, freeDiskObject, diskObjectType,
         setDiskObjectType } from std.amiga.icon;
import { String, stringOf, toCString } from std.text;

func main(): i32 {
    let name: String = stringOf(\"CNEXT:icon-test\");
    let icon: DiskObject? = getDefDiskObject(ICON_TOOL);
    if (icon == null) {
        return 1;
    }
    let kind: i32 = diskObjectType(icon);
    if (!setDiskObjectType(icon, ICON_APPICON)) {
        freeDiskObject(icon);
        return 2;
    }
    if (!putDiskObject(toCString(name), icon)) {
        freeDiskObject(icon);
        return 3;
    }
    freeDiskObject(icon);
    let loaded: DiskObject? = getDiskObject(toCString(name));
    if (loaded == null) {
        return 4;
    }
    let loadedKind: i32 = diskObjectType(loaded);
    freeDiskObject(loaded);
    return kind + loadedKind;
}
";

// The icon surface compiles on hosted-amiga with zero diagnostics.
#[test]
fn the_icon_surface_builds_on_hosted_amiga() {
    let scratch = ScratchDir::new("icon-surface");
    let source_path = write_fixture(&scratch, "main.clo", ICON_SOURCE);
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
    assert_exit("icon-surface", &output, 0);
    assert!(
        stderr(&output).is_empty(),
        "the icon surface must compile with zero diagnostics; stderr:\n{}",
        stderr(&output)
    );
    let c = std::fs::read_to_string(&output_path).expect("the emitted C is readable");
    for call in [
        "cl_get_disk_object",
        "cl_put_disk_object",
        "cl_free_disk_object",
        "cl_get_def_disk_object",
        "cl_disk_object_type",
        "cl_set_disk_object_type",
    ] {
        assert!(c.contains(call), "the emitted C is missing {call}");
    }
}

// The icon module is 2.0+ only; the A500 profile rejects it.
#[test]
fn the_a500_profile_rejects_the_icon_surface() {
    let scratch = ScratchDir::new("icon-a500");
    let source_path = write_fixture(&scratch, "main.clo", ICON_SOURCE);
    let output_path = scratch.path().join("out.c");
    let output = run_clc(
        scratch.path(),
        &[
            "--profile",
            "hosted-amiga-13",
            "-o",
            output_path.to_str().expect("the output path is UTF-8"),
            source_path.to_str().expect("the source path is UTF-8"),
        ],
    );
    assert_exit("icon-a500", &output, 1);
    assert!(
        stderr(&output).contains("E11200"),
        "expected E11200, got:\n{}",
        stderr(&output)
    );
    assert!(!output_path.exists(), "a rejected program must not emit C");
}

// The icon surface cross-builds with the 2.0 runtime.
#[test]
fn the_icon_surface_cross_builds_with_nix20() {
    cross_build("icon-build20", "hosted-amiga", "nix20", ICON_SOURCE);
}

// The stdlib appendix §14.1 documents every icon export.
#[test]
fn the_icon_exports_match_the_stdlib_appendix() {
    let appendix = std::fs::read_to_string(repo_root().join("spec/appendix-stdlib.md"))
        .expect("the stdlib appendix is readable");
    let start = appendix
        .find("## 14. Icon")
        .expect("the appendix has the section 14 heading");
    let section = &appendix[start..];
    let end = section.find("## 15.").unwrap_or(section.len());
    let section = &section[..end];
    let module = clc::stdlib::lookup("std.amiga.icon").expect("the icon module is registered");
    for name in module.exports {
        assert!(
            mentions(section, name),
            "the appendix is missing the export '{name}'"
        );
    }
}

// True when `name` appears as a whole identifier in `text`.
fn mentions(text: &str, name: &str) -> bool {
    text.split(|character: char| !(character.is_ascii_alphanumeric() || character == '_'))
        .any(|token| token == name)
}

/// The Workbench surface: a screen/window, a public port, all three
/// application objects, a public-screen capture, and one AppMessage.
const WORKBENCH_SOURCE: &str = "\
module tests.wb.workbench;

import { MsgPort, createPublicPort, deleteMsgPort } from std.amiga.exec;
import { Screen, Window, openScreen, openWindow, closeWindow }
    from std.amiga.intuition;
import { String, stringOf, toCString } from std.text;
import { AppIcon, AppWindow, AppMenuItem, AppMessage, APPMSG_APPICON,
         addAppIcon, removeAppIcon, addAppWindow, removeAppWindow,
         addAppMenuItem, removeAppMenuItem, waitAppMessage,
         appMessageType, appMessageId, appMessageUserData, appMessageClass,
         appMessageArgCount, replyAppMessage, savePublicScreen }
    from std.amiga.workbench;

func main(): i32 {
    let title: String = stringOf(\"Clorraine WB\");
    let screen: Screen? = openScreen(320, 80, 3, toCString(title));
    if (screen == null) {
        return 1;
    }
    let window: Window? = openWindow(screen, 8, 8, 300, 64, toCString(title));
    if (window == null) {
        return 2;
    }
    let portName: String = stringOf(\"ClorraineWbTest\");
    let port: MsgPort? = createPublicPort(toCString(portName));
    if (port == null) {
        return 3;
    }
    let icon: AppIcon? = addAppIcon(port, toCString(title), 7, 42);
    if (icon == null) {
        return 4;
    }
    let appWindow: AppWindow? = addAppWindow(window, port, 1, 0);
    if (appWindow == null) {
        return 5;
    }
    let item: AppMenuItem? = addAppMenuItem(port, toCString(title), 2, 0);
    if (item == null) {
        return 6;
    }
    let shotName: String = stringOf(\"Workbench\");
    let shotPath: String = stringOf(\"CNEXT:workbench.cscr\");
    let saved: bool = savePublicScreen(toCString(shotName), toCString(shotPath));
    let message: AppMessage? = waitAppMessage(port);
    if (message == null) {
        return 8;
    }
    let kind: i32 = appMessageType(message);
    let id: u32 = appMessageId(message);
    let user: u32 = appMessageUserData(message);
    let class: i32 = appMessageClass(message);
    let args: i32 = appMessageArgCount(message);
    let expected: i32 = APPMSG_APPICON;
    replyAppMessage(message);
    removeAppMenuItem(item);
    removeAppWindow(appWindow);
    removeAppIcon(icon);
    closeWindow(window);
    deleteMsgPort(port);
    return kind + i32(id) + i32(user) + class + args - expected + (saved ? 0 : 1);
}
";

// The Workbench surface compiles on hosted-amiga with zero diagnostics.
#[test]
fn the_workbench_surface_builds_on_hosted_amiga() {
    let scratch = ScratchDir::new("workbench-surface");
    let source_path = write_fixture(&scratch, "main.clo", WORKBENCH_SOURCE);
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
    assert_exit("workbench-surface", &output, 0);
    assert!(
        stderr(&output).is_empty(),
        "the Workbench surface must compile with zero diagnostics; stderr:\n{}",
        stderr(&output)
    );
    let c = std::fs::read_to_string(&output_path).expect("the emitted C is readable");
    for call in [
        "cl_add_app_icon",
        "cl_add_app_window",
        "cl_add_app_menu_item",
        "cl_wait_app_message",
        "cl_app_message_type",
        "cl_reply_app_message",
        "cl_save_public_screen",
    ] {
        assert!(c.contains(call), "the emitted C is missing {call}");
    }
}

// The Workbench module is 2.0+ only; the A500 profile rejects it.
#[test]
fn the_a500_profile_rejects_the_workbench_surface() {
    let scratch = ScratchDir::new("workbench-a500");
    let source_path = write_fixture(&scratch, "main.clo", WORKBENCH_SOURCE);
    let output_path = scratch.path().join("out.c");
    let output = run_clc(
        scratch.path(),
        &[
            "--profile",
            "hosted-amiga-13",
            "-o",
            output_path.to_str().expect("the output path is UTF-8"),
            source_path.to_str().expect("the source path is UTF-8"),
        ],
    );
    assert_exit("workbench-a500", &output, 1);
    assert!(
        stderr(&output).contains("E11200"),
        "expected E11200, got:\n{}",
        stderr(&output)
    );
    assert!(!output_path.exists(), "a rejected program must not emit C");
}

// The Workbench surface cross-builds with the 2.0 runtime.
#[test]
fn the_workbench_surface_cross_builds_with_nix20() {
    cross_build("workbench-build20", "hosted-amiga", "nix20", WORKBENCH_SOURCE);
}

// The stdlib appendix §14.2 documents every Workbench export.
#[test]
fn the_workbench_exports_match_the_stdlib_appendix() {
    let appendix = std::fs::read_to_string(repo_root().join("spec/appendix-stdlib.md"))
        .expect("the stdlib appendix is readable");
    let start = appendix
        .find("### 14.2 Workbench")
        .expect("the appendix has the section 14.2 heading");
    let section = &appendix[start..];
    let end = section.find("## 15.").unwrap_or(section.len());
    let section = &section[..end];
    let module =
        clc::stdlib::lookup("std.amiga.workbench").expect("the Workbench module is registered");
    for name in module.exports {
        assert!(
            mentions(section, name),
            "the appendix is missing the export '{name}'"
        );
    }
}
