//! Implementation tests for the Bundle 8 Workbench, icon, and expansion
//! surface.
//!
//! The suite pins the public message port, the separate 2.0 shim suffix,
//! the icon and Workbench ownership, the expansion/card/keymap calls,
//! the demos, the fixtures, and the two gates.

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
            "clc-workbench-impl-{}-{tag}-{sequence}",
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

fn assert_contains(tag: &str, haystack: &str, needle: &str) {
    assert!(
        haystack.contains(needle),
        "{tag}: emitted C does not contain {needle:?}"
    );
}

/// Asserts a labeled caller cleanup block for `var` calls `call(var)`.
fn assert_caller_cleanup(tag: &str, c: &str, var: &str, call: &str) {
    let block = format!("    if ({var}) {{ {call}({var}); }}");
    let mut previous = "";
    for line in c.lines() {
        if line == block && previous.trim_start().starts_with("__cl_c") {
            return;
        }
        previous = line;
    }
    panic!("{tag}: missing labeled caller cleanup for {var}");
}

/// The shared (1.3-safe) shim source.
fn shared_shim() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("runtime/shim_hosted_amiga.c");
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
}

/// The shared shim plus the 2.0+ suffix, which is what a `hosted-amiga`
/// translation unit contains.
fn shim() -> String {
    format!(
        "{}{}",
        shared_shim(),
        clc::profile::Profile::HostedAmiga.shim_suffix()
    )
}

const PUBLIC_PORT_SOURCE: &str = r#"module tests.wb.publicportimpl;

import { MsgPort, createPublicPort, deleteMsgPort } from std.amiga.exec;
import { String, stringOf, toCString } from std.text;

func main(): i32 {
    let name: String = stringOf("ClorraineWbTest");
    let port: MsgPort? = createPublicPort(toCString(name));
    if (port == null) {
        return 1;
    }
    deleteMsgPort(port);
    return 0;
}
"#;

/// The public port wrapper lowers to the shim, and the caller cleanup block
/// deletes the port exactly once.
#[test]
fn the_public_port_wrapper_lowers_to_the_shim() {
    let c = emit("public-port-routing", "hosted-amiga", PUBLIC_PORT_SOURCE);
    assert_contains("public-port-routing", &c, "cl_create_public_port");
    assert_contains(
        "public-port-routing",
        &c,
        "__cl_c1:\n    if (port) { cl_delete_msg_port(port); }",
    );
}

/// The shim rejects a null or empty name and uses the 1.3-safe `CreatePort`
/// constructor.
#[test]
fn the_public_port_shim_validates_the_name() {
    let shim = shim();
    for needle in [
        "void *cl_create_public_port(char *name)",
        "name == NULL",
        "name[0] == '\\0'",
        "CreatePort((STRPTR)name, 0)",
    ] {
        assert_contains("public-port-shim", &shim, needle);
    }
}

/// The 2.0+ suffix is appended only for `hosted-amiga`; the 1.3 emit has no
/// 2.0 function or header macro.
#[test]
fn the_2_0_shim_suffix_is_hosted_amiga_only() {
    let source = "module tests.wb.define;\n\nfunc main(): i32 {\n    return 0;\n}\n";
    let modern = emit("define-20", "hosted-amiga", source);
    assert_contains("define-20", &modern, "cl_add_app_icon");
    assert_contains("define-20", &modern, "cl_save_public_screen");
    let main_at = modern.find("int main(void)").expect("the generated main");
    let suffix_at = modern.find("cl_add_app_icon").expect("the 2.0 suffix");
    assert!(
        main_at < suffix_at,
        "the 2.0 suffix must follow the generated program"
    );
    assert!(
        !modern.contains("#define CL_HOSTED_AMIGA_20"),
        "the suffix is selected by the emitter, not a macro"
    );
    let a500 = emit("define-13", "hosted-amiga-13", source);
    assert!(
        !a500.contains("cl_add_app_icon"),
        "the 1.3 emit must not contain the 2.0 icon shim"
    );
    assert!(
        !a500.contains("cl_save_public_screen"),
        "the 1.3 emit must not contain the 2.0 Workbench shim"
    );
    assert!(
        !a500.contains("#define CL_HOSTED_AMIGA_20"),
        "the 1.3 emit must not define the profile macro"
    );
}

