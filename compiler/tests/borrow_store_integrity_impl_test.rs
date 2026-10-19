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

/// A constant slice store emits the bounds trap before the store line.
///
/// The trap is the read path's spelling, driven by the store target's index
/// chain: the guard statement runs before the target's old value is touched,
/// and the store itself stays a direct element write.
#[test]
fn should_emit_the_bounds_trap_before_a_constant_slice_store() {
    let unit = emit_host(
        "guard-plain",
        r#"module spec.bsi.impl.guardplain;

import { slice } from std.mem;

func plain(borrow xs: i32[]?): i32 {
    if (xs != null) { xs[0] = 1; }
    return 0;
}

func main(): i32 {
    let d: i32[2] = [1, 2];
    let v: i32[]? = slice(d, 0, 2);
    return plain(borrow v);
}
"#,
    );
    let body = instance_body(
        &unit,
        "static i32 cl_spec_bsi_impl_guardplain_plain(ClSlice_i32 *xs)",
    );
    let trap = body
        .find("cl_trap(\"E11211\", \"index out of range\")")
        .unwrap_or_else(|| panic!("no bounds trap; body:\n{body}"));
    let store = body
        .find(".ptr[0] = 1")
        .unwrap_or_else(|| panic!("no element store; body:\n{body}"));
    assert!(
        trap < store,
        "the trap must precede the store; body:\n{body}"
    );
}

/// Compound and update slice stores carry the same bounds trap.
#[test]
fn should_emit_the_bounds_trap_for_compound_and_update_slice_stores() {
    let unit = emit_host(
        "guard-compound",
        r#"module spec.bsi.impl.guardcompound;

import { slice } from std.mem;

func comp(borrow xs: i32[]?): i32 {
    if (xs != null) { xs[0] += 1; }
    return 0;
}

func upd(borrow xs: i32[]?): i32 {
    if (xs != null) { xs[0]++; }
    return 0;
}

func main(): i32 {
    let d: i32[2] = [1, 2];
    let v: i32[]? = slice(d, 0, 2);
    let a: i32 = comp(borrow v);
    let b: i32 = upd(borrow v);
    return a + b;
}
"#,
    );
    for (signature, tag) in [
        (
            "static i32 cl_spec_bsi_impl_guardcompound_comp(ClSlice_i32 *xs)",
            "compound store",
        ),
        (
            "static i32 cl_spec_bsi_impl_guardcompound_upd(ClSlice_i32 *xs)",
            "update",
        ),
    ] {
        let body = instance_body(&unit, signature);
        let trap = body
            .find("cl_trap(\"E11211\", \"index out of range\")")
            .unwrap_or_else(|| panic!("{tag}: no bounds trap; body:\n{body}"));
        let store = body
            .find(".ptr[0] = ")
            .unwrap_or_else(|| panic!("{tag}: no element store; body:\n{body}"));
        assert!(
            trap < store,
            "{tag}: the trap must precede the store; body:\n{body}"
        );
    }
}

/// A fixed-array-only target builds no guard and keeps its emission.
///
/// The semantic layer validates the constant index, so no trap line appears,
/// and the `for`-step store stays in the structured header.
#[test]
fn should_keep_the_fixed_array_emission_when_no_element_level_is_dynamic() {
    let unit = emit_host(
        "guard-fixed",
        r#"module spec.bsi.impl.guardfixed;

func fixed(): i32 {
    let rows: i32[4] = [1, 2, 3, 4];
    rows[0] = 9;
    let i: i32 = 0;
    for (; i < 2; rows[0] = i) { }
    return rows[0];
}

func main(): i32 {
    return fixed();
}
"#,
    );
    let body = instance_body(&unit, "static i32 cl_spec_bsi_impl_guardfixed_fixed(void)");
    assert!(
        !body.contains("cl_trap(\"E11211\""),
        "a fixed-array-only target must not guard; body:\n{body}"
    );
    assert!(
        body.contains("for (; (i < 2); rows[0] = i)"),
        "the for-step store must stay structured; body:\n{body}"
    );
}

/// The guard program re-emits byte-identically.
#[test]
fn should_re_emit_identically_when_the_guard_program_is_emitted_twice() {
    let (first, second) = emit_twice(
        "guard-reemission",
        r#"module spec.bsi.impl.guardreemission;

import { slice } from std.mem;

func store(borrow xs: i32[]?): i32 {
    if (xs != null) { xs[0] = 7; return i32(xs[0]); }
    return 0;
}

func main(): i32 {
    let d: i32[2] = [1, 2];
    let v: i32[]? = slice(d, 0, 2);
    return store(borrow v);
}
"#,
    );
    assert_eq!(first, second, "the two emissions must be byte-identical");
}

