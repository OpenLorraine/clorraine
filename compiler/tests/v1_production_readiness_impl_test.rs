//! Implementation pins for the v1.0 production-readiness residual fixes.
//!
//! These tests read the emitted C and pin the ownership shape behind the
//! specification cases: which fresh call root is bound, which slot is emptied,
//! how many releases run, and that the run-time index is shared between the
//! read and the post-store move-out. They are implementation tests, so an
//! intended emission change may update them; the behaviour itself is covered by
//! `v1_production_readiness_spec_test.rs`.

use std::path::{Path, PathBuf};
use std::process::Command;
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
            "clc-v1pr-impl-{}-{tag}-{sequence}",
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

/// Emits `source` into `scratch` and returns the translation unit named `c_name`.
fn emit_into(tag: &str, scratch: &ScratchDir, source: &str, c_name: &str) -> String {
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source)
        .unwrap_or_else(|err| panic!("{tag}: cannot write the program: {err}"));
    let c_path = scratch.path().join(c_name);
    let output = Command::new(env!("CARGO_BIN_EXE_clc"))
        .args([
            "--profile",
            "host",
            "-o",
            c_path.to_str().expect("C path"),
            source_path.to_str().expect("source path"),
        ])
        .output()
        .expect("failed to spawn the compiler binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "{tag}: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let emitted = std::fs::read_to_string(&c_path).unwrap_or_default();
    assert!(!emitted.trim().is_empty(), "{tag}: the emitted C is empty");
    emitted
}

/// Emits one host-profile program and returns the translation unit.
fn emit_host(tag: &str, source: &str) -> String {
    let scratch = ScratchDir::new(tag);
    emit_into(tag, &scratch, source, "out.c")
}

/// Asserts the emitted C contains `needle`.
fn assert_contains(tag: &str, emitted: &str, needle: &str) {
    assert!(
        emitted.contains(needle),
        "{tag}: expected emitted C to contain {needle:?}"
    );
}

/// Asserts the emitted C does not contain `needle`.
fn assert_absent(tag: &str, emitted: &str, needle: &str) {
    assert!(
        !emitted.contains(needle),
        "{tag}: emitted C unexpectedly contains {needle:?}"
    );
}

/// Asserts `first` appears before `second` in the emitted C.
fn assert_before(tag: &str, emitted: &str, first: &str, second: &str) {
    let first_at = emitted
        .find(first)
        .unwrap_or_else(|| panic!("{tag}: missing {first:?}"));
    let second_at = emitted
        .find(second)
        .unwrap_or_else(|| panic!("{tag}: missing {second:?}"));
    assert!(
        first_at < second_at,
        "{tag}: expected {first:?} before {second:?}"
    );
}

/// ST-2: bind both nested fill rows in source order and free the unselected one.
#[test]
fn should_bind_nested_fill_rows_and_free_only_the_unselected_root() {
    let source = r#"module spec.v1pr.nestedfill;

import { stringOf } from std.text;

struct Grid { rows: str[2][2]; }

let first: str = Grid { rows: [[; stringOf("aa")], [; stringOf("bb")]] }.rows[0][0];

func main(): i32 {
    return i32(len(first)) - 2;
}
"#;
    let emitted = emit_host("nested-fill-pin", source);
    assert_before(
        "nested-fill-pin",
        &emitted,
        "cl_std_text_stringOf(cl_str(\"aa\"))",
        "cl_std_text_stringOf(cl_str(\"bb\"))",
    );
    assert_contains(
        "nested-fill-pin",
        &emitted,
        "ClString __cl_itmp1 = cl_std_text_stringOf(cl_str(\"aa\"));",
    );
    assert_contains(
        "nested-fill-pin",
        &emitted,
        "ClString __cl_itmp2 = cl_std_text_stringOf(cl_str(\"bb\"));",
    );
    assert_contains(
        "nested-fill-pin",
        &emitted,
        "if (__cl_itmp2) { FreeString(__cl_itmp2); }",
    );
    assert_absent("nested-fill-pin", &emitted, "FreeString(__cl_itmp1)");
}

/// ST-3: move the selected member out and release the unselected root whole.
#[test]
fn should_release_the_unselected_multi_element_root_but_not_the_selected_member() {
    let source = r#"module spec.v1pr.multielement;

import { String, stringOf } from std.text;

struct H { v: String; w: String; }
struct Box { names: str[2]; tag: String; }

func mk(): H { return H { v: stringOf("aa"), w: stringOf("bb") }; }

let picked: str = Box { names: [mk().v, mk().w], tag: stringOf("t") }.names[0];

func main(): i32 {
    return i32(len(picked)) - 2;
}
"#;
    let emitted = emit_host("multi-element-pin", source);
    assert_contains(
        "multi-element-pin",
        &emitted,
        "__cl_itmp3.names[0] = cl_zero_ClStr();",
    );
    assert_contains(
        "multi-element-pin",
        &emitted,
        "if (__cl_itmp1.v) { FreeString(__cl_itmp1.v); }",
    );
    assert_contains(
        "multi-element-pin",
        &emitted,
        "if (__cl_itmp1.w) { FreeString(__cl_itmp1.w); }",
    );
    assert_contains(
        "multi-element-pin",
        &emitted,
        "if (__cl_itmp0.w) { FreeString(__cl_itmp0.w); }",
    );
    assert_before(
        "multi-element-pin",
        &emitted,
        "__cl_itmp0.v = NULL;",
        "if (__cl_itmp0.v) { FreeString(__cl_itmp0.v); }",
    );
}

