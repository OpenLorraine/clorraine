//! Implementation tests for the borrow-store integrity batch.
//!
//! These cases pin the observable internals of the per-instance whole-slot
//! self-assignment judgment: the one-report-per-site contract with the first
//! offending instance in instantiation order, the separate report of two
//! sites in one body, the accepted duplicable instance's plain self-copy
//! emission, and the byte-identical re-emission of an accepted program. They
//! complement the specification suite, which pins the accepted and rejected
//! program behavior.

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
            "clc-borrowstoreimpl-{}-{tag}-{sequence}",
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

/// Emits one host-profile program and asserts the front end accepted it.
///
/// Returns the emitted translation unit.
fn emit_host(tag: &str, source: &str) -> String {
    let (_scratch, output, c_path) = compile_only(tag, source);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{tag}: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    std::fs::read_to_string(&c_path).unwrap_or_else(|err| panic!("{tag}: no unit: {err}"))
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

/// Asserts the program was rejected with exactly the given error codes.
fn assert_rejected(tag: &str, source: &str, expected: &[&str]) {
    let (_scratch, output, c_path) = compile_only(tag, source);
    let codes = error_codes(&output);
    assert_eq!(
        codes,
        expected,
        "{tag}: expected {expected:?}, saw {codes:?}; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !c_path.exists(),
        "{tag}: a rejected program must not produce a translation unit"
    );
}

/// The emitted body of the named instance starting at its definition.
///
/// The signature is matched with the block opener that follows it so the
/// prototype near the top of the unit is skipped and the definition is
/// returned.
fn instance_body<'a>(unit: &'a str, signature: &str) -> &'a str {
    let needle = format!("{signature}\n{{");
    let start = unit
        .find(&needle)
        .unwrap_or_else(|| panic!("no definition for {signature}"));
    let rest = &unit[start..];
    let end = rest.find("\n}").map(|end| end + 2).unwrap_or(rest.len());
    &rest[..end]
}

/// One self-assign site instantiated at an owning and a nullable type reports
/// once — the first offending instance in instantiation order.
///
/// The site is visited once per recorded instance; the audit breaks after the
/// first offending instance, so the owning-first program keeps one diagnostic
/// and the nullable-first program keeps its own. A second report would mean
/// the break is missing; a wrong code means the per-instance judgment is not
/// following the substitution.
#[test]
fn should_report_the_first_offending_instance_when_a_self_assign_site_has_two() {
    assert_rejected(
        "two-instances-owning-first",
        r#"module spec.bsi.impl.twoinstances;

import { String, stringOf } from std.text;

func copySelf<T>(borrow slot: T): void { slot = slot; }

func main(): i32 {
    let a: String = stringOf("ab");
    copySelf<String>(borrow a);
    let b: String? = stringOf("cd");
    copySelf<String?>(borrow b);
    return 0;
}
"#,
        &["E10034"],
    );
    assert_rejected(
        "two-instances-nullable-first",
        r#"module spec.bsi.impl.twoinstances;

import { String, stringOf } from std.text;

func copySelf<T>(borrow slot: T): void { slot = slot; }

func main(): i32 {
    let b: String? = stringOf("cd");
    copySelf<String?>(borrow b);
    let a: String = stringOf("ab");
    copySelf<String>(borrow a);
    return 0;
}
"#,
        &["E10020"],
    );
}

/// Two self-assign sites in one body report separately, one per site.
///
/// The recording-time dedupe is keyed by site, so two parameters self-assigning
/// at different spans stay two duties; collapsing them would hide the second
/// offending parameter from the diagnostic.
#[test]
fn should_report_each_self_assign_site_when_two_parameters_self_assign() {
    let (_scratch, output, c_path) = compile_only(
        "two-sites",
        r#"module spec.bsi.impl.twosites;

import { String, stringOf } from std.text;

func fill<T>(borrow a: T, borrow b: T): void {
    a = a;
    b = b;
}

func main(): i32 {
    let s: String = stringOf("ab");
    let t: String = stringOf("cd");
    fill<String>(borrow s, borrow t);
    return 0;
}
"#,
    );
    assert_eq!(
        error_codes(&output),
        vec!["E10034", "E10034"],
        "expected one report per site; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !c_path.exists(),
        "a rejected program must not produce a translation unit"
    );
}

