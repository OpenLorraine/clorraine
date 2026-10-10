//! Runnable specification tests for store rules whose judgment or emission
//! must hold at the whole-value, element-view, type-order, and module-init
//! boundaries.
//!
//! Each case compiles a small Clorraine program with the built `clc` binary,
//! builds the emitted C with the host C compiler under the fixed warning
//! flags, runs the binary, and asserts the process result. Programs that must
//! be rejected assert the diagnostics instead, and no translation unit may be
//! written for them.
//!
//! The conditional-store group pins the whole-value store through a lend when
//! the stored value is a conditional whose only lend source is the target arm
//! itself: the store rule must judge the assignment exactly as the direct
//! self-store — one `E10034` naming the target — because the other arm would
//! still release the caller's value. A conditional harmed by a read-only lend
//! arm keeps its single report on that arm, the direct spellings keep theirs,
//! and the nullable out-slot carve-out stays accepted and memory-clean.
//!
//! The immutable-view group pins the write rule for views of immutable bytes
//! (`str` and `static str`): a plain, compound, or update element write over a
//! parameter, a local, a field, or a module-level view is each rejected with
//! one `E10042` naming the written path, while a whole-view rebind and a write
//! through a writable (borrow-derived) slice stay accepted. The registered
//! golden is compiled directly, so the frozen fixture itself is asserted.
//!
//! The memory helper runs a case as an ordinary strict host build and adds
//! leak and use-after-free detection when `RDC_ASAN` is set.

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// How long one program may run before the case fails as a hang.
const RUN_TIMEOUT: Duration = Duration::from_secs(30);

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
            "clc-storeemitspec-{}-{tag}-{sequence}",
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

/// Compiles `source` for the host profile and returns the scratch directory,
/// the compiler output, and the emitted-C path.
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

/// Compiles an existing source file for the host profile.
///
/// Used by the golden-program case: the file under `spec/` is compiled where
/// it lives, so the case asserts the frozen fixture itself rather than a copy.
fn compile_path(tag: &str, source_path: &Path) -> (ScratchDir, Output, PathBuf) {
    let scratch = ScratchDir::new(tag);
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

/// Asserts one compiler run rejected the program with exactly one `code`
/// diagnostic, that the message contains every fragment, and that no
/// translation unit is written.
///
/// The count assertion pins the one-report rule: a site judged by two paths
/// must still produce a single diagnostic.
fn assert_rejected_output(
    tag: &str,
    output: &Output,
    c_path: &Path,
    code: &str,
    fragments: &[&str],
) {
    assert_ne!(
        output.status.code(),
        Some(0),
        "{tag}: the front end accepted a program that must be rejected"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    let marker = format!("error {code}:");
    assert_eq!(
        stderr.matches(&marker).count(),
        1,
        "{tag}: expected exactly one {code}; stderr:\n{stderr}"
    );
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

/// Compiles `source` and asserts the rejection contract on the output.
fn assert_rejected_once(tag: &str, source: &str, code: &str, fragments: &[&str]) {
    let (_scratch, output, c_path) = compile_only(tag, source);
    assert_rejected_output(tag, &output, &c_path, code, fragments);
}

/// Compiles an existing file and asserts the rejection contract on the
/// output.
fn assert_rejected_once_at(tag: &str, source_path: &Path, code: &str, fragments: &[&str]) {
    let (_scratch, output, c_path) = compile_path(tag, source_path);
    assert_rejected_output(tag, &output, &c_path, code, fragments);
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
    let emitted = std::fs::read_to_string(&c_path).unwrap_or_default();
    assert!(
        !emitted.trim().is_empty(),
        "{tag}: the emitted C is empty, so the emitter is not implemented yet"
    );
    emitted
}

/// Compiles the emitted C with the host compiler under the strict flag set.
///
/// Panics with the compiler's stderr when the build fails. Returns the built
/// program path.
fn build_host_c(tag: &str, scratch: &ScratchDir, emitted: &str) -> PathBuf {
    let c_path = scratch.path().join("build.c");
    std::fs::write(&c_path, emitted).expect("write the emitted C");
    let program = scratch.path().join("prog");
    let built = Command::new("cc")
        .args([
            "-std=c99",
            "-O0",
            "-g",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-o",
            program.to_str().expect("program path"),
            c_path.to_str().expect("emitted C path"),
        ])
        .output()
        .expect("failed to spawn the host C compiler");
    assert!(
        built.status.success(),
        "{tag}: the host C compiler rejected the emitted C; stderr:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
    program
}

/// Runs a built program with a time limit and the given environment values.
///
/// A program that is still running when the limit passes is killed and the
/// case fails, so a regression that loops forever surfaces as one failing
/// case instead of a frozen suite.
fn run_with_timeout(tag: &str, program: &Path, env: &[(&str, &str)]) -> Output {
    let mut command = Command::new(program);
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    for (key, value) in env {
        command.env(key, value);
    }
    let mut child = command
        .spawn()
        .unwrap_or_else(|err| panic!("{tag}: cannot run the program: {err}"));
    let deadline = Instant::now() + RUN_TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(20));
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                panic!("{tag}: the program did not finish within {RUN_TIMEOUT:?}");
            }
            Err(err) => panic!("{tag}: cannot wait for the program: {err}"),
        }
    }
    child
        .wait_with_output()
        .unwrap_or_else(|err| panic!("{tag}: cannot collect the program output: {err}"))
}

