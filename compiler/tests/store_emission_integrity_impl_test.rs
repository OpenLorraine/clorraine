//! Implementation tests for the store-emission integrity batch.
//!
//! These cases pin the observable internals of the conditional-store
//! judgment through a lend: the exact diagnostic sequence and message name per
//! shape, the per-instance duty that reports one owning instance at most
//! whatever the instantiation order, and the byte-identical re-emission of the
//! accepted shapes. They complement the specification suite, which pins the
//! accepted and rejected program behavior.

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
            "clc-storeemitimpl-{}-{tag}-{sequence}",
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

/// Compiles `source` and returns the scratch directory, output, and C path.
fn compile_only(tag: &str, source: &str) -> (ScratchDir, Output, PathBuf) {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source)
        .unwrap_or_else(|err| panic!("{tag}: cannot write the program: {err}"));
    let c_path = scratch.path().join("out.c");
    let output = run_clc(&[
        "--profile".to_owned(),
        "host".to_owned(),
        "-o".to_owned(),
        c_path.display().to_string(),
        source_path.display().to_string(),
    ]);
    (scratch, output, c_path)
}

/// Emits the same program twice into one directory and returns both units.
///
/// Both emissions share the source path, so their `#line` directives match and
/// a difference between the two strings is a difference in the translation
/// itself, not in the scratch directory name.
fn emit_twice(tag: &str, source: &str) -> (String, String) {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source)
        .unwrap_or_else(|err| panic!("{tag}: cannot write the program: {err}"));
    let mut emitted = Vec::new();
    for name in ["first.c", "second.c"] {
        let c_path = scratch.path().join(name);
        let output = run_clc(&[
            "--profile".to_owned(),
            "host".to_owned(),
            "-o".to_owned(),
            c_path.display().to_string(),
            source_path.display().to_string(),
        ]);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{tag}: the front end rejected the program; stderr:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        emitted.push(std::fs::read_to_string(&c_path).unwrap_or_default());
    }
    (emitted[0].clone(), emitted[1].clone())
}

/// The `error E…:` codes the compiler reported on stderr, in order.
fn error_codes(output: &Output) -> Vec<String> {
    String::from_utf8_lossy(&output.stderr)
        .lines()
        .filter_map(|line| {
            let rest = line.strip_prefix("error ")?;
            let code = rest.split(':').next()?;
            code.starts_with('E').then(|| code.to_owned())
        })
        .collect()
}

/// Asserts the program was rejected with exactly the given error codes, that
/// the message contains every fragment, and that no translation unit exists.
fn assert_rejected(tag: &str, source: &str, expected: &[&str], fragments: &[&str]) {
    let (_scratch, output, c_path) = compile_only(tag, source);
    let codes = error_codes(&output);
    assert_eq!(
        codes,
        expected,
        "{tag}: expected {expected:?}, saw {codes:?}; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    for fragment in fragments {
        assert!(
            stderr.contains(fragment),
            "{tag}: expected the message to contain {fragment:?}; stderr:\n{stderr}"
        );
    }
    assert!(
        !c_path.exists(),
        "{tag}: a rejected program must not produce a translation unit"
    );
}

/// Every rejected conditional-store shape reports exactly one code and names
/// the right binding.
///
/// The amended shapes (the concrete and generic conditional self-store) name
/// the target `slot`; the unchanged shapes keep their existing naming — the
/// target for a conditional with no lend arm and for the direct self-store,
/// the source for the direct and conditional read-only lends. A second code
/// anywhere would mean the site is double-reported; a wrong name would mean
/// the wrong side is judged.
#[test]
fn should_report_one_named_error_per_conditional_store_shape() {
    assert_rejected(
        "concrete-conditional-self",
        r#"module spec.sei.impl.concrete;

import { String, stringOf } from std.text;

func st(borrow slot: String, pick: bool): void {
    let other: String = stringOf("bbbb");
    slot = pick ? slot : other;
}

func main(): i32 {
    let s: String = stringOf("aaaa");
    st(borrow s, false);
    return i32(len(s));
}
"#,
        &["E10034"],
        &["cannot release or store borrowed parameter 'slot'"],
    );
    assert_rejected(
        "generic-conditional-self",
        r#"module spec.sei.impl.generic;

import { String, stringOf } from std.text;

func st<T>(borrow slot: T, pick: bool, take other: T): void {
    slot = pick ? slot : other;
}

func main(): i32 {
    let s: String = stringOf("aaaa");
    let o: String = stringOf("bbbb");
    st<String>(borrow s, false, take o);
    return i32(len(s));
}
"#,
        &["E10034"],
        &["cannot release or store borrowed parameter 'slot'"],
    );
    assert_rejected(
        "conditional-two-takes",
        r#"module spec.sei.impl.twotakes;

import { String, stringOf } from std.text;

func f(borrow slot: String, take a: String, take b: String, pick: bool): void {
    slot = pick ? a : b;
}

func main(): i32 {
    let s: String = stringOf("aaaa");
    f(borrow s, stringOf("bbbb"), stringOf("cccc"), true);
    return i32(len(s));
}
"#,
        &["E10034"],
        &["cannot release or store borrowed parameter 'slot'"],
    );
    assert_rejected(
        "direct-self",
        r#"module spec.sei.impl.directself;

import { String, stringOf } from std.text;

func copySelf(borrow slot: String): void {
    slot = slot;
}

func main(): i32 {
    let s: String = stringOf("abc");
    copySelf(borrow s);
    return 0;
}
"#,
        &["E10034"],
        &["cannot release or store borrowed parameter 'slot'"],
    );
    assert_rejected(
        "read-only-source",
        r#"module spec.sei.impl.readonlysource;

import { String, stringOf } from std.text;

func f(borrow slot: String, other: String): void {
    slot = other;
}

func main(): i32 {
    let s: String = stringOf("aaaa");
    let o: String = stringOf("bbbb");
    f(borrow s, o);
    return i32(len(s));
}
"#,
        &["E10034"],
        &["cannot release or store borrowed parameter 'other'"],
    );
    assert_rejected(
        "conditional-read-only-arm",
        r#"module spec.sei.impl.readonlyarm;

import { String, stringOf } from std.text;

func g<T>(borrow slot: T, other: T, c: bool): void {
    slot = c ? slot : other;
}

func main(): i32 {
    let s: String = stringOf("aaaa");
    let o: String = stringOf("bbbb");
    g<String>(borrow s, o, false);
    return i32(len(s));
}
"#,
        &["E10034"],
        &["cannot release or store borrowed parameter 'other'"],
    );
}

