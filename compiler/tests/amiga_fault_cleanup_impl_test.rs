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
            "while ((foreign = GetMsg(port)) != NULL)",
            "foreign->mn_Node.ln_Type == NT_MESSAGE",
            "foreign->mn_ReplyPort != NULL && foreign->mn_ReplyPort != port",
            "cl_exit_reply_filter(foreign);",
            "ReplyMsg(foreign);",
            "keep = keep || cl_port_kept(port);",
            "RemPort(port);",
            "port->mp_Flags = PA_IGNORE;",
            "port->mp_SigTask = NULL;",
            "FreeSignal((LONG)port->mp_SigBit);",
            "} else {",
            "DeletePort(port);",
            "Permit();",
        ],
    );
    let holds = body(&shim, "int cl_own_ports_hold(struct Node *node)");
    assert_order(
        "own port holds",
        holds,
        &["record = cl_res_head; record != NULL; record = record->next", "record->release == cl_release_msg_port", "cl_port_holds("],
    );
    assert_contains(
        "own message",
        body(&shim, "static int cl_own_port_holds(cl_message *message)"),
        "cl_own_ports_hold(&message->msg.mn_Node)",
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

/// The hook runs its phases in a fixed order: interrupt servers, the registry,
/// the custom-chip state, then the library bases the shim opened. Each step
/// clears its state before it acts, so a nested run cannot repeat it.
#[test]
fn the_exit_hook_runs_its_phases_in_order() {
    let shim = shim();
    let hook = body(&shim, "void cl_exit_hook(void)");
    assert_order(
        "phases",
        hook,
        &[
            "cl_exiting = 1;",
            "if (cl_vblank_handler != NULL)",
            "cl_vblank_handler = NULL;",
            "RemIntServer(INTB_VERTB, &cl_vblank_server);",
            "if (cl_ports_handler != NULL)",
            "cl_ports_handler = NULL;",
            "RemIntServer(INTB_PORTS, &cl_ports_server);",
            "while ((worker = cl_worker_head) != NULL)",
            "while ((node = cl_res_tail) != NULL)",
            "release(handle);",
            "if (cl_sprite_collisions_used)",
            "cl_sprite_collisions_used = 0;",
            "CL_SPRITE_CUSTOM->clxcon = 0;",
            "while (cl_lib_count > 0)",
            "cl_lib_slots[--cl_lib_count]",
            "memcpy(slot, &none, sizeof none);",
            "CloseLibrary((struct Library *)base);",
        ],
    );
}

/// Installing an interrupt server installs the hook first, and a server is
/// refused when the hook cannot be installed, so no server can outlive the
/// program.
#[test]
fn interrupt_servers_install_the_hook_first() {
    let shim = shim();
    for (signature, add) in [
        ("ULONG cl_add_handler(cl_handler_fn handler)", "AddIntServer(INTB_VERTB"),
        ("ULONG cl_add_port_handler(cl_handler_fn handler)", "AddIntServer(INTB_PORTS"),
    ] {
        assert_order(
            signature,
            body(&shim, signature),
            &["handler == NULL || !cl_exit_hook_install()", "return 0;", add],
        );
    }
}

/// `cl_lib_open` leaves a base someone else opened alone and records only a
/// base it opened itself; every base-shim open goes through it.
#[test]
fn library_bases_open_through_the_recording_helper() {
    let shim = shim();
    assert_order(
        "lib open",
        body(&shim, "void *cl_lib_open(void *slot, const char *name, ULONG version)"),
        &[
            "memcpy(&base, slot, sizeof base);",
            "if (base != NULL)",
            "return base;",
            "cl_lib_count >= CL_LIB_TABLE_MAX || !cl_exit_hook_install()",
            "OpenLibrary((STRPTR)name, version)",
            "cl_lib_slots[cl_lib_count++] = slot;",
            "memcpy(slot, &base, sizeof base);",
        ],
    );
    for base in ["&GfxBase", "&IntuitionBase", "&DiskfontBase", "&LayersBase"] {
        assert_contains("lib sites", &shim, &format!("cl_lib_open({base}, "));
    }
    // The only other open is the user `Library` handle, which the registry
    // records separately.
    assert_eq!(shim.matches("OpenLibrary(").count(), 2, "an unrecorded base open remains");
    assert_contains("user library", body(&shim, "struct Library *cl_open_library(char *name, UWORD version)"), "OpenLibrary(");
}

/// Every base-shim kind of this phase registers after a successful acquire,
/// undoes the acquire when the record fails, and unregisters at its free.
#[test]
fn the_display_and_device_kinds_register_after_success() {
    let shim = shim();
    let cases: &[(&str, &[&str], &str, &str)] = &[
        (
            "static void *cl_open_screen_mode(LONG width, LONG height, UBYTE depth,",
            &["opened = OpenScreen(&screen);", "!cl_res_track(cl_release_screen, opened)", "CloseScreen(opened);", "return NULL;"],
            "void cl_close_screen(void *screen_handle)",
            "cl_res_untrack(cl_release_screen, screen_handle);",
        ),
        (
            "void *cl_video_chip_alloc(ULONG size)",
            &["buffer->size = size;", "!cl_res_add(&buffer->res, cl_video_chip_free, buffer)", "cl_video_chip_free(buffer);", "return NULL;"],
            "void cl_video_chip_free(void *handle)",
            "cl_res_remove(&buffer->res);",
        ),
        (
            "void *cl_video_bitmap_new(LONG width, LONG height, UBYTE depth)",
            &["AllocRaster(", "!cl_res_add(&bitmap->res, cl_video_bitmap_free, bitmap)", "cl_video_bitmap_free(bitmap);", "return NULL;"],
            "void cl_video_bitmap_free(void *handle)",
            "cl_res_remove(&bitmap->res);",
        ),
        (
            "void *cl_video_copper_new(void)",
            &["UCopperListInit(", "!cl_res_add(&copper->res, cl_video_copper_free, copper)", "cl_video_copper_destroy(copper);", "return NULL;"],
            "static void cl_video_copper_destroy(cl_video_copper *copper)",
            "cl_res_remove(&copper->res);",
        ),
        (
            "void *cl_video_display_open_at(void *front_handle, void *back_handle,",
            &["InitView(", "!cl_res_add(&display->res, cl_video_display_free, display)", "cl_video_display_free(display);", "return NULL;"],
            "void cl_video_display_free(void *handle)",
            "cl_res_remove(&display->res);",
        ),
        (
            "void *cl_sprite_new(UBYTE channel, UBYTE height)",
            &["GetSprite(", "!cl_res_add(&sprite->res, cl_sprite_free, sprite)", "cl_sprite_destroy(sprite, 1);", "return NULL;"],
            "static void cl_sprite_destroy(cl_sprite *sprite, int free_images)",
            "cl_res_remove(&sprite->res);",
        ),
        (
            "void *cl_sprite_animation_new(UBYTE channel, UBYTE height, UBYTE frames)",
            &["cl_res_remove(&sprite->res);", "!cl_res_add(&animation->res, cl_sprite_animation_free, animation)", "cl_sprite_animation_free(animation);", "return NULL;"],
            "void cl_sprite_animation_free(void *handle)",
            "cl_res_remove(&animation->res);",
        ),
        (
            "void *cl_audio_open(UBYTE channels)",
            &["OpenDevice((STRPTR) \"audio.device\"", "!cl_res_add(&audio->res, cl_audio_free, audio)", "cl_audio_free(audio);", "return NULL;"],
            "void cl_audio_free(void *handle)",
            "cl_res_remove(&audio->res);",
        ),
        (
            "void *cl_sample_new(APTR data, ULONG count, UWORD period, UBYTE volume,",
            &["sample->cycles = cycles;", "!cl_res_add(&sample->res, cl_sample_free, sample)", "cl_sample_free(sample);", "return NULL;"],
            "void cl_sample_free(void *handle)",
            "cl_res_remove(&sample->res);",
        ),
        (
            "void *cl_timer_open(void)",
            &["OpenDevice((STRPTR) \"timer.device\"", "!cl_res_add(&timer->res, cl_timer_free, timer)", "cl_timer_free(timer);", "return NULL;"],
            "void cl_timer_free(void *handle)",
            "cl_res_remove(&timer->res);",
        ),
        (
            "static void *cl_font_wrap(struct TextFont *font, int owns)",
            &["handle->owns = owns;", "owns && !cl_res_add(&handle->res, cl_font_free, handle)", "cl_font_free(handle);", "return NULL;"],
            "void cl_font_free(void *handle)",
            "cl_res_remove(&font->res);",
        ),
        (
            "void *cl_layer_new(void *bitmap_handle, LONG x0, LONG y0, LONG x1, LONG y1)",
            &["CreateUpfrontLayer(", "!cl_res_add(&handle->res, cl_layer_free, handle)", "cl_layer_free(handle);", "return NULL;"],
            "void cl_layer_free(void *handle)",
            "cl_res_remove(&layer->res);",
        ),
    ];
    for (acquire, order, release, remove) in cases {
        assert_order(acquire, body_from(&shim, acquire), order);
        let free = body_from(&shim, release);
        assert_contains(release, free, remove);
        // The record goes before anything is released.
        let remove_at = free.find(remove).expect("remove");
        for call in ["FreeMem(", "CloseScreen(", "CloseDevice(", "CloseFont(", "DeleteLayer(", "LoadView("] {
            if let Some(at) = free.find(call) {
                assert!(remove_at < at, "{release}: unregister after {call}");
            }
        }
    }
}

/// The text of the definition that starts with `prefix` (a signature that may
/// continue on the next line), up to its closing brace in column 0.
fn body_from<'a>(source: &'a str, prefix: &str) -> &'a str {
    let mut from = 0;
    loop {
        let at = source[from..]
            .find(&format!("\n{prefix}"))
            .unwrap_or_else(|| panic!("the shim has no definition {prefix:?}"))
            + from
            + 1;
        let rest = &source[at..];
        let header_end = rest.find(['{', ';']).expect("a declaration ends");
        if rest.as_bytes()[header_end] == b'{' {
            let end = rest.find("\n}\n").expect("closing brace");
            return &rest[..end + 2];
        }
        from = at;
    }
}

