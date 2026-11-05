//! Intuition window and event-loop implementation tests.
//!
//! These cases inspect the C the compiler emits and pin the internal contract
//! that the public specification cases do not need to name: the generated
//! wrappers route to the shim, the `cl_window` wrapper clears its state, the
//! `NewWindow` flags and IDCMP set are exact, the event loop uses inner-window
//! mouse coordinates and acknowledges refresh, and the close path releases the
//! wrapper-owned title with the matching size.

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
            "clc-intuition-impl-{}-{tag}-{sequence}",
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

/// The window/event surface used by every implementation case.
const WINDOW_SOURCE: &str = r#"module tests.intuition.impl;

import {
    Screen, openScreen, Window, openWindow, closeWindow,
    windowWidth, windowHeight, WindowEvent, waitEvent, pollEvent,
    EVENT_NONE, EVENT_CLOSE
} from std.amiga.intuition;
import { setColor, windowClear, windowFill, windowDraw } from std.amiga.graphics;
import { Font, openFont, windowText } from std.amiga.text;
import { String, stringOf, toCString, bytes } from std.text;

func main(): i32 {
    let screenTitle: String = stringOf("Impl");
    let screen: Screen? = openScreen(320, 200, 4, toCString(screenTitle));
    if (screen == null) { return 1; }
    let window: Window? = openWindow(screen, 10, 10, 200, 100,
                                      toCString(stringOf("Window")));
    if (window == null) { return 2; }
    let width: i32 = windowWidth(window);
    let height: i32 = windowHeight(window);
    windowClear(window, 0);
    windowFill(window, 0, 0, width - 1, 8, 1);
    windowDraw(window, 0, 9, width - 1, 9, 2);
    let font: Font? = openFont(toCString(stringOf("topaz.font")), 8, 0);
    if (font == null) { return 3; }
    windowText(window, font, 2, 8, 1, bytes(stringOf("Text")));
    let polled: WindowEvent = pollEvent(window);
    if (polled.kind == EVENT_NONE) {
        let event: WindowEvent = waitEvent(window);
        if (event.kind == EVENT_CLOSE) {
            closeWindow(window);
            return height;
        }
    }
    closeWindow(window);
    return width + height;
}
"#;

/// The generated wrappers route to the window shim symbols.
#[test]
fn the_window_event_wrappers_lower_to_the_shim() {
    let c = emit("routing", "hosted-amiga", WINDOW_SOURCE);
    for symbol in [
        "return cl_open_window(screen, x, y, width, height, title);",
        "cl_close_window(window);",
        "return cl_window_width(window);",
        "return cl_window_height(window);",
        "cl_window_next_event(window, 1)",
        "cl_window_next_event(window, 0)",
        "cl_window_event_kind(window)",
        "cl_window_event_code(window)",
        "cl_window_event_qualifier(window)",
        "cl_window_event_mouse_x(window)",
        "cl_window_event_mouse_y(window)",
        "cl_window_event_gadget(window)",
        "cl_window_event_menu(window)",
        "cl_window_fill(window, x1, y1, x2, y2, pen);",
        "cl_window_draw(window, x1, y1, x2, y2, pen);",
        "cl_window_clear(window, pen);",
        "cl_window_text_draw(window, font, x, y, pen, (u8 *)",
    ] {
        assert_contains("routing", &c, symbol);
    }
}

/// The shim defines the wrapper and clears every allocation.
#[test]
fn the_shim_defines_the_window_wrapper_and_zeroes_it() {
    let c = emit("types", "hosted-amiga", WINDOW_SOURCE);
    for needle in [
        "typedef struct cl_window_event",
        "typedef struct cl_window",
        "cl_window_copy_text",
        "AllocMem((ULONG)sizeof *handle, MEMF_CLEAR)",
        "memset(&nw, 0, sizeof nw)",
        "handle->last_event",
        "GZZWidth",
        "GZZHeight",
    ] {
        assert_contains("types", &c, needle);
    }
}