/// One conditional store site reports once whatever the instantiation order.
///
/// The site is recorded once and the per-instance audit reports at most the
/// first owning instance; a nullable instance is accepted by the out-slot
/// carve-out. With the nullable instance first and the owning one second the
/// site must still report exactly once, and the owning-first order must not
/// change the count — a second report would mean the duty repeats, and a
/// missing one would mean the nullable instance suppressed the owning one.
#[test]
fn should_report_one_error_per_conditional_site_across_instances() {
    assert_rejected(
        "instances-nullable-first",
        r#"module spec.sei.impl.nullablefirst;

import { String, stringOf } from std.text;

func st<T>(borrow slot: T, pick: bool, take other: T): void {
    slot = pick ? slot : other;
}

func main(): i32 {
    let b: String? = stringOf("cd");
    let ob: String? = stringOf("ee");
    st<String?>(borrow b, false, take ob);
    let a: String = stringOf("ab");
    let oa: String = stringOf("ff");
    st<String>(borrow a, false, take oa);
    return 0;
}
"#,
        &["E10034"],
        &["'slot'"],
    );
    assert_rejected(
        "instances-owning-first",
        r#"module spec.sei.impl.owningfirst;

import { String, stringOf } from std.text;

func st<T>(borrow slot: T, pick: bool, take other: T): void {
    slot = pick ? slot : other;
}

func main(): i32 {
    let a: String = stringOf("ab");
    let oa: String = stringOf("ff");
    st<String>(borrow a, false, take oa);
    let b: String? = stringOf("cd");
    let ob: String? = stringOf("ee");
    st<String?>(borrow b, false, take ob);
    return 0;
}
"#,
        &["E10034"],
        &["'slot'"],
    );
}

/// The accepted shapes keep their emission, byte-identical across runs.
///
/// The duplicable instance compiles the conditional into one direct store of
/// the selected value, and the nullable shapes keep their save-and-null
/// conditional emission. Re-emitting each accepted program proves the
/// judgment change did not disturb the translation.
#[test]
fn should_keep_the_accepted_shapes_emission_byte_identical() {
    let duplicable = r#"module spec.sei.impl.duplicable;

func st<T>(borrow slot: T, pick: bool, take other: T): T {
    slot = pick ? slot : other;
    return slot;
}

func main(): i32 {
    let x: i32 = 5;
    return st<i32>(borrow x, false, 7);
}
"#;
    let (first, second) = emit_twice("reemission-duplicable", duplicable);
    assert_eq!(
        first, second,
        "reemission-duplicable: the two emissions differ"
    );
    assert!(
        first.contains("slot = (pick ? slot : other);"),
        "reemission-duplicable: the conditional store is not emitted as one direct store"
    );

    let nullable_out_slot = r#"module spec.sei.impl.nullableoutslot;

import { String, stringOf } from std.text;

func g(borrow slot: String?, pick: bool): void {
    let other: String? = stringOf("bbbb");
    slot = pick ? slot : other;
}

func main(): i32 {
    let s: String? = stringOf("aaaa");
    g(borrow s, true);
    if (s == null) {
        return 9;
    }
    return i32(len(s));
}
"#;
    let (first, second) = emit_twice("reemission-nullable-out-slot", nullable_out_slot);
    assert_eq!(
        first, second,
        "reemission-nullable-out-slot: the two emissions differ"
    );

    let nullable_generic = r#"module spec.sei.impl.nullablegeneric;

import { String, stringOf } from std.text;

func st<T>(borrow slot: T, pick: bool, take other: T): void {
    slot = pick ? slot : other;
}

func main(): i32 {
    let s: String? = stringOf("aaaa");
    let o: String? = stringOf("bbbb");
    st<String?>(borrow s, true, take o);
    if (s == null) {
        return 9;
    }
    return i32(len(s));
}
"#;
    let (first, second) = emit_twice("reemission-nullable-generic", nullable_generic);
    assert_eq!(
        first, second,
        "reemission-nullable-generic: the two emissions differ"
    );
}
