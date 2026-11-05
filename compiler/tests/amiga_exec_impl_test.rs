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

/// The repository root, for the demo and appendix sources.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
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

const LIBRARY_SOURCE: &str = r#"module tests.exec.libimpl;

import { Library, openLibrary, closeLibrary, libraryVersion,
         libraryRevision } from std.amiga.exec;
import { String, stringOf, toCString } from std.text;

func main(): i32 {
    let name: String = stringOf("exec.library");
    let library: Library? = openLibrary(toCString(name), 33);
    if (library == null) { return 1; }
    let version: u16 = libraryVersion(library);
    let revision: u16 = libraryRevision(library);
    closeLibrary(library);
    return i32(version) + i32(revision);
}
"#;

const LIBRARY_DROP_SOURCE: &str = r#"module tests.exec.libdrop;

import { Library, openLibrary, libraryVersion } from std.amiga.exec;
import { String, stringOf, toCString } from std.text;

func main(): i32 {
    let name: String = stringOf("exec.library");
    let library: Library? = openLibrary(toCString(name), 33);
    if (library == null) { return 1; }
    return i32(libraryVersion(library));
}
"#;

const DEVICE_SOURCE: &str = r#"module tests.exec.deviceimpl;

import { Device, CMD_INVALID, CMD_RESET, CMD_READ, CMD_WRITE,
         CMD_UPDATE, CMD_CLEAR, CMD_STOP, CMD_START, CMD_FLUSH,
         openDevice, openConsole, closeDevice, deviceCommand,
         deviceSetBuffer, deviceSetOffset, deviceDoIO, deviceSendIO,
         deviceWaitIO, deviceCheckIO, deviceAbortIO, deviceActual,
         deviceError } from std.amiga.exec;
import { Window } from std.amiga.intuition;
import { String, stringOf, toCString } from std.text;
import { slice } from std.mem;

export func useDevice(window: Window): i32 {
    let name: String = stringOf("trackdisk.device");
    let created: Device? = openDevice(toCString(name), 0, 0);
    if (created == null) { return 1; }
    let device: Device = created;
    let buffer: u8[64] = [; 0];
    let command_ok: bool = deviceCommand(device, CMD_READ);
    if (!command_ok) { return 2; }
    let buffer_ok: bool = deviceSetBuffer(device, borrow slice(buffer, 0, 64));
    if (!buffer_ok) { return 3; }
    let offset_ok: bool = deviceSetOffset(device, 0);
    if (!offset_ok) { return 4; }
    let error: i32 = deviceDoIO(device);
    let actual: u32 = deviceActual(device);
    let last: i32 = deviceError(device);
    let send_ok: bool = deviceSendIO(device);
    if (!send_ok) { return 5; }
    let checked: bool = deviceCheckIO(device);
    let waited: i32 = deviceWaitIO(device);
    deviceAbortIO(device);
    closeDevice(device);

    let console_created: Device? = openConsole(window, 0);
    if (console_created == null) { return 6; }
    let console: Device = console_created;
    let console_command_ok: bool = deviceCommand(console, CMD_WRITE);
    if (!console_command_ok) { return 7; }
    let console_buffer_ok: bool =
        deviceSetBuffer(console, borrow slice(buffer, 0, 64));
    if (!console_buffer_ok) { return 8; }
    let consoleError: i32 = deviceDoIO(console);
    closeDevice(console);
    return error + last + waited + consoleError + i32(actual) +
           (checked ? 1 : 0) + i32(CMD_INVALID) + i32(CMD_RESET) +
           i32(CMD_UPDATE) + i32(CMD_CLEAR) + i32(CMD_STOP) +
           i32(CMD_START) + i32(CMD_FLUSH);
}

func main(): i32 {
    return 0;
}
"#;

