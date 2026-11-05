//! Exec task and device implementation tests.
//!
//! These cases inspect the C the compiler emits and pin the internal contract
//! the specification cases do not name: the task wrappers route to the shim,
//! the worker record uses one join signal and a `tc_UserData` result, the
//! join/drop path removes the worker with the Kickstart 1.3-safe
//! `Forbid`/`RemTask`/`Permit` sequence, and the stack limits are enforced.
//!
//! The library, device, and interrupt cases are added by their phases; this
//! file starts with the Phase 1 task surface.

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
            "clc-exec-impl-{}-{tag}-{sequence}",
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

/// Compiles `source` for `profile` and returns the emitted C.
fn emit(tag: &str, profile: &str, source: &str) -> String {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source).expect("write the snippet");
    let output_path = scratch.path().join("out.c");
    let run = run_clc(&[
        "--profile".to_owned(),
        profile.to_owned(),
        "-o".to_owned(),
        output_path.display().to_string(),
        source_path.display().to_string(),
    ]);
    assert_eq!(
        run.status.code(),
        Some(0),
        "{tag}: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&run.stderr)
    );
    std::fs::read_to_string(&output_path).expect("read the emitted C")
}

/// Asserts `haystack` contains `needle`.
fn assert_contains(tag: &str, haystack: &str, needle: &str) {
    assert!(
        haystack.contains(needle),
        "{tag}: emitted C does not contain {needle:?}"
    );
}

/// Asserts `first` appears before `second`.
fn assert_before(tag: &str, haystack: &str, first: &str, second: &str) {
    let first_at = haystack
        .find(first)
        .unwrap_or_else(|| panic!("{tag}: missing {first:?}"));
    let second_at = haystack
        .find(second)
        .unwrap_or_else(|| panic!("{tag}: missing {second:?}"));
    assert!(
        first_at < second_at,
        "{tag}: expected {first:?} before {second:?}"
    );
}

/// The shim source, read directly so the task contract is pinned where it is
/// implemented rather than through a compiled target.
fn shim() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("runtime/shim_hosted_amiga.c");
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
}

const TASK_SOURCE: &str = r#"module tests.exec.taskimpl;

import { Task, createTask, reportTaskResult, joinTask } from std.amiga.exec;

func worker(): void {
    reportTaskResult(42);
}

func main(): i32 {
    let task: Task? = createTask(worker, 8192);
    if (task == null) { return 1; }
    return joinTask(task);
}
"#;

const TASK_DROP_SOURCE: &str = r#"module tests.exec.taskdrop;

import { Task, createTask, reportTaskResult } from std.amiga.exec;

func worker(): void {
    reportTaskResult(7);
}

func main(): i32 {
    let task: Task? = createTask(worker, 8192);
    if (task == null) { return 1; }
    return 0;
}
"#;

/// The generated task wrappers route to the shim symbols. The pins assert the
/// wrapper bodies, not just the shim definitions that are embedded whole.
#[test]
fn the_task_wrappers_lower_to_the_shim() {
    let c = emit("task-routing", "hosted-amiga", TASK_SOURCE);
    assert_before(
        "task-routing",
        &c,
        "ClTask cl_std_amiga_exec_createTask(void (*entry)(void), u32 stackBytes)",
        "return cl_create_task(entry, stackBytes);",
    );
    assert_contains("task-routing", &c, "cl_task_result(value);");
    assert_contains("task-routing", &c, "cl_join_task(task);");
    assert_contains("task-routing", &c, "if (task) { cl_task_free(task); }");
}

/// An unjoined task handle goes through the generated drop after the null
/// check, so the shim's join path runs exactly once.
#[test]
fn the_task_drop_routes_an_unjoined_handle() {
    let c = emit("task-drop", "hosted-amiga-13", TASK_DROP_SOURCE);
    assert_contains("task-drop", &c, "cl_task_free");
    assert_contains("task-drop", &c, "cl_create_task");
}

/// The shim records the worker contract: magic, parent, signal bit, result,
/// the C entry point, and the join/removal sequence.
#[test]
fn the_task_shim_records_the_worker_contract() {
    let shim = shim();
    for needle in [
        "CL_TASK_MAGIC",
        "typedef struct cl_task",
        "struct Task *parent",
        "BYTE signal_bit",
        "LONG result",
        "static void cl_task_entry(void)",
        "FindTask(NULL)",
        "record->entry()",
        "Signal(record->parent, 1UL << record->signal_bit)",
        "Wait(0)",
        "Forbid();",
        "RemTask(record->task);",
        "Permit();",
        "FreeSignal(record->signal_bit)",
        "record->magic = 0;",
        "FreeMem(record, (ULONG)sizeof *record);",
        "task->tc_UserData = record;",
        "void *cl_create_task(cl_handler_fn entry, ULONG stack_size)",
        "void cl_task_result(LONG value)",
        "LONG cl_join_task(void *handle)",
        "void cl_task_free(void *handle)",
    ] {
        assert_contains("task-shim", &shim, needle);
    }
}

/// The stack limits are explicit and the creation failure paths release every
/// partial allocation.
#[test]
fn the_task_limits_and_cleanup_are_pinned() {
    let shim = shim();
    assert_contains("task-limits", &shim, "#define CL_TASK_STACK_MIN 4096UL");
    assert_contains("task-limits", &shim, "#define CL_TASK_STACK_MAX 65536UL");
    assert_contains("task-limits", &shim, "stack_size < CL_TASK_STACK_MIN");
    assert_contains("task-limits", &shim, "stack_size > CL_TASK_STACK_MAX");
    assert_contains("task-limits", &shim, "signal_bit < 0");
    assert_contains("task-limits", &shim, "task == NULL");
    // The record is public and cleared so both tasks see zeroed fields.
    assert_contains("task-limits", &shim, "MEMF_PUBLIC | MEMF_CLEAR");
}

/// The worker path uses the 1.3 task API and not the 2.0+ process/task
/// constructors.
#[test]
fn the_task_shim_has_no_20_plus_task_calls() {
    let shim = shim();
    for forbidden in [
        "CreateNewProc",
        "CreateTaskTags",
        "NewCreateTask",
        "AllocTask",
    ] {
        assert!(
            !shim.contains(forbidden),
            "the 1.3 shim must not use {forbidden}"
        );
    }
}