/// The shared shim is 1.3-clean; the 2.0 functions live only in the suffix.
#[test]
fn the_2_0_shim_suffix_is_separate_from_the_shared_shim() {
    let shared = shared_shim();
    assert!(
        !shared.contains("cl_add_app_icon"),
        "the shared shim must not contain the 2.0 icon code"
    );
    assert!(
        !shared.contains("cl_save_public_screen"),
        "the shared shim must not contain the 2.0 Workbench code"
    );
    let suffix = clc::profile::Profile::HostedAmiga.shim_suffix();
    assert_contains("shim-suffix", suffix, "cl_add_app_icon");
    assert_contains("shim-suffix", suffix, "cl_expansion_board_count");
    assert_contains("shim-suffix", suffix, "cl_add_app_window");
    assert_contains("shim-suffix", suffix, "cl_save_public_screen");
    assert!(
        clc::profile::Profile::HostedAmiga13
            .shim_suffix()
            .is_empty(),
        "the 1.3 profile has no 2.0 suffix"
    );
}

const ICON_SOURCE: &str = r#"module tests.wb.iconimpl;

import { DiskObject, ICON_TOOL, ICON_APPICON, getDefDiskObject,
         getDiskObject, putDiskObject, freeDiskObject, diskObjectType,
         setDiskObjectType } from std.amiga.icon;
import { String, stringOf, toCString } from std.text;

func main(): i32 {
    let name: String = stringOf("CNEXT:icon-test");
    let icon: DiskObject? = getDefDiskObject(ICON_TOOL);
    if (icon == null) { return 1; }
    let kind: i32 = diskObjectType(icon);
    if (!setDiskObjectType(icon, ICON_APPICON)) { freeDiskObject(icon); return 2; }
    if (!putDiskObject(toCString(name), icon)) { freeDiskObject(icon); return 3; }
    freeDiskObject(icon);
    let loaded: DiskObject? = getDiskObject(toCString(name));
    if (loaded == null) { return 4; }
    let loadedKind: i32 = diskObjectType(loaded);
    freeDiskObject(loaded);
    return kind + loadedKind;
}
"#;

const ICON_DROP_SOURCE: &str = r#"module tests.wb.icondrop;

import { DiskObject, ICON_TOOL, getDefDiskObject, diskObjectType }
    from std.amiga.icon;

func main(): i32 {
    let icon: DiskObject? = getDefDiskObject(ICON_TOOL);
    if (icon == null) { return 1; }
    return diskObjectType(icon);
}
"#;

/// The icon wrappers lower to the shim and an un-freed icon goes through the
/// generated caller cleanup.
#[test]
fn the_icon_wrappers_lower_to_the_shim() {
    let c = emit("icon-routing", "hosted-amiga", ICON_SOURCE);
    for call in [
        "cl_get_disk_object",
        "cl_put_disk_object",
        "cl_free_disk_object",
        "cl_get_def_disk_object",
        "cl_disk_object_type",
        "cl_set_disk_object_type",
    ] {
        assert_contains("icon-routing", &c, call);
    }

    let drop = emit("icon-drop", "hosted-amiga", ICON_DROP_SOURCE);
    assert_caller_cleanup("icon-drop", &drop, "icon", "cl_free_disk_object");
}

/// The icon shim pins the library base, the library calls, and the type guard.
#[test]
fn the_icon_shim_pins_the_calls_and_guards() {
    let shim = shim();
    for needle in [
        "struct Library *IconBase;",
        "OpenLibrary((STRPTR)\"icon.library\", 37)",
        "GetDiskObject((STRPTR)name)",
        "PutDiskObject((STRPTR)name, (const struct DiskObject *)icon)",
        "FreeDiskObject((struct DiskObject *)icon)",
        "GetDefDiskObject(kind)",
        "kind < WBDISK || kind > WBAPPICON",
        "object->do_Type = (UBYTE)kind;",
        "return object == NULL ? 0 : (LONG)object->do_Type;",
    ] {
        assert_contains("icon-shim", &shim, needle);
    }
}