/// An accepted duplicable instance emits a plain self-copy with no release.
///
/// The invocation passes the handle by value and the body copies it, so an
/// owning-substitution release or zeroing must never appear here: either would
/// be dead code for a scalar or a double release for a handle type.
#[test]
fn should_emit_a_plain_self_copy_when_the_instance_is_duplicable() {
    let unit = emit_host(
        "duplicable-copy",
        r#"module spec.bsi.impl.duplicablecopy;

func copySelf<T>(borrow slot: T): void { slot = slot; }

func main(): i32 {
    let n: i32 = 1;
    copySelf<i32>(borrow n);
    return n;
}
"#,
    );
    let body = instance_body(&unit, "static void cl_copySelf__i32(i32 slot)");
    assert!(
        body.contains("slot = slot;"),
        "the accepted instance must emit its plain self-copy; body:\n{body}"
    );
    assert!(
        !body.contains("Free") && !body.contains("cl_drop"),
        "the accepted duplicable instance must not release anything; body:\n{body}"
    );
}

/// The accepted self-assign program re-emits byte-identically.
///
/// The judgment adds no emission; two emissions over the same source path
/// must produce the same translation unit, so a map-order dependency or a
/// stray temporary would show up as a difference here.
#[test]
fn should_re_emit_identically_when_the_self_assign_program_is_emitted_twice() {
    let (first, second) = emit_twice(
        "self-assign-reemission",
        r#"module spec.bsi.impl.reemission;

import { slice } from std.mem;

func copySelf<T>(borrow slot: T): void { slot = slot; }

func main(): i32 {
    let n: i32 = 3;
    copySelf<i32>(borrow n);
    let d: u8[2] = [1, 2];
    let v: u8[] = slice(d, 0, 2);
    copySelf<u8[]>(borrow v);
    return n;
}
"#,
    );
    assert_eq!(first, second, "the two emissions must be byte-identical");
}

/// A direct whole-slot return emits a block-local copy and empties the slot.
///
/// The returned handle copies the caller's slot, so the slot must be zeroed
/// before control leaves or both the slot and the result would release one
/// allocation. The value store precedes the zeroing so the copy still reads
/// the payload.
#[test]
fn should_empty_the_caller_slot_when_a_direct_return_copies_it() {
    let unit = emit_host(
        "direct-return-empty",
        r#"module spec.bsi.impl.directreturn;

import { String, stringOf } from std.text;

func cid(borrow v: String?): String? { return v; }

func main(): i32 {
    let s: String? = stringOf("abc");
    let r: String? = cid(borrow s);
    if (s != null) { return 90; }
    if (r != null) { return i32(len(r)); }
    return -1;
}
"#,
    );
    let body = instance_body(&unit, "static ClString cl_spec_bsi_impl_directreturn_cid(ClString *v)");
    assert!(
        body.contains("__cl_return"),
        "the direct return must use a block-local; body:\n{body}"
    );
    assert!(
        body.contains("(*v) = NULL;"),
        "the caller's slot must be emptied; body:\n{body}"
    );
    let store = body.find("= (*v);").expect("the value store");
    let empty = body.find("(*v) = NULL;").expect("the emptying");
    assert!(
        store < empty,
        "the value must be read before the slot is emptied; body:\n{body}"
    );
}

/// A view return keeps its direct emission: a view owns no storage to empty.
///
/// The returned slice aliases the caller's view; zeroing it would not release
/// anything and the ownership transfer rule does not apply to a view, so the
/// emission stays a plain return.
#[test]
fn should_keep_the_direct_emission_when_a_view_slot_is_returned() {
    let unit = emit_host(
        "view-return-direct",
        r#"module spec.bsi.impl.viewreturn;

func vid(borrow v: u8[]?): u8[]? { return v; }

func main(): i32 {
    return 0;
}
"#,
    );
    let body =
        instance_body(&unit, "static ClSlice_u8 cl_spec_bsi_impl_viewreturn_vid(ClSlice_u8 *v)");
    assert!(
        body.contains("return (*v);"),
        "the view return must stay direct; body:\n{body}"
    );
    assert!(
        !body.contains("(*v) = NULL;"),
        "a view has no storage to empty; body:\n{body}"
    );
}

/// The out-slot return program re-emits byte-identically.
#[test]
fn should_re_emit_identically_when_the_out_slot_return_program_is_emitted_twice() {
    let (first, second) = emit_twice(
        "out-slot-reemission",
        r#"module spec.bsi.impl.outslotreemission;

import { String, stringOf } from std.text;

func cid(borrow v: String?): String? { return v; }

func main(): i32 {
    let s: String? = stringOf("abc");
    let r: String? = cid(borrow s);
    if (s != null) { return 90; }
    if (r != null) { return i32(len(r)); }
    return -1;
}
"#,
    );
    assert_eq!(first, second, "the two emissions must be byte-identical");
}