/// The NewWindow flags and IDCMP mask are pinned.
#[test]
fn the_new_window_flags_and_idcmp_bits_are_pinned() {
    let c = emit("flags", "hosted-amiga", WINDOW_SOURCE);
    for needle in [
        "WFLG_DRAGBAR",
        "WFLG_DEPTHGADGET",
        "WFLG_CLOSEGADGET",
        "WFLG_ACTIVATE",
        "WFLG_GIMMEZEROZERO",
        "CUSTOMSCREEN",
        "DetailPen = 0",
        "BlockPen = 1",
        "IDCMP_CLOSEWINDOW",
        "IDCMP_GADGETUP",
        "IDCMP_MENUPICK",
        "IDCMP_MOUSEBUTTONS",
        "IDCMP_RAWKEY",
        "IDCMP_VANILLAKEY",
        "IDCMP_ACTIVEWINDOW",
        "IDCMP_INACTIVEWINDOW",
        "IDCMP_REFRESHWINDOW",
    ] {
        assert_contains("flags", &c, needle);
    }
}

/// The event loop uses GZZ mouse coordinates and acknowledges refresh.
#[test]
fn the_event_loop_maps_gzz_mouse_and_acknowledges_refresh() {
    let c = emit("events", "hosted-amiga", WINDOW_SOURCE);
    for needle in [
        "BeginRefresh(handle->window)",
        "EndRefresh(handle->window, TRUE)",
        "ReplyMsg((struct Message *)msg)",
        "memset(&handle->last_event, 0, sizeof handle->last_event)",
        "if (!cl_window_fill_event(handle, msg))",
        "cl_window_gadget_id(handle, msg->IAddress)",
        "GZZMouseX",
        "GZZMouseY",
        "IDCMP_REFRESHWINDOW",
        "CL_EVENT_CLOSE",
        "CL_EVENT_GADGET",
        "CL_EVENT_MENU",
        "CL_EVENT_MOUSE",
        "CL_EVENT_KEY",
        "CL_EVENT_ACTIVE",
        "CL_EVENT_INACTIVE",
    ] {
        assert_contains("events", &c, needle);
    }
}

/// The close path clears the pointer/menu and frees the title with its size.
#[test]
fn the_window_close_order_and_free_sizes_are_pinned() {
    let c = emit("close", "hosted-amiga", WINDOW_SOURCE);
    let close_start = c
        .find("static void cl_window_destroy(cl_window *handle)")
        .expect("close: missing cl_window_destroy");
    let close_entry = c
        .find("void cl_close_window(void *window_handle)")
        .expect("close: missing cl_close_window");
    let close_entry = &c[close_entry..];
    assert_contains("close", close_entry, "if (handle->console_refs > 0)");
    assert_contains("close", close_entry, "cl_window_destroy(handle);");
    let close_end = c[close_start..]
        .find("/* Inner drawable width")
        .map(|offset| close_start + offset)
        .unwrap_or(c.len());
    let close = &c[close_start..close_end];
    assert_before("close", close, "ClearPointer(handle->window)",
                  "ClearMenuStrip(handle->window)");
    assert_before("close", close, "ClearMenuStrip(handle->window)",
                  "CloseWindow(handle->window)");
    assert_before("close", close, "CloseWindow(handle->window)",
                  "FreeMem(handle->title, CL_WINDOW_TEXT_MAX + 1)");
    assert_before("close", close, "FreeMem(handle->title, CL_WINDOW_TEXT_MAX + 1)",
                  "FreeMem(handle, (ULONG)sizeof *handle)");
    assert_contains(
        "close",
        close,
        "FreeMem(handle, (ULONG)sizeof *handle)",
    );
}

/// Locals drop in reverse declaration order; struct fields drop in declaration
/// order. A struct that holds both handles must declare the window before the
/// screen so the window closes first.
#[test]
fn the_window_and_screen_drop_order_is_pinned() {
    let locals = r#"module tests.intuition.order;

import { Screen, openScreen, Window, openWindow } from std.amiga.intuition;
import { String, stringOf, toCString } from std.text;

func main(): i32 {
    let screenTitle: String = stringOf("Order");
    let screen: Screen? = openScreen(320, 200, 4, toCString(screenTitle));
    if (screen == null) { return 1; }
    let window: Window? = openWindow(screen, 10, 10, 100, 80,
                                      toCString(stringOf("W")));
    if (window == null) { return 2; }
    return 0;
}
"#;
    let c = emit("order-locals", "hosted-amiga", locals);
    let main_start = c
        .rfind("__cl_c1:")
        .expect("order-locals: missing generated cleanup");
    let main = &c[main_start..];
    assert_before("order-locals", main, "cl_close_window(",
                  "cl_close_screen(");

    let fields = r#"module tests.intuition.orderfields;

import { Screen, openScreen, Window, openWindow } from std.amiga.intuition;
import { String, stringOf, toCString } from std.text;

struct Ui {
    window: Window?;
    screen: Screen?;
}

func main(): i32 {
    let screenTitle: String = stringOf("Order");
    let screen: Screen? = openScreen(320, 200, 4, toCString(screenTitle));
    if (screen == null) { return 1; }
    let window: Window? = openWindow(screen, 10, 10, 100, 80,
                                      toCString(stringOf("W")));
    if (window == null) { return 2; }
    let ui: Ui = Ui { window: window, screen: screen };
    return 0;
}
"#;
    let c = emit("order-fields", "hosted-amiga", fields);
    let main_start = c
        .rfind("__cl_c1:")
        .expect("order-fields: missing generated cleanup");
    let main = &c[main_start..];
    assert_before("order-fields", main, "cl_close_window(",
                  "cl_close_screen(");
}

