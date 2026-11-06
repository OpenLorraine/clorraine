//! Specification tests for the Bundle 9 optional shim and the first module
//! group. The host cases always run; the cross-build cases skip themselves
//! when the m68k toolchain is absent.

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
            "clc-optional-{}-{tag}-{sequence}",
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
    let build = Command::new("m68k-amigaos-gcc")
        .args([
            "-std=c99",
            &format!("-mcrt={crt}"),
            "-m68000",
            "-fwrapv",
            "-O0",
            "-g",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-o",
            binary.to_str().expect("the output path is UTF-8"),
            c_path.to_str().expect("the C path is UTF-8"),
            "-lamiga",
            "-lm",
        ])
        .output()
        .expect("run m68k-amigaos-gcc");
    assert!(
        build.status.success(),
        "{tag}: the surface must build with {crt}; stderr:\n{}",
        stderr(&build)
    );
}

const EXEC_SOURCE: &str = "\
module tests.opt.exec;

import { Semaphore, createSemaphore, freeSemaphore, obtainSemaphore,
         releaseSemaphore, attemptSemaphore } from std.amiga.semaphore;
import { TagList, DateStamp, Hook, newTagList, freeTagList, tagSet,
         tagGet, tagPresent, tagCount, currentDateStamp, freeDateStamp,
         dateStampDays, dateStampMinute, dateStampTick, newHook, callHook,
         hookData, hookCurrentData, hookSetResult, freeHook } from std.amiga.utility;
import { UtilityPool, UtilityBlock, createUtilityPool, allocUtilityBlock,
         utilityBlockSize, writeUtilityBlock, readUtilityBlock,
         freeUtilityBlock, freeUtilityPool } from std.amiga.pool;
import { slice } from std.mem;

func note(): void {
    if (hookCurrentData() == 7) {
        hookSetResult(42);
    }
}

func main(): i32 {
    let sem: Semaphore? = createSemaphore();
    if (sem == null) { return 1; }
    let s: Semaphore = sem;
    obtainSemaphore(s);
    if (attemptSemaphore(s)) { releaseSemaphore(s); }
    releaseSemaphore(s);
    freeSemaphore(s);

    let tags: TagList? = newTagList(2);
    if (tags == null) { return 2; }
    let t: TagList = tags;
    if (!tagSet(t, 1, 42)) { return 3; }
    let tagValue: u32 = tagGet(t, 1);
    let present: bool = tagPresent(t, 1);
    let count: i32 = tagCount(t);
    freeTagList(t);

    let stamp: DateStamp? = currentDateStamp();
    if (stamp == null) { return 4; }
    let d: DateStamp = stamp;
    let days: i32 = dateStampDays(d);
    let minutes: i32 = dateStampMinute(d);
    let ticks: i32 = dateStampTick(d);
    freeDateStamp(d);

    let hook: Hook? = newHook(note, 7);
    if (hook == null) { return 5; }
    let h: Hook = hook;
    let result: i32 = callHook(h);
    let payload: i32 = hookData(h);
    freeHook(h);

    let pool: UtilityPool? = createUtilityPool(0, 1024, 1024);
    if (pool == null) { return 6; }
    let p: UtilityPool = pool;
    let block: UtilityBlock? = allocUtilityBlock(p, 8);
    if (block == null) { return 7; }
    let b: UtilityBlock = block;
    let size: u32 = utilityBlockSize(b);
    let bytes: u8[4] = [1, 2, 3, 4];
    if (!writeUtilityBlock(b, slice(bytes, 0, 4))) { return 8; }
    let out: u8[8] = [0; 8];
    let read: i32 = readUtilityBlock(b, slice(out, 0, 8));
    freeUtilityPool(p);
    freeUtilityBlock(b);

    return i32(tagValue) + count + days + minutes + ticks + result +
           payload + i32(size) + read + (present ? 1 : 0);
}
";

fn appendix_section(name: &str) -> String {
    let appendix = std::fs::read_to_string(repo_root().join("spec/appendix-stdlib.md"))
        .expect("the stdlib appendix is readable");
    let start = appendix
        .find(name)
        .unwrap_or_else(|| panic!("the appendix has {name}"));
    let section = &appendix[start..];
    let end = section.find("\n### ").unwrap_or(section.len());
    section[..end].to_owned()
}