/// The conditional carrier copy's loan write reports once in every form.
///
/// The both-place, mixed place/literal, and assignment forms each report the
/// concrete twin's `E10031`; the accepted control (the same copy without the
/// later write) emits a stable translation unit.
#[test]
fn should_report_one_write_through_a_conditional_carrier_loan() {
    let both = r#"module impl.bsi.ccboth;

import { String, stringOf } from std.text;

struct Holder { names: str[2]; }
struct Box { v: String; }

func use(c: bool): i32 {
    let b: Box = Box { v: stringOf("aa") };
    let h1: Holder = Holder { names: [b.v, "EE"] };
    let h3: Holder = Holder { names: ["CC", "DD"] };
    let h2: Holder = c ? h1 : h3;
    b.v = stringOf("zz");
    return i32(h2.names[0][0]);
}

func main(): i32 {
    return use(true);
}
"#;
    assert_rejected("ccimplboth", both, &["E10031"]);

    let mixed = both
        .replace("ccboth", "ccimplmixed")
        .replace("c ? h1 : h3", "c ? h1 : Holder { names: [\"CC\", \"DD\"] }");
    assert_rejected("ccimplmixed", &mixed, &["E10031"]);

    let accepted = r#"module impl.bsi.ccaccepted;

import { String, stringOf } from std.text;

struct Holder { names: str[2]; }
struct Box { v: String; }

func use(c: bool): i32 {
    let b: Box = Box { v: stringOf("aa") };
    let h1: Holder = Holder { names: [b.v, "EE"] };
    let h3: Holder = Holder { names: ["CC", "DD"] };
    let h2: Holder = c ? h1 : h3;
    return i32(h2.names[0][0]);
}

func main(): i32 {
    return use(true);
}
"#;
    let (first, second) = emit_twice("ccimplaccepted", accepted);
    assert_eq!(
        first, second,
        "ccimplaccepted: the two emissions differ; first:\n{first}\nsecond:\n{second}"
    );
}

/// The batch-P copy destinations each report the concrete twin's code once.
///
/// The `const` copy, the global conditional, the call-carrier copy, the mixed
/// aggregate, the conditional shadow, and the alias return keep one code per
/// site; the accepted static copy re-emits byte-identically.
#[test]
fn should_report_one_code_per_copy_destination() {
    let const_form = r#"module impl.bsi.pconst;

import { String, stringOf } from std.text;

struct Holder { names: str[2]; }
struct Box { v: String; }

func use(c: bool): i32 {
    let b: Box = Box { v: stringOf("aa") };
    let h1: Holder = Holder { names: [b.v, "EE"] };
    let h3: Holder = Holder { names: ["CC", "DD"] };
    const h2: Holder = c ? h1 : h3;
    b.v = stringOf("zz");
    return i32(h2.names[0][0]);
}

func main(): i32 {
    return use(true);
}
"#;
    assert_rejected("pconst", const_form, &["E10031"]);

    let global = r#"module impl.bsi.pglobal;

import { String, stringOf } from std.text;

struct Holder { names: str[2]; }
struct Box { v: String; }

let g: Holder = Holder { names: ["CC", "DD"] };

func set(c: bool): void {
    let b: Box = Box { v: stringOf("aa") };
    let h1: Holder = Holder { names: [b.v, "EE"] };
    let h3: Holder = Holder { names: ["FF", "GG"] };
    g = c ? h1 : h3;
}

func main(): i32 {
    set(true);
    return i32(g.names[0][0]) - 97;
}
"#;
    assert_rejected("pglobal", global, &["E10033"]);

    let call = r#"module impl.bsi.pcall;

import { String, stringOf } from std.text;

struct Holder { names: str[2]; }
struct Box { v: String; }

func makeHolder(v: str): Holder { return Holder { names: [v, "EE"] }; }

func use(): i32 {
    let b: Box = Box { v: stringOf("aa") };
    let h: Holder = makeHolder(b.v);
    b.v = stringOf("zz");
    return i32(h.names[0][0]);
}

func main(): i32 {
    return use() - 97;
}
"#;
    assert_rejected("pcall", call, &["E10031"]);

    let mixed = r#"module impl.bsi.pmixed;

import { String, stringOf } from std.text;

struct H { name: String; tag: static str; }

func store(h: H, borrow slot: str?): void { slot = h.name; }

func outer(p: static str, borrow slot: str?): void {
    let h: H = H { name: stringOf("aa"), tag: p };
    store(h, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer("tag", borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
    assert_rejected("pmixed", mixed, &["E10030"]);

    let shadow = r#"module impl.bsi.pshadow;

import { String, stringOf } from std.text;

struct Holder { names: str[2]; }
struct Box { v: String; }

func use(): i32 {
    let b: Box = Box { v: stringOf("aa") };
    let h: Holder = Holder { names: [b.v, "EE"] };
    {
        let h: Holder = true ? h : h;
        b.v = stringOf("zz");
        return i32(h.names[0][0]);
    }
}

func main(): i32 {
    return use() - 97;
}
"#;
    assert_rejected("pshadow", shadow, &["E10031"]);

    let alias = r#"module impl.bsi.palias;

import { String, stringOf } from std.text;

struct Box { v: String; }
struct Holder { names: str[2]; }

func use(): Holder {
    let b: Box = Box { v: stringOf("aa") };
    let hs: Holder[1] = [Holder { names: [b.v, "EE"] }];
    for (borrow e in hs) {
        return e;
    }
    return Holder { names: ["CC", "DD"] };
}

func main(): i32 {
    let h: Holder = use();
    return i32(h.names[0][0]) - 97;
}
"#;
    assert_rejected("palias", alias, &["E10030"]);

    let accepted = r#"module impl.bsi.paccepted;

struct Holder { names: str[2]; }

func use(c: bool): i32 {
    let h1: Holder = Holder { names: ["AA", "EE"] };
    let h3: Holder = Holder { names: ["CC", "DD"] };
    const h2: Holder = c ? h1 : h3;
    return i32(h2.names[0][0]) - 65;
}

func main(): i32 {
    return use(true);
}
"#;
    let (first, second) = emit_twice("paccepted", accepted);
    assert_eq!(
        first, second,
        "paccepted: the two emissions differ; first:\n{first}\nsecond:\n{second}"
    );
}