/// The gadget/menu surface used by the Phase 2 implementation cases.
const GADGET_SOURCE: &str = r#"module tests.intuition.gadgetsimpl;

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
    let file: i32 = addMenu(window, toCString(stringOf("File")));
    if (file < 0) { return 5; }
    let item: bool = addMenuItem(window, file, toCString(stringOf("Quit")), 11);
    if (!item) { return 6; }
    closeWindow(window);
    return count + file;
}
"#;

/// The generated gadget/menu wrappers route to the shim symbols.
#[test]
fn the_gadget_and_menu_wrappers_lower_to_the_shim() {
    let c = emit("gadget-routing", "hosted-amiga", GADGET_SOURCE);
    for needle in [
        "return cl_window_add_button(window, x, y, width, height, label, id);",
        "return cl_window_add_string(window, x, y, width, height, label, initial, maxChars, id);",
        "return cl_window_string(window, id, (u8 *)(buf.ptr), (u32)((buf).len));",
        "return cl_window_add_menu(window, title);",
        "return cl_window_add_menu_item(window, menu, label, id);",
    ] {
        assert_contains("gadget-routing", &c, needle);
    }
}

/// The gadget nodes own and free the render records, labels, and buffers.
#[test]
fn the_gadget_nodes_own_and_free_every_render_record() {
    let c = emit("gadget-free", "hosted-amiga", GADGET_SOURCE);
    for needle in [
        "typedef struct cl_gadget_node",
        "typedef struct cl_menu_item_node",
        "typedef struct cl_menu_node",
        "cl_window_free_gadget",
        "cl_window_free_gadgets",
        "cl_window_free_menus",
        "FreeMem(node->string_buffer, node->string_buffer_size)",
        "FreeMem(node->string_info, (ULONG)sizeof *node->string_info)",
        "FreeMem(node->box_xy, (ULONG)(10 * sizeof(WORD)))",
        "FreeMem(node->box, (ULONG)sizeof *node->box)",
        "FreeMem(node->normal_image, (ULONG)sizeof *node->normal_image)",
        "FreeMem(node->select_image, (ULONG)sizeof *node->select_image)",
        "FreeMem(node->label, (ULONG)sizeof *node->label)",
        "FreeMem(node->label_text, CL_WINDOW_TEXT_MAX + 1)",
        "FreeMem(node->gadget, (ULONG)sizeof *node->gadget)",
        "FreeMem(menu->title, CL_WINDOW_TEXT_MAX + 1)",
    ] {
        assert_contains("gadget-free", &c, needle);
    }
    let close_start = c
        .find("static void cl_window_destroy(cl_window *handle)")
        .expect("gadget-free: missing cl_window_destroy");
    let close = &c[close_start..];
    assert_before("gadget-free", close, "cl_window_free_gadgets(handle)",
                  "cl_window_free_menus(handle)");
    assert_before("gadget-free", close, "cl_window_free_menus(handle)",
                  "FreeMem(handle->title, CL_WINDOW_TEXT_MAX + 1)");
}