/// Emits, builds, and runs one host program, returning its output.
fn run_host(tag: &str, source: &str) -> Output {
    let scratch = ScratchDir::new(tag);
    let emitted = emit_host(tag, source);
    let program = build_host_c(tag, &scratch, &emitted);
    run_with_timeout(tag, &program, &[])
}

/// Emits a program, builds it with AddressSanitizer, and runs it with leak
/// detection on, returning its output.
///
/// A leaked allocation fails the run and a double free aborts, so a clean exit
/// pins release-once for the case. The sanitizer build is linked non-PIE
/// (`-no-pie`): the pinned container's sanitizer runtime crash-loops during
/// start-up on PIE images when the host kernel's page-address randomization
/// uses its full entropy range, while a non-PIE image starts reliably, and
/// leak and use-after-free detection are unaffected by the link layout.
fn run_asan(tag: &str, source: &str) -> Output {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source)
        .unwrap_or_else(|err| panic!("{tag}: cannot write the program: {err}"));
    let c_path = scratch.path().join("out.c");
    let emitted = run_clc(&[
        "--profile".to_owned(),
        "host".to_owned(),
        "-o".to_owned(),
        c_path.display().to_string(),
        source_path.display().to_string(),
    ]);
    assert_eq!(
        emitted.status.code(),
        Some(0),
        "{tag}: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&emitted.stderr)
    );
    let program = scratch.path().join("prog");
    let built = Command::new("cc")
        .args([
            "-std=c99",
            "-O0",
            "-g",
            "-fsanitize=address",
            "-no-pie",
            "-o",
            program.to_str().expect("program path"),
            c_path.to_str().expect("emitted C path"),
        ])
        .output()
        .expect("failed to spawn the host C compiler");
    assert!(
        built.status.success(),
        "{tag}: the sanitizer build failed; stderr:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
    run_with_timeout(
        tag,
        &program,
        &[("ASAN_OPTIONS", "detect_leaks=1:symbolize=0")],
    )
}

/// Runs a memory-safety case: under AddressSanitizer when `RDC_ASAN` is set,
/// otherwise as an ordinary strict host build.
///
/// The plain build still aborts in the C library on a double release, so the
/// exit code stays observable; the sanitizer run adds leak and use-after-free
/// detection.
fn run_memory(tag: &str, source: &str) -> Output {
    if std::env::var_os("RDC_ASAN").is_some() {
        run_asan(tag, source)
    } else {
        run_host(tag, source)
    }
}

