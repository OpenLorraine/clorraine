//! Implementation tests for the Bundle 9 optional surface.
//!
//! The suite pins wrapper routing, ownership/drop, the optional suffix order,
//! and the first module group's shim behavior.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

struct ScratchDir {
    path: PathBuf,
}

impl ScratchDir {
    fn new(tag: &str) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "clc-optional-impl-{}-{tag}-{sequence}",
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

fn shim() -> String {
    format!(
        "{}{}",
        std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("runtime/shim_hosted_amiga.c")
        )
        .expect("the shared shim is readable"),
        clc::profile::Profile::HostedAmiga.shim_suffix()
    )
}

const EXEC_SOURCE: &str = r#"module tests.opt.execimpl;

import { Semaphore, createSemaphore, freeSemaphore, obtainSemaphore,
         releaseSemaphore } from std.amiga.semaphore;
import { TagList, newTagList, freeTagList, tagSet, tagGet }
    from std.amiga.utility;
import { UtilityPool, UtilityBlock, createUtilityPool, allocUtilityBlock,
         writeUtilityBlock, readUtilityBlock, freeUtilityBlock,
         freeUtilityPool } from std.amiga.pool;
import { slice } from std.mem;

func main(): i32 {
    let sem: Semaphore? = createSemaphore();
    if (sem == null) { return 1; }
    let s: Semaphore = sem;
    obtainSemaphore(s);
    releaseSemaphore(s);
    freeSemaphore(s);

    let tags: TagList? = newTagList(2);
    if (tags == null) { return 2; }
    let t: TagList = tags;
    if (!tagSet(t, 1, 7)) { return 3; }
    let value: u32 = tagGet(t, 1);
    freeTagList(t);

    let pool: UtilityPool? = createUtilityPool(0, 1024, 1024);
    if (pool == null) { return 4; }
    let p: UtilityPool = pool;
    let block: UtilityBlock? = allocUtilityBlock(p, 8);
    if (block == null) { return 5; }
    let b: UtilityBlock = block;
    let bytes: u8[4] = [1, 2, 3, 4];
    if (!writeUtilityBlock(b, slice(bytes, 0, 4))) { return 6; }
    let out: u8[8] = [0; 8];
    let read: i32 = readUtilityBlock(b, slice(out, 0, 8));
    freeUtilityPool(p);
    freeUtilityBlock(b);
    return i32(value) + read;
}
"#;

#[test]
fn the_optional_exec_wrappers_route_to_the_shim() {
    let c = emit("exec-routing", "hosted-amiga", EXEC_SOURCE);
    for call in [
        "cl_create_semaphore",
        "cl_obtain_semaphore",
        "cl_release_semaphore",
        "cl_new_tag_list",
        "cl_tag_set",
        "cl_create_utility_pool",
        "cl_alloc_utility_block",
        "cl_write_utility_block",
        "cl_read_utility_block",
    ] {
        assert_contains("exec-routing", &c, call);
    }
}

#[test]
fn the_semaphore_caller_drop_is_generated() {
    let c = emit(
        "exec-drop",
        "hosted-amiga",
        "module tests.opt.semdrtop;\n\nimport { Semaphore, createSemaphore } from std.amiga.semaphore;\n\nfunc main(): i32 {\n    let sem: Semaphore? = createSemaphore();\n    if (sem == null) { return 1; }\n    return 0;\n}\n",
    );
    assert_caller_cleanup("exec-drop", &c, "sem", "cl_free_semaphore");
}

#[test]
fn the_semaphore_shim_owns_its_signal_semaphore() {
    let shim = shim();
    for needle in [
        "struct SignalSemaphore semaphore;",
        "InitSemaphore(&handle->semaphore);",
        "ObtainSemaphore(&semaphore->semaphore);",
        "ReleaseSemaphore(&semaphore->semaphore);",
        "AttemptSemaphore(&semaphore->semaphore)",
        "FreeMem(handle, (ULONG)sizeof(cl_semaphore));",
    ] {
        assert_contains("semaphore-shim", &shim, needle);
    }
}

#[test]
fn the_utility_shim_pins_tags_dates_hooks_and_pool_refcount() {
    let shim = shim();
    for needle in [
        "list->items[list->count].ti_Tag = TAG_DONE;",
        "tag == TAG_DONE",
        "DateStamp(&handle->stamp);",
        "cl_hook_trampoline",
        "cl_hook_current_data",
        "cl_hook_set_result",
        "SysBase->LibNode.lib_Version < 39",
        "pool->released && pool->live_blocks == 0",
        "FreePooled(pool->pool, block->memory, block->size);",
        "DeletePool(pool->pool);",
    ] {
        assert_contains("utility-shim", &shim, needle);
    }
}

