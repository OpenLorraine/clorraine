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