/// Windows: recorded after either open path, unrecorded only where the window
/// is really destroyed (a close can defer while a console or UI child still
/// refers to it), and released at exit through that same deferring close.
#[test]
fn windows_unregister_where_they_are_destroyed() {
    let shim = shim();
    assert_order(
        "window record",
        body(&shim, "static void *cl_window_recorded(cl_window *handle)"),
        &["!cl_res_add(&handle->res, cl_close_window, handle)", "cl_close_window(handle);", "return NULL;"],
    );
    for signature in [
        "void *cl_open_window(void *screen_handle, LONG x, LONG y, LONG width,",
        "void *cl_open_workbench_window(LONG x, LONG y, LONG width, LONG height,",
    ] {
        assert_order(signature, body_from(&shim, signature), &["OpenWindow(&nw);", "return cl_window_recorded(handle);"]);
    }
    assert_order(
        "destroy",
        body(&shim, "static void cl_window_destroy(cl_window *handle)"),
        &["cl_res_remove(&handle->res);", "CloseWindow(handle->window);"],
    );
    assert_lacks("close", body(&shim, "void cl_close_window(void *window_handle)"), "cl_res_remove");
}

/// A copper moved into a display stops being recorded; every path that frees
/// the wrapper unrecords it first.
#[test]
fn a_copper_moved_into_a_display_is_unrecorded() {
    let shim = shim();
    assert_order(
        "set copper",
        body(&shim, "BOOL cl_video_display_set_copper(void *handle, void *copper_handle)"),
        &[
            "cl_video_copper_destroy(copper);",
            "return FALSE;",
            "cl_video_copper_destroy(copper);",
            "return FALSE;",
            "copper->lists[index] = NULL;",
            "cl_res_remove(&copper->res);",
            "FreeMem(copper, (ULONG)sizeof *copper);",
            "return TRUE;",
        ],
    );
}