#[test]
fn the_optional_exec_surface_builds_on_hosted_amiga() {
    let scratch = ScratchDir::new("exec-surface");
    let source_path = write_fixture(&scratch, "main.clo", EXEC_SOURCE);
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
    assert_exit("exec-surface", &output, 0);
    assert!(
        stderr(&output).is_empty(),
        "the optional exec surface must compile with zero diagnostics; stderr:\n{}",
        stderr(&output)
    );
    let c = std::fs::read_to_string(&output_path).expect("the emitted C is readable");
    for call in [
        "cl_create_semaphore",
        "cl_new_tag_list",
        "cl_date_stamp_new",
        "cl_new_hook",
        "cl_create_utility_pool",
        "cl_alloc_utility_block",
    ] {
        assert!(c.contains(call), "the emitted C is missing {call}");
    }
}

#[test]
fn the_a500_profile_rejects_the_optional_exec_surface() {
    let scratch = ScratchDir::new("exec-a500");
    let source_path = write_fixture(&scratch, "main.clo", EXEC_SOURCE);
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
    assert_exit("exec-a500", &output, 1);
    assert!(
        stderr(&output).contains("E11200"),
        "expected E11200, got:\n{}",
        stderr(&output)
    );
    assert!(!output_path.exists(), "a rejected program must not emit C");
}

#[test]
fn the_optional_exec_surface_cross_builds_with_nix20() {
    cross_build("exec-build20", "hosted-amiga", "nix20", EXEC_SOURCE);
}

#[test]
fn the_optional_exec_exports_match_the_stdlib_appendix() {
    for (section, module) in [
        ("### 14.4", "std.amiga.semaphore"),
        ("### 14.5", "std.amiga.utility"),
        ("### 14.6", "std.amiga.pool"),
    ] {
        let text = appendix_section(section);
        let module = clc::stdlib::lookup(module).expect("the module is registered");
        for name in module.exports {
            assert!(
                text.split(|c: char| !c.is_alphanumeric() && c != '_')
                    .any(|word| word == *name),
                "the appendix section {section} is missing {name}"
            );
        }
    }
}

fn repo_text(path: &str) -> String {
    let full = repo_root().join(path);
    std::fs::read_to_string(&full)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", full.display()))
}

#[test]
fn the_optional_suffix_is_hosted_amiga_only() {
    let modern = clc::profile::Profile::HostedAmiga.shim_suffix();
    assert!(modern.contains("Bundle 9 optional shim"));
    assert!(modern.contains("cl_add_app_icon"));
    assert!(clc::profile::Profile::HostedAmiga13.shim_suffix().is_empty());
    assert!(clc::profile::Profile::Host.shim_suffix().is_empty());
    assert!(clc::profile::Profile::Freestanding.shim_suffix().is_empty());
}

#[test]
fn the_optional_suffix_follows_the_2_0_suffix() {
    let suffix = clc::profile::Profile::HostedAmiga.shim_suffix();
    let workbench = suffix.find("cl_add_app_icon").expect("the 2.0 suffix");
    let optional = suffix.find("Bundle 9 optional shim").expect("the optional suffix");
    assert!(workbench < optional, "the optional suffix must follow the 2.0 suffix");
}

#[test]
fn the_generic_device_rejects_clipboard() {
    let shim = repo_text("compiler/runtime/shim_hosted_amiga.c");
    let start = shim
        .find("static int cl_device_is_extended")
        .expect("the extended-device guard");
    let end = shim[start..].find("\n}").expect("the extended-device guard end");
    let block = &shim[start..start + end];
    assert!(block.contains("\"clipboard.device\""));
    assert!(block.contains("\"serial.device\""));
    assert!(block.contains("\"parallel.device\""));
    assert!(!block.contains("\"printer.device\""));
}

const DOS_SOURCE: &str = r#"module tests.opt.dos;

import { Args, readArgsFrom, freeArgs, argCount, argPresent, argNumber,
         argText, argSwitch } from std.amiga.args;
import { DTObject, openPicture, disposeDTObject, dtWidth, dtHeight,
         dtDepth, dtColors, drawDTObject } from std.amiga.datatypes;
import { Region, RegionBounds, newRegion, freeRegion, regionOrRect,
         regionAndRect, regionClearRect, regionBounds, regionIsEmpty }
    from std.amiga.regions;
import { EClock, EClockTicks, openEClock, readEClock, eclockFrequency,
         freeEClock } from std.amiga.eclock;
import { readJoyPort, joyPortType, decodeJoyPort, readJoyPortDecoded }
    from std.amiga.lowlevel;
import { Screen, Window, openScreen, openWindow, closeWindow }
    from std.amiga.intuition;
