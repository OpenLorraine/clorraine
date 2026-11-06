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