/// Asserts the program exited with exactly `expected`, reporting its stderr.
fn assert_exit(tag: &str, output: &Output, expected: i32) {
    assert_eq!(
        output.status.code(),
        Some(expected),
        "{tag}: expected exit code {expected}; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Asserts the run produced no AddressSanitizer report.
///
/// The check is meaningful under `RDC_ASAN`; a plain strict build has no
/// sanitizer output to find. A leak or use-after-free that aborts the run
/// fails the exit-code assertion first.
fn assert_no_asan(tag: &str, output: &Output) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("AddressSanitizer"),
        "{tag}: the run reported a memory error; stderr:\n{stderr}"
    );
}

/// A conditional store whose only lend source is the target arm is rejected
/// once.
///
/// The callee receives `slot` as a writable lend. One arm of the conditional
/// names `slot` itself. Storing the target into itself moves nothing out of
/// it, but the other arm stores a different value over the caller's storage,
/// so the assignment still releases the caller's value — exactly the release
/// the lend-store rule forbids. The offense is judged on the target, so
/// exactly one `E10034` names `slot`.
#[test]
fn should_reject_a_conditional_self_store_through_a_concrete_lend() {
    let source = r#"module spec.sei.conditionalconcrete;

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
"#;
    assert_rejected_once("conditional-concrete-self", source, "E10034", &["'slot'"]);
}

/// The same conditional store through a parameter-typed lend is rejected once
/// when the instantiation owns.
///
/// `slot` is declared `borrow slot: T`; substituting `String` makes the slot
/// owning, so the conditional release is judged per instance exactly as the
/// concrete spelling is. Exactly one `E10034` names `slot`.
#[test]
fn should_reject_a_conditional_self_store_when_the_generic_instance_owns() {
    let source = r#"module spec.sei.conditionalgneric;

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
"#;
    assert_rejected_once("conditional-generic-owning", source, "E10034", &["'slot'"]);
}

/// The same conditional store through a duplicable instance is accepted and
/// runs.
///
/// Substituting `i32` makes the slot duplicable: the assignment copies, so no
/// release of caller storage happens and the store stays accepted. A scalar
/// lend binds a copy, so the case observes the stored value through the
/// callee's own copy.
#[test]
fn should_accept_and_run_a_conditional_self_store_when_the_instance_copies() {
    let source = r#"module spec.sei.conditionali32;

func st<T>(borrow slot: T, pick: bool, take other: T): T {
    slot = pick ? slot : other;
    return slot;
}

func main(): i32 {
    let x: i32 = 5;
    return st<i32>(borrow x, false, 7);
}
"#;
    let output = run_memory("conditional-generic-i32", source);
    assert_exit("conditional-generic-i32", &output, 7);
    assert_no_asan("conditional-generic-i32", &output);
}

/// A conditional store of two taken values into a lend is rejected once on
/// the target.
///
/// Neither arm names the target; both are callee-owned values. Storing either
/// arm over the lend's storage releases the caller's value, so exactly one
/// `E10034` names `slot` — the unchanged report for a conditional with no
/// lend arm.
#[test]
fn should_reject_a_conditional_of_two_taken_values_into_a_lend() {
    let source = r#"module spec.sei.conditionaltwoakes;

import { String, stringOf } from std.text;

func f(borrow slot: String, take a: String, take b: String, pick: bool): void {
    slot = pick ? a : b;
}

func main(): i32 {
    let s: String = stringOf("aaaa");
    f(borrow s, stringOf("bbbb"), stringOf("cccc"), true);
    return i32(len(s));
}
"#;
    assert_rejected_once("conditional-two-takes", source, "E10034", &["'slot'"]);
}

/// A direct store from a read-only lend is rejected once on the source.
///
/// The right side names `other`, an unmarked (read-only) parameter. Storing it
/// as a whole value would move out of the read-only lend, so exactly one
/// `E10034` names `other` — the unchanged report.
#[test]
fn should_reject_a_direct_store_from_a_read_only_lend_source() {
    let source = r#"module spec.sei.readonlysource;

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
"#;
    assert_rejected_once("read-only-source-store", source, "E10034", &["'other'"]);
}