/// Sprites restore only the chip state the runtime changed: the attachment
/// that turned the screen's SPRITES mode on owns it, hands it to another
/// attached channel of the same screen on release, and the last one clears
/// it. Collisions record their use so the exit can reset CLXCON.
#[test]
fn sprites_restore_only_the_state_they_changed() {
    let shim = shim();
    assert_order(
        "attach",
        body(&shim, "BOOL cl_sprite_attach_screen(void *sprite_handle, void *screen_handle)"),
        &["(screen->ViewPort.Modes & SPRITES) == 0", "sprite->owns_sprites_flag = 1;", "screen->ViewPort.Modes |= SPRITES;"],
    );
    assert_order(
        "release mode",
        body(&shim, "static void cl_sprite_release_mode(cl_sprite *sprite)"),
        &[
            "if (!sprite->owns_sprites_flag)",
            "sprite->owns_sprites_flag = 0;",
            "other != sprite && other->screen == sprite->screen",
            "other->owns_sprites_flag = 1;",
            "return;",
            "sprite->screen->ViewPort.Modes &= (UWORD)~SPRITES;",
        ],
    );
    assert_order(
        "destroy",
        body(&shim, "static void cl_sprite_destroy(cl_sprite *sprite, int free_images)"),
        &["FreeSprite(", "cl_sprite_release_mode(sprite);", "RethinkDisplay();"],
    );
    assert_contains("detach", body(&shim, "void cl_sprite_detach(void *handle)"), "cl_sprite_release_mode(sprite);");
    assert_order(
        "screen closing",
        body(&shim, "static void cl_sprite_screen_closing(struct Screen *screen)"),
        &["sprite->owns_sprites_flag = 0;", "sprite->screen = NULL;"],
    );
    assert_order(
        "collisions",
        body(&shim, "void cl_sprite_enable_collisions(void)"),
        &["!cl_exit_hook_install()", "return;", "cl_sprite_collisions_used = 1;", "clxcon = 0xF000;"],
    );
}

/// Chip memory, a sample, and a timer are live when an index goes out of
/// range; `vamos` must report nothing left behind.
const CHIP_FAULT_SOURCE: &str = r#"module tests.fault_cleanup.chip_fault;

import { ChipBuffer, allocChip } from std.amiga.video;
import { Sample, newSample } from std.amiga.audio;
import { Timer, openTimer } from std.amiga.timer;
import { slice } from std.mem;

func main(): i32 {
    let chip: ChipBuffer? = allocChip(256);
    if (chip == null) { return 1; }
    let data: u8[4] = [1, 2, 3, 4];
    let sample: Sample? = newSample(slice(data, 0, 4), 428, 64, 1);
    if (sample == null) { return 2; }
    let timer: Timer? = openTimer();
    if (timer == null) { return 3; }
    let xs: u32[2] = [1, 2];
    let i: usize = 5;
    return i32(xs[i]);
}
"#;

/// The phase's kinds that run without a display are released at a fault exit.
#[test]
fn chip_memory_a_sample_and_a_timer_are_released_at_a_fault() {
    if !tool_available("m68k-amigaos-gcc") || !tool_available("vamos") {
        eprintln!("skip: the hosted-amiga toolchain or vamos is absent");
        return;
    }
    for (profile, crt) in [("hosted-amiga", "nix20"), ("hosted-amiga-13", "nix13")] {
        let (code, output) = build_and_run(&format!("chip-{profile}"), profile, crt, CHIP_FAULT_SOURCE);
        assert_eq!(code, Some(20), "{profile}: the probe must fault:\n{output}");
        assert!(output.starts_with("error E11211: index out of range\n"), "{profile}: unexpected output:\n{output}");
        assert!(!leaked(&output), "{profile}: chip memory, the sample, or the timer was left behind:\n{output}");
    }
}

