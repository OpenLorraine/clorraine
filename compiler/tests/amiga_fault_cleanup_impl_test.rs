//! Implementation tests for the hosted-amiga exit-time resource registry.
//!
//! The specification cases prove that faulting programs leave nothing behind
//! under `vamos`. These cases pin the internals that make that true: the
//! registry's list operations, the exit hook's pop-before-release loop, the
//! fault paths staying free of registry code, and each handle kind registering
//! after a successful acquire and unregistering where the resource is really
//! freed or handed to a port. Two `vamos` cases exercise paths the
//! specification probes do not reach: a full normal release of every recorded
//! kind, and a message whose reply port is released before the port holding
//! it.

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
            "clc-fault-cleanup-impl-{}-{tag}-{sequence}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path)
            .unwrap_or_else(|error| panic!("cannot create {}: {error}", path.display()));
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

/// The base hosted-amiga shim source, read directly so each contract is pinned
/// where it is implemented.
fn shim() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("runtime/shim_hosted_amiga.c");
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
}

/// The text of the function whose definition starts with `signature`, up to
/// its closing brace in column 0.
fn body<'a>(source: &'a str, signature: &str) -> &'a str {
    let start = source
        .find(&format!("\n{signature}\n{{"))
        .unwrap_or_else(|| panic!("the shim has no definition {signature:?}"))
        + 1;
    let rest = &source[start..];
    let end = rest
        .find("\n}\n")
        .unwrap_or_else(|| panic!("{signature:?} has no closing brace"));
    &rest[..end + 2]
}

/// Asserts `haystack` contains `needle`.
fn assert_contains(tag: &str, haystack: &str, needle: &str) {
    assert!(
        haystack.contains(needle),
        "{tag}: missing {needle:?} in:\n{haystack}"
    );
}

/// Asserts `haystack` does not contain `needle`.
fn assert_lacks(tag: &str, haystack: &str, needle: &str) {
    assert!(
        !haystack.contains(needle),
        "{tag}: unexpected {needle:?} in:\n{haystack}"
    );
}

/// Asserts that each needle appears, in the given order.
fn assert_order(tag: &str, haystack: &str, needles: &[&str]) {
    let mut from = 0;
    for needle in needles {
        let at = haystack[from..]
            .find(needle)
            .unwrap_or_else(|| panic!("{tag}: missing {needle:?} after offset {from} in:\n{haystack}"));
        from += at + needle.len();
    }
}

/// The registry operations: embedded nodes link without allocation, side
/// records are allocated, every link installs the hook first, removal is
/// idempotent, and a side-record lookup searches newest first and matches the
/// release function as well as the handle.
#[test]
fn the_registry_operations_are_pinned() {
    let shim = shim();
    let install = body(&shim, "static int cl_exit_hook_install(void)");
    assert_order(
        "install",
        install,
        &["if (!cl_exit_hook_installed)", "atexit(cl_exit_hook) != 0", "return 0;", "cl_exit_hook_installed = 1;"],
    );

    let add = body(&shim, "int cl_res_add(cl_res *node, void (*release)(void *), void *handle)");
    assert_order("add", add, &["cl_exit_hook_install()", "node->side = 0;", "cl_res_link(node, release, handle);"]);
    assert_lacks("add", add, "AllocMem");

    let remove = body(&shim, "void cl_res_remove(cl_res *node)");
    assert_order("remove", remove, &["if (node->linked)", "cl_res_unlink(node);"]);

    let track = body(&shim, "int cl_res_track(void (*release)(void *), void *handle)");
    assert_order(
        "track",
        track,
        &[
            "cl_exit_hook_install()",
            "AllocMem((ULONG)sizeof *node, MEMF_PUBLIC | MEMF_CLEAR)",
            "if (node == NULL)",
            "node->side = 1;",
            "cl_res_link(node, release, handle);",
        ],
    );

    let untrack = body(&shim, "int cl_res_untrack(void (*release)(void *), void *handle)");
    assert_order(
        "untrack",
        untrack,
        &[
            "node = cl_res_tail; node != NULL; node = node->prev",
            "node->side && node->release == release && node->handle == handle",
            "cl_res_unlink(node);",
            "FreeMem(node, (ULONG)sizeof *node);",
            "return 1;",
            "return 0;",
        ],
    );

    let unlink = body(&shim, "static void cl_res_unlink(cl_res *node)");
    assert_contains("unlink", unlink, "node->linked = 0;");
    let link = body(&shim, "static void cl_res_link(cl_res *node, void (*release)(void *), void *handle)");
    assert_order("link", link, &["node->prev = cl_res_tail;", "cl_res_tail = node;", "node->linked = 1;"]);
}