/// The button image pair, string border, and StringInfo limits are pinned.
#[test]
fn the_button_image_and_string_border_are_pinned() {
    let c = emit("gadget-shapes", "hosted-amiga", GADGET_SOURCE);
    for needle in [
        "normal->LeftEdge = 0;",
        "normal->Width = (WORD)width;",
        "normal->Height = (WORD)height;",
        "normal->Depth = 1;",
        "normal->ImageData = NULL;",
        "normal->PlanePick = 0;",
        "normal->PlaneOnOff = 1;",
        "select->PlaneOnOff = 2;",
        "box->Count = 5;",
        "box->XY = box_xy;",
        "box_xy[0] = 0;",
        "node->string_buffer_size = (ULONG)max_chars + 1;",
        "info->MaxChars = (WORD)(max_chars + 1);",
        "info->NumChars = (WORD)strlen(string_buffer);",
        "gadget->GadgetType = GTYP_BOOLGADGET;",
        "gadget->GadgetType = GTYP_STRGADGET;",
        "gadget->Flags = GFLG_GADGIMAGE | GFLG_GADGHIMAGE;",
        "gadget->Activation = GACT_RELVERIFY;",
        "gadget->Activation = GACT_IMMEDIATE | GACT_RELVERIFY;",
        "gadget->GadgetRender = normal;",
        "gadget->SelectRender = select;",
        "gadget->GadgetText = text;",
        "gadget->SpecialInfo = info;",
        "AddGList(handle->window, gadget, (ULONG)-1, 1, NULL);",
        "RefreshGList(gadget, handle->window, NULL, 1);",
    ] {
        assert_contains("gadget-shapes", &c, needle);
    }
}

/// The menu chain links and the attach/reattach calls are pinned.
#[test]
fn the_menu_chain_and_attach_calls_are_pinned() {
    let c = emit("gadget-menus", "hosted-amiga", GADGET_SOURCE);
    for needle in [
        "menu->Flags = MENUENABLED;",
        "menu->FirstItem = NULL;",
        "menu->NextMenu = NULL;",
        "item->Flags = ITEMTEXT | ITEMENABLED;",
        "item->Flags |= COMMSEQ;",
        "item->Command = (BYTE)key;",
        "item->ItemFill = text;",
        "item->SelectFill = NULL;",
        "menu->menu->FirstItem = item;",
        "tail->item->NextItem = item;",
        "SetMenuStrip(handle->window, head)",
        "ClearMenuStrip(handle->window)",
        "ItemAddress(handle->window->MenuStrip, code)",
        "cl_window_menu_detach(handle);",
        "cl_window_menu_link(handle);",
        "cl_window_menu_set(handle)",
        "cl_window_menu_head",
        "node->menu->FirstItem == NULL",
        "previous->NextMenu = node->menu;",
        "(UWORD)code == MENUNULL",
    ] {
        assert_contains("gadget-menus", &c, needle);
    }
    let add_menu = c
        .find("LONG cl_window_add_menu(")
        .expect("gadget-menus: missing add_menu");
    let add_menu_end = c[add_menu..]
        .find("BOOL cl_window_add_menu_item")
        .map(|offset| add_menu + offset)
        .unwrap_or(c.len());
    let add_menu_body = &c[add_menu..add_menu_end];
    assert!(
        !add_menu_body.contains("SetMenuStrip"),
        "gadget-menus: addMenu must not attach an empty header"
    );
    let add_item = c
        .find("BOOL cl_window_add_menu_item")
        .expect("gadget-menus: missing add_menu_item");
    let add_item = &c[add_item..];
    assert!(
        !add_item.contains("item->Command = 0;"),
        "gadget-menus: a menu item must not clear the command key it just set"
    );
    assert_before("gadget-menus", add_item, "cl_window_menu_detach(handle);",
                  "tail->item->NextItem = item;");
    assert_before("gadget-menus", add_item, "cl_window_menu_link(handle);",
                  "cl_window_menu_set(handle)");
    let menu_id = c
        .find("static LONG cl_window_menu_id")
        .expect("gadget-menus: missing menu id");
    let menu_id = &c[menu_id..];
    assert_before("gadget-menus", menu_id, "(UWORD)code == MENUNULL",
                  "ItemAddress(");
}

/// The gadget/menu limits and the bounded string readback are pinned.
#[test]
fn the_gadget_and_menu_limits_are_pinned() {
    let c = emit("gadget-limits", "hosted-amiga", GADGET_SOURCE);
    for needle in [
        "label == NULL",
        "id < 1",
        "max_chars < 1 || max_chars > 64",
        "cl_window_id_taken(handle, id)",
        "cl_window_gadget_geometry_ok(handle, x, y, width, height)",
        "length > max",
        "node->string_info->NumChars",
        "(LONG)node->string_info->MaxChars - 1",
        "(ULONG)length > capacity",
        "out[length] = '\\0';",
    ] {
        assert_contains("gadget-limits", &c, needle);
    }
}

