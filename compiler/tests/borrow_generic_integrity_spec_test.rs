//! Runnable specification tests for the borrow/generic integrity batch.
//!
//! Each case compiles a small Clorraine program with the built `clc` binary,
//! builds the emitted C with the host C compiler under the fixed warning
//! flags, runs the binary, and asserts the process result. Programs that must
//! be rejected assert the diagnostics instead, and no translation unit may be
//! written for them.
//!
//! Cases cover four surfaces:
//!
//! * stores into lend parameters, including the parameter-typed form whose
//!   legality depends on the instantiation, the exact out-slot
//!   self-assignment, and the warning exemption for nullable out-slots;
//! * type keys for instances that carry nullable type arguments;
//! * comparisons of a nullable view with `null`;
//! * the callee identity of a func-typed parameter whose name matches a module
//!   function.
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
            "clc-borrowgenericspec-{}-{tag}-{sequence}",
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

/// Asserts the front end rejects the program with exactly one `code`
/// diagnostic, that the message contains every fragment, and that no
/// translation unit is written.
///
/// The count assertion pins the one-report rule: a site judged by two paths
/// must still produce a single diagnostic.
fn assert_rejected_once(tag: &str, source: &str, code: &str, fragments: &[&str]) {
    let (_scratch, output, c_path) = compile_only(tag, source);
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

/// Compiles `source` and asserts the front end accepted it.
///
/// Returns the compiler's stderr so warning assertions can inspect it.
fn compile_accepted(tag: &str, source: &str) -> String {
    let (_scratch, output, c_path) = compile_only(tag, source);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{tag}: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        c_path.exists(),
        "{tag}: an accepted program must produce a translation unit"
    );
    String::from_utf8_lossy(&output.stderr).into_owned()
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
/// pins release-once for the case.
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

/// A store of a `take` parameter into a non-nullable lend parameter is
/// rejected once.
///
/// The callee receives `v` as an owner and `slot` as a lend. Writing `v` into
/// `slot` would release the caller's value that `slot` refers to, which the
/// callee must not do. The rejection is a single `E10034` naming the lend
/// parameter, and no translation unit is written.
#[test]
fn should_reject_a_store_into_a_concrete_lend_parameter() {
    let source = r#"module spec.bgi.concrete;

import { String, stringOf } from std.text;

func fill(take v: String, borrow slot: String): void {
    slot = v;
}

func main(): i32 {
    let s: String = stringOf("ab");
    fill(stringOf("xyz"), borrow s);
    return i32(len(s));
}
"#;
    assert_rejected_once("concrete-store", source, "E10034", &["'slot'"]);
}

/// A store of a fresh value into a non-nullable lend parameter is rejected
/// once.
///
/// The right side owns a new string and the assignment would first release the
/// caller's value through `slot`. The offense is the target release, not the
/// right side, so exactly one `E10034` names `slot`.
#[test]
fn should_reject_a_fresh_value_store_into_a_lend_parameter() {
    let source = r#"module spec.bgi.fresh;

import { String, stringOf } from std.text;

func setF(borrow slot: String): void {
    slot = stringOf("x");
}

func main(): i32 {
    let s: String = stringOf("ab");
    setF(borrow s);
    return 0;
}
"#;
    assert_rejected_once("fresh-store", source, "E10034", &["'slot'"]);
}

/// A store whose both sides are lend parameters reports one diagnostic per
/// site.
///
/// The right side already reports the illegal move out of `v`; the target side
/// must not add a second report for the same assignment. The concrete and the
/// parameter-typed spellings behave the same way, each with exactly one
/// `E10034`.
#[test]
fn should_report_one_borrow_store_error_when_both_sides_are_lends() {
    let concrete = r#"module spec.bgi.bothconcrete;

import { String, stringOf } from std.text;

func copy(borrow v: String, borrow slot: String): void {
    slot = v;
}

func main(): i32 {
    let s: String = stringOf("ab");
    let t: String = stringOf("cd");
    copy(borrow s, borrow t);
    return 0;
}
"#;
    assert_rejected_once("both-lends-concrete", concrete, "E10034", &[]);

    let generic = r#"module spec.bgi.bothgeneric;

import { String, stringOf } from std.text;

func copy<T>(borrow v: T, borrow slot: T): void {
    slot = v;
}

func main(): i32 {
    let s: String = stringOf("ab");
    let t: String = stringOf("cd");
    copy<String>(borrow s, borrow t);
    return 0;
}
"#;
    assert_rejected_once("both-lends-generic", generic, "E10034", &[]);
}

/// A parameter-typed store is rejected for the instantiation that owns the
/// slot.
///
/// The generic body cannot decide the store while the slot type is still a
/// parameter. The owning `String` instantiation decides it: exactly one
/// `E10034` names the slot parameter.
#[test]
fn should_reject_an_owning_store_into_a_parameter_typed_lend_slot() {
    let source = r#"module spec.bgi.genericowning;

import { String, stringOf } from std.text;

func fill<T>(take v: T, borrow slot: T): void {
    slot = v;
}

func main(): i32 {
    let s: String = stringOf("ab");
    fill<String>(stringOf("xyz"), borrow s);
    return i32(len(s));
}
"#;
    assert_rejected_once("generic-owning-store", source, "E10034", &["'slot'"]);
}

/// The same parameter-typed store stays accepted for an instantiation that
/// copies.
///
/// An `i32` slot is duplicable, so the lend passes a copy and the write cannot
/// disturb the caller. The call warns `W10001` (a needless lend), the program
/// compiles, and the caller's value is unchanged at run time.
#[test]
fn should_accept_and_warn_when_a_parameter_typed_slot_is_duplicable() {
    let source = r#"module spec.bgi.duplicableslot;

func fill<T>(take v: T, borrow slot: T): void {
    slot = v;
}

func main(): i32 {
    let x: i32 = 1;
    fill(2, borrow x);
    return x;
}
"#;
    let stderr = compile_accepted("duplicable-slot", source);
    assert_eq!(
        stderr.matches("warning W10001:").count(),
        1,
        "duplicable-slot: expected exactly one W10001; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("unnecessary lend of duplicable 'x'"),
        "duplicable-slot: expected the warning to name 'x'; stderr:\n{stderr}"
    );

    let output = run_host("duplicable-slot-run", source);
    assert_exit("duplicable-slot-run", &output, 1);
}

/// Nullable slot instances stay accepted and run cleanly.
///
/// Three shapes must keep working: a concrete `String?` out-slot receiving a
/// taken string, a parameter-typed `T[]?` slot receiving `null` through a
/// dot call, and a parameter-typed nullable slot receiving a borrowed value.
/// Each run observes the stored value and reports no memory error, which
/// proves the value is released exactly once.
#[test]
fn should_accept_and_run_when_a_slot_instantiation_is_nullable() {
    let concrete = r#"module spec.bgi.nullableconcrete;

import { String, stringOf } from std.text;

func fillString(take v: String, borrow slot: String?): void {
    slot = v;
}

func main(): i32 {
    let slot: String? = null;
    fillString(stringOf("xyz"), borrow slot);
    if (slot == null) {
        return 9;
    }
    return i32(len(slot));
}
"#;
    let output = run_memory("nullable-concrete", concrete);
    assert_exit("nullable-concrete", &output, 3);
    assert_no_asan("nullable-concrete", &output);

    let slice_slot = r#"module spec.bgi.nullableslice;

func prep<T>(take v: T, borrow slot: T[]?): void {
    slot = null;
}

func main(): i32 {
    let value: i32 = 1;
    let xs: i32[]? = null;
    value.prep(borrow xs);
    return 0;
}
"#;
    let output = run_memory("nullable-slice-slot", slice_slot);
    assert_exit("nullable-slice-slot", &output, 0);
    assert_no_asan("nullable-slice-slot", &output);

    let generic = r#"module spec.bgi.nullablegeneric;

import { String, stringOf } from std.text;

func copy<T>(borrow v: T, borrow slot: T): void {
    slot = v;
}

func main(): i32 {
    let v: String? = stringOf("xyz");
    let slot: String? = null;
    copy<String?>(borrow v, borrow slot);
    if (slot == null) {
        return 91;
    }
    return i32(len(slot));
}
"#;
    let output = run_memory("nullable-generic", generic);
    assert_exit("nullable-generic", &output, 3);
    assert_no_asan("nullable-generic", &output);
}

/// An out-slot self-assignment is rejected once.
///
/// Writing a slot into itself drops the caller's value and stores back the
/// same now-empty storage, so the caller loses the value. The store is
/// rejected with exactly one `E10020` naming `slot`, the same diagnostic a
/// moved binding receives.
#[test]
fn should_reject_an_out_slot_self_assignment() {
    let source = r#"module spec.bgi.selfassign;

import { String, stringOf } from std.text;

func self(borrow slot: String?): void {
    slot = slot;
}

func main(): i32 {
    let s: String? = stringOf("ab");
    self(borrow s);
    return 0;
}
"#;
    assert_rejected_once("out-slot-self", source, "E10020", &["'slot'"]);
}

/// Nullable out-slot calls stop warning while a duplicable lend still does.
///
/// A writable lend of a duplicable value is a needless lend and warns
/// `W10001`; writing `null` through a nullable out-slot is a legal use of the
/// slot, so it must not warn. In one program, only the `i32` lend warns: the
/// explicit, inferred, and concrete nullable slot calls stay silent, and the
/// run proves the copy semantics of the duplicable lend.
#[test]
fn should_warn_only_for_the_duplicable_lend_when_nullable_out_slots_are_written() {
    let source = r#"module spec.bgi.nullablewarning;

func set(borrow x: raw u8?): void {
    x = null;
}

func setIt<T>(take v: T, borrow slot: T): void {
    slot = v;
}

func bump(borrow p: i32): void {
    p = p + 1;
}

func main(): i32 {
    let n: i32 = 1;
    bump(n);
    unsafe {
        let value: u8 = 0;
        let r: raw u8? = &value;
        set(borrow r);
        let a: raw u8? = &value;
        let b: raw u8? = &value;
        setIt(a, borrow b);
        let c: raw u8? = &value;
        let d: raw u8? = &value;
        setIt<raw u8?>(c, borrow d);
    }
    return n;
}
"#;
    let stderr = compile_accepted("nullable-warning", source);
    assert_eq!(
        stderr.matches("warning W10001:").count(),
        1,
        "nullable-warning: expected exactly one W10001; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("unnecessary lend of duplicable 'n'"),
        "nullable-warning: expected the sole warning to name 'n'; stderr:\n{stderr}"
    );

    let output = run_host("nullable-warning-run", source);
    assert_exit("nullable-warning-run", &output, 1);
}
