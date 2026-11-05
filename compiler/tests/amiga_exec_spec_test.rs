//! End-to-end specification tests for the Amiga `exec` surface.
//!
//! The host cases always run: the host profile must reject the module and the
//! hosted-amiga emitter must name the shim calls. The program cases need the
//! cross toolchain and `vamos`; they skip themselves with a note when those
//! tools are absent, matching the existing hosted-amiga suites.

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
            "clorraine-amiga-{}-{tag}-{sequence}",
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

fn run_binary(binary: &str, cwd: &Path, args: &[&str]) -> Output {
    Command::new(binary)
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap_or_else(|error| panic!("failed to spawn {binary}: {error}"))
}

fn run_clc(cwd: &Path, args: &[&str]) -> Output {
    run_binary(env!("CARGO_BIN_EXE_clc"), cwd, args)
}

fn run_clorraine(cwd: &Path, args: &[&str]) -> Output {
    run_binary(env!("CARGO_BIN_EXE_clorraine"), cwd, args)
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn assert_exit(tag: &str, output: &Output, expected: i32) {
    assert_eq!(
        output.status.code(),
        Some(expected),
        "{tag}: expected exit {expected}; stdout:\n{}\nstderr:\n{}",
        stdout(output),
        stderr(output)
    );
}

fn tool_available(tool: &str) -> bool {
    Command::new(tool).arg("--version").output().is_ok()
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// A source that imports the module and calls every surface the emitter needs.
const EXEC_SOURCE: &str = "\
module tests.exec;

import { MsgPort, NewMessage, createMsgPort, deleteMsgPort, createMessage, putMsg, signals, waitSignals, getMsg, replyMsg } from std.amiga.exec;

func main(): i32 {
    let created: MsgPort? = createMsgPort();
    if (created == null) {
        return 1;
    }
    let port: MsgPort = created;
    let message_opt: NewMessage? = createMessage(port, 7);
    if (message_opt != null) {
        let message: NewMessage = message_opt;
        putMsg(port, message);
    }
    let mask: u32 = signals(port);
    let _received: u32 = waitSignals(mask);
    let message = getMsg(port);
    if (message != null) {
        replyMsg(message);
    }
    deleteMsgPort(port);
    return 0;
}
";

/// The worker-task surface: create a capture-less task, report a result, and
/// join it. This is the Phase 1 part of the bundle's full surface.
const TASK_SOURCE: &str = "\
module tests.exec.task;

import { Task, createTask, reportTaskResult, joinTask } from std.amiga.exec;

func worker(): void {
    reportTaskResult(42);
}

func main(): i32 {
    let task: Task? = createTask(worker, 8192);
    if (task == null) {
        return 1;
    }
    let result: i32 = joinTask(task);
    return result == 42 ? 0 : 2;
}
";

// The worker-task surface compiles on both hosted profiles and names its shim
// calls. The A500 runtime proof is the demo; this case is the compile gate.
#[test]
fn the_task_surface_builds_on_both_hosted_profiles() {
    for profile in ["hosted-amiga", "hosted-amiga-13"] {
        let scratch = ScratchDir::new("task-surface");
        let source_path = write_fixture(&scratch, "main.clo", TASK_SOURCE);
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
            "{profile}: the task surface must compile with zero diagnostics; stderr:\n{}",
            stderr(&output)
        );
        let c = std::fs::read_to_string(&output_path).expect("the emitted C is readable");
        for call in [
            "cl_create_task",
            "cl_task_result",
            "cl_join_task",
            "cl_task_free",
        ] {
            assert!(
                c.contains(call),
                "{profile}: the emitted C is missing {call}"
            );
        }
    }
}

// The worker-task surface cross-builds with the 1.3 and 2.0 runtimes. The
// host-only CI has no m68k compiler, so the case skips itself there.
#[test]
fn the_task_surface_cross_builds_with_nix13_and_nix20() {
    if !tool_available("m68k-amigaos-gcc") {
        eprintln!("skip: m68k-amigaos-gcc is not on PATH");
        return;
    }
    for (tag, profile, crt) in [
        ("task-build13", "hosted-amiga-13", "nix13"),
        ("task-build20", "hosted-amiga", "nix20"),
    ] {
        let scratch = ScratchDir::new(tag);
        let source_path = write_fixture(&scratch, "main.clo", TASK_SOURCE);
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
        let binary = scratch.path().join("task");
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
            "{tag}: the task surface must build with {crt}; stderr:\n{}",
            stderr(&build)
        );
    }
}

// The host profile must reject the Amiga module with the profile diagnostic.
#[test]
fn the_host_profile_rejects_the_exec_module() {
    let scratch = ScratchDir::new("host-reject");
    write_fixture(&scratch, "main.clo", EXEC_SOURCE);
    let output = run_clc(
        scratch.path(),
        &["--profile", "host", "-o", "out.c", "main.clo"],
    );
    assert_exit("host rejection", &output, 1);
    assert!(
        stderr(&output).contains("E11200"),
        "expected E11200, got:\n{}",
        stderr(&output)
    );
    assert!(
        !scratch.path().join("out.c").exists(),
        "a rejected program must not emit C"
    );
}