/// The hook marks the exit first, rereads the newest record on every pass,
/// pops it, frees a side record, and only then releases the handle from the
/// copied fields. The loop keeps no saved neighbour, because a release may
/// remove other records.
#[test]
fn the_exit_hook_pops_each_record_before_releasing_it() {
    let shim = shim();
    let hook = body(&shim, "void cl_exit_hook(void)");
    assert_order(
        "hook",
        hook,
        &[
            "cl_exiting = 1;",
            "while ((node = cl_res_tail) != NULL)",
            "void (*release)(void *) = node->release;",
            "void *handle = node->handle;",
            "UBYTE side = node->side;",
            "cl_res_unlink(node);",
            "FreeMem(node, (ULONG)sizeof *node);",
            "release(handle);",
        ],
    );
    assert_lacks("hook", hook, "->next");
    assert_lacks("hook", hook, "->prev");
    assert_lacks("hook", hook, "printf");
}

/// The fault and panic paths reference no registry or hook code, so a program
/// with a fault site links none of it, and they keep their plain `exit(20)`.
#[test]
fn the_fault_paths_name_no_registry_code() {
    let shim = shim();
    for signature in [
        "static inline void cl_trap(const char *code, const char *message)",
        "static inline void cl_panic(const unsigned char *message, unsigned long len)",
    ] {
        let path = body(&shim, signature);
        assert_contains(signature, path, "exit(20);");
        for name in ["cl_exit_hook", "cl_res", "atexit", "cl_exiting"] {
            assert_lacks(signature, path, name);
        }
    }
}

/// DOS kinds: a lock gets a side record after `Lock` succeeds and loses it
/// before `UnLock`; the info blocks are recorded through their embedded node
/// and removed before `FreeMem`. Each failed registration undoes the acquire.
#[test]
fn the_dos_kinds_register_after_success_and_unregister_before_release() {
    let shim = shim();
    let lock = body(&shim, "BPTR cl_dos_lock(char *path)");
    assert_order(
        "lock",
        lock,
        &["lock = Lock(path, SHARED_LOCK);", "!cl_res_track(cl_release_lock, (void *)lock)", "UnLock(lock);", "return 0;"],
    );
    let unlock = body(&shim, "void cl_dos_unlock(BPTR lock)");
    assert_order("unlock", unlock, &["cl_res_untrack(cl_release_lock, (void *)lock);", "UnLock(lock);"]);
    assert_contains("lock thunk", body(&shim, "static void cl_release_lock(void *handle)"), "UnLock((BPTR)handle);");

    for (kind, new_signature, free_signature, free_name) in [
        (
            "pathinfo",
            "void *cl_dos_pathinfo_new(char *path)",
            "void cl_dos_pathinfo_free(void *handle)",
            "cl_dos_pathinfo_free",
        ),
        (
            "volumeinfo",
            "void *cl_dos_volumeinfo_new(char *path)",
            "void cl_dos_volumeinfo_free(void *handle)",
            "cl_dos_volumeinfo_free",
        ),
    ] {
        let acquire = body(&shim, new_signature);
        let register = format!("!cl_res_add(&info->res, {free_name}, info)");
        assert_order(
            kind,
            acquire,
            &[&register, "FreeMem(info, (ULONG)sizeof *info);", "return NULL;", "return info;"],
        );
        // The temporary lock is gone before the block is recorded, so the
        // record never outlives a lock it does not own.
        let last_unlock = acquire.rfind("UnLock(lock);").expect("the temporary lock is released");
        assert!(
            last_unlock < acquire.find(&register).expect("registered"),
            "{kind}: the info block must be recorded after its temporary lock is released"
        );
        let release = body(&shim, free_signature);
        assert_order(kind, release, &["cl_res_remove(&info->res);", "FreeMem(info, (ULONG)sizeof *info);"]);
    }
}