/// The requester/alert/pointer surface used by the Phase 3 cases.
const REQUESTER_SOURCE: &str = r#"module tests.intuition.requestersimpl;

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

/// The generated requester/alert/pointer wrappers route to the shim.
#[test]
fn the_requester_and_pointer_wrappers_lower_to_the_shim() {
    let c = emit("requester-routing", "hosted-amiga", REQUESTER_SOURCE);
    for needle in [
        "return cl_window_ask_yes_no(window, body);",
        "cl_show_alert(body);",
        "cl_window_set_pointer(window, show);",
    ] {
        assert_contains("requester-routing", &c, needle);
    }
}

/// The 1.3 requester/alert calls and the chip-memory pointer are pinned.
#[test]
fn the_requester_alert_and_pointer_calls_are_pinned() {
    let c = emit("requester-calls", "hosted-amiga", REQUESTER_SOURCE);
    for needle in [
        "AutoRequest(handle->window, &body_text, &yes_text, &no_text,",
        "0, 0, 240, 40);",
        "DisplayAlert(1, (STRPTR)message, 20)",
        "message[0] = 0;",
        "message[1] = 20;",
        "message[2] = 20;",
        "memcpy(message + 3, body, length);",
        "FreeMem(body_copy, CL_WINDOW_BODY_MAX + 1)",
        "FreeMem(message, CL_ALERT_MESSAGE_MAX)",
        "#define CL_WINDOW_BODY_MAX 28",
        "SetPointer(handle->window, handle->pointer, 16, 16, 0, 0)",
        "ClearPointer(handle->window)",
        "AllocMem(CL_WINDOW_POINTER_BYTES,",
        "MEMF_CHIP | MEMF_CLEAR",
        "if (handle->pointer == NULL) {\n        handle->pointer = (UWORD *)AllocMem(",
        "if (handle->pointer == NULL) {\n            return;\n        }",
        "#define CL_WINDOW_POINTER_BYTES 64",
        "#define CL_ALERT_MESSAGE_MAX (CL_WINDOW_BODY_MAX + 5)",
        "cl_window_pointer_template[32]",
        "memcpy(handle->pointer, cl_window_pointer_template,",
        "FreeMem(handle->pointer, CL_WINDOW_POINTER_BYTES)",
        "cl_window_copy_text(body, CL_WINDOW_BODY_MAX, CL_WINDOW_BODY_MAX + 1)",
        "pen < 32",
    ] {
        assert_contains("requester-calls", &c, needle);
    }
    let show_alert = c
        .find("void cl_show_alert(char *body)")
        .expect("requester-calls: missing show_alert");
    let show_alert = &c[show_alert..];
    assert_before("requester-calls", show_alert, "message[0] = 0;",
                  "DisplayAlert(1, (STRPTR)message, 20)");
    assert_before("requester-calls", show_alert,
                  "DisplayAlert(1, (STRPTR)message, 20)",
                  "FreeMem(message, CL_ALERT_MESSAGE_MAX)");
    let close_start = c
        .find("static void cl_window_destroy(cl_window *handle)")
        .expect("requester-calls: missing cl_window_destroy");
    let close = &c[close_start..];
    assert_before("requester-calls", close, "cl_window_free_menus(handle)",
                  "FreeMem(handle->pointer, CL_WINDOW_POINTER_BYTES)");
    assert_before("requester-calls", close,
                  "FreeMem(handle->pointer, CL_WINDOW_POINTER_BYTES)",
                  "FreeMem(handle->title, CL_WINDOW_TEXT_MAX + 1)");
}

/// The 1.3 shim contains no 2.0+ requester, palette, or window call.
#[test]
fn the_intuition_shim_has_no_20_plus_calls() {
    let c = emit("requester-forbidden", "hosted-amiga", REQUESTER_SOURCE);
    for forbidden in [
        "OpenWindowTags",
        "OpenWindowTagList",
        "EasyRequest",
        "CreateGadget",
        "SetRGB32",
        "GetVisualInfo",
        "AllocAslRequest",
        "AslRequest",
        "NewObject",
        "DoMethod",
        "SetAttrs",
        "OpenScreenTagList",
    ] {
        assert!(
            !c.contains(forbidden),
            "requester-forbidden: emitted C must not contain {forbidden:?}"
        );
    }
}