#[test]
fn the_optional_suffix_follows_the_generated_program() {
    let c = emit(
        "suffix-order",
        "hosted-amiga",
        "module tests.opt.order;\n\nfunc main(): i32 { return 0; }\n",
    );
    let main = c.find("int main(void)").expect("the generated main");
    let suffix = c
        .find("Bundle 9 optional shim")
        .expect("the optional suffix");
    assert!(main < suffix, "the optional suffix must follow main");
}

const DOS_SOURCE: &str = r#"module tests.opt.dosimpl;

import { Args, readArgsFrom, freeArgs, argCount, argPresent, argNumber,
         argText, argSwitch } from std.amiga.args;
import { DTObject, openPicture, disposeDTObject, dtWidth, dtHeight,
         dtDepth, dtColors } from std.amiga.datatypes;
import { Region, RegionBounds, newRegion, freeRegion, regionOrRect,
         regionAndRect, regionClearRect, regionBounds, regionIsEmpty }
    from std.amiga.regions;
import { EClock, EClockTicks, openEClock, readEClock, eclockFrequency,
         freeEClock } from std.amiga.eclock;
import { readJoyPort, joyPortType, decodeJoyPort, readJoyPortDecoded }
    from std.amiga.lowlevel;
import { String, stringOf, bytes } from std.text;
import { slice } from std.mem;

func main(): i32 {
    let text: String = stringOf("FILE=x NUM=7 VERBOSE");
    let parsed: Args? = readArgsFrom("FILE/A,NUM/N,VERBOSE/S", 3, bytes(text));
    if (parsed == null) { return 1; }
    let args: Args = parsed;
    let count: i32 = argCount(args);
    let present: bool = argPresent(args, 0);
    let number: i32 = argNumber(args, 1);
    let switchValue: bool = argSwitch(args, 2);
    let buffer: u8[16] = [0; 16];
    let textLength: i32 = argText(args, 0, slice(buffer, 0, 16));
    freeArgs(args);

    let pictureCreated: DTObject? = openPicture("CNEXT:image.ilbm");
    if (pictureCreated == null) { return 2; }
    let picture: DTObject = pictureCreated;
    let width: i32 = dtWidth(picture);
    let height: i32 = dtHeight(picture);
    let depth: i32 = dtDepth(picture);
    let colors: i32 = dtColors(picture);
    disposeDTObject(picture);

    let regionCreated: Region? = newRegion();
    if (regionCreated == null) { return 3; }
    let region: Region = regionCreated;
    if (!regionOrRect(region, 10, 10, 20, 20)) { return 4; }
    regionAndRect(region, 12, 12, 18, 18);
    if (!regionClearRect(region, 13, 13, 14, 14)) { return 5; }
    let bounds: RegionBounds = regionBounds(region);
    let empty: bool = regionIsEmpty(region);
    freeRegion(region);

    let clockCreated: EClock? = openEClock();
    if (clockCreated == null) { return 6; }
    let clock: EClock = clockCreated;
    let ticks: EClockTicks = readEClock(clock);
    let rate: u32 = eclockFrequency(clock);
    freeEClock(clock);

    let rawValue: u32 = readJoyPort(0);
    let kind: u32 = joyPortType(rawValue);
    let bits: u8 = decodeJoyPort(rawValue);
    let direct: u8 = readJoyPortDecoded(1);

    return count + (present ? 1 : 0) + number + (switchValue ? 1 : 0) +
           textLength + width + height + depth + colors + i32(bounds.x1) +
           (empty ? 1 : 0) + i32(rate) + i32(kind) + i32(bits) +
           i32(direct) + i32(ticks.hi) + i32(ticks.lo);
}
"#;

#[test]
fn the_dos_graphics_input_wrappers_route_to_the_shim() {
    let c = emit("dos-routing", "hosted-amiga", DOS_SOURCE);
    for call in [
        "cl_read_args_from",
        "cl_arg_text",
        "cl_dt_open_picture",
        "cl_dt_width",
        "cl_region_new",
        "cl_region_or_rect",
        "cl_eclock_open",
        "cl_eclock_read",
        "cl_lowlevel_read",
        "cl_lowlevel_decode",
    ] {
        assert_contains("dos-routing", &c, call);
    }
}