/// Ports get a side record whose exit release frees the program's own
/// messages, and the user-facing delete removes the record before
/// `DeletePort`.
#[test]
fn ports_register_through_one_helper_and_unregister_before_delete() {
    let shim = shim();
    let recorded = body(&shim, "static void *cl_msg_port_recorded(struct MsgPort *port)");
    assert_order(
        "port record",
        recorded,
        &["port != NULL && !cl_res_track(cl_release_msg_port, port)", "DeletePort(port);", "return NULL;"],
    );
    assert_contains(
        "private port",
        body(&shim, "void *cl_create_msg_port(void)"),
        "cl_msg_port_recorded(CreatePort(NULL, 0))",
    );
    assert_contains(
        "public port",
        body(&shim, "void *cl_create_public_port(char *name)"),
        "cl_msg_port_recorded(CreatePort((STRPTR)name, 0))",
    );
    assert_order(
        "delete port",
        body(&shim, "void cl_delete_msg_port(void *port_handle)"),
        &["cl_res_untrack(cl_release_msg_port, port);", "DeletePort(port);"],
    );
}

/// The exit release of a port runs its list walks under Forbid. It frees the
/// own messages queued on it, frees own messages naming it as reply port that
/// another own port holds, and keeps the port only when a foreign task holds
/// such a message. `DeletePort` runs after Permit and only when nothing keeps
/// the port.
#[test]
fn the_port_exit_release_frees_own_messages_and_keeps_foreign_holders() {
    let shim = shim();
    let release = body(&shim, "static void cl_release_msg_port(void *handle)");
    assert_order(
        "port release",
        release,
        &[
            "Forbid();",
            "node = port->mp_MsgList.lh_Head; node->ln_Succ != NULL; node = next",
            "next = node->ln_Succ;",
            "message = cl_flight_find(node);",
            "Remove(node);",
            "cl_flight_remove(message);",
            "FreeMem(message, (ULONG)sizeof *message);",
            "message->msg.mn_ReplyPort != port",
            "cl_own_port_holds(message)",
            "Remove(&message->msg.mn_Node);",
            "keep = 1;",
            "Permit();",
            "if (!keep)",
            "DeletePort(port);",
        ],
    );
    let holds = body(&shim, "static int cl_own_port_holds(cl_message *message)");
    assert_order(
        "own port holds",
        holds,
        &["record = cl_res_head; record != NULL; record = record->next", "record->release == cl_release_msg_port", "cl_port_holds("],
    );
    // The in-flight lookup compares pointers only, so a foreign message is
    // never read as one of the program's own.
    let find = body(&shim, "static cl_message *cl_flight_find(const void *pointer)");
    assert_contains("flight find", find, "(const void *)message == pointer");
}

/// Messages: a new message is recorded through its node; `putMsg` hands it to a
/// port (record removed, in-flight list joined) before `PutMsg`; `getMsg`
/// takes an own message back out of the in-flight list and records it again,
/// and gives a foreign message a side record; replying an own message hands it
/// back to a port; freeing removes the record first.
#[test]
fn messages_move_between_the_registry_and_the_in_flight_list() {
    let shim = shim();
    let create = body(&shim, "void *cl_create_message(void *reply_handle, LONG payload)");
    assert_order(
        "create",
        create,
        &["message->payload = payload;", "!cl_res_add(&message->res, cl_free_message, message)", "FreeMem(message, sizeof *message);", "return NULL;"],
    );
    assert_order(
        "put",
        body(&shim, "void cl_put_msg(void *port_handle, void *message_handle)"),
        &["cl_res_remove(&message->res);", "cl_flight_add(message);", "PutMsg("],
    );
    assert_order(
        "get",
        body(&shim, "void *cl_get_msg(void *port_handle)"),
        &[
            "GetMsg(",
            "own = cl_flight_find(message);",
            "cl_flight_remove(own);",
            "!cl_res_add(&own->res, cl_free_message, own)",
            "return own;",
            "!cl_res_track(cl_release_foreign_message, message)",
            "cl_release_foreign_message(message);",
            "return NULL;",
            "return message;",
        ],
    );
    assert_order(
        "reply",
        body(&shim, "void cl_reply_msg(void *message_handle)"),
        &[
            "cl_res_untrack(cl_release_foreign_message, message)",
            "cl_release_foreign_message(message);",
            "return;",
            "message->mn_ReplyPort == NULL",
            "cl_res_remove(&own->res);",
            "cl_flight_add(own);",
            "ReplyMsg(message);",
        ],
    );
    assert_order(
        "free",
        body(&shim, "void cl_free_message(void *message_handle)"),
        &["cl_res_remove(&message->res);", "FreeMem(message, sizeof *message);"],
    );
    assert_order(
        "release",
        body(&shim, "void cl_release_message(void *message_handle)"),
        &["cl_res_untrack(cl_release_foreign_message, message_handle)", "FreeMem(", "cl_free_message(message_handle);"],
    );
    assert_contains(
        "foreign thunk",
        body(&shim, "static void cl_release_foreign_message(void *handle)"),
        "ReplyMsg(message);",
    );
}