const WORKBENCH_SOURCE: &str = r#"module tests.wb.workbenchimpl;

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
    let title: String = stringOf("Clorraine WB");
    let screen: Screen? = openScreen(320, 80, 3, toCString(title));
    if (screen == null) { return 1; }
    let window: Window? = openWindow(screen, 8, 8, 300, 64, toCString(title));
    if (window == null) { return 2; }
    let portName: String = stringOf("ClorraineWbTest");
    let port: MsgPort? = createPublicPort(toCString(portName));
    if (port == null) { return 3; }
    let icon: AppIcon? = addAppIcon(port, toCString(title), 7, 42);
    if (icon == null) { return 4; }
    let appWindow: AppWindow? = addAppWindow(window, port, 1, 0);
    if (appWindow == null) { return 5; }
    let item: AppMenuItem? = addAppMenuItem(port, toCString(title), 2, 0);
    if (item == null) { return 6; }
    let saved: bool = savePublicScreen(toCString(stringOf("Workbench")),
                                       toCString(stringOf("CNEXT:workbench.cscr")));
    let message: AppMessage? = waitAppMessage(port);
    if (message == null) { return 8; }
    let kind: i32 = appMessageType(message);
    let id: u32 = appMessageId(message);
    let user: u32 = appMessageUserData(message);
    let class: i32 = appMessageClass(message);
    let args: i32 = appMessageArgCount(message);
    replyAppMessage(message);
    removeAppMenuItem(item);
    removeAppWindow(appWindow);
    removeAppIcon(icon);
    closeWindow(window);
    deleteMsgPort(port);
    return kind + i32(id) + i32(user) + class + args + (saved ? 0 : 1);
}
"#;

const APP_ICON_DROP_SOURCE: &str = r#"module tests.wb.appicondrop;

import { MsgPort, createPublicPort } from std.amiga.exec;
import { AppIcon, addAppIcon } from std.amiga.workbench;
import { String, stringOf, toCString } from std.text;

func main(): i32 {
    let name: String = stringOf("ClorraineWbTest");
    let port: MsgPort? = createPublicPort(toCString(name));
    if (port == null) { return 1; }
    let icon: AppIcon? = addAppIcon(port, toCString(stringOf("Clorraine")), 7, 0);
    if (icon == null) { return 2; }
    return 0;
}
"#;

/// The Workbench wrappers lower to the shim.
#[test]
fn the_workbench_wrappers_lower_to_the_shim() {
    let c = emit("workbench-routing", "hosted-amiga", WORKBENCH_SOURCE);
    for call in [
        "cl_add_app_icon",
        "cl_remove_app_icon",
        "cl_add_app_window",
        "cl_remove_app_window",
        "cl_add_app_menu_item",
        "cl_remove_app_menu_item",
        "cl_wait_app_message",
        "cl_app_message_type",
        "cl_app_message_id",
        "cl_app_message_user_data",
        "cl_app_message_class",
        "cl_app_message_arg_count",
        "cl_reply_app_message",
        "cl_save_public_screen",
    ] {
        assert_contains("workbench-routing", &c, call);
    }
}

/// An unremoved AppIcon goes through the generated caller cleanup.
#[test]
fn the_app_icon_drop_routes_an_unremoved_handle() {
    let c = emit("appicon-drop", "hosted-amiga", APP_ICON_DROP_SOURCE);
    assert_contains(
        "appicon-drop",
        &c,
        "__cl_c1:\n    if (icon) { cl_remove_app_icon(icon); }",
    );
}

/// The AppIcon record owns the title copy and the default DiskObject and pins
/// the visible position.
#[test]
fn the_app_icon_record_owns_the_title_and_object() {
    let shim = shim();
    for needle in [
        "typedef struct cl_app_icon",
        "struct AppIcon *icon;",
        "struct DiskObject *object;",
        "char *title;",
        "ULONG title_size;",
        "cl_workbench_copy_title",
        "GetDefDiskObject(WBTOOL)",
        "object->do_Type = 0;",
        "object->do_CurrentX = 400;",
        "object->do_CurrentY = 140;",
        "WBAPPICONA_PropagatePosition",
        "AddAppIconA(id, user_data, (STRPTR)title_copy, port, 0,",
        "FreeDiskObject(record->object);",
        "FreeMem(record->title, record->title_size);",
    ] {
        assert_contains("appicon-record", &shim, needle);
    }
}