/// A program that imports every `std.amiga` module its profile supports
/// cross-builds with warnings as errors on that profile's runtime, so every
/// shim file the registry touches stays warning-free on both CRTs.
#[test]
fn a_program_importing_every_amiga_module_builds_warning_free() {
    if !tool_available("m68k-amigaos-gcc") {
        eprintln!("skip: m68k-amigaos-gcc is not on PATH");
        return;
    }
    for (profile_name, crt) in [("hosted-amiga", "nix20"), ("hosted-amiga-13", "nix13")] {
        let profile = clc::profile::Profile::parse(profile_name).expect("a known profile");
        let mut source = String::from("module tests.fault_cleanup.every_module;\n\n");
        let mut imported = 0;
        for module in clc::stdlib::SOURCES {
            if !module.name.starts_with("std.amiga.") || !profile.supports_module(module.name) {
                continue;
            }
            let export = clc::stdlib::lookup(module.name)
                .and_then(|found| found.exports.first())
                .unwrap_or_else(|| panic!("{} exports nothing", module.name));
            source.push_str(&format!("import {{ {export} }} from {};\n", module.name));
            imported += 1;
        }
        source.push_str("\nfunc main(): i32 {\n    return 0;\n}\n");
        assert!(imported > 10, "{profile_name}: only {imported} modules found");

        let scratch = ScratchDir::new(&format!("every-{profile_name}"));
        let source_path = scratch.path().join("main.clo");
        std::fs::write(&source_path, &source).expect("write the program");
        let c_path = scratch.path().join("out.c");
        let emit = Command::new(env!("CARGO_BIN_EXE_clc"))
            .args(["--profile", profile_name, "-o"])
            .arg(&c_path)
            .arg(&source_path)
            .output()
            .expect("run clc");
        assert!(emit.status.success(), "{profile_name}: clc:\n{}\n{source}", text(&emit.stderr));
        let gcc = Command::new("m68k-amigaos-gcc")
            .args(["-std=c99", &format!("-mcrt={crt}"), "-m68000", "-fwrapv", "-O0", "-g", "-Wall", "-Wextra", "-Werror", "-o"])
            .arg(scratch.path().join("program"))
            .arg(&c_path)
            .args(["-lamiga", "-lm"])
            .output()
            .expect("run m68k-amigaos-gcc");
        assert!(gcc.status.success(), "{profile_name}: gcc:\n{}", text(&gcc.stderr));
    }
}

/// Builds a C harness that wraps an emitted program (its `main` renamed) and
/// runs `harness_main` under `vamos`. Returns the exit code and all output.
fn run_shim_harness(tag: &str, profile: &str, crt: &str, harness_main: &str) -> (Option<i32>, String) {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, "module tests.fault_cleanup.harness;\n\nfunc main(): i32 {\n    return 0;\n}\n")
        .expect("write the program");
    let emitted = scratch.path().join("program.c");
    let emit = Command::new(env!("CARGO_BIN_EXE_clc"))
        .args(["--profile", profile, "-o"])
        .arg(&emitted)
        .arg(&source_path)
        .output()
        .expect("run clc");
    assert!(emit.status.success(), "{tag}: clc:\n{}", text(&emit.stderr));
    let harness = scratch.path().join("harness.c");
    std::fs::write(
        &harness,
        format!("#define main cl_emitted_main\n#include \"program.c\"\n#undef main\n\n{harness_main}"),
    )
    .expect("write the harness");
    let binary = scratch.path().join("harness");
    let gcc = Command::new("m68k-amigaos-gcc")
        .args(["-std=c99", &format!("-mcrt={crt}"), "-m68000", "-fwrapv", "-O0", "-g", "-Wall", "-Wextra", "-Werror", "-o"])
        .arg(&binary)
        .arg(&harness)
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