/// The user library and the generic devices: recorded after a successful
/// open, closed again when the record fails, and unrecorded where they close.
#[test]
fn libraries_and_devices_register_after_open() {
    let shim = shim();
    assert_order(
        "open library",
        body(&shim, "struct Library *cl_open_library(char *name, UWORD version)"),
        &["library = OpenLibrary(", "!cl_res_track(cl_release_library, library)", "CloseLibrary(library);", "return NULL;"],
    );
    assert_order(
        "close library",
        body(&shim, "void cl_close_library(struct Library *library)"),
        &["cl_res_untrack(cl_release_library, library);", "CloseLibrary(library);"],
    );
    assert_order(
        "device record",
        body(&shim, "static void *cl_device_recorded(cl_device *device)"),
        &["!cl_res_add(&device->res, cl_release_device, device)", "cl_device_close(device);", "return NULL;"],
    );
    for signature in [
        "void *cl_open_device(char *name, LONG unit, ULONG flags)",
        "void *cl_open_console(void *window_handle, UBYTE unit)",
    ] {
        assert_order(signature, body(&shim, signature), &["OpenDevice(", "cl_device_discard(device);", "return cl_device_recorded(device);"]);
    }
    assert_order(
        "device close",
        body(&shim, "static void cl_device_close(cl_device *device)"),
        &["cl_res_remove(&device->res);", "CloseDevice("],
    );
}

fn tool_available(tool: &str) -> bool {
    Command::new(tool).arg("--version").output().is_ok()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace('\r', "")
}

/// Emits `source` for `profile`, builds it with warnings as errors, and runs it
/// under `vamos`. Returns the exit code and all output.
fn build_and_run(tag: &str, profile: &str, crt: &str, source: &str) -> (Option<i32>, String) {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source).expect("write the probe");
    let c_path = scratch.path().join("out.c");
    let emit: Output = Command::new(env!("CARGO_BIN_EXE_clc"))
        .args(["--profile", profile, "-o"])
        .arg(&c_path)
        .arg(&source_path)
        .output()
        .expect("run clc");
    assert!(emit.status.success() && emit.stderr.is_empty(), "{tag}: clc:\n{}", text(&emit.stderr));
    let binary = scratch.path().join("probe");
    let gcc = Command::new("m68k-amigaos-gcc")
        .args(["-std=c99", &format!("-mcrt={crt}"), "-m68000", "-fwrapv", "-O0", "-g", "-Wall", "-Wextra", "-Werror", "-o"])
        .arg(&binary)
        .arg(&c_path)
        .args(["-lamiga", "-lm"])
        .output()
        .expect("run m68k-amigaos-gcc");
    assert!(gcc.status.success(), "{tag}: gcc:\n{}", text(&gcc.stderr));
    let run = Command::new("timeout")
        .arg("120")
        .arg("vamos")
        .arg(&binary)
        .current_dir(scratch.path())
        .output()
        .expect("run vamos");
    (run.status.code(), format!("{}{}", text(&run.stdout), text(&run.stderr)))
}

/// True for a `vamos` report of something left behind at shutdown.
fn leaked(output: &str) -> bool {
    output
        .lines()
        .any(|line| line.contains("can't expunge") || (line.contains("mem_alloc:WARNING:") && line.contains("orphan")))
}

/// Acquires every phase-one kind and releases each through its own API, then
/// returns normally. Every record must be removed at its real release point:
/// a missed removal would release the resource a second time at exit.
const NORMAL_RELEASE_SOURCE: &str = r#"module tests.fault_cleanup.normal_release;