/// The AppWindow borrows the wrapper's raw window and the AppMenuItem owns a
/// copied title.
#[test]
fn the_app_window_and_menu_item_are_pinned() {
    let shim = shim();
    for needle in [
        "const cl_window *window = (const cl_window *)window_handle;",
        "window == NULL || window->window == NULL || port == NULL",
        "AddAppWindowA(id, user_data, window->window, port, NULL)",
        "RemoveAppWindow((struct AppWindow *)handle)",
        "typedef struct cl_app_menu_item",
        "AddAppMenuItemA(id, user_data, (STRPTR)title_copy, port,",
        "if (record->item != NULL)",
        "(void)RemoveAppMenuItem(record->item);",
    ] {
        assert_contains("appwindow-shim", &shim, needle);
    }
}

/// The AppMessage path waits, reads every published field, and replies once.
#[test]
fn the_app_message_path_is_pinned() {
    let shim = shim();
    for needle in [
        "void *cl_wait_app_message(void *port_handle)",
        "(void)WaitPort(port);",
        "return (void *)GetMsg(port);",
        "message->am_Type",
        "message->am_ID",
        "message->am_UserData",
        "message->am_Class",
        "message->am_NumArgs",
        "ReplyMsg((struct Message *)handle);",
    ] {
        assert_contains("appmessage-shim", &shim, needle);
    }
}

/// The public-screen capture locks, saves, and unlocks.
#[test]
fn the_public_screen_capture_is_pinned() {
    let shim = shim();
    for needle in [
        "BOOL cl_save_public_screen(char *name, char *path)",
        "screen = LockPubScreen((STRPTR)name);",
        "cl_screen_save_native((void *)screen, path)",
        "UnlockPubScreen(NULL, screen);",
    ] {
        assert_contains("public-screen-shim", &shim, needle);
    }
}

const EXPANSION_SOURCE: &str = r#"module tests.wb.expansionimpl;

import { String } from std.text;
import { expansionBoardCount, expansionBoardManufacturer,
         expansionBoardProduct, expansionBoardSerial, expansionBoardKind,
         cardPresent, cardStatus, mapRawKey } from std.amiga.expansion;

func main(): i32 {
    let boards: i32 = expansionBoardCount();
    let manufacturer: u16 = expansionBoardManufacturer(0);
    let product: u8 = expansionBoardProduct(0);
    let serial: u32 = expansionBoardSerial(0);
    let kind: u8 = expansionBoardKind(0);
    let present: bool = cardPresent();
    let status: u32 = cardStatus();
    let text: String? = mapRawKey(0x20, 0);
    let mapped: i32 = text == null ? 0 : 1;
    return boards + i32(manufacturer) + i32(product) + i32(serial) +
           i32(kind) + i32(status) + mapped + (present ? 1 : 0);
}
"#;

/// The expansion wrappers lower to the shim.
#[test]
fn the_expansion_wrappers_lower_to_the_shim() {
    let c = emit("expansion-routing", "hosted-amiga", EXPANSION_SOURCE);
    for call in [
        "cl_expansion_board_count",
        "cl_expansion_board_manufacturer",
        "cl_expansion_board_product",
        "cl_expansion_board_serial",
        "cl_expansion_board_kind",
        "cl_card_present",
        "cl_card_status",
        "cl_map_raw_key",
    ] {
        assert_contains("expansion-routing", &c, call);
    }
}

/// The expansion shim walks the read-only ConfigDev list and bounds-checks the
/// index.
#[test]
fn the_expansion_shim_walks_the_board_list() {
    let shim = shim();
    for needle in [
        "struct Library *ExpansionBase;",
        "OpenLibrary((STRPTR)\"expansion.library\", 37)",
        "FindConfigDev(NULL, -1, -1)",
        "FindConfigDev(board, -1, -1)",
        "board->cd_Rom.er_Manufacturer",
        "board->cd_Rom.er_Product",
        "board->cd_Rom.er_SerialNumber",
        "board->cd_Rom.er_Type",
        "index < 0 || !cl_expansion_open()",
    ] {
        assert_contains("expansion-shim", &shim, needle);
    }
}