// The hosted-amiga emitter must name every shim call the wrapper uses.
#[test]
fn the_hosted_amiga_emitter_names_the_exec_shim_calls() {
    let scratch = ScratchDir::new("emit");
    write_fixture(&scratch, "main.clo", EXEC_SOURCE);
    let output = run_clc(
        scratch.path(),
        &["--profile", "hosted-amiga", "-o", "out.c", "main.clo"],
    );
    assert_exit("hosted-amiga emission", &output, 0);
    let c = std::fs::read_to_string(scratch.path().join("out.c"))
        .expect("the emitted C is readable");
    for call in [
        "cl_create_msg_port",
        "cl_create_message",
        "cl_put_msg",
        "cl_signals",
        "cl_wait_signals",
        "cl_get_msg",
        "cl_reply_msg",
        "cl_delete_msg_port",
    ] {
        assert!(c.contains(call), "the emitted C is missing {call}");
    }
}

// The first-party acceptance program must match its fixed transcript.
#[test]
fn the_exec_self_test_matches_its_transcript() {
    if !tool_available("m68k-amigaos-gcc") || !tool_available("vamos") {
        eprintln!("skip: the hosted-amiga toolchain or vamos is absent");
        return;
    }
    let scratch = ScratchDir::new("self-test");
    let program = repo_root().join("examples/amiga/exec-self-test.clo");
    let expected = std::fs::read_to_string(repo_root().join(
        "examples/amiga/exec-self-test.expected.txt",
    ))
    .expect("the expected transcript is readable");
    for run in 0..2 {
        let output = run_clorraine(
            scratch.path(),
            &[
                "run",
                "--profile",
                "hosted-amiga",
                program.to_str().expect("the program path is UTF-8"),
            ],
        );
        assert_exit(&format!("exec-self-test run {run}"), &output, 0);
        assert_eq!(stdout(&output), expected, "run {run}");
    }
}

// The stdlib appendix must document every exported exec name.
#[test]
fn the_exec_exports_match_the_stdlib_appendix() {
    let appendix = std::fs::read_to_string(repo_root().join("spec/appendix-stdlib.md"))
        .expect("the stdlib appendix is readable");
    let start = appendix
        .find("## 13. Exec")
        .expect("the appendix has the exec section");
    let section = &appendix[start..];
    let end = section.find("## 14.").unwrap_or(section.len());
    let section = &section[..end];
    let module = clc::stdlib::lookup("std.amiga.exec").expect("the exec module is registered");
    for name in module.exports {
        assert!(
            mentions(section, name),
            "the appendix is missing the export '{name}'"
        );
    }
}

// True when `name` appears as a whole identifier in `text`, so `Message` is
// not satisfied by `NewMessage`.
fn mentions(text: &str, name: &str) -> bool {
    text.split(|character: char| !(character.is_ascii_alphanumeric() || character == '_'))
        .any(|token| token == name)
}

// The handle drop routing must free unsent messages and reply fetched ones.
#[test]
fn the_message_drop_routes_each_handle() {
    if !tool_available("m68k-amigaos-gcc") || !tool_available("vamos") {
        eprintln!("skip: the hosted-amiga toolchain or vamos is absent");
        return;
    }
    let scratch = ScratchDir::new("message-drop");
    let program = repo_root().join("examples/amiga/message-drop.clo");
    let output = run_clorraine(
        scratch.path(),
        &[
            "run",
            "--profile",
            "hosted-amiga",
            program.to_str().expect("the program path is UTF-8"),
        ],
    );
    assert_exit("message-drop", &output, 0);
    let expected = std::fs::read_to_string(repo_root().join(
        "examples/amiga/message-drop.expected.txt",
    ))
    .expect("the expected transcript is readable");
    assert_eq!(stdout(&output), expected);
}

// Hamurabi must match its deterministic transcript.
#[test]
fn hamurabi_matches_its_transcript() {
    if !tool_available("m68k-amigaos-gcc") || !tool_available("vamos") {
        eprintln!("skip: the hosted-amiga toolchain or vamos is absent");
        return;
    }
    let scratch = ScratchDir::new("hamurabi");
    let program = repo_root().join("examples/amiga/hamurabi.clo");
    let output = run_clorraine(
        scratch.path(),
        &[
            "run",
            "--profile",
            "hosted-amiga",
            program.to_str().expect("the program path is UTF-8"),
        ],
    );
    assert_exit("hamurabi", &output, 0);
    let expected = std::fs::read_to_string(repo_root().join(
        "examples/amiga/hamurabi.expected.txt",
    ))
    .expect("the expected transcript is readable");
    assert_eq!(stdout(&output), expected);
}