/// The library wrappers route to the shim and use the contract `struct
/// Library *` type. The generated drop closes an unclosed library.
#[test]
fn the_library_wrappers_lower_to_the_contract_type() {
    let c = emit("library-routing", "hosted-amiga", LIBRARY_SOURCE);
    assert_before(
        "library-routing",
        &c,
        "struct Library * cl_std_amiga_exec_openLibrary(char *name, u16 version)",
        "return cl_open_library(name, version);",
    );
    assert_contains("library-routing", &c, "cl_close_library(library);");
    assert_contains("library-routing", &c, "cl_library_version(library);");
    assert_contains("library-routing", &c, "cl_library_revision(library);");

    let drop = emit("library-drop", "hosted-amiga-13", LIBRARY_DROP_SOURCE);
    assert_contains("library-drop", &drop, "if (library) { cl_close_library(library); }");
}

/// The device wrappers route to the shim, the buffer wrapper passes the
/// pointer and length explicitly, and the drop closes an unclosed device.
#[test]
fn the_device_wrappers_lower_to_the_shim() {
    let c = emit("device-routing", "hosted-amiga", DEVICE_SOURCE);
    for wrapper in [
        "cl_std_amiga_exec_openDevice",
        "cl_std_amiga_exec_openConsole",
        "cl_std_amiga_exec_closeDevice",
        "cl_std_amiga_exec_deviceCommand",
        "cl_std_amiga_exec_deviceSetBuffer",
        "cl_std_amiga_exec_deviceSetOffset",
        "cl_std_amiga_exec_deviceDoIO",
        "cl_std_amiga_exec_deviceSendIO",
        "cl_std_amiga_exec_deviceWaitIO",
        "cl_std_amiga_exec_deviceCheckIO",
        "cl_std_amiga_exec_deviceAbortIO",
        "cl_std_amiga_exec_deviceActual",
        "cl_std_amiga_exec_deviceError",
    ] {
        assert_contains("device-routing", &c, wrapper);
    }
    assert_contains(
        "device-routing",
        &c,
        "cl_device_set_buffer(device, (u8 *)(data.ptr), (u32)((data).len))",
    );
    assert_contains("device-routing", &c, "if (device) { cl_close_device(device); }");
}

/// The device shim owns a reply port and implements the documented pending
/// state machine and failure values.
#[test]
fn the_device_shim_pins_the_reply_port_and_state_machine() {
    let shim = shim();
    for needle in [
        "typedef struct cl_device",
        "struct IOStdReq request;",
        "struct MsgPort *port;",
        "int pending;",
        "device->port = CreatePort(NULL, 0);",
        "device->request.io_Message.mn_ReplyPort = device->port;",
        "static void cl_device_discard(cl_device *device)",
        "DeletePort(device->port);",
        "static void cl_device_close(cl_device *device)",
        "AbortIO((struct IORequest *)&device->request)",
        "WaitIO((struct IORequest *)&device->request)",
        "CloseDevice((struct IORequest *)&device->request)",
        "if (device == NULL || device->pending)",
        "return FALSE;",
        "return -1;",
        "device->request.io_Command = command;",
        "device->request.io_Data = data;",
        "device->request.io_Length = length;",
        "device->request.io_Offset = offset;",
        "device->pending = 1;",
        "device->pending = 0;",
        "CheckIO((struct IORequest *)&device->request) != NULL",
        "return (LONG)device->request.io_Error;",
        "return device == NULL ? 0 : device->request.io_Actual;",
        "return device == NULL ? -1 : (LONG)device->request.io_Error;",
    ] {
        assert_contains("device-shim", &shim, needle);
    }
}

const INTERRUPT_SOURCE: &str = r#"module tests.exec.interruptimpl;

import { HandlerToken, addHandler, addPortHandler,
         removeHandler } from std.amiga.interrupt;

volatile let ticks: u32 = 0;

func onTick(): void { ticks = ticks + 1; }
func onPort(): void { }