/// The card resource is optional and its base is assigned before the read.
#[test]
fn the_card_resource_is_optional() {
    let shim = shim();
    for needle in [
        "struct Library *CardResource;",
        "OpenResource((STRPTR)\"card.resource\")",
        "CardResource = (struct Library *)card;",
        "status = ReadCardStatus();",
        "RemResource(card);",
        "CardResource = NULL;",
    ] {
        assert_contains("card-shim", &shim, needle);
    }
}

/// The keymap call fills an InputEvent and builds an owned String.
#[test]
fn the_keymap_call_is_pinned() {
    let shim = shim();
    for needle in [
        "struct Library *KeymapBase;",
        "OpenLibrary((STRPTR)\"keymap.library\", 37)",
        "keymap = AskKeyMapDefault();",
        "memset(&event, 0, sizeof event);",
        "event.ie_Class = IECLASS_RAWKEY;",
        "event.ie_SubClass = 0;",
        "event.ie_Code = code;",
        "event.ie_Qualifier = qualifier;",
        "MapRawKey(&event, (STRPTR)buffer, (LONG)sizeof buffer, keymap)",
        "text = cl_string_new((ULONG)length);",
        "cl_string_append(text, buffer, (ULONG)length);",
    ] {
        assert_contains("keymap-shim", &shim, needle);
    }
}

fn repo_text(path: &str) -> String {
    let full = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join(path);
    std::fs::read_to_string(&full)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", full.display()))
}

/// The Workbench demo pins the port/name lifetimes, all three objects, the
/// native capture, the three messages, the report line, and the removes.
#[test]
fn the_workbench_demo_pins_the_sequence() {
    let demo = repo_text("examples/amiga/workbench.clo");
    for needle in [
        "let portName: String = stringOf(\"ClorraineWbTest\");",
        "let screenTitle: String = stringOf(\"Clorraine WB\");",
        "createPublicPort(toCString(portName))",
        "addAppIcon(port, toCString(iconTitle), 7, 0)",
        "addAppWindow(window, port, 1, 0)",
        "addAppMenuItem(port, toCString(menuTitle), 2, 0)",
        "delayTicks(u32(10))",
        "savePublicScreen(toCString(shotName), toCString(shotPath))",
        "CNEXT:wb-ready.txt",
        "waitAppMessage(port)",
        "replyAppMessage(first)",
        "replyAppMessage(second)",
        "replyAppMessage(third)",
        "wb: ok icon 1 win 1 menu 1 types {type1} {type2} {type3} ids {id1} {id2} {id3} users {user1} {user2} {user3} classes {class1} {class2} {class3} args {args1} {args2} {args3}",
        "removeAppMenuItem(item)",
        "removeAppWindow(appWindow)",
        "removeAppIcon(icon)",
        "closeWindow(window)",
        "deleteMsgPort(port)",
        "CNEXT:wb-done.txt",
    ] {
        assert!(
            demo.contains(needle),
            "the Workbench demo is missing {needle:?}"
        );
    }
}

/// The services demo pins the icon round-trip, the board/card/keymap calls, and
/// the report line; it prints only (the gate redirects stdout).
#[test]
fn the_services_demo_pins_the_sequence() {
    let demo = repo_text("examples/amiga/services.clo");
    for needle in [
        "getDefDiskObject(ICON_TOOL)",
        "putDiskObject(toCString(name), icon)",
        "getDiskObject(toCString(name))",
        "expansionBoardCount()",
        "expansionBoardManufacturer(0)",
        "expansionBoardProduct(0)",
        "cardPresent()",
        "cardStatus()",
        "mapRawKey(0x20, 0)",
        "svc: ok icon {loadedKind} exp {boards} man {i32(manufacturer)} prod {i32(product)} card {card} status {i32(status)} key {mapped}",
    ] {
        assert!(
            demo.contains(needle),
            "the services demo is missing {needle:?}"
        );
    }
    assert!(
        !demo.contains("createFile"),
        "the services demo must print only; the gate redirects stdout"
    );
}