/// The exit hook closes a library base the runtime opened through its
/// recording helper, and leaves alone a base that was already open: that one
/// belongs to whoever opened it. `vamos` counts library opens, so an unclosed
/// open shows as a library it cannot expunge at shutdown.
#[test]
fn the_exit_hook_closes_only_the_library_bases_the_runtime_opened() {
    if !tool_available("m68k-amigaos-gcc") || !tool_available("vamos") {
        eprintln!("skip: the hosted-amiga toolchain or vamos is absent");
        return;
    }
    const OPENED_HERE: &str = "\
/* A base global only this test uses, so the runtime opens it itself. */
struct Library *cl_test_base = NULL;

int main(void)
{
    return cl_lib_open(&cl_test_base, \"dos.library\", 0) == NULL ? 2 : 0;
}
";
    const ALREADY_OPEN: &str = "\
/* Opened by the test itself and deliberately never closed. */
struct Library *cl_test_held = NULL;
/* Opened through the runtime, which must close it. */
struct Library *cl_test_base = NULL;

int main(void)
{
    cl_test_held = OpenLibrary((STRPTR)\"dos.library\", 0);
    if (cl_test_held == NULL) {
        return 2;
    }
    if (cl_lib_open(&cl_test_held, \"dos.library\", 0) != (void *)cl_test_held) {
        return 3;
    }
    return cl_lib_open(&cl_test_base, \"dos.library\", 0) == NULL ? 4 : 0;
}
";
    for (profile, crt) in [("hosted-amiga", "nix20"), ("hosted-amiga-13", "nix13")] {
        let (code, output) = run_shim_harness(&format!("lib-opened-{profile}"), profile, crt, OPENED_HERE);
        assert_eq!(code, Some(0), "{profile}: the harness failed:\n{output}");
        assert!(!leaked(&output), "{profile}: a base the runtime opened was left open:\n{output}");

        let (code, output) = run_shim_harness(&format!("lib-held-{profile}"), profile, crt, ALREADY_OPEN);
        assert_eq!(code, Some(0), "{profile}: the harness failed:\n{output}");
        let held: Vec<&str> = output
            .lines()
            .filter(|line| line.contains("can't expunge") && line.contains("'dos.library'"))
            .collect();
        assert!(
            held.len() == 1 && held[0].contains("open count 1"),
            "{profile}: only the test's own open may remain, exactly once:\n{output}"
        );
    }
}

/// One hosted-amiga suffix shim file, read directly.
fn suffix(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("runtime").join(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
}

/// Every suffix file and the base shim open library bases only through the
/// recording helper; the base shim's helper and the user `Library` handle are
/// the only direct opens left.
#[test]
fn no_shim_file_opens_a_base_outside_the_recording_helper() {
    for name in [
        "shim_hosted_amiga_20.c",
        "shim_hosted_amiga_optional.c",
        "shim_hosted_amiga_optional_ui.c",
        "shim_hosted_amiga_optional_services.c",
        "shim_hosted_amiga_optional_net.c",
        "shim_hosted_amiga_optional_rtg.c",
    ] {
        assert_lacks(name, &suffix(name), "OpenLibrary(");
    }
    for (name, bases) in [
        ("shim_hosted_amiga_20.c", &["&IconBase", "&WorkbenchBase", "&ExpansionBase", "&KeymapBase"][..]),
        ("shim_hosted_amiga_optional.c", &["&UtilityBase", "&LowLevelBase", "&GfxBase", "&LayersBase", "&DataTypesBase"][..]),
        ("shim_hosted_amiga_optional_ui.c", &["&GadToolsBase", "&AslBase"][..]),
        ("shim_hosted_amiga_optional_services.c", &["&CxBase", "&LocaleBase", "&RexxSysBase", "&NVBase"][..]),
        ("shim_hosted_amiga_optional_net.c", &["&SocketBase", "&AmigaGuideBase"][..]),
        ("shim_hosted_amiga_optional_rtg.c", &["&P96Base", "&GfxBase"][..]),
    ] {
        let text = suffix(name);
        for base in bases {
            assert_contains(name, &text, &format!("cl_lib_open({base}, "));
        }
    }
}

/// Every suffix kind registers after a successful acquire, undoes the acquire
/// when the record fails, and unregisters first in the function that really
/// frees it.
#[test]
fn every_suffix_kind_registers_after_success_and_unregisters_at_its_free() {
    // (file, acquire definition prefix, registration needle, free definition prefix, removal needle)
    let cases: &[(&str, &str, &str, &str, &str)] = &[
        ("shim_hosted_amiga_20.c", "static void *cl_disk_object_recorded(struct DiskObject *object)", "!cl_res_track(cl_release_disk_object, object)", "void cl_free_disk_object(void *icon)", "cl_res_untrack(cl_release_disk_object, icon);"),
        ("shim_hosted_amiga_20.c", "void *cl_add_app_icon(void *port_handle, char *title, ULONG id,", "!cl_res_add(&record->res, cl_remove_app_icon, record)", "void cl_remove_app_icon(void *handle)", "cl_res_remove(&record->res);"),
        ("shim_hosted_amiga_20.c", "void *cl_add_app_menu_item(void *port_handle, char *title, ULONG id,", "!cl_res_add(&record->res, cl_remove_app_menu_item, record)", "void cl_remove_app_menu_item(void *handle)", "cl_res_remove(&record->res);"),
        ("shim_hosted_amiga_20.c", "void *cl_add_app_window(void *window_handle, void *port_handle, ULONG id,", "!cl_res_track(cl_release_app_window, app_window)", "void cl_remove_app_window(void *handle)", "cl_res_untrack(cl_release_app_window, handle);"),
        ("shim_hosted_amiga_20.c", "void *cl_wait_app_message(void *port_handle)", "!cl_res_track(cl_release_app_message, message)", "void cl_reply_app_message(void *handle)", "cl_res_untrack(cl_release_app_message, handle);"),
        ("shim_hosted_amiga_optional.c", "void *cl_create_semaphore(void)", "!cl_res_add(&handle->res, cl_free_semaphore, handle)", "void cl_free_semaphore(void *handle)", "cl_res_remove("),
        ("shim_hosted_amiga_optional.c", "void *cl_new_tag_list(LONG capacity)", "!cl_res_add(&list->res, cl_free_tag_list, list)", "void cl_free_tag_list(void *handle)", "cl_res_remove(&list->res);"),
        ("shim_hosted_amiga_optional.c", "void *cl_date_stamp_new(void)", "!cl_res_add(&handle->res, cl_free_date_stamp, handle)", "void cl_free_date_stamp(void *handle)", "cl_res_remove("),
        ("shim_hosted_amiga_optional.c", "void *cl_new_hook(cl_hook_fn entry, LONG data)", "!cl_res_add(&record->res, cl_free_hook, record)", "void cl_free_hook(void *handle)", "cl_res_remove("),
        ("shim_hosted_amiga_optional.c", "void *cl_create_utility_pool(ULONG mem_flags, ULONG puddle_size,", "!cl_res_add(&pool->res, cl_free_utility_pool, pool)", "static void cl_utility_pool_release(cl_utility_pool *pool)", "cl_res_remove(&pool->res);"),
        ("shim_hosted_amiga_optional.c", "void *cl_alloc_utility_block(void *handle, ULONG size)", "!cl_res_add(&block->res, cl_free_utility_block, block)", "void cl_free_utility_block(void *handle)", "cl_res_remove(&block->res);"),
        ("shim_hosted_amiga_optional.c", "void *cl_read_args(char *template, LONG slots)", "!cl_res_add(&args->res, cl_free_args, args)", "static void cl_args_discard(cl_args *args)", "cl_res_remove(&args->res);"),
        ("shim_hosted_amiga_optional.c", "void *cl_read_args_from(char *template, LONG slots, APTR source,", "!cl_res_add(&args->res, cl_free_args, args)", "static void cl_args_discard(cl_args *args)", "cl_res_remove(&args->res);"),
        ("shim_hosted_amiga_optional.c", "void *cl_eclock_open(void)", "!cl_res_add(&clock->res, cl_free_eclock, clock)", "void cl_free_eclock(void *handle)", "cl_res_remove(&clock->res);"),
        ("shim_hosted_amiga_optional.c", "void *cl_region_new(void)", "!cl_res_add(&handle->res, cl_free_region, handle)", "void cl_free_region(void *handle)", "cl_res_remove(&region->res);"),
        ("shim_hosted_amiga_optional.c", "void *cl_dt_open_picture(char *path)", "!cl_res_add(&handle->res, cl_dispose_dt_object, handle)", "void cl_dispose_dt_object(void *handle)", "cl_res_remove(&object->res);"),
        ("shim_hosted_amiga_optional_ui.c", "void *cl_gt_new(void *window_handle, char *font_name, UWORD font_size)", "!cl_res_add(&list->res, cl_gt_free, list)", "void cl_gt_free(void *handle)", "cl_res_remove(&list->res);"),
        ("shim_hosted_amiga_optional_ui.c", "void *cl_asl_new(char *title, char *initial, BOOL save)", "!cl_res_add(&handle->res, cl_asl_free, handle)", "void cl_asl_free(void *handle_void)", "cl_res_remove(&handle->res);"),
        ("shim_hosted_amiga_optional_ui.c", "void *cl_boopsi_new_string(void *window_handle, LONG id, LONG x, LONG y,", "!cl_res_add(&handle->res, cl_boopsi_dispose, handle)", "void cl_boopsi_dispose(void *handle_void)", "cl_res_remove(&handle->res);"),
        ("shim_hosted_amiga_optional_services.c", "void *cl_commodity_open(char *name, char *title, char *description,", "!cl_res_add(&commodity->res, cl_commodity_close, commodity)", "void cl_commodity_close(void *handle)", "cl_res_remove(&commodity->res);"),
        ("shim_hosted_amiga_optional_services.c", "void *cl_locale_open(char *name)", "!cl_res_add(&handle->res, cl_locale_close, handle)", "void cl_locale_close(void *handle)", "cl_res_remove(&locale->res);"),
        ("shim_hosted_amiga_optional_services.c", "static void *cl_rexx_recorded(cl_rexx_message *handle)", "!cl_res_add(&handle->res, cl_release_rexx_message, handle)", "void cl_rexx_delete(void *handle_void)", "cl_res_remove(&handle->res);"),
        ("shim_hosted_amiga_optional_services.c", "void *cl_clipboard_open(LONG unit)", "!cl_res_add(&clipboard->res, cl_clipboard_close, clipboard)", "void cl_clipboard_close(void *handle)", "cl_res_remove(&clipboard->res);"),
        ("shim_hosted_amiga_optional_services.c", "void *cl_serial_open(LONG unit, ULONG baud)", "!cl_res_add(&serial->res, cl_serial_close, serial)", "void cl_serial_close(void *handle)", "cl_res_remove(&serial->res);"),
        ("shim_hosted_amiga_optional_services.c", "void *cl_parallel_open(LONG unit, ULONG flags)", "!cl_res_add(&parallel->res, cl_parallel_close, parallel)", "void cl_parallel_close(void *handle)", "cl_res_remove(&parallel->res);"),
        ("shim_hosted_amiga_optional_services.c", "void *cl_printer_open(LONG unit)", "!cl_res_add(&printer->res, cl_printer_close, printer)", "void cl_printer_close(void *handle)", "cl_res_remove(&printer->res);"),
        ("shim_hosted_amiga_optional_services.c", "void *cl_nv_get(char *app, char *item)", "!cl_res_add(&handle->res, cl_nv_free, handle)", "void cl_nv_free(void *handle_void)", "cl_res_remove(&handle->res);"),
        ("shim_hosted_amiga_optional_net.c", "void *cl_socket_open(LONG kind, LONG protocol)", "!cl_res_add(&handle->res, cl_socket_close, handle)", "void cl_socket_close(void *handle_void)", "cl_res_remove(&handle->res);"),
        ("shim_hosted_amiga_optional_net.c", "void *cl_guide_open(char *path, char *public_screen)", "!cl_res_add(&guide->res, cl_guide_close, guide)", "void cl_guide_close(void *handle_void)", "cl_res_remove(&guide->res);"),
        ("shim_hosted_amiga_optional_net.c", "void *cl_guide_wait(void *handle_void)", "!cl_res_add(&message->res, cl_guide_reply, message)", "void cl_guide_reply(void *handle_void)", "cl_res_remove(&message->res);"),
        ("shim_hosted_amiga_optional_rtg.c", "void *cl_rtg_open_screen(LONG width, LONG height, LONG depth, char *title)", "!cl_res_add(&handle->res, cl_rtg_close_screen, handle)", "void cl_rtg_close_screen(void *handle_void)", "cl_res_remove(&handle->res);"),
        ("shim_hosted_amiga_optional_rtg.c", "void *cl_rtg_alloc_bitmap(LONG width, LONG height, LONG depth)", "!cl_res_add(&handle->res, cl_rtg_free_bitmap, handle)", "void cl_rtg_free_bitmap(void *handle_void)", "cl_res_remove(&handle->res);"),
    ];
    for (file, acquire, register, release, remove) in cases {
        let text = suffix(file);
        let acquired = body_from(&text, acquire);
        assert_contains(acquire, acquired, register);
        let after = &acquired[acquired.find(register).expect("registered")..];
        assert_contains(acquire, after, "return NULL;");
        let freed = body_from(&text, release);
        assert_contains(release, freed, remove);
        let removed_at = freed.find(remove).expect("removed");
        for call in ["FreeMem(", "CloseDevice(", "DeleteCxObjAll(", "FreeDiskObject(", "RemoveAppIcon(", "RemoveAppMenuItem(", "ReplyMsg(", "DisposeObject(", "FreeAslRequest(", "CloseSocket(", "CloseAmigaGuide(", "p96CloseScreen(", "p96FreeBitMap(", "DisposeDTObject(", "DisposeRegion(", "CloseLocale(", "FreeNVData(", "DeletePool("] {
            if let Some(at) = freed.find(call) {
                assert!(removed_at < at, "{release}: unregister after {call}");
            }
        }
    }
}

/// An ARexx packet the program created is recorded as in flight while a port
/// holds it, taken back when the program receives it, and handed off again
/// when the program replies or sends it. At exit, an in-flight packet queued
/// on one of the program's own ports is freed; one a foreign task holds stays
/// allocated and keeps its reply port alive.
#[test]
fn arexx_packets_in_flight_keep_their_reply_port_for_a_foreign_holder() {
    let services = suffix("shim_hosted_amiga_optional_services.c");
    assert_order(
        "in-flight release",
        body(&services, "static void cl_release_rexx_in_flight(void *handle)"),
        &[
            "Forbid();",
            "own = cl_own_ports_hold(&message->rm_Node.mn_Node);",
            "Remove(&message->rm_Node.mn_Node);",
            "Permit();",
            "cl_rexx_release_packet(message, cl_rexx_take_lifetime(message));",
            "cl_port_keep(message->rm_Node.mn_ReplyPort);",
        ],
    );
    assert_order(
        "hand off",
        body(&services, "static void cl_rexx_hand_off(struct RexxMsg *message)"),
        &["cl_rexx_has_lifetime(message)", "!cl_res_track(cl_release_rexx_in_flight, message)", "cl_port_keep(message->rm_Node.mn_ReplyPort);"],
    );
    assert_order(
        "send",
        body(&services, "void cl_rexx_send(void *port, void *handle_void)"),
        &["cl_res_remove(&handle->res);", "cl_rexx_hand_off(handle->message);", "PutMsg("],
    );
    assert_order(
        "receive",
        body(&services, "void *cl_rexx_receive(void *port)"),
        &["cl_rexx_is_packet(message)", "cl_res_untrack(cl_release_rexx_in_flight, message);", "return cl_rexx_recorded(handle);"],
    );
    assert_order(
        "reply",
        body(&services, "void cl_rexx_reply(void *handle_void)"),
        &["cl_res_remove(&handle->res);", "cl_rexx_hand_off(handle->message);", "cl_rexx_return(handle->message);"],
    );
    let shim = shim();
    assert_order(
        "keep",
        body(&shim, "void cl_port_keep(struct MsgPort *port)"),
        &[
            "port == NULL || (!cl_kept_overflow && cl_port_kept(port))",
            "return;",
            "cl_kept_count < CL_KEPT_PORTS_MAX",
            "cl_kept_ports[cl_kept_count++] = port;",
            "cl_kept_overflow = 1;",
        ],
    );
    // A command another program sent, still queued or held at exit, is
    // replied as failed rather than as a success.
    assert_order(
        "reply filter",
        body(&services, "static void cl_rexx_exit_reply_filter(struct Message *message)"),
        &["message->mn_Length >= sizeof(struct RexxMsg)", "IsRexxMsg(", "cl_rexx_fail("],
    );
    assert_order(
        "rexx open",
        body(&services, "static int cl_rexx_open(void)"),
        &["cl_lib_open(&RexxSysBase", "cl_exit_reply_filter = cl_rexx_exit_reply_filter;"],
    );
    assert_order(
        "held command",
        body(&services, "static void cl_release_rexx_message(void *handle_void)"),
        &["!handle->owns && handle->message != NULL", "!cl_rexx_has_lifetime(handle->message)", "cl_rexx_fail(handle->message);", "cl_rexx_delete(handle);"],
    );
    assert_order(
        "fail",
        body(&services, "static void cl_rexx_fail(struct RexxMsg *message)"),
        &["message->rm_Result1 = RC_FATAL;", "message->rm_Result2 = 0;"],
    );
    assert_order(
        "kept",
        body(&shim, "static int cl_port_kept(struct MsgPort *port)"),
        &["if (cl_kept_overflow)", "return 1;", "cl_kept_ports[index] == port"],
    );
}

/// A fault on a worker task never runs stdio or `exit` on the worker. The
/// fault paths reach the worker path only through a pointer that creating a
/// worker sets, so a program without workers links none of it; the worker
/// finds its record under Forbid by task pointer, never through `tc_UserData`,
/// records the fault, signals the parent, and parks.
#[test]
fn a_worker_fault_is_recorded_for_the_parent_and_the_worker_parks() {
    let shim = shim();
    for signature in [
        "static inline void cl_trap(const char *code, const char *message)",
        "static inline void cl_panic(const unsigned char *message, unsigned long len)",
    ] {
        assert_order(signature, body(&shim, signature), &["if (cl_worker_fault_fn != NULL)", "cl_worker_fault_fn(", "exit(20);"]);
    }
    let path = body_from(&shim, "static void cl_worker_fault(UBYTE kind, const char *code,");
    assert_order(
        "worker path",
        path,
        &[
            "struct Task *self = FindTask(NULL);",
            "Forbid();",
            "record = cl_worker_head; record != NULL; record = record->next_worker",
            "record->task == self",
            "record->fault_code = code;",
            "record->fault_kind = kind;",
            "Permit();",
            "if (record == NULL)",
            "return;",
            "Signal(record->parent, 1UL << record->signal_bit);",
            "Wait(0);",
        ],
    );
    for forbidden in ["tc_UserData", "printf", "fwrite", "fflush", "exit("] {
        assert_lacks("worker path", path, forbidden);
    }
    assert_order(
        "create",
        body(&shim, "void *cl_create_task(cl_handler_fn entry, ULONG stack_size)"),
        &[
            "!cl_exit_hook_install()",
            "record->task = task;",
            "Forbid();",
            "record->next_worker = cl_worker_head;",
            "cl_worker_head = record;",
            "Permit();",
            "cl_worker_fault_fn = cl_worker_fault;",
        ],
    );
    assert_order(
        "unlink",
        body(&shim, "static void cl_worker_unlink(cl_task *record)"),
        &["Forbid();", "*link = record->next_worker;", "Permit();"],
    );
}

/// The parent reports a worker's fault at join and leaves the parked worker
/// for the exit to remove; the exit reports any fault nobody reported. In
/// both places the record is marked reported before the line is printed, and
/// the line is printed before the worker is removed, so its text is still
/// valid and a nested exit prints nothing twice.
#[test]
fn a_worker_fault_is_reported_once_and_before_the_worker_is_removed() {
    let shim = shim();
    let join = body(&shim, "static LONG cl_task_join_record(cl_task *record)");
    assert_order(
        "join",
        join,
        &[
            "Wait(1UL << record->signal_bit);",
            "if (record->fault_kind != CL_FAULT_NONE)",
            "record->fault_reported = 1;",
            "cl_worker_print_fault(record);",
            "exit(20);",
            "record->removed = 1;",
            "RemTask(record->task);",
            "FreeSignal(record->signal_bit);",
            "cl_worker_unlink(record);",
            "FreeMem(record, (ULONG)sizeof *record);",
        ],
    );
    let hook = body(&shim, "void cl_exit_hook(void)");
    assert_order(
        "exit removal",
        hook,
        &[
            "while ((worker = cl_worker_head) != NULL)",
            "worker->fault_kind != CL_FAULT_NONE && !worker->fault_reported",
            "worker->fault_reported = 1;",
            "cl_worker_print_fault(worker);",
            "if (!worker->removed)",
            "worker->removed = 1;",
            "RemTask(worker->task);",
            "SetSignal(0, 1UL << worker->signal_bit);",
            "FreeSignal(worker->signal_bit);",
            "cl_worker_unlink(worker);",
            "FreeMem(worker, (ULONG)sizeof *worker);",
        ],
    );
    let removal = &hook[hook.find("while ((worker = cl_worker_head)").expect("worker phase")..hook.find("while ((node = cl_res_tail)").expect("registry phase")];
    assert_lacks("exit removal", removal, "Wait(");
    assert_order(
        "print",
        body(&shim, "static void cl_worker_print_fault(const cl_task *record)"),
        &["record->fault_kind == CL_FAULT_TRAP", "printf(\"error %s: %s\\n\"", "fwrite(\"panic: \", 1, 7, stdout);", "fflush(stdout);"],
    );
}