import { Lock, PathInfo, VolumeInfo, lockPath, unlockPath, pathInfo, volumeInfo } from std.amiga.dos;
import { MsgPort, NewMessage, Message, Library, createMsgPort, createPublicPort, deleteMsgPort,
         createMessage, putMsg, getMsg, replyMsg, releaseMsg, openLibrary, closeLibrary } from std.amiga.exec;
import { println } from std.io;

func main(): i32 {
    let lock: Lock? = lockPath("");
    if (lock == null) { return 1; }
    unlockPath(lock);
    let info: PathInfo? = pathInfo("");
    if (info == null) { return 2; }
    let volume: VolumeInfo? = volumeInfo("");
    if (volume == null) { return 3; }
    let library: Library? = openLibrary("dos.library", 33);
    if (library == null) { return 4; }
    closeLibrary(library);
    let public_opt: MsgPort? = createPublicPort("ClFaultCleanupImpl");
    if (public_opt == null) { return 5; }
    deleteMsgPort(public_opt);
    let reply_opt: MsgPort? = createMsgPort();
    if (reply_opt == null) { return 6; }
    let reply: MsgPort = reply_opt;
    let dest_opt: MsgPort? = createMsgPort();
    if (dest_opt == null) { return 7; }
    let dest: MsgPort = dest_opt;
    let unsent: NewMessage? = createMessage(reply, 1);
    if (unsent == null) { return 8; }
    let message_opt: NewMessage? = createMessage(reply, 2);
    if (message_opt == null) { return 9; }
    let message: NewMessage = message_opt;
    putMsg(dest, message);
    let fetched_opt: Message? = getMsg(dest);
    if (fetched_opt == null) { return 10; }
    let fetched: Message = fetched_opt;
    replyMsg(fetched);
    let back_opt: Message? = getMsg(reply);
    if (back_opt == null) { return 11; }
    let back: Message = back_opt;
    releaseMsg(back);
    println("released");
    return 0;
}
"#;

/// A message queued on a port created before its reply port. The reply port is
/// released first at exit, while the message still sits on the other own port;
/// that message must be freed then, so the reply port is deleted too.
const HELD_BY_OLDER_PORT_SOURCE: &str = r#"module tests.fault_cleanup.held_by_older_port;

import { MsgPort, NewMessage, createMsgPort, createMessage, putMsg } from std.amiga.exec;

func main(): i32 {
    let dest_opt: MsgPort? = createMsgPort();
    if (dest_opt == null) { return 1; }
    let dest: MsgPort = dest_opt;
    let reply_opt: MsgPort? = createMsgPort();
    if (reply_opt == null) { return 2; }
    let reply: MsgPort = reply_opt;
    let message_opt: NewMessage? = createMessage(reply, 3);
    if (message_opt == null) { return 3; }
    let message: NewMessage = message_opt;
    putMsg(dest, message);
    let xs: u32[2] = [1, 2];
    let i: usize = 5;
    return i32(xs[i]);
}
"#;

/// Releasing every phase-one kind normally leaves no record behind that would
/// be released twice, and a reply port whose message sits on an older own port
/// is still deleted at a fault exit.
#[test]
fn normal_releases_and_cross_port_messages_leave_nothing_behind() {
    if !tool_available("m68k-amigaos-gcc") || !tool_available("vamos") {
        eprintln!("skip: the hosted-amiga toolchain or vamos is absent");
        return;
    }
    for (profile, crt) in [("hosted-amiga", "nix20"), ("hosted-amiga-13", "nix13")] {
        let (code, output) = build_and_run(&format!("normal-{profile}"), profile, crt, NORMAL_RELEASE_SOURCE);
        assert_eq!(code, Some(0), "{profile}: the normal release run failed:\n{output}");
        assert!(output.starts_with("released\n"), "{profile}: unexpected output:\n{output}");
        assert!(!leaked(&output), "{profile}: the normal release left something behind:\n{output}");

        let (code, output) = build_and_run(&format!("older-port-{profile}"), profile, crt, HELD_BY_OLDER_PORT_SOURCE);
        assert_eq!(code, Some(20), "{profile}: the probe must fault:\n{output}");
        assert!(output.starts_with("error E11211: index out of range\n"), "{profile}: unexpected output:\n{output}");
        assert!(!leaked(&output), "{profile}: the cross-port message was left behind:\n{output}");
    }
}