/// The test-only fixtures pin the private start vector, the AppMessage header,
/// and the three application-object types.
#[test]
fn the_workbench_fixtures_pin_the_start_and_messages() {
    let start = repo_text("docker/scripts/workbench-start.c");
    for needle in [
        "jsr %/a6@(-42:W)",
        "OpenLibrary((STRPTR)\"workbench.library\", 37)",
        "LockPubScreen((STRPTR)\"Workbench\")",
        "startwb %ld screen %lx",
    ] {
        assert!(start.contains(needle), "the start fixture is missing {needle:?}");
    }
    let post = repo_text("docker/scripts/workbench-post.c");
    for needle in [
        "SystemTags((STRPTR)\"CNEXT:wb-demo\", SYS_Asynch, TRUE, TAG_END)",
        "FindPort((STRPTR)\"ClorraineWbTest\")",
        "mn_Length = (UWORD)sizeof *message",
        "mn_Node.ln_Type = NT_MESSAGE",
        "mn_ReplyPort = reply",
        "am_Version = AM_VERSION",
        "AMTYPE_APPICON",
        "AMTYPE_APPWINDOW",
        "AMTYPE_APPMENUITEM",
        "printf(\"posted\\n\")",
    ] {
        assert!(post.contains(needle), "the post fixture is missing {needle:?}");
    }
}

/// The gates clear the HD and pin the ROM/model, the start/ready handshakes,
/// the report lines, and the capture comparison.
#[test]
fn the_gates_pin_the_handshakes_and_reports() {
    let a1200 = repo_text("docker/scripts/a1200-workbench-smoke.sh");
    for needle in [
        "rm -rf \"${work}\"",
        "--amiga-model=A1200",
        "--chip-memory=2048 --fast-memory=0 --slow-memory=0",
        "e21545723fe8374e91342617604f1b3d703094f1",
        "grep -aq '^startwb 1 screen [1-9a-f]'",
        "grep -aq '^ready$'",
        "grep -aq '^posted$'",
        "grep -aq '^done$'",
        "wb: ok icon 1 win 1 menu 1 types 8 7 9 ids 7 8 9 users 42 43 44 classes 0 0 0 args 0 0 0",
        "svc: ok icon 3 exp 2 man 2011 prod 82 card 1 status 0 key 1",
        "cscr2png.py",
        "a1200-workbench-icon.png",
        "160x120+800+280",
        "identify -format '%k'",
        "compare -metric AE",
    ] {
        assert!(a1200.contains(needle), "the A1200 gate is missing {needle:?}");
    }
    let a600 = repo_text("docker/scripts/a600-services-smoke.sh");
    for needle in [
        "rm -rf \"${work}\"",
        "--amiga-model=A600",
        "--chip-memory=1024 --fast-memory=0 --slow-memory=0",
        "02843c4253bbd29aba535b0aa3bd9a85034ecde4",
        "grep -aq '^startwb 1 screen [1-9a-f]'",
        "grep -aq '^ready$'",
        "grep -aq '^posted$'",
        "grep -aq '^done$'",
        "wb: ok icon 1 win 1 menu 1 types 8 7 9 ids 7 8 9 users 42 43 44 classes 0 0 0 args 0 0 0",
        "svc: ok icon 3 exp 2 man 2011 prod 82 card 1 status 0 key 1",
    ] {
        assert!(a600.contains(needle), "the A600 gate is missing {needle:?}");
    }
}

const APP_WINDOW_DROP_SOURCE: &str = r#"module tests.wb.appwindowdrop;

import { MsgPort, createPublicPort } from std.amiga.exec;
import { Screen, Window, openScreen, openWindow } from std.amiga.intuition;
import { AppWindow, addAppWindow } from std.amiga.workbench;
import { String, stringOf, toCString } from std.text;