/// ST-4: bind the whole `mk()` root for a field-call lend argument.
#[test]
fn should_bind_the_whole_call_root_for_a_field_call_lend() {
    let source = r#"module spec.v1pr.fieldcall;

import { String, stringOf } from std.text;

struct H { xs: String[2]; }
struct Box { n: i32; }

func mk(): H { return H { xs: [stringOf("aa"), stringOf("bb")] }; }
func sink(s: str): i32 { return i32(len(s)); }

func main(): i32 {
    let b: Box = Box { n: sink(mk().xs[0]) };
    return b.n - 2;
}
"#;
    let emitted = emit_host("field-call-pin", source);
    assert_contains(
        "field-call-pin",
        &emitted,
        "ClH __cl_owned0 = {0};",
    );
    assert_contains(
        "field-call-pin",
        &emitted,
        "__cl_owned0 = cl_spec_v1pr_fieldcall_mk();",
    );
    assert_contains(
        "field-call-pin",
        &emitted,
        "cl_spec_v1pr_fieldcall_sink(cl_string_view(__cl_owned0.xs[0]))",
    );
    assert_contains(
        "field-call-pin",
        &emitted,
        "if (__cl_owned0.xs[1]) { FreeString(__cl_owned0.xs[1]); }",
    );
}

/// ST-4b: the func-typed callee twin binds and releases the same root.
#[test]
fn should_bind_the_whole_call_root_for_a_func_typed_field_call_lend() {
    let source = r#"module spec.v1pr.fieldcallfunc;

import { String, stringOf } from std.text;

struct H { xs: String[2]; }
struct Box { n: i32; }

func mk(): H { return H { xs: [stringOf("aa"), stringOf("bb")] }; }
func sink(s: str): i32 { return i32(len(s)); }

func main(): i32 {
    let f: func(str): i32 = sink;
    let b: Box = Box { n: f(mk().xs[0]) };
    return b.n - 2;
}
"#;
    let emitted = emit_host("field-call-func-pin", source);
    assert_contains(
        "field-call-func-pin",
        &emitted,
        "ClH __cl_owned0 = {0};",
    );
    assert_contains(
        "field-call-func-pin",
        &emitted,
        "__cl_owned0 = cl_spec_v1pr_fieldcallfunc_mk();",
    );
    assert_contains(
        "field-call-func-pin",
        &emitted,
        "f(cl_string_view(__cl_owned0.xs[0]))",
    );
    assert_contains(
        "field-call-func-pin",
        &emitted,
        "if (__cl_owned0.xs[1]) { FreeString(__cl_owned0.xs[1]); }",
    );
}

/// ST-5: the run-time index is evaluated once and shared by read and release.
#[test]
fn should_share_the_run_time_index_between_the_read_and_the_release() {
    let source = r#"module spec.v1pr.indexedview;

import { String, stringOf } from std.text;

func mk(): String[2] { return [stringOf("aa"), stringOf("bb")]; }
func pick(): usize { return 1; }

let selected: str = mk()[pick()];

func main(): i32 {
    return i32(len(selected)) - 2;
}
"#;
    let emitted = emit_host("indexed-view-pin", source);
    assert_contains(
        "indexed-view-pin",
        &emitted,
        "usize __cl_itmp1 = cl_spec_v1pr_indexedview_pick();",
    );
    assert_before(
        "indexed-view-pin",
        &emitted,
        "cl_spec_v1pr_indexedview_selected = cl_string_view(__cl_itmp0[__cl_itmp1]);",
        "__cl_itmp0[__cl_itmp1] = NULL;",
    );
    assert_contains(
        "indexed-view-pin",
        &emitted,
        "__cl_itmp0[__cl_itmp1] = NULL;",
    );
    assert_contains(
        "indexed-view-pin",
        &emitted,
        "if (__cl_itmp0[0]) { FreeString(__cl_itmp0[0]); }",
    );
    assert_contains(
        "indexed-view-pin",
        &emitted,
        "if (__cl_itmp0[1]) { FreeString(__cl_itmp0[1]); }",
    );
}

/// ST-2 re-emits byte-identically, so the pinned ownership shape is stable.
#[test]
fn should_reemit_the_nested_fill_initializer_byte_identically() {
    let source = r#"module spec.v1pr.nestedfill;

import { stringOf } from std.text;

struct Grid { rows: str[2][2]; }

let first: str = Grid { rows: [[; stringOf("aa")], [; stringOf("bb")]] }.rows[0][0];

func main(): i32 {
    return i32(len(first)) - 2;
}
"#;
    let scratch = ScratchDir::new("nested-fill-repeat");
    let first = emit_into("nested-fill-repeat-a", &scratch, source, "a.c");
    let second = emit_into("nested-fill-repeat-b", &scratch, source, "b.c");
    assert_eq!(
        first, second,
        "nested-fill-repeat: the emitted translation unit is not deterministic"
    );
}