#[test]
fn the_dos_graphics_input_shim_pins_the_native_calls() {
    let shim = shim();
    for needle in [
        "ReadArgs((STRPTR)template, args->slots, NULL)",
        "FreeArgs(args->rdargs)",
        "args->kinds[index] != 1",
        "PDTA_BitMapHeader",
        "PDTA_NumColors",
        "DoDTMethodA(object->object, window->window, NULL, (Msg)&draw)",
        "AndRectRegion(region->region, &rect)",
        "InstallClipRegion(layer->layer, region->region)",
        "UNIT_ECLOCK",
        "ReadEClock(&clock->last)",
        "JP_TYPE_GAMECTLR",
        "JP_TYPE_JOYSTK",
        "ReadJoyPort(port)",
    ] {
        assert_contains("dos-shim", &shim, needle);
    }
}

const UI_SOURCE: &str = r#"module tests.opt.uiimpl;

import { Screen, Window, openScreen, openWindow, closeWindow }
    from std.amiga.intuition;
import { GadgetList, newGadgetList, addButton, buildGadgets, freeGadgetList }
    from std.amiga.gadtools;
import { FileRequester, newFileRequester, requestFile, freeFileRequester }
    from std.amiga.asl;
import { BoopsiObject, newColorWheel, attachObject, detachObject,
         disposeObject } from std.amiga.boopsi;
import { String, stringOf, toCString } from std.text;

func main(): i32 {
    let title: String = stringOf("Bundle9");
    let screenCreated: Screen? = openScreen(320, 200, 4, toCString(title));
    if (screenCreated == null) { return 1; }
    let screen: Screen = screenCreated;
    let windowCreated: Window? = openWindow(screen, 0, 0, 320, 200, toCString(title));
    if (windowCreated == null) { return 2; }
    let window: Window = windowCreated;
    let listCreated: GadgetList? = newGadgetList(window, toCString(stringOf("topaz.font")), 8);
    if (listCreated == null) { return 3; }
    let list: GadgetList = listCreated;
    if (!addButton(list, 1, toCString(stringOf("OK")), 8, 8, 80, 20)) { return 4; }
    if (!buildGadgets(list)) { return 5; }
    freeGadgetList(list);
    let requesterCreated: FileRequester? =
        newFileRequester(toCString(stringOf("Open")), toCString(stringOf("CNEXT:x")), false);
    if (requesterCreated == null) { return 6; }
    let requester: FileRequester = requesterCreated;
    if (requestFile(requester, window)) { }
    freeFileRequester(requester);
    let objectCreated: BoopsiObject? = newColorWheel(window, 10, 8, 8, 100, 60);
    if (objectCreated == null) { return 7; }
    let object: BoopsiObject = objectCreated;
    if (!attachObject(window, object)) { return 8; }
    detachObject(object);
    disposeObject(object);
    closeWindow(window);
    return 0;
}
"#;

#[test]
fn the_intuition_advanced_wrappers_route_to_the_shim() {
    let c = emit("ui-routing", "hosted-amiga", UI_SOURCE);
    for call in [
        "cl_gt_new",
        "cl_gt_add_button",
        "cl_gt_build",
        "cl_asl_new",
        "cl_asl_request",
        "cl_boopsi_new_colorwheel",
        "cl_boopsi_attach",
    ] {
        assert_contains("ui-routing", &c, call);
    }
}

#[test]
fn the_intuition_advanced_shim_pins_the_native_calls() {
    let shim = shim();
    for needle in [
        "CreateContext(&list->glist)",
        "CreateGadgetA(item->kind, previous, &item->ng,",
        "RefreshGList(list->glist",
        "GT_RefreshWindow(list->window, NULL)",
        "GT_GetIMsg(",
        "GT_ReplyIMsg(",
        "AllocAslRequest(",
        "AslRequest(handle->request, tags)",
        "NewObjectA(NULL,",
        "WHEEL_Screen",
        "SetGadgetAttrsA((struct Gadget *)handle->object",
        "GetAttr(WHEEL_Hue",
        "DisposeObject(",
    ] {
        assert_contains("ui-shim", &shim, needle);
    }
}

#[test]
fn the_shared_gadget_lookup_uses_the_public_list() {
    let shim = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("runtime/shim_hosted_amiga.c"),
    )
    .expect("the shared shim is readable");
    for needle in [
        "handle->window->FirstGadget",
        "gadget->GadgetID",
        "ActivateGadget(gadget, handle->window, NULL)",
    ] {
        assert_contains("shared-gadget", &shim, needle);
    }
}
