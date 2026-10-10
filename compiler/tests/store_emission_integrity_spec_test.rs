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
//! The typedef-order group pins the emission order contract: a slice of a user
//! struct, a slice of a user enum, and a struct with an enum field each build
//! under the strict flags and run, so every emitted typedef precedes its uses;
//! a closure capturing a slice keeps building as the order's regression
//! control.
//!
//! The module-initializer group pins the init-function behavior: an
//! array-typed module binding fills element-wise (literal, nested rows, fill
//! tails, and a copy from another module array), a module-level `slice` call
//! constructs the view with its bounds check, an impure offset and a
//! conditional value each evaluate their impure operand exactly once, and
//! `len` of a module array folds.
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

/// Asserts the run failed and its first stderr line is exactly `expected`.
///
/// The `cl_trap` contract is `error CODE: message` on the first line followed
/// by an abort, so a successful exit or a different first line is a failure.
fn assert_trap_first_line(tag: &str, output: &Output, expected: &str) {
    assert!(
        !output.status.success(),
        "{tag}: expected a runtime trap; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let first = String::from_utf8_lossy(&output.stderr)
        .lines()
        .next()
        .unwrap_or_default()
        .to_owned();
    assert_eq!(first, expected, "{tag}: unexpected first stderr line");
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

/// An update in value position through a `str` view is rejected once.
///
/// `i32(s[0]++)` yields the old value while storing the incremented one, so
/// the expression contains the same element write the statement form does and
/// keeps the same single report.
#[test]
fn should_reject_a_value_position_update_through_a_str_view() {
    let source = r#"module spec.sei.p2.valueupdate;

func f(borrow s: str): i32 {
    return i32(s[0]++);
}

func main(): i32 {
    let t: str = "abc";
    return f(borrow t);
}
"#;
    assert_rejected_once("str-value-update", source, "E10042", &["'s'"]);
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

/// A slice of a user struct builds under the strict flags and runs.
///
/// The slice typedef embeds the element struct by name, so the struct must be
/// declared before the slice typedef. The run writes through the slice and
/// reads both elements back, proving the emitted order preserves the element
/// type and the write.
#[test]
fn should_build_and_run_a_slice_of_a_user_struct() {
    let source = r#"module spec.sei.p3.structslice;

import { slice } from std.mem;

struct P {
    x: i32;
    y: i32;
}

func main(): i32 {
    let arr: P[2] = [P { x: 1, y: 2 }, P { x: 3, y: 4 }];
    let ps: P[] = slice(arr, 0, 2);
    ps[0].x = 5;
    return ps[0].x + ps[1].y;
}
"#;
    let output = run_host("slice-of-struct", source);
    assert_exit("slice-of-struct", &output, 9);
}

/// A slice of a user enum builds under the strict flags and runs.
///
/// The enum typedef must precede the slice typedef that names it; the run
/// reads an element through the slice.
#[test]
fn should_build_and_run_a_slice_of_a_user_enum() {
    let source = r#"module spec.sei.p3.enumslice;

import { slice } from std.mem;

enum E : u8 {
    A = 1,
    B = 2,
}

func main(): i32 {
    let arr: E[2] = [E.A, E.B];
    let es: E[] = slice(arr, 0, 2);
    return i32(es[1]);
}
"#;
    let output = run_host("slice-of-enum", source);
    assert_exit("slice-of-enum", &output, 2);
}

/// A struct with an enum field builds under the strict flags and runs.
///
/// The struct definition uses the enum typedef, so the enum typedef must
/// precede the definition; the run reads both fields back.
#[test]
fn should_build_and_run_a_struct_with_an_enum_field() {
    let source = r#"module spec.sei.p3.structenum;

enum E : u8 {
    A = 1,
    B = 2,
}

struct S {
    e: E;
    n: i32;
}

func main(): i32 {
    let v: S = S { e: E.B, n: 4 };
    return i32(v.e) + v.n;
}
"#;
    let output = run_host("struct-with-enum-field", source);
    assert_exit("struct-with-enum-field", &output, 6);
}

/// A closure capturing a slice builds under the strict flags and runs.
///
/// The closure environment embeds the slice, so the slice typedef must
/// precede the closure declarations; this case is the regression control for
/// the reordered phases.
#[test]
fn should_build_and_run_a_closure_capturing_a_slice() {
    let source = r#"module spec.sei.p3.closurecapture;

import { slice } from std.mem;

func main(): i32 {
    let a: i32[4] = [1, 2, 3, 4];
    let s0: i32[] = slice(a, 0, 2);
    return ([s0] () => i32(s0[1]))();
}
"#;
    let output = run_host("closure-capturing-slice", source);
    assert_exit("closure-capturing-slice", &output, 2);
}

/// A module array and its slice view initialize and run.
///
/// The init function fills the fixed array element-wise (an array literal has
/// no C assignment form) and constructs the slice view from the array, with
/// the view length readable at run time.
#[test]
fn should_initialize_a_module_array_and_slice_view() {
    let source = r#"module spec.sei.p4.arrayandview;

import { slice } from std.mem;

let backing: i32[2] = [1, 2];
let view: i32[] = slice(backing, 0, 2);

func main(): i32 {
    return i32(len(view));
}
"#;
    let output = run_host("module-array-and-view", source);
    assert_exit("module-array-and-view", &output, 2);
}

/// A nested module array literal fills every row.
///
/// The fill recurses one extent per level, so each row's elements are stored
/// in declaration order and a row read selects the stored value.
#[test]
fn should_initialize_a_nested_module_array() {
    let source = r#"module spec.sei.p4.nested;

let m: i32[2][2] = [[1, 2], [3, 4]];

func main(): i32 {
    return m[1][0];
}
"#;
    let output = run_host("module-nested-array", source);
    assert_exit("module-nested-array", &output, 3);
}

/// A filled module array literal stores the fill value in the tail.
///
/// The elements after the list take the fill expression's value, exactly as
/// the function-body element stores already do.
#[test]
fn should_initialize_a_filled_module_array() {
    let source = r#"module spec.sei.p4.fill;

let a: i32[3] = [1, 2; 7];

func main(): i32 {
    return a[2];
}
"#;
    let output = run_host("module-filled-array", source);
    assert_exit("module-filled-array", &output, 7);
}

/// A module array copied from another module array keeps the source values.
///
/// A non-literal array value is copied element-wise from its rendered base, so
/// the destination holds the source's elements after the init runs.
#[test]
fn should_copy_a_module_array_from_another_global() {
    let source = r#"module spec.sei.p4.placecopy;

let backing: i32[2] = [1, 2];
let b: i32[2] = backing;

func main(): i32 {
    return b[1];
}
"#;
    let output = run_host("module-array-copy", source);
    assert_exit("module-array-copy", &output, 2);
}

/// A module slice view with a constant offset starts at that element.
///
/// The view length is the requested element count and the first element is the
/// source's element at the offset.
#[test]
fn should_initialize_a_slice_view_from_a_runtime_offset() {
    let source = r#"module spec.sei.p4.runtimeoffset;

import { slice } from std.mem;

let backing: i32[3] = [1, 2, 3];
let view: i32[] = slice(backing, 1, 3);

func main(): i32 {
    if (i32(len(view)) != 2) {
        return 1;
    }
    return view[0];
}
"#;
    let output = run_host("module-view-offset", source);
    assert_exit("module-view-offset", &output, 2);
}

/// A module slice view with an impure offset evaluates the offset once.
///
/// The constructed view reads its offset expression more than once, so the
/// init function must materialize the operand; the run observes a single call
/// and the view starting at the returned offset.
#[test]
fn should_evaluate_an_impure_slice_offset_once() {
    let source = r#"module spec.sei.p4.operandonce;

import { slice } from std.mem;

let calls: i32 = 0;

func bump(): i32 {
    calls = calls + 1;
    return 1;
}

let backing: i32[3] = [1, 2, 3];
let v: i32[] = slice(backing, bump(), 3);

func main(): i32 {
    if (calls != 1) {
        return 9;
    }
    return v[0];
}
"#;
    let output = run_host("module-impure-offset", source);
    assert_exit("module-impure-offset", &output, 2);
}

/// A conditional module array value evaluates its condition once.
///
/// The fill stores each element from the chosen arm — the arms are indexed per
/// element, so no arm array decays to a pointer — and the condition runs a
/// single time.
#[test]
fn should_evaluate_a_conditional_module_value_once() {
    let source = r#"module spec.sei.p4.conditional;

let x: i32[2] = [10, 11];
let y: i32[2] = [20, 21];
let calls: i32 = 0;

func pick(): bool {
    calls = calls + 1;
    return true;
}

let a: i32[2] = pick() ? x : y;

func main(): i32 {
    if (calls != 1) {
        return 9;
    }
    return a[1];
}
"#;
    let output = run_host("module-conditional-value", source);
    assert_exit("module-conditional-value", &output, 11);
}

/// A module conditional whose condition is out of range traps.
///
/// The condition reads past the source array, and the module initializer
/// carries the same bounds guard a function statement would: the run traps
/// with the index diagnostic instead of reading past the array.
#[test]
fn should_trap_an_out_of_range_module_condition() {
    let source = r#"module spec.sei.p4.guardedcondition;

let backing: i32[3] = [1, 2, 3];
let x: i32[2] = [10, 11];
let y: i32[2] = [20, 21];
let idx: usize = 5;
let a: i32[2] = backing[idx] > 0 ? x : y;

func main(): i32 {
    return a[0];
}
"#;
    let output = run_host("module-guarded-condition", source);
    assert_trap_first_line(
        "module-guarded-condition",
        &output,
        "error E11211: index out of range",
    );
}

/// `len` of a module array folds at module level and runs.
///
/// The module binding's initializer reads the array's element count, which is
/// known statically, so the run observes the extent.
#[test]
fn should_fold_len_of_a_module_array() {
    let source = r#"module spec.sei.p4.dotlen;

let backing: i32[3] = [1, 2, 3];
let n: usize = backing.len();

func main(): i32 {
    return i32(n);
}
"#;
    let output = run_host("module-dot-len", source);
    assert_exit("module-dot-len", &output, 3);
}