import { String, stringOf, toCString, bytes } from std.text;
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

    let screenTitle: String = stringOf("Bundle9");
    let screenCreated: Screen? = openScreen(320, 80, 3, toCString(screenTitle));
    if (screenCreated == null) { return 2; }
    let screen: Screen = screenCreated;
    let windowCreated: Window? = openWindow(screen, 0, 0, 300, 64, toCString(screenTitle));
    if (windowCreated == null) { return 3; }
    let window: Window = windowCreated;
    let pictureCreated: DTObject? = openPicture("CNEXT:image.ilbm");
    if (pictureCreated == null) { return 4; }
    let picture: DTObject = pictureCreated;
    let width: i32 = dtWidth(picture);
    let height: i32 = dtHeight(picture);
    let depth: i32 = dtDepth(picture);
    let colors: i32 = dtColors(picture);
    if (!drawDTObject(picture, window, 0, 0)) { return 5; }
    disposeDTObject(picture);
    closeWindow(window);

    let regionCreated: Region? = newRegion();
    if (regionCreated == null) { return 6; }
    let region: Region = regionCreated;
    if (!regionOrRect(region, 10, 10, 20, 20)) { return 7; }
    regionAndRect(region, 12, 12, 18, 18);
    if (!regionClearRect(region, 13, 13, 14, 14)) { return 8; }
    let bounds: RegionBounds = regionBounds(region);
    let empty: bool = regionIsEmpty(region);
    freeRegion(region);

    let clockCreated: EClock? = openEClock();
    if (clockCreated == null) { return 9; }
    let clock: EClock = clockCreated;
    let ticks: EClockTicks = readEClock(clock);
    let rate: u32 = eclockFrequency(clock);
    freeEClock(clock);

    let rawValue: u32 = readJoyPort(0);
    let kind: u32 = joyPortType(rawValue);
    let bits: u8 = decodeJoyPort(rawValue);
    let direct: u8 = readJoyPortDecoded(1);

    return count + (present ? 1 : 0) + number + (switchValue ? 1 : 0) +
           textLength + width + height + depth + colors +
           i32(bounds.x1) + (empty ? 1 : 0) + i32(rate) + i32(kind) +
           i32(bits) + i32(direct) + i32(ticks.hi) + i32(ticks.lo);
}
"#;

#[test]
fn the_dos_graphics_input_surface_builds_on_hosted_amiga() {
    let scratch = ScratchDir::new("dos-surface");
    let source_path = write_fixture(&scratch, "main.clo", DOS_SOURCE);
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
    assert_exit("dos-surface", &output, 0);
    assert!(
        stderr(&output).is_empty(),
        "the DOS/graphics/input surface must compile with zero diagnostics; stderr:\n{}",
        stderr(&output)
    );
    let c = std::fs::read_to_string(&output_path).expect("the emitted C is readable");
    for call in [
        "cl_read_args_from",
        "cl_dt_open_picture",
        "cl_region_new",
        "cl_eclock_open",
        "cl_lowlevel_read",
    ] {
        assert!(c.contains(call), "the emitted C is missing {call}");
    }
}

#[test]
fn the_a500_profile_rejects_the_dos_graphics_input_surface() {
    let scratch = ScratchDir::new("dos-a500");
    let source_path = write_fixture(&scratch, "main.clo", DOS_SOURCE);
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
    assert_exit("dos-a500", &output, 1);
    assert!(stderr(&output).contains("E11200"), "expected E11200, got:\n{}", stderr(&output));
    assert!(!output_path.exists(), "a rejected program must not emit C");
}

#[test]
fn the_dos_graphics_input_surface_cross_builds_with_nix20() {
    cross_build("dos-build20", "hosted-amiga", "nix20", DOS_SOURCE);
}

#[test]
fn the_dos_graphics_input_exports_match_the_stdlib_appendix() {
    for (section, module) in [
        ("### 14.7", "std.amiga.args"),
        ("### 14.8", "std.amiga.datatypes"),
        ("### 14.9", "std.amiga.regions"),
        ("### 14.10", "std.amiga.eclock"),
        ("### 14.11", "std.amiga.lowlevel"),
    ] {
        let text = appendix_section(section);
        let module = clc::stdlib::lookup(module).expect("the module is registered");
        for name in module.exports {
            assert!(
                text.split(|c: char| !c.is_alphanumeric() && c != '_')
                    .any(|word| word == *name),
                "the appendix section {section} is missing {name}"
            );
        }
    }
}