func main(): i32 {
    let vblank: HandlerToken = addHandler(onTick);
    let ports: HandlerToken = addPortHandler(onPort);
    removeHandler(vblank);
    removeHandler(ports);
    return i32(ticks);
}
"#;

/// The interrupt wrappers route to the shim, and the shim has two independent
/// server slots with one shared token counter.
#[test]
fn the_interrupt_wrappers_and_server_slots_are_pinned() {
    let c = emit("interrupt-routing", "hosted-amiga-13", INTERRUPT_SOURCE);
    for call in ["cl_add_handler", "cl_add_port_handler", "cl_remove_handler"] {
        assert_contains("interrupt-routing", &c, call);
    }
    assert_contains(
        "interrupt-routing",
        &c,
        "return (ClHandlerToken){ cl_add_port_handler(hook) };",
    );

    let shim = shim();
    for needle in [
        "static cl_handler_fn cl_vblank_handler;",
        "static cl_handler_fn cl_ports_handler;",
        "static struct Interrupt cl_ports_server;",
        "static ULONG cl_ports_token;",
        "static ULONG cl_next_token = 1;",
        "static ULONG cl_take_token(void)",
        "static void cl_ports_code(void)",
        "ULONG cl_add_port_handler(cl_handler_fn handler)",
        "RemIntServer(INTB_PORTS, &cl_ports_server);",
        "AddIntServer(INTB_PORTS, &cl_ports_server);",
        "id == cl_vblank_token && cl_vblank_handler != NULL",
        "id == cl_ports_token && cl_ports_handler != NULL",
    ] {
        assert_contains("interrupt-shim", &shim, needle);
    }
}

/// The A500 demo pins the worker stress loop, the console helper, the
/// trackdisk/console/input/keyboard sequence, and the report line.
#[test]
fn the_exec_demo_pins_the_worker_and_device_sequence() {
    let path = repo_root().join("examples/amiga/exec-demo.clo");
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
    for needle in [
        "const WORKER_SUM: i32 = 55;",
        "const WORKER_PRODUCT: i32 = 24;",
        "const WORKER_COUNT: i32 = 8;",
        "const CONSOLE_LENGTH: u32 = 13;",
        "func workerSum(): void",
        "func workerProduct(): void",
        "while (index < WORKER_COUNT)",
        "func writeConsole(): ConsoleResult",
        "openConsole(window, 0)",
        "openDevice(toCString(stringOf(\"trackdisk.device\")), 0, 0)",
        "deviceSendIO(disk)",
        "deviceWaitIO(disk)",
        "openDevice(toCString(stringOf(\"input.device\")), 0, 0)",
        "openDevice(toCString(stringOf(\"keyboard.device\")), 0, 0)",
        "deviceCommand(console, CMD_READ)",
        "deviceSendIO(console)",
        "deviceAbortIO(console)",
        "keyError != -2",
        "exec: ok task {result.task} lib {result.lib} dev {result.dev} io {result.io} key {result.key} in {result.input}",
        "DF0:exec.txt",
    ] {
        assert!(
            source.contains(needle),
            "the exec demo is missing {needle:?}"
        );
    }
}

/// The console helper fills the request from the caller's Intuition window and
/// the device request uses the 1.3 `IOStdReq`/`exec/io.h` types.
#[test]
fn the_console_helper_pins_the_window_request() {
    let shim = shim();
    for needle in [
        "#include <exec/io.h>",
        "void *cl_open_console(void *window_handle, UBYTE unit)",
        "const cl_window *window = (const cl_window *)window_handle;",
        "window->window == NULL",
        "device->request.io_Data = (APTR)window->window;",
        "device->request.io_Length = (ULONG)sizeof(struct Window);",
        "OpenDevice((STRPTR)\"console.device\", (ULONG)unit,",
        "(struct IORequest *)&device->request",
        "MEMF_PUBLIC | MEMF_CLEAR",
    ] {
        assert_contains("console-shim", &shim, needle);
    }
}