func main(): i32 {
    let title: String = stringOf("W");
    let screenCreated: Screen? = openScreen(320, 80, 3, toCString(title));
    if (screenCreated == null) { return 1; }
    let screen: Screen = screenCreated;
    let windowCreated: Window? = openWindow(screen, 0, 0, 300, 64, toCString(title));
    if (windowCreated == null) { return 2; }
    let window: Window = windowCreated;
    let name: String = stringOf("ClorraineWbTest");
    let portCreated: MsgPort? = createPublicPort(toCString(name));
    if (portCreated == null) { return 3; }
    let port: MsgPort = portCreated;
    let appCreated: AppWindow? = addAppWindow(window, port, 1, 0);
    if (appCreated == null) { return 4; }
    let appWindow: AppWindow = appCreated;
    return 0;
}
"#;

const APP_MENU_DROP_SOURCE: &str = r#"module tests.wb.appmenudrop;

import { MsgPort, createPublicPort } from std.amiga.exec;
import { AppMenuItem, addAppMenuItem } from std.amiga.workbench;
import { String, stringOf, toCString } from std.text;

func main(): i32 {
    let name: String = stringOf("ClorraineWbTest");
    let portCreated: MsgPort? = createPublicPort(toCString(name));
    if (portCreated == null) { return 1; }
    let port: MsgPort = portCreated;
    let itemCreated: AppMenuItem? =
        addAppMenuItem(port, toCString(stringOf("Clorraine")), 2, 0);
    if (itemCreated == null) { return 2; }
    let item: AppMenuItem = itemCreated;
    return 0;
}
"#;

const APP_MESSAGE_DROP_SOURCE: &str = r#"module tests.wb.appmessagedrop;

import { MsgPort, createPublicPort } from std.amiga.exec;
import { AppMessage, waitAppMessage } from std.amiga.workbench;
import { String, stringOf, toCString } from std.text;

func main(): i32 {
    let name: String = stringOf("ClorraineWbTest");
    let portCreated: MsgPort? = createPublicPort(toCString(name));
    if (portCreated == null) { return 1; }
    let port: MsgPort = portCreated;
    let messageCreated: AppMessage? = waitAppMessage(port);
    if (messageCreated == null) { return 2; }
    let message: AppMessage = messageCreated;
    return 0;
}
"#;

/// An unremoved AppWindow, AppMenuItem, or AppMessage goes through the
/// generated caller cleanup. The helper checks the labeled caller block; the
/// wrapper's own dead-parameter drop is unlabeled.
#[test]
fn the_other_workbench_handles_drop_through_the_caller_cleanup() {
    let window = emit("appwindow-drop", "hosted-amiga", APP_WINDOW_DROP_SOURCE);
    assert_caller_cleanup(
        "appwindow-drop",
        &window,
        "appWindow",
        "cl_remove_app_window",
    );
    let menu = emit("appmenu-drop", "hosted-amiga", APP_MENU_DROP_SOURCE);
    assert_caller_cleanup("appmenu-drop", &menu, "item", "cl_remove_app_menu_item");
    let message = emit("appmessage-drop", "hosted-amiga", APP_MESSAGE_DROP_SOURCE);
    assert_caller_cleanup(
        "appmessage-drop",
        &message,
        "message",
        "cl_reply_app_message",
    );
}

/// The AppIcon failure paths and the removal order are pinned, including the
/// V44 position tag.
#[test]
fn the_app_icon_failure_and_removal_paths_are_pinned() {
    let shim = shim();
    assert_contains("appicon-failure", &shim, "FreeDiskObject(object);");
    assert_contains("appicon-failure", &shim, "FreeMem(title_copy, title_size);");
    assert_contains("appicon-failure", &shim, "tags[0].ti_Tag = WBAPPICONA_PropagatePosition;");
    assert_contains("appicon-failure", &shim, "tags[0].ti_Data = TRUE;");
    assert_contains("appicon-failure", &shim, "tags[1].ti_Tag = TAG_DONE;");
    let remove = shim
        .find("(void)RemoveAppIcon(record->icon);")
        .expect("the AppIcon removal guard");
    let free_object = shim
        .find("FreeDiskObject(record->object);")
        .expect("the DiskObject free");
    assert!(
        remove < free_object,
        "RemoveAppIcon must run before FreeDiskObject"
    );
    assert_contains("appicon-failure", &shim, "if (record->item != NULL)");
    assert_contains(
        "appicon-failure",
        &shim,
        "(void)RemoveAppMenuItem(record->item);",
    );
}