const INTUITION_SOURCE: &str = r#"module tests.opt.intuition;

import { Screen, Window, WindowEvent, openScreen, openWindow, closeWindow }
    from std.amiga.intuition;
import { GadgetList, newGadgetList, addButton, addText, addString,
         addInteger, buildGadgets, freeGadgetList, gadgetPollEvent }
    from std.amiga.gadtools;
import { FileRequester, newFileRequester, requestFile, fileRequesterPath,
         freeFileRequester } from std.amiga.asl;
import { BoopsiObject, newColorWheel, attachObject, detachObject,
         setColorWheelHue, colorWheelHue, disposeObject } from std.amiga.boopsi;
import { String, stringOf, toCString } from std.text;
import { slice } from std.mem;

func main(): i32 {
    let title: String = stringOf("Bundle9 Intuition");
    let screenCreated: Screen? = openScreen(320, 200, 4, toCString(title));
    if (screenCreated == null) { return 1; }
    let screen: Screen = screenCreated;
    let windowCreated: Window? = openWindow(screen, 0, 0, 320, 200, toCString(title));
    if (windowCreated == null) { return 2; }
    let window: Window = windowCreated;

    let listCreated: GadgetList? =
        newGadgetList(window, toCString(stringOf("topaz.font")), 8);
    if (listCreated == null) { return 3; }
    let list: GadgetList = listCreated;
    if (!addButton(list, 1, toCString(stringOf("OK")), 8, 8, 80, 20)) { return 4; }
    if (!addText(list, 2, toCString(stringOf("Text")), 8, 32, 80, 20)) { return 5; }
    if (!addString(list, 3, toCString(stringOf("abc")), 8, 8, 56, 120, 20)) { return 6; }
    if (!addInteger(list, 4, 42, 4, 8, 88, 80, 20)) { return 7; }
    if (!buildGadgets(list)) { return 8; }
    let event: WindowEvent = gadgetPollEvent(window);
    freeGadgetList(list);

    let requesterCreated: FileRequester? =
        newFileRequester(toCString(stringOf("Open")), toCString(stringOf("CNEXT:asl.txt")), false);
    if (requesterCreated == null) { return 9; }
    let requester: FileRequester = requesterCreated;
    let requested: bool = requestFile(requester, window);
    let path: u8[64] = [0; 64];
    let pathLength: i32 = fileRequesterPath(requester, slice(path, 0, 64));
    freeFileRequester(requester);

    let objectCreated: BoopsiObject? = newColorWheel(window, 10, 8, 120, 100, 60);
    if (objectCreated == null) { return 10; }
    let object: BoopsiObject = objectCreated;
    if (!attachObject(window, object)) { return 11; }
    if (!setColorWheelHue(object, 42)) { return 12; }
    let hue: u32 = colorWheelHue(object);
    detachObject(object);
    disposeObject(object);

    closeWindow(window);
    return i32(event.kind) + (requested ? 1 : 0) + pathLength +
           i32(hue);
}
"#;

#[test]
fn the_intuition_advanced_surface_builds_on_hosted_amiga() {
    let scratch = ScratchDir::new("intuition-surface");
    let source_path = write_fixture(&scratch, "main.clo", INTUITION_SOURCE);
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
    assert_exit("intuition-surface", &output, 0);
    assert!(
        stderr(&output).is_empty(),
        "the intuition surface must compile with zero diagnostics; stderr:\n{}",
        stderr(&output)
    );
    let c = std::fs::read_to_string(&output_path).expect("the emitted C is readable");
    for call in ["cl_gt_new", "cl_asl_new", "cl_boopsi_new_colorwheel"] {
        assert!(c.contains(call), "the emitted C is missing {call}");
    }
}

#[test]
fn the_a500_profile_rejects_the_intuition_advanced_surface() {
    let scratch = ScratchDir::new("intuition-a500");
    let source_path = write_fixture(&scratch, "main.clo", INTUITION_SOURCE);
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
    assert_exit("intuition-a500", &output, 1);
    assert!(stderr(&output).contains("E11200"), "expected E11200, got:\n{}", stderr(&output));
    assert!(!output_path.exists(), "a rejected program must not emit C");
}

#[test]
fn the_intuition_advanced_surface_cross_builds_with_nix20() {
    cross_build("intuition-build20", "hosted-amiga", "nix20", INTUITION_SOURCE);
}