/// A conditional with a read-only lend arm keeps its single report on that
/// arm.
///
/// The self arm names the target and moves nothing out of it; the read-only
/// arm `other` would be moved out of a lend by the store, which the source
/// side reports. Exactly one `E10034` names `other`, and the target side must
/// not add a second report for the same site.
#[test]
fn should_report_one_error_for_a_mixed_conditional_with_a_read_only_arm() {
    let source = r#"module spec.sei.conditionalreadonlyarm;

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
"#;
    assert_rejected_once("conditional-read-only-arm", source, "E10034", &["'other'"]);
}

/// The nullable out-slot conditional store stays accepted, builds, and runs
/// memory-clean.
///
/// The callee receives `slot` as a nullable writable lend and stores itself
/// back through the conditional; the save is nulled instead of released, so
/// the caller's value stays live. The run picks the self arm and observes the
/// caller's original value, with no release report either.
#[test]
fn should_accept_and_run_a_nullable_out_slot_conditional_self_store() {
    let source = r#"module spec.sei.nullableoutslot;

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
    let output = run_memory("nullable-out-slot", source);
    assert_exit("nullable-out-slot", &output, 4);
    assert_no_asan("nullable-out-slot", &output);
}

/// The generic conditional store instantiated at a nullable slot stays
/// accepted and runs memory-clean.
///
/// Substituting `String?` makes the slot nullable, so the out-slot
/// carve-out applies to the instance exactly as it does to the concrete
/// nullable spelling. The run picks the self arm and observes the caller's
/// original value.
#[test]
fn should_accept_and_run_a_nullable_generic_conditional_self_store() {
    let source = r#"module spec.sei.nullablegeneric;

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
    let output = run_memory("nullable-generic", source);
    assert_exit("nullable-generic", &output, 4);
    assert_no_asan("nullable-generic", &output);
}

/// A direct self-store through a lend is rejected once on the target.
///
/// The assignment reads the lend as the stored value, which would release the
/// caller's value through the same lend before storing back the emptied
/// storage. Exactly one `E10034` names `slot` — the unchanged report.
#[test]
fn should_reject_a_direct_self_store_through_a_lend() {
    let source = r#"module spec.sei.directself;

import { String, stringOf } from std.text;

func copySelf(borrow slot: String): void {
    slot = slot;
}

func main(): i32 {
    let s: String = stringOf("abc");
    copySelf(borrow s);
    return 0;
}
"#;
    assert_rejected_once("direct-self-store", source, "E10034", &["'slot'"]);
}

/// An element write through a `str` parameter view is rejected once.
///
/// A `str` view names immutable byte storage, so no conforming C compiler
/// accepts the element store the emitter would produce. The parameter is a
/// writable lend, so the write reaches the immutable-view rule instead of the
/// read-only-mode rule; exactly one `E10042` names the binding and no
/// translation unit is written.
#[test]
fn should_reject_an_element_write_through_a_str_parameter() {
    let source = r#"module spec.sei.p2.paramwrite;

func f(borrow s: str): i32 {
    s[0] = 65;
    return 0;
}

func main(): i32 {
    let t: str = "abc";
    return f(borrow t);
}
"#;
    assert_rejected_once("str-parameter-write", source, "E10042", &["'s'"]);
}

/// An element write through a local `str` binding is rejected once.
///
/// The local names immutable bytes (a literal's static storage here), and a
/// local declaration never admits a different mutability than the type
/// declares, so the write is rejected exactly as the parameter spelling is.
#[test]
fn should_reject_an_element_write_through_a_str_local() {
    let source = r#"module spec.sei.p2.localwrite;

func main(): i32 {
    let t: str = "abc";
    t[0] = 65;
    return 0;
}
"#;
    assert_rejected_once("str-local-write", source, "E10042", &["'t'"]);
}