#[test]
fn the_intuition_advanced_exports_match_the_stdlib_appendix() {
    for (section, module) in [
        ("### 14.12", "std.amiga.gadtools"),
        ("### 14.13", "std.amiga.asl"),
        ("### 14.14", "std.amiga.boopsi"),
    ] {
        let text = appendix_section(section);
        let module = clc::stdlib::lookup(module).expect("the module is registered");
        for name in module.exports {
            assert!(
                text.split(|c: char| !c.is_alphanumeric() && c != '_')
                    .any(|word| word == *name),
                "the appendix section {section} is missing {name}"
            );
        }
    }
}

const SERVICES_SOURCE: &str = r#"module tests.opt.services;

import { MsgPort, createMsgPort, deleteMsgPort } from std.amiga.exec;
import { Commodity, openCommodity, closeCommodity }
    from std.amiga.commodities;
import { Locale, openLocale, closeLocale, localeLanguageName,
         localeCountryCode, localeGmtOffset, localeDateTimeFormat }
    from std.amiga.locale;
import { RexxMessage, createRexxMessage, setRexxAction, setRexxArg, sendRexxMessage, receiveRexxMessage,
         replyRexxMessage, reclaimRexxMessage, rexxMessageAction,
         rexxMessageResult1, rexxMessageResult2, rexxMessageArg }
    from std.amiga.arexx;
import { Clipboard, openClipboard, closeClipboard, clipboardWrite,
         clipboardRead, clipboardClear } from std.amiga.clipboard;
import { Serial, Parallel, Printer, openSerial, closeSerial, serialWrite,
         serialRead, openParallel, closeParallel, parallelWrite,
         openPrinter, closePrinter, printerWrite } from std.amiga.devices;
import { NonvolatileData, getNonvolatile, freeNonvolatileData,
         nonvolatileSize, nonvolatileRead, storeNonvolatile,
         deleteNonvolatile } from std.amiga.nonvolatile;
import { String, stringOf, toCString, bytes } from std.text;
import { slice } from std.mem;

func main(): i32 {
    let name: String = stringOf("Bundle9");
    let portCreated: MsgPort? = createMsgPort();
    if (portCreated == null) { return 1; }
    let port: MsgPort = portCreated;
    let commodityCreated: Commodity? =
        openCommodity(toCString(name), toCString(name), toCString(name),
                      port, true, 0);
    if (commodityCreated == null) { return 2; }
    let commodity: Commodity = commodityCreated;
    closeCommodity(commodity);

    let empty: String = stringOf("");
    let localeCreated: Locale? = openLocale(toCString(empty));
    if (localeCreated == null) { return 3; }
    let locale: Locale = localeCreated;
    let language: String? = localeLanguageName(locale);
    let country: u32 = localeCountryCode(locale);
    let gmt: i32 = localeGmtOffset(locale);
    let pattern: String? = localeDateTimeFormat(locale);
    closeLocale(locale);

    let replyCreated: MsgPort? = createMsgPort();
    if (replyCreated == null) { return 4; }
    let reply: MsgPort = replyCreated;
    let extension: String = stringOf("");
    let host: String = stringOf("");
    let messageCreated: RexxMessage? =
        createRexxMessage(reply, toCString(extension), toCString(host));
    if (messageCreated == null) { return 5; }
    let message: RexxMessage = messageCreated;
    if (!setRexxAction(message, 1)) { return 6; }
    if (!setRexxArg(message, 0, toCString(name))) { return 7; }
    sendRexxMessage(port, message);
    let receivedCreated: RexxMessage? = receiveRexxMessage(port);
    if (receivedCreated == null) { return 8; }
    let received: RexxMessage = receivedCreated;
    let action: i32 = rexxMessageAction(received);
    let result1: i32 = rexxMessageResult1(received);
    let result2: i32 = rexxMessageResult2(received);
    let argument: u8[32] = [0; 32];
    let argumentLength: i32 = rexxMessageArg(received, 0, slice(argument, 0, 32));
    replyRexxMessage(received);
    let returnedCreated: RexxMessage? = receiveRexxMessage(reply);
    if (returnedCreated == null) { return 9; }
    let returned: RexxMessage = returnedCreated;
    reclaimRexxMessage(returned);

    let clipboardCreated: Clipboard? = openClipboard(0);
    if (clipboardCreated == null) { return 10; }
    let clipboard: Clipboard = clipboardCreated;
    let clipText: String = stringOf("clip");
    if (!clipboardWrite(clipboard, bytes(clipText))) { return 11; }
    let clipBuf: u8[16] = [0; 16];
    let clipRead: i32 = clipboardRead(clipboard, slice(clipBuf, 0, 16));
    if (!clipboardClear(clipboard)) { return 12; }
    closeClipboard(clipboard);

    let serialCreated: Serial? = openSerial(0, 9600);
    if (serialCreated == null) { return 13; }
    let serial: Serial = serialCreated;
    let serialText: String = stringOf("serial");
    let serialWritten: i32 = serialWrite(serial, bytes(serialText));
    let serialBuf: u8[16] = [0; 16];
    let serialReadCount: i32 = serialRead(serial, slice(serialBuf, 0, 16));
    closeSerial(serial);

    let parallelCreated: Parallel? = openParallel(0, 0);
    if (parallelCreated == null) { return 14; }
    let parallel: Parallel = parallelCreated;
    let parallelWritten: i32 = parallelWrite(parallel, bytes(serialText));
    closeParallel(parallel);

    let printerCreated: Printer? = openPrinter(0);
    if (printerCreated == null) { return 15; }
    let printer: Printer = printerCreated;
    let printerWritten: i32 = printerWrite(printer, bytes(serialText));
    closePrinter(printer);

    let app: String = stringOf("Bundle9");
    let item: String = stringOf("item");
    let value: String = stringOf("value");
    if (!storeNonvolatile(toCString(app), toCString(item), bytes(value))) { return 16; }
    let storedCreated: NonvolatileData? = getNonvolatile(toCString(app), toCString(item));
    if (storedCreated == null) { return 17; }
    let stored: NonvolatileData = storedCreated;
    let size: u32 = nonvolatileSize(stored);
    let nvBuf: u8[32] = [0; 32];
    let nvRead: i32 = nonvolatileRead(stored, slice(nvBuf, 0, 32));
    freeNonvolatileData(stored);
    if (!deleteNonvolatile(toCString(app), toCString(item))) { return 18; }

    deleteMsgPort(reply);
    deleteMsgPort(port);
    return action + result1 + result2 + argumentLength + clipRead +
           serialWritten + serialReadCount + parallelWritten + printerWritten +
           i32(size) + nvRead + i32(country) + gmt +
           (language != null ? 1 : 0) + (pattern != null ? 1 : 0);
}
"#;

#[test]
fn the_workbench_services_surface_builds_on_hosted_amiga() {
    let scratch = ScratchDir::new("services-surface");
    let source_path = write_fixture(&scratch, "main.clo", SERVICES_SOURCE);
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
    assert_exit("services-surface", &output, 0);
    assert!(
        stderr(&output).is_empty(),
        "the services surface must compile with zero diagnostics; stderr:\n{}",
        stderr(&output)
    );
    let c = std::fs::read_to_string(&output_path).expect("the emitted C is readable");
    for call in [
        "cl_commodity_open",
        "cl_locale_open",
        "cl_rexx_create",
        "cl_clipboard_open",
        "cl_serial_open",
        "cl_nv_get",
    ] {
        assert!(c.contains(call), "the emitted C is missing {call}");
    }
}

#[test]
fn the_a500_profile_rejects_the_workbench_services_surface() {
    let scratch = ScratchDir::new("services-a500");
    let source_path = write_fixture(&scratch, "main.clo", SERVICES_SOURCE);
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
    assert_exit("services-a500", &output, 1);
    assert!(stderr(&output).contains("E11200"), "expected E11200, got:\n{}", stderr(&output));
    assert!(!output_path.exists(), "a rejected program must not emit C");
}

#[test]
fn the_workbench_services_surface_cross_builds_with_nix20() {
    cross_build("services-build20", "hosted-amiga", "nix20", SERVICES_SOURCE);
}

#[test]
fn the_workbench_services_exports_match_the_stdlib_appendix() {
    for (section, module) in [
        ("### 14.15", "std.amiga.commodities"),
        ("### 14.16", "std.amiga.locale"),
        ("### 14.17", "std.amiga.arexx"),
        ("### 14.18", "std.amiga.clipboard"),
        ("### 14.19", "std.amiga.devices"),
        ("### 14.20", "std.amiga.nonvolatile"),
    ] {
        let text = appendix_section(section);
        let module = clc::stdlib::lookup(module).expect("the module is registered");
        for name in module.exports {
            assert!(
                text.split(|c: char| !c.is_alphanumeric() && c != '_')
                    .any(|word| word == *name),
                "the appendix section {section} is missing {name}"
            );
        }
    }
}