/// An element write through a `static str` struct field is rejected once.
///
/// The written place is a field projection followed by an element projection,
/// so the report names the field path (`s.tag`) rather than the bare binding.
#[test]
fn should_reject_an_element_write_through_a_static_str_field() {
    let source = r#"module spec.sei.p2.fieldwrite;

struct S {
    tag: static str;
    n: i32;
}

func f(borrow s: S): void {
    s.tag[0] = 65;
}

func main(): i32 {
    let v: S = S { tag: "abc", n: 1 };
    f(borrow v);
    return 0;
}
"#;
    assert_rejected_once("static-str-field-write", source, "E10042", &["'s.tag'"]);
}

/// A compound element write through a `str` view is rejected once.
///
/// `s[0] += 1` reads the element and stores the combined value back, so it
/// contains the same element store as the plain form and is rejected by the
/// same rule with the same single report.
#[test]
fn should_reject_a_compound_element_write_through_a_str_view() {
    let source = r#"module spec.sei.p2.compoundwrite;

func f(borrow s: str): i32 {
    s[0] += 1;
    return 0;
}

func main(): i32 {
    let t: str = "abc";
    return f(borrow t);
}
"#;
    assert_rejected_once("str-compound-write", source, "E10042", &["'s'"]);
}

/// An update element write through a `str` view is rejected once.
///
/// `s[0]++` is the same read-modify-write as the compound form, so it is
/// rejected by the same rule with the same single report.
#[test]
fn should_reject_an_update_element_write_through_a_str_view() {
    let source = r#"module spec.sei.p2.updatewrite;

func f(borrow s: str): i32 {
    s[0]++;
    return 0;
}

func main(): i32 {
    let t: str = "abc";
    return f(borrow t);
}
"#;
    assert_rejected_once("str-update-write", source, "E10042", &["'s'"]);
}

/// An element write through a module-level `str` view is rejected once.
///
/// A module binding is writable storage by default, but the view type still
/// names immutable bytes, so the element store is rejected exactly as the
/// local and parameter spellings are.
#[test]
fn should_reject_an_element_write_through_a_module_str_view() {
    let source = r#"module spec.sei.p2.globalwrite;

let g: str = "abc";

func main(): i32 {
    g[0] = 65;
    return 0;
}
"#;
    assert_rejected_once("module-str-write", source, "E10042", &["'g'"]);
}

/// Rebinding a whole `str` view stays accepted and runs.
///
/// The rebind replaces the view value itself — pointer and length — rather
/// than storing through the bytes it names, so it is not an element write. The
/// run reads the new literal's length.
#[test]
fn should_accept_and_run_a_whole_str_rebind() {
    let source = r#"module spec.sei.p2.rebind;

func main(): i32 {
    let t: str = "abc";
    t = "xyz";
    return i32(len(t));
}
"#;
    let output = run_host("str-rebind", source);
    assert_exit("str-rebind", &output, 3);
}

/// A write through a writable slice view stays accepted and runs.
///
/// The writable view derives from a mutable fixed array through `slice`, so
/// the element store reaches owned storage the program may write, and the
/// accepted path is unaffected by the immutable-view rule.
#[test]
fn should_accept_and_run_a_write_through_a_writable_slice_view() {
    let source = r#"module spec.sei.p2.writableslice;

import { slice } from std.mem;

func f(borrow xs: u8[]): i32 {
    xs[0] = 65;
    return 0;
}

func main(): i32 {
    let a: u8[2] = [0, 0];
    return f(borrow slice(a, 0, 2));
}
"#;
    let output = run_host("writable-slice-write", source);
    assert_exit("writable-slice-write", &output, 0);
}

/// The immutable-view golden is rejected exactly as the frozen suite expects.
///
/// The case compiles `spec/tests/compile-fail/E10042_write_immutable_view.clo`
/// directly, so the specification fixture itself — not a copy — must produce
/// the one registered diagnostic.
#[test]
fn should_reject_the_immutable_view_golden_directly() {
    let golden = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../spec/tests/compile-fail/E10042_write_immutable_view.clo");
    assert_rejected_once_at("immutable-view-golden", &golden, "E10042", &["'s'"]);
}
