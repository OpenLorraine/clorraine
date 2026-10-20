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

/// A store whose right side names a judged lend reports one diagnostic per
/// site.
///
/// The right side already reports the illegal move out of a lend arm; the
/// target side must not add a second report for the same assignment. The
/// concrete and the parameter-typed spellings behave the same way, each with
/// exactly one `E10034`, and so does a conditional that mixes an owned local
/// arm with a lend arm.
#[test]
fn should_report_one_borrow_store_error_per_site() {
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

    let mixed = r#"module spec.bgi.bothmixed;

import { String, stringOf } from std.text;

func pick(borrow slot: String, borrow v: String, c: bool): void {
    let local: String = stringOf("q");
    slot = c ? local : v;
}

func main(): i32 {
    let s: String = stringOf("ab");
    let t: String = stringOf("cd");
    pick(borrow s, borrow t, true);
    return 0;
}
"#;
    assert_rejected_once("both-mixed-conditional", mixed, "E10034", &[]);
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

/// A generic nullable lend cannot move into an owned local.
///
/// A by-value parameter is a read-only lend, and the concrete nullable
/// spelling is rejected for a move into an owned local. The generic body is
/// judged the same way once its instance substitutes a nullable type: the move
/// destination is not a slot, so the strict rule applies and the program is
/// rejected with exactly one `E10034` naming the parameter. Only a store into
/// the callee's own out-parameter slot is exempt, so the writable-lend source
/// form is rejected too — the concrete spelling of that form is accepted only
/// because its release is a separate, unfixed defect.
#[test]
fn should_reject_a_generic_nullable_lend_moved_into_an_owned_local() {
    let source = r#"module spec.bgi.owneddestination;

import { String, stringOf } from std.text;

func moveout<T>(a: T): i32 {
    let b: T = a;
    return 0;
}

func main(): i32 {
    let s: String? = stringOf("ab");
    return moveout<String?>(s);
}
"#;
    assert_rejected_once("owned-destination", source, "E10034", &["'a'"]);

    let borrow_source = r#"module spec.bgi.owneddestinationborrow;

import { String, stringOf } from std.text;

func moveout<T>(borrow a: T): i32 {
    let b: T = a;
    return 0;
}

func main(): i32 {
    let s: String? = stringOf("ab");
    return moveout<String?>(borrow s);
}
"#;
    assert_rejected_once("owned-destination-borrow", borrow_source, "E10034", &["'a'"]);
}

/// A generic nullable lend cannot be returned from a borrow parameter.
///
/// A return transfers ownership to the caller. The concrete `borrow T?` return
/// is accepted today but leaves the source aliasing the result, a pre-existing
/// release bug filed as a residual defect; the generic spelling must not newly
/// admit the same aliasing. A nullable substitution is exempt only for a store
/// into the callee's out-parameter slot, so this program stays rejected with
/// exactly one `E10034` naming the parameter.
#[test]
fn should_reject_a_generic_nullable_lend_returned_from_a_borrow_parameter() {
    let source = r#"module spec.bgi.returnedlend;

import { String, stringOf } from std.text;

func gid<T>(borrow v: T): T {
    return v;
}

func main(): i32 {
    let s: String? = stringOf("ab");
    let r: String? = gid<String?>(borrow s);
    if (r == null) {
        return 0;
    }
    return i32(len(r));
}
"#;
    assert_rejected_once("returned-lend", source, "E10034", &["'v'"]);
}

/// A read-only lend source cannot feed a generic nullable slot.
///
/// The concrete `func put(borrow slot: String?, v: String?)` rejects the
/// store, because a by-value parameter is a read-only lend the callee may not
/// empty. The generic body is judged the same way once its instance
/// substitutes a nullable type: the exemption needs both a writable lend
/// source and a slot destination, so this store stays rejected with exactly
/// one `E10034` naming the source parameter.
#[test]
fn should_reject_a_read_only_lend_source_stored_into_a_generic_nullable_slot() {
    let source = r#"module spec.bgi.readonlysource;

import { String, stringOf } from std.text;

func put<T>(borrow slot: T, v: T): void {
    slot = v;
}

func main(): i32 {
    let d: String? = null;
    let s: String? = stringOf("ab");
    put<String?>(borrow d, s);
    return 0;
}
"#;
    assert_rejected_once("read-only-source", source, "E10034", &["'v'"]);
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

/// Struct instances that differ only by nullability get distinct C types.
///
/// `Wrap<String>` and `Wrap<String?>` share one type key today, so the emitter
/// writes two definitions of the same C type and the strict host build fails.
/// Distinct keys make the program build, run, and read both fields: the
/// non-nullable field measures two and the nullable field measures three, so
/// the run exits with five.
#[test]
fn should_keep_struct_instances_distinct_when_they_differ_only_by_nullability() {
    let source = r#"module spec.bgi.keys.structs;

import { String, stringOf } from std.text;

struct Wrap<T> {
    v: T;
}

func main(): i32 {
    let a: Wrap<String> = Wrap<String> { v: stringOf("ab") };
    let b: Wrap<String?> = Wrap<String?> { v: stringOf("cde") };
    if (b.v == null) {
        return 9;
    }
    return i32(len(a.v)) + i32(len(b.v));
}
"#;
    let output = run_memory("keys-structs", source);
    assert_exit("keys-structs", &output, 5);
    assert_no_asan("keys-structs", &output);
}

/// Top-level function instances that differ only by nullability keep working.
///
/// A nullable top-level type argument already keyed its own symbol before the
/// key change, so this case is a regression pin: `pick<String>` and
/// `pick<String?>` both build and run, and the program exits with four.
#[test]
fn should_run_when_top_level_function_instances_differ_only_by_nullability() {
    let source = r#"module spec.bgi.keys.picktop;

import { String, stringOf } from std.text;

func pick<T>(a: T, b: T): i32 {
    return 0;
}

func main(): i32 {
    let one: i32 = pick<String>(stringOf("ab"), stringOf("cd"));
    let two: i32 = pick<String?>(stringOf("ef"), null);
    return one + two + 4;
}
"#;
    let output = run_memory("keys-pick-top", source);
    assert_exit("keys-pick-top", &output, 4);
    assert_no_asan("keys-pick-top", &output);
}

/// Nested function instances that differ only by nullability get distinct
/// symbols.
///
/// `pick<Wrap<String>>` and `pick<Wrap<String?>>` both key `Wrap__String`
/// today, so their instance definitions collide and the strict host build
/// fails. The nested nullable argument must carry its own marker inside the
/// key; the run exits with four once both instances build.
#[test]
fn should_run_when_nested_function_instances_differ_only_by_nullability() {
    let source = r#"module spec.bgi.keys.picknested;

import { String, stringOf } from std.text;

struct Wrap<T> {
    v: T;
}

func pick<T>(a: T, b: T): i32 {
    return 0;
}

func main(): i32 {
    let a1: Wrap<String> = Wrap<String> { v: stringOf("ab") };
    let a2: Wrap<String> = Wrap<String> { v: stringOf("cd") };
    let one: i32 = pick<Wrap<String>>(a1, a2);
    let b1: Wrap<String?> = Wrap<String?> { v: stringOf("ef") };
    let b2: Wrap<String?> = Wrap<String?> { v: null };
    let two: i32 = pick<Wrap<String?>>(b1, b2);
    return one + two + 4;
}
"#;
    let output = run_memory("keys-pick-nested", source);
    assert_exit("keys-pick-nested", &output, 4);
    assert_no_asan("keys-pick-nested", &output);
}

/// Instances nested two levels deep and nullable-slice instances stay
/// distinct.
///
/// `Outer<Wrap<String>>` and `Outer<Wrap<String?>>` differ two levels down, so
/// both key `Outer__Wrap__String` today and the strict host build fails on the
/// redefinition. A struct field cannot hold a view (`E10205`), so the slice
/// witness rides a function type argument instead: `pick<String[]>` and
/// `pick<String[]?>` differ by the slice's own nullability, while an element
/// type is never nullable (`String?[]` is not a type). The run then reads the
/// fields and exits with four.
#[test]
fn should_keep_nested_and_slice_instances_distinct_when_they_differ_only_by_nullability() {
    let source = r#"module spec.bgi.keys.nestedslice;

import { String, stringOf } from std.text;
import { slice } from std.mem;

struct Wrap<T> {
    v: T;
}

struct Outer<T> {
    w: T;
}

func pick<T>(a: T, b: T): i32 {
    return 0;
}

func main(): i32 {
    let innerA: Wrap<String> = Wrap<String> { v: stringOf("ab") };
    let a: Outer<Wrap<String>> = Outer<Wrap<String>> { w: innerA };
    let innerB: Wrap<String?> = Wrap<String?> { v: null };
    let b: Outer<Wrap<String?>> = Outer<Wrap<String?>> { w: innerB };
    let xs: String[2] = [stringOf("a"), stringOf("b")];
    let fixed: i32 = pick<String[]>(slice(xs, 0, 2), slice(xs, 0, 1));
    let view: String[]? = slice(xs, 0, 2);
    let nullable: i32 = pick<String[]?>(view, view);
    if (b.w.v != null) {
        return 8;
    }
    return i32(len(a.w.v)) + fixed + nullable + 2;
}
"#;
    let output = run_memory("keys-nested-slice", source);
    assert_exit("keys-nested-slice", &output, 4);
    assert_no_asan("keys-nested-slice", &output);
}

/// A program with nullable-argument instances re-emits byte-identically.
///
/// Determinism needs no state carried between emissions: two independent runs
/// of the same accepted program must produce the same translation unit,
/// whatever the key scheme does with the nullability marker.
#[test]
fn should_emit_byte_identical_units_when_a_nullable_instance_program_is_emitted_twice() {
    let source = r#"module spec.bgi.keys.determinism;

import { String, stringOf } from std.text;

struct Wrap<T> {
    v: T;
}

func main(): i32 {
    let a: Wrap<String> = Wrap<String> { v: stringOf("ab") };
    let b: Wrap<String?> = Wrap<String?> { v: stringOf("cde") };
    if (b.v == null) {
        return 9;
    }
    return i32(len(a.v)) + i32(len(b.v));
}
"#;
    let (first, second) = emit_twice("keys-determinism", source);
    assert_eq!(
        first, second,
        "keys-determinism: two emissions of one program must be byte-identical"
    );
}

/// A nullable view compares with `null` in either operand order.
///
/// A view is a nullable type, so `view == null`, `null == view`, and the two
/// `!=` forms must be accepted, exactly as they are for handles. The run pins
/// the truth values: a live view is not null and an empty view is.
#[test]
fn should_accept_a_nullable_view_compared_with_null_in_both_orders() {
    let source = r#"module spec.bgi.vieworders;

import { slice } from std.mem;

func main(): i32 {
    let xs: u8[2] = [10, 20];
    let view: u8[]? = slice(xs, 0, 2);
    let empty: u8[]? = null;
    if (view == null) {
        return 9;
    }
    if (null == view) {
        return 8;
    }
    if (empty != null) {
        return 7;
    }
    if (null != empty) {
        return 6;
    }
    return 0;
}
"#;
    let output = run_memory("view-orders", source);
    assert_exit("view-orders", &output, 0);
    assert_no_asan("view-orders", &output);
}

/// A null check narrows a nullable view so its elements can be read.
///
/// The `!= null` branch reads the first element, and the narrowing survives an
/// element write inside the same branch, so the stored value can be read back.
/// The `== null` branch returns the fallback. The run combines both forms over
/// a live view and an empty one, so the exit code pins the narrowed reads and
/// the read-back: the first element (four), the fallback (five), the second
/// element (six), and the stored eight sum to 59.
#[test]
fn should_read_through_a_narrowed_nullable_view() {
    let source = r#"module spec.bgi.viewnarrow;

import { slice } from std.mem;

func first(xs: u8[]?, fallback: u8): u8 {
    if (xs != null) {
        return xs[0];
    }
    return fallback;
}

func second(xs: u8[]?, fallback: u8): u8 {
    if (xs == null) {
        return fallback;
    }
    return xs[1];
}

func setFirst(borrow xs: u8[]?, value: u8): i32 {
    if (xs != null) {
        xs[0] = value;
        return i32(xs[0]);
    }
    return 0;
}

func main(): i32 {
    let xs: u8[2] = [4, 6];
    let view: u8[]? = slice(xs, 0, 2);
    let a: u8 = first(view, 9);
    let b: u8 = second(null, 5);
    let c: u8 = second(view, 5);
    let stored: i32 = setFirst(borrow view, 8);
    return i32(a) * 10 + i32(b) + i32(c) + stored;
}
"#;
    let output = run_memory("view-narrow", source);
    assert_exit("view-narrow", &output, 59);
    assert_no_asan("view-narrow", &output);
}

/// Equality between two slice views stays rejected.
///
/// Two views of the same kind have no defined equality — their storage may
/// overlap in ways a pointer-and-length comparison cannot express — so the
/// operator report stays unchanged. A `null` operand does not appear here.
#[test]
fn should_reject_equality_between_two_slice_views() {
    let source = r#"module spec.bgi.viewpairsame;

import { slice } from std.mem;

func main(): i32 {
    let a: u8[2] = [1, 2];
    let b: u8[2] = [3, 4];
    let xs: u8[] = slice(a, 0, 2);
    let ys: u8[] = slice(b, 0, 2);
    if (xs == ys) {
        return 1;
    }
    return 0;
}
"#;
    assert_rejected_once("view-pair-same", source, "E10222", &["operator '=='"]);
}

/// Ordering between two slice views stays rejected.
///
/// A slice has no defined order, so `<` reports the operator rule on the view
/// operand; the acceptance for `null` comparisons must not widen to view
/// pairs.
#[test]
fn should_reject_ordering_between_two_slice_views() {
    let source = r#"module spec.bgi.viewpairorder;

import { slice } from std.mem;

func main(): i32 {
    let a: u8[2] = [1, 2];
    let b: u8[2] = [3, 4];
    let xs: u8[] = slice(a, 0, 2);
    let ys: u8[] = slice(b, 0, 2);
    if (xs < ys) {
        return 1;
    }
    return 0;
}
"#;
    assert_rejected_once("view-pair-order", source, "E10222", &["operator '<'"]);
}

/// A non-nullable view also compares with `null`.
///
/// The empty state is a property of the value, not of its declared type, so a
/// view that is not spelled `T[]?` still evaluates `== null` to false and
/// `!= null` to true. The run pins both outcomes with its exit code.
#[test]
fn should_run_when_a_non_nullable_view_is_compared_with_null() {
    let source = r#"module spec.bgi.viewnonnull;

import { slice } from std.mem;

func main(): i32 {
    let xs: u8[2] = [10, 20];
    let view: u8[] = slice(xs, 0, 2);
    let flagged: i32 = 0;
    if (view == null) {
        return 8;
    }
    if (view != null) {
        flagged = 1;
    }
    return flagged;
}
"#;
    let output = run_memory("view-nonnull", source);
    assert_exit("view-nonnull", &output, 1);
    assert_no_asan("view-nonnull", &output);
}

/// An out-slot element assigned from itself is rejected once the null check
/// makes the element reachable.
///
/// The null comparison on a parameter-typed nullable slice is what makes the
/// element store expressible; the exact self-assignment through the slot root
/// is then rejected with the moved-value diagnostic, exactly as the
/// whole-binding form is. One report names the slot.
#[test]
fn should_reject_an_out_slot_projected_self_assignment() {
    let source = r#"module spec.bgi.outslotprojected;

import { String, stringOf } from std.text;

func prep<T>(take v: T, borrow xs: T[]?): void {
    if (xs != null) {
        xs[0] = xs[0];
    }
}

func main(): i32 {
    let xs: String[]? = null;
    prep<String>(stringOf("ab"), borrow xs);
    return 0;
}
"#;
    assert_rejected_once("out-slot-projected", source, "E10020", &["'xs'"]);
}

/// An untyped operand does not pair with a view.
///
/// The null side of the pairing is the `null` literal, which types as a
/// nullable unknown; a local declared without a type is unknown without that
/// flag, so `x == xs` keeps the operand rule and reports on the view side
/// instead of emitting a comparison the C compiler cannot represent.
#[test]
fn should_reject_an_untyped_operand_compared_with_a_view() {
    let source = r#"module spec.bgi.untypedview;

import { slice } from std.mem;

func main(): i32 {
    let x;
    x = 5;
    let a: u8[2] = [1, 2];
    let xs: u8[] = slice(a, 0, 2);
    if (x == xs) {
        return 1;
    }
    return 0;
}
"#;
    assert_rejected_once("untyped-view", source, "E10222", &["operator '=='"]);
}

/// A call through a func-typed parameter named like a module lender is bound.
///
/// The parameter `give` shares its written name with a module function, but
/// the call goes through the parameter's value. The owned result must be bound
/// and released like any other owned temporary; before the callee identity is
/// fixed the gate mistakes the call for the module lender, skips binding the
/// result, and the host sanitizer reports the leak. The run also pins the
/// result length (one).
#[test]
fn should_bound_and_release_when_a_func_typed_parameter_masks_a_module_lender() {
    let source = r#"module spec.bgi.maylend;

import { String, stringOf } from std.text;

func give(borrow v: str): str {
    return v;
}

func copier(borrow v: String): String {
    return stringOf("y");
}

func demo(give: func(borrow String): String): i32 {
    let s: String = stringOf("x");
    return i32(len(give(s)));
}

func main(): i32 {
    return demo(copier);
}
"#;
    let output = run_memory("maylend-param", source);
    assert_exit("maylend-param", &output, 1);
    assert_no_asan("maylend-param", &output);
}

/// A func-typed field call stays clean when the field name matches a lender.
///
/// A field call lowers through the receiver projection, not through a name, so
/// the module lookup never sees the field. The result is bound and released
/// once as before; the run exits with one.
#[test]
fn should_stay_clean_when_a_func_typed_field_is_named_like_a_module_lender() {
    let source = r#"module spec.bgi.maylendfield;

import { String, stringOf } from std.text;

func give(borrow v: str): str {
    return v;
}

func copier(borrow v: String): String {
    return stringOf("y");
}

struct Holder {
    give: func(borrow String): String;
}

func callIt(h: Holder): i32 {
    let s: String = stringOf("x");
    return i32(len(h.give(s)));
}

func main(): i32 {
    let h: Holder = Holder { give: copier };
    return callIt(h);
}
"#;
    let output = run_memory("maylend-field", source);
    assert_exit("maylend-field", &output, 1);
    assert_no_asan("maylend-field", &output);
}

/// A genuine module lender call releases its argument exactly once.
///
/// No binding shadows the lender here, so the call resolves to the module
/// function; the borrowed argument stays alive for the call and is released
/// once at scope end. The run measures both the view and the result: 45.
#[test]
fn should_release_once_when_a_genuine_module_lender_is_called() {
    let source = r#"module spec.bgi.maylendgenuine;

import { String, stringOf } from std.text;

func lend(borrow v: String): i32 {
    return i32(len(v));
}

func main(): i32 {
    let s: String = stringOf("abc");
    let n: i32 = lend(s);
    return i32(len(s)) + n + 39;
}
"#;
    let output = run_memory("maylend-genuine", source);
    assert_exit("maylend-genuine", &output, 45);
    assert_no_asan("maylend-genuine", &output);
}

/// A module call by its written name still reaches the lender, and a
/// non-func parameter that collides with the name emits agreeing C.
///
/// `useModule` declares no colliding binding, so `give(41)` resolves to the
/// module function and returns 42. `sibling` declares an `i32` parameter named
/// `give`; whatever spelling that parameter receives, the signature, the local
/// table, and the body must agree or the strict C build fails. The run reaches
/// both: 42 + 1 = 43.
#[test]
fn should_reach_the_lender_when_a_sibling_parameter_collides() {
    let source = r#"module spec.bgi.maylendmixed;

func give(v: i32): i32 {
    return v + 1;
}

func useModule(): i32 {
    return give(41);
}

func sibling(give: i32): i32 {
    return give;
}

func main(): i32 {
    return useModule() + sibling(1);
}
"#;
    let output = run_host("maylend-mixed", source);
    assert_exit("maylend-mixed", &output, 43);
}

/// A masked parameter name cannot collide with a literally-named sibling.
///
/// A parameter named like a module callee keeps a masked IR spelling whose
/// sanitized C form (`give_0`) could equal a sibling parameter's literal name;
/// the emitted names must stay unique, so the program builds and runs. `give`
/// holds one and `give_0` holds two, so the run exits with three.
#[test]
fn should_distinguish_a_masked_parameter_from_a_literal_suffixed_sibling() {
    let source = r#"module spec.bgi.paramcollide;

func give(v: i32): i32 {
    return v + 1;
}

func demo(give: i32, give_0: i32): i32 {
    return give + give_0;
}

func main(): i32 {
    return demo(1, 2);
}
"#;
    let output = run_host("param-collide", source);
    assert_exit("param-collide", &output, 3);
}

/// A generic conditional store of a by-value parameter obeys the lend rule.
///
/// `let r: String? = c ? h : null` moves `h` when the instance owns, so the
/// owning instantiation reports one `E10034` at the store, exactly as the
/// concrete twin. The simple generic whole move keeps its report. A
/// duplicable instance of a whole move copies and runs, and a generic body
/// that is never instantiated with an owning type is not reported at the
/// definition.
#[test]
fn should_reject_a_generic_conditional_store_through_a_lend_parameter() {
    let generic = r#"module spec.bgi.genericcondnull;

import { String, stringOf } from std.text;

func pick<T>(c: bool, h: T): i32 {
    let r: String? = c ? h : null;
    if (r == null) { return 1; }
    return 0;
}

func main(): i32 {
    let h: String = stringOf("ab");
    return pick<String>(true, h);
}
"#;
    assert_rejected_once("generic-condnull", generic, "E10034", &["'h'"]);

    let concrete = r#"module spec.bgi.concretecondnull;

import { String, stringOf } from std.text;

func pick(c: bool, h: String): i32 {
    let r: String? = c ? h : null;
    if (r == null) { return 1; }
    return 0;
}

func main(): i32 {
    let h: String = stringOf("ab");
    return pick(true, h);
}
"#;
    assert_rejected_once("concrete-condnull", concrete, "E10034", &["'h'"]);

    let whole = r#"module spec.bgi.genericwhole;

import { String, stringOf } from std.text;

func whole<T>(h: T): i32 {
    let r: T = h;
    return 0;
}

func main(): i32 {
    let h: String = stringOf("ab");
    return whole<String>(h);
}
"#;
    assert_rejected_once("generic-whole", whole, "E10034", &["'h'"]);

    let duplicable = r#"module spec.bgi.genericdup;

struct P { x: i32; }

func pick<T>(h: T): i32 {
    let r: T = h;
    return r.x;
}

func main(): i32 {
    let h: P = P { x: 4 };
    return pick<P>(h) - 4;
}
"#;
    let output = run_host("generic-dup", duplicable);
    assert_exit("generic-dup", &output, 0);

    let definition = r#"module spec.bgi.genericdefinition;

func pick<T>(c: bool, h: T): i32 {
    let r: String? = c ? h : null;
    if (r == null) { return 1; }
    return 0;
}

func main(): i32 {
    return 0;
}
"#;
    let _ = compile_accepted("generic-definition", definition);

    let view_binding = r#"module spec.bgi.genericview;

import { String, stringOf } from std.text;

func pick<T>(c: bool, h: T): i32 {
    let v: str = c ? h : h;
    return i32(len(v)) - 2;
}

func main(): i32 {
    let h: String = stringOf("ab");
    return pick<String>(true, h);
}
"#;
    let output = run_host("generic-view", view_binding);
    assert_exit("generic-view", &output, 0);

    let mixed = r#"module spec.bgi.genericmixed;

import { String, stringOf } from std.text;

func pick<T>(c: bool, h: T, s: String): i32 {
    let r: String? = c ? h : s;
    return 0;
}

func main(): i32 {
    let h: String = stringOf("ab");
    let s: String = stringOf("cd");
    return pick<String>(true, h, s);
}
"#;
    let (_scratch, output, c_path) = compile_only("generic-mixed", mixed);
    assert_ne!(
        output.status.code(),
        Some(0),
        "generic-mixed: the owning instance must be rejected"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        stderr.matches("error E10034:").count(),
        2,
        "generic-mixed: expected one report per lend arm; stderr:\n{stderr}"
    );
    assert!(stderr.contains("'h'"), "generic-mixed: missing 'h'; stderr:\n{stderr}");
    assert!(stderr.contains("'s'"), "generic-mixed: missing 's'; stderr:\n{stderr}");
    assert!(
        !c_path.exists(),
        "generic-mixed: a rejected program must not produce a translation unit"
    );

    let mixed_local = r#"module spec.bgi.genericmixedlocal;

import { String, stringOf } from std.text;

func pick<T>(c: bool, h: T): i32 {
    let s: String = stringOf("cd");
    let r: String? = c ? h : s;
    let n: i32 = i32(len(s));
    return n - 2;
}

func main(): i32 {
    let h: String = stringOf("ab");
    return pick<String>(true, h);
}
"#;
    let (_scratch, output, c_path) = compile_only("generic-mixed-local", mixed_local);
    assert_ne!(
        output.status.code(),
        Some(0),
        "generic-mixed-local: the owning instance must be rejected"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        stderr.matches("error E10034:").count(),
        1,
        "generic-mixed-local: expected the lend duty for 'h'; stderr:\n{stderr}"
    );
    assert_eq!(
        stderr.matches("error E10020:").count(),
        1,
        "generic-mixed-local: expected the later local use; stderr:\n{stderr}"
    );
    assert!(
        !c_path.exists(),
        "generic-mixed-local: a rejected program must not produce a translation unit"
    );

    let maybe_view = r#"module spec.bgi.genericmaybeview;

import { String, stringOf } from std.text;

func pick<T>(c: bool, h: T, s: String): i32 {
    let r: T = c ? h : s;
    return i32(len(r)) - 2;
}

func main(): i32 {
    let h: str = "ab";
    let s: String = stringOf("cd");
    return pick<str>(true, h, s);
}
"#;
    let output = run_host("generic-maybe-view", maybe_view);
    assert_exit("generic-maybe-view", &output, 0);

    let duplicable_target = r#"module spec.bgi.genericduptarget;

struct P { x: i32; }

func pick<T>(c: bool, take h: T): i32 {
    let r: P = c ? h : h;
    let n: i32 = h.x;
    return n + r.x - 8;
}

func main(): i32 {
    let h: P = P { x: 4 };
    return pick<P>(true, h);
}
"#;
    let output = run_host("generic-duplicable-target", duplicable_target);
    assert_exit("generic-duplicable-target", &output, 0);

    let outslot = r#"module spec.bgi.genericoutslot;

import { String, stringOf } from std.text;

func pick<T>(c: bool, take h: T, borrow slot: String?): i32 {
    let r: String = c ? h : slot;
    let n: i32 = i32(len(slot));
    return n - 1;
}

func main(): i32 {
    let slot: String? = null;
    return pick<String>(false, take stringOf("xx"), borrow slot);
}
"#;
    let (_scratch, output, c_path) = compile_only("generic-outslot", outslot);
    assert_ne!(
        output.status.code(),
        Some(0),
        "generic-outslot: the moved out-slot must be rejected"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        stderr.matches("error E10020:").count(),
        1,
        "generic-outslot: expected the out-slot's later use; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("'slot'"),
        "generic-outslot: the report must name the out-slot; stderr:\n{stderr}"
    );
    assert!(
        !c_path.exists(),
        "generic-outslot: a rejected program must not produce a translation unit"
    );
}

// ---------------------------------------------------------------------------
// genericcondlend: a generic conditional return/projection of a lend arm is
// judged per instance, exactly as the concrete twin.
// ---------------------------------------------------------------------------

/// A generic conditional return and a projected field report `E10034`.
///
/// `genericreturn` returns `c ? h : null`; `genericfield` copies
/// `c ? h.name : null` into an owned local. At `T = String`/`T = H` the
/// concrete twins report `E10034` at the return and the local store, so each
/// generic body records the duty once.
#[test]
fn should_reject_a_generic_conditional_return_of_a_lend_arm() {
    let generic_return = r#"module spec.bgi.genericreturn;

import { String, stringOf } from std.text;

func f<T>(c: bool, h: T): String? { return c ? h : null; }

func use(): i32 {
    let s: String = stringOf("aa");
    let r: String? = f<String>(true, s);
    if (r == null) { return 9; }
    return i32(len(r)) - 2;
}

func main(): i32 { return use(); }
"#;
    assert_rejected_once(
        "genericreturn",
        generic_return,
        "E10034",
        &["cannot release or store borrowed parameter 'h'"],
    );

    let generic_field = r#"module spec.bgi.genericfield;

import { String, stringOf } from std.text;

struct H { name: String; }

func g<T>(c: bool, h: T): String? {
    let r: String? = c ? h.name : null;
    return r;
}

func use(): i32 {
    let h: H = H { name: stringOf("aa") };
    let r: String? = g<H>(true, h);
    if (r == null) { return 9; }
    return i32(len(r)) - 2;
}

func main(): i32 { return use(); }
"#;
    assert_rejected_once(
        "genericfield",
        generic_field,
        "E10034",
        &["cannot release or store borrowed parameter 'h'"],
    );

    // A duplicable member of an owning lend is a copy; only the selected
    // member's duplicability is judged.
    let duplicable_member = r#"module spec.bgi.genericdupfield;

import { String, stringOf } from std.text;

struct H { name: i32; tag: String; }

func g<T, U>(c: bool, h: T, d: U): U { return c ? h.name : d; }

func main(): i32 {
    let h: H = H { name: 1, tag: stringOf("x") };
    let r: i32 = g<H, i32>(true, h, 2);
    return r - 1;
}
"#;
    let output = run_memory("genericdupfield", duplicable_member);
    assert_no_asan("genericdupfield", &output);
    assert_exit("genericdupfield", &output, 0);

    // The element of a view is an owned String at the instance, so the generic
    // body reports the concrete twin's E10034.
    let view_element = r#"module spec.bgi.genericvielelem;

import { String, stringOf } from std.text;
import { slice } from std.mem;

func f<T>(c: bool, xs: T, i: i32): String? { return c ? xs[i] : null; }

func main(): i32 {
    let fixed: String[1] = [stringOf("aa")];
    let xs: String[] = slice(fixed, 0, 1);
    let r: String? = f<String[]>(true, xs, 0);
    if (r == null) { return 9; }
    return i32(len(r)) - 2;
}
"#;
    assert_rejected_once(
        "genericvielelem",
        view_element,
        "E10034",
        &["cannot release or store borrowed parameter 'xs'"],
    );
}

/// Asserts the front end rejects the program with exactly `count` `code`
/// diagnostics and writes no translation unit.
fn assert_rejected_count(tag: &str, source: &str, code: &str, count: usize, fragments: &[&str]) {
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
        count,
        "{tag}: expected exactly {count} {code}; stderr:\n{stderr}"
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

/// A generic conditional take of an opaque projected member records the
/// instance duty.
///
/// At `T = H { name: String }` the selected member is an owning lend, so
/// `sink(take (c ? h.name : null))` reports `E10034` once; a duplicable member
/// instance stays accepted and clean.
#[test]
fn should_reject_a_generic_conditional_take_of_an_owning_member() {
    let source = r#"module spec.bgi.generictakemember;

import { String, stringOf } from std.text;

struct H { name: String; tag: i32; }

func sink(take s: String?): i32 { return 0; }

func f<T>(c: bool, h: T): i32 { return sink(take (c ? h.name : null)); }

func use(): i32 {
    let h: H = H { name: stringOf("aa"), tag: 1 };
    return f<H>(true, h);
}

func main(): i32 { return use(); }
"#;
    assert_rejected_once(
        "generictakemember",
        source,
        "E10034",
        &["cannot release or store borrowed parameter 'h'"],
    );

    let control = r#"module spec.bgi.generictakememberok;

struct HD { p: raw u8; tag: i32; }

func sinkR(take s: raw u8?): i32 { return 0; }

func f<T>(c: bool, h: T): i32 { return sinkR(take (c ? h.p : null)); }

func main(): i32 {
    let h: HD = HD { p: null, tag: 2 };
    return f<HD>(true, h);
}
"#;
    let output = run_memory("generictakememberok", control);
    assert_no_asan("generictakememberok", &output);
    assert_exit("generictakememberok", &output, 0);
}

/// A lend field projection through a raw pointer is judged at the instance.
///
/// At `T = raw H` the projection selects the pointee's owning `name` member, so
/// the take reports `E10034` once; a duplicable selected member stays accepted
/// and clean.
#[test]
fn should_reject_a_generic_take_field_projection_through_a_raw_pointer() {
    let source = r#"module spec.bgi.generictakeraw;

import { String, stringOf } from std.text;

struct H { name: String; tag: i32; }

func sink(take s: String?): i32 { return 0; }

func f<T>(c: bool, xs: T): i32 { return sink(take (c ? (unsafe xs.name) : null)); }

func use(): i32 {
    let h: H = H { name: stringOf("aa"), tag: 1 };
    let p: raw H = unsafe &h;
    return f<raw H>(true, p);
}

func main(): i32 { return use(); }
"#;
    assert_rejected_once(
        "generictakeraw",
        source,
        "E10034",
        &["cannot release or store borrowed parameter 'xs'"],
    );

    let control = r#"module spec.bgi.generictakerawok;

struct HD { p: raw u8; tag: i32; }

func sinkR(take s: raw u8?): i32 { return 0; }

func f<T>(c: bool, xs: T): i32 { return sinkR(take (c ? (unsafe xs.p) : null)); }

func main(): i32 {
    let h: HD = HD { p: null, tag: 2 };
    let q: raw HD = unsafe &h;
    return f<raw HD>(true, q);
}
"#;
    let output = run_memory("generictakerawok", control);
    assert_no_asan("generictakerawok", &output);
    assert_exit("generictakerawok", &output, 0);
}

/// A generic conditional arm in an owning struct-literal field or projected
/// assignment records the instance duty.
///
/// Both forms report `E10034` once at `T = H { name: String }`; the duplicable
/// field instance stays accepted and clean.
#[test]
fn should_reject_a_generic_conditional_arm_in_an_owning_struct_field() {
    let literal = r#"module spec.bgi.genericlitfield;

import { String, stringOf } from std.text;

struct H { name: String; }
struct S { name: String?; tag: i32; }

func f<T>(c: bool, h: T): i32 {
    let s: S = S { name: c ? h.name : null, tag: 1 };
    if (s.name == null) { return 9; }
    return i32(len(s.name)) - 2;
}

func use(): i32 {
    let h: H = H { name: stringOf("aa") };
    return f<H>(true, h);
}

func main(): i32 { return use(); }
"#;
    assert_rejected_once(
        "genericlitfield",
        literal,
        "E10034",
        &["cannot release or store borrowed parameter 'h'"],
    );

    let assignment = r#"module spec.bgi.genericasgnfield;

import { String, stringOf } from std.text;

struct H { name: String; }
struct S { name: String?; tag: i32; }

func f<T>(c: bool, h: T): i32 {
    let s: S = S { name: null, tag: 1 };
    s.name = c ? h.name : null;
    if (s.name == null) { return 9; }
    return i32(len(s.name)) - 2;
}

func use(): i32 {
    let h: H = H { name: stringOf("aa") };
    return f<H>(true, h);
}

func main(): i32 { return use(); }
"#;
    assert_rejected_once(
        "genericasgnfield",
        assignment,
        "E10034",
        &["cannot release or store borrowed parameter 'h'"],
    );

    let control = r#"module spec.bgi.genericfieldok;

struct HD { p: raw u8; }
struct S { p: raw u8?; tag: i32; }

func f<T>(c: bool, h: T): i32 {
    let s: S = S { p: c ? h.p : null, tag: 1 };
    if (s.p == null) { return 9; }
    return 0;
}

func main(): i32 {
    let v: u8 = 1;
    let h: HD = HD { p: unsafe &v };
    return f<HD>(true, h);
}
"#;
    let output = run_memory("genericfieldok", control);
    assert_no_asan("genericfieldok", &output);
    assert_exit("genericfieldok", &output, 0);
}

/// A projection over a conditional of type-parameter places reports each lend
/// arm.
///
/// At `T = H { name: String }` both `h1.name` and `h2.name` are owning lend
/// moves, so the return reports `E10034` twice; the duplicable instance copies
/// both members and stays clean.
#[test]
fn should_reject_a_projection_over_a_conditional_of_type_parameter_places() {
    let source = r#"module spec.bgi.genericprojcond;

import { String, stringOf } from std.text;

struct H { name: String; }

func f<T>(c: bool, d: bool, h1: T, h2: T): String? {
    return c ? (d ? h1 : h2).name : null;
}

func use(): i32 {
    let h1: H = H { name: stringOf("aa") };
    let h2: H = H { name: stringOf("bb") };
    let r: String? = f<H>(true, true, h1, h2);
    if (r == null) { return 9; }
    return i32(len(r)) - 2;
}

func main(): i32 { return use(); }
"#;
    assert_rejected_count(
        "genericprojcond",
        source,
        "E10034",
        2,
        &["'h1'", "'h2'"],
    );

    let control = r#"module spec.bgi.genericprojcondok;

struct HD { name: i32; }

func f<T, U>(c: bool, d: bool, h1: T, h2: T, z: U): U {
    return c ? (d ? h1 : h2).name : z;
}

func main(): i32 {
    let h1: HD = HD { name: 1 };
    let h2: HD = HD { name: 2 };
    return f<HD, i32>(true, true, h1, h2, 1) - 1;
}
"#;
    let output = run_memory("genericprojcondok", control);
    assert_no_asan("genericprojcondok", &output);
    assert_exit("genericprojcondok", &output, 0);
}

/// A bare projection over a conditional of type-parameter places reports each
/// lend arm at every consumed position.
///
/// The take, return, binding, struct-literal field, and projected assignment
/// forms each report `E10034` twice; the three-arm return reports three. A
/// duplicable member's later read stays accepted.
#[test]
fn should_reject_every_bare_projection_over_a_type_parameter_conditional() {
    let take = r#"module spec.bgi.baretake;

import { String, stringOf } from std.text;

struct H { name: String; }

func sink(take s: String?): i32 { return 0; }

func f<T>(d: bool, h1: T, h2: T): i32 { return sink(take ((d ? h1 : h2).name)); }

func main(): i32 {
    let h1: H = H { name: stringOf("aa") };
    let h2: H = H { name: stringOf("bb") };
    return f<H>(true, h1, h2);
}
"#;
    assert_rejected_count("baretake", take, "E10034", 2, &["'h1'", "'h2'"]);

    let ret = r#"module spec.bgi.barereturn;

import { String, stringOf } from std.text;

struct H { name: String; }

func f<T>(d: bool, h1: T, h2: T): String? { return (d ? h1 : h2).name; }

func main(): i32 {
    let h1: H = H { name: stringOf("aa") };
    let h2: H = H { name: stringOf("bb") };
    let s: String? = f<H>(true, h1, h2);
    if (s == null) { return 9; }
    return i32(len(s)) - 2;
}
"#;
    assert_rejected_count("barereturn", ret, "E10034", 2, &["'h1'", "'h2'"]);

    let binding = r#"module spec.bgi.barebinding;

import { String, stringOf } from std.text;

struct H { name: String; }

func f<T>(d: bool, h1: T, h2: T): i32 {
    let s: String? = (d ? h1 : h2).name;
    if (s == null) { return 9; }
    return i32(len(s)) - 2;
}

func main(): i32 {
    let h1: H = H { name: stringOf("aa") };
    let h2: H = H { name: stringOf("bb") };
    return f<H>(true, h1, h2);
}
"#;
    assert_rejected_count("barebinding", binding, "E10034", 2, &["'h1'", "'h2'"]);

    let field = r#"module spec.bgi.barefield;

import { String, stringOf } from std.text;

struct H { name: String; }
struct S { name: String?; tag: i32; }

func f<T>(d: bool, h1: T, h2: T): i32 {
    let s: S = S { name: (d ? h1 : h2).name, tag: 1 };
    if (s.name == null) { return 9; }
    return i32(len(s.name)) - 2;
}

func main(): i32 {
    let h1: H = H { name: stringOf("aa") };
    let h2: H = H { name: stringOf("bb") };
    return f<H>(true, h1, h2);
}
"#;
    assert_rejected_count("barefield", field, "E10034", 2, &["'h1'", "'h2'"]);

    let assignment = r#"module spec.bgi.bareassign;

import { String, stringOf } from std.text;

struct H { name: String; }
struct S { name: String?; tag: i32; }

func f<T>(d: bool, h1: T, h2: T): i32 {
    let s: S = S { name: null, tag: 1 };
    s.name = (d ? h1 : h2).name;
    if (s.name == null) { return 9; }
    return i32(len(s.name)) - 2;
}

func main(): i32 {
    let h1: H = H { name: stringOf("aa") };
    let h2: H = H { name: stringOf("bb") };
    return f<H>(true, h1, h2);
}
"#;
    assert_rejected_count("bareassign", assignment, "E10034", 2, &["'h1'", "'h2'"]);

    let three_arm = r#"module spec.bgi.barethree;

import { String, stringOf } from std.text;

struct H { name: String; }

func f<T>(c: bool, d: bool, h1: T, h2: T, h3: T): String? {
    return c ? (d ? h1 : h2).name : h3.name;
}

func main(): i32 {
    let h1: H = H { name: stringOf("aa") };
    let h2: H = H { name: stringOf("bb") };
    let h3: H = H { name: stringOf("cc") };
    let s: String? = f<H>(true, true, h1, h2, h3);
    if (s == null) { return 9; }
    return i32(len(s)) - 2;
}
"#;
    assert_rejected_count(
        "barethree",
        three_arm,
        "E10034",
        3,
        &["'h1'", "'h2'", "'h3'"],
    );

    let duplicable_read = r#"module spec.bgi.baredup;

import { String, stringOf } from std.text;

struct H { name: String; tag: raw u8; }

func f<T>(c: bool, h: T): i32 {
    let s: raw u8? = c ? h.tag : null;
    if (h.tag == null) { return 0; }
    return 1;
}

func main(): i32 {
    let h: H = H { name: stringOf("aa"), tag: null };
    return f<H>(true, h);
}
"#;
    let output = run_memory("baredup", duplicable_read);
    assert_no_asan("baredup", &output);
    assert_exit("baredup", &output, 0);
}

/// A later-use duty judges the moved path, not the use path.
///
/// A whole move followed by a duplicable-member read reports the moved binding
/// once, and a duplicable member duty does not hide a later owning-member use.
#[test]
fn should_judge_the_moved_path_and_not_the_later_use_path() {
    let whole_move = r#"module spec.bgi.wholemove;

import { String, stringOf } from std.text;

struct H { name: String; tag: i32; }

func f<T>(take h: T): i32 {
    let c: T = h;
    return h.tag;
}

func main(): i32 {
    let h: H = H { name: stringOf("aa"), tag: 3 };
    return f<H>(h);
}
"#;
    assert_rejected_count("wholemove", whole_move, "E10020", 1, &["'h'"]);

    let dedupe = r#"module spec.bgi.dedupemembers;

import { String, stringOf } from std.text;

struct H { name: String; tag: raw u8; }

func sink(take s: raw u8?): i32 { return 0; }
func first(s: String): i32 { return i32(len(s)); }

func f<T, U>(take h: T, u: U): i32 {
    let r: i32 = sink(take (true ? h.tag : null));
    let a: raw u8? = h.tag;
    let s: U = h.name;
    return first(h.name) + r;
}

func main(): i32 {
    let h: H = H { name: stringOf("aa"), tag: null };
    let u: String = stringOf("bb");
    return f<H, String>(h, u);
}
"#;
    assert_rejected_count("dedupemembers", dedupe, "E10020", 1, &["'h'"]);

    let user_len_field = r#"module spec.bgi.userlenfield;

import { String, stringOf } from std.text;

struct H { len: i32; name: String; }

func f<T, U>(take h: T, u: U): i32 {
    let a: U = h.len;
    return h.len + i32(a);
}

func main(): i32 {
    let h: H = H { len: 1, name: stringOf("aa") };
    return f<H, i32>(h, 5) - 2;
}
"#;
    let output = run_memory("userlenfield", user_len_field);
    assert_no_asan("userlenfield", &output);
    assert_exit("userlenfield", &output, 0);

    let fixed_array_element = r#"module spec.bgi.fixedarrayelem;

func f<T, U>(take xs: T, u: U): i32 {
    let a: U = xs[0];
    return xs[0] + i32(a);
}

func main(): i32 {
    let xs: i32[2] = [1, 2];
    return f<i32[2], i32>(xs, 5) - 2;
}
"#;
    let output = run_memory("fixedarrayelem", fixed_array_element);
    assert_no_asan("fixedarrayelem", &output);
    assert_exit("fixedarrayelem", &output, 0);
}

// ---------------------------------------------------------------------------
// Generic conditional wrappers and moved-state residue (Batch Q)
// ---------------------------------------------------------------------------

/// A conditional `take` of a `take` parameter's owning member moves the
/// selected member out of the parameter per arm.
///
/// At `T = H { name: String }` the concrete twin moves `h.name` into the callee
/// and drops it there, so the parameter's remaining fields release exactly
/// once. The generic body must emit the same per-arm move-out; the program runs
/// clean.
#[test]
fn should_move_out_a_conditional_take_of_a_take_parameter_member() {
    let source = r#"module spec.bgi.condtakemove;

import { String, stringOf } from std.text;

struct H { name: String; }

func sink(take s: String?): i32 { return 0; }

func f<T>(c: bool, take h: T): i32 { return sink(take (c ? h.name : null)); }

func main(): i32 {
    let h: H = H { name: stringOf("aa") };
    return f<H>(true, h);
}
"#;
    let output = run_memory("condtakemove", source);
    assert_no_asan("condtakemove", &output);
    assert_exit("condtakemove", &output, 0);

    // The null arm may come first; the selected member still moves out.
    let reversed = r#"module spec.bgi.condtakerev;

import { String, stringOf } from std.text;

struct H { name: String; }

func sink(take s: String?): i32 { return 0; }

func f<T>(c: bool, take h: T): i32 { return sink(take (c ? null : h.name)); }

func main(): i32 {
    let h: H = H { name: stringOf("aa") };
    return f<H>(false, h);
}
"#;
    let output = run_memory("condtakerev", reversed);
    assert_no_asan("condtakerev", &output);
    assert_exit("condtakerev", &output, 0);

    // A nested conditional on the owning side moves the selected member and
    // keeps the nullable spelling of the outer value.
    let nested = r#"module spec.bgi.condtakenested;

import { String, stringOf } from std.text;

struct H { name: String; }

func sink(take s: String?): i32 { return 0; }

func f<T>(c: bool, d: bool, take h: T): i32 {
    return sink(take (c ? h.name : (d ? h.name : null)));
}

func main(): i32 {
    let h: H = H { name: stringOf("aa") };
    return f<H>(true, true, h);
}
"#;
    let output = run_memory("condtakenested", nested);
    assert_no_asan("condtakenested", &output);
    assert_exit("condtakenested", &output, 0);

    // The two arms may have different nullable spellings of the same carrier.
    let mixed = r#"module spec.bgi.condtakemixed;

import { String, stringOf } from std.text;

struct H { name: String; }
struct HN { name: String?; }

func sink(take s: String?): i32 { return 0; }

func f<T, U>(c: bool, take h: T, take g: U): i32 {
    return sink(take (c ? h.name : g.name));
}

func main(): i32 {
    let h: H = H { name: stringOf("aa") };
    let g: HN = HN { name: stringOf("bb") };
    return f<H, HN>(true, h, g);
}
"#;
    let output = run_memory("condtakemixed", mixed);
    assert_no_asan("condtakemixed", &output);
    assert_exit("condtakemixed", &output, 0);
}

/// An `unsafe`-wrapped conditional take is judged and lowered exactly like its
/// bare twin.
#[test]
fn should_move_out_an_unsafe_wrapped_conditional_take() {
    let source = r#"module spec.bgi.condtakemoveunsafe;

import { String, stringOf } from std.text;

struct H { name: String; }

func sink(take s: String?): i32 { return 0; }

func f<T>(c: bool, take h: T): i32 { return sink(take (unsafe (c ? h.name : null))); }

func main(): i32 {
    let h: H = H { name: stringOf("aa") };
    return f<H>(true, h);
}
"#;
    let output = run_memory("condtakemoveunsafe", source);
    assert_no_asan("condtakemoveunsafe", &output);
    assert_exit("condtakemoveunsafe", &output, 0);
}

/// An owned array-literal element whose value is a generic conditional records
/// the per-instance duty.
///
/// At `T = H { name: String }` the element moves an owning member out of a lend
/// parameter, so the concrete twin reports `E10034`; the generic body reports
/// the same code once. A duplicable element instance stays accepted and clean.
#[test]
fn should_reject_a_generic_conditional_array_element() {
    let source = r#"module spec.bgi.genericarrayelem;

import { String, stringOf } from std.text;

struct H { name: String; }

func f<T>(c: bool, h: T): i32 {
    let arr: String[1] = [c ? h.name : h.name];
    return i32(len(arr[0])) - 2;
}

func main(): i32 {
    let h: H = H { name: stringOf("aa") };
    return f<H>(true, h);
}
"#;
    assert_rejected_once(
        "genericarrayelem",
        source,
        "E10034",
        &["cannot release or store borrowed parameter 'h'"],
    );

    let control = r#"module spec.bgi.genericarrayelemok;

struct HD { name: i32; }

func f<T>(c: bool, h: T): i32 {
    let arr: i32[1] = [c ? h.name : h.name];
    return arr[0] - 1;
}

func main(): i32 {
    let h: HD = HD { name: 1 };
    return f<HD>(true, h);
}
"#;
    let output = run_memory("genericarrayelemok", control);
    assert_no_asan("genericarrayelemok", &output);
    assert_exit("genericarrayelemok", &output, 0);
}

/// An `unsafe`-wrapped generic conditional source is judged like its bare twin.
///
/// The struct-literal field spelling at `T = H { name: String }` reports
/// `E10034` once. The duplicable field instance stays accepted and clean.
#[test]
fn should_reject_an_unsafe_wrapped_generic_conditional_source() {
    let source = r#"module spec.bgi.genericunsafefield;

import { String, stringOf } from std.text;

struct H { name: String; }
struct S { name: String?; tag: i32; }

func f<T>(c: bool, h: T): i32 {
    let s: S = S { name: unsafe (c ? h.name : null), tag: 1 };
    if (s.name == null) { return 9; }
    return i32(len(s.name)) - 2;
}

func main(): i32 {
    let h: H = H { name: stringOf("aa") };
    return f<H>(true, h);
}
"#;
    assert_rejected_once(
        "genericunsafefield",
        source,
        "E10034",
        &["cannot release or store borrowed parameter 'h'"],
    );

    let control = r#"module spec.bgi.genericunsafefieldok;

struct HD { p: raw u8; }
struct SD { name: raw u8?; tag: i32; }

func f<T>(c: bool, h: T): i32 {
    let s: SD = SD { name: unsafe (c ? h.p : null), tag: 1 };
    if (s.name == null) { return 0; }
    return 1;
}

func main(): i32 {
    let h: HD = HD { p: null };
    return f<HD>(true, h);
}
"#;
    let output = run_memory("genericunsafefieldok", control);
    assert_no_asan("genericunsafefieldok", &output);
    assert_exit("genericunsafefieldok", &output, 0);

    let binding = r#"module spec.bgi.genericunsafebind;

import { String, stringOf } from std.text;

struct H { name: String; }

func f<T>(c: bool, h: T): i32 {
    let s: String? = unsafe (c ? h.name : null);
    if (s == null) { return 9; }
    return i32(len(s)) - 2;
}

func main(): i32 {
    let h: H = H { name: stringOf("aa") };
    return f<H>(true, h);
}
"#;
    assert_rejected_once(
        "genericunsafebind",
        binding,
        "E10034",
        &["cannot release or store borrowed parameter 'h'"],
    );

    let constant = r#"module spec.bgi.genericunsafeconst;

import { String, stringOf } from std.text;

struct H { name: String; }

func f<T>(c: bool, h: T): i32 {
    const s: String? = unsafe (c ? h.name : null);
    if (s == null) { return 9; }
    return i32(len(s)) - 2;
}

func main(): i32 {
    let h: H = H { name: stringOf("aa") };
    return f<H>(true, h);
}
"#;
    assert_rejected_once(
        "genericunsafeconst",
        constant,
        "E10034",
        &["cannot release or store borrowed parameter 'h'"],
    );

    let assignment = r#"module spec.bgi.genericunsafeassign;

import { String, stringOf } from std.text;

struct H { name: String; }

func f<T>(c: bool, h: T): i32 {
    let s: String? = null;
    s = unsafe (c ? h.name : null);
    if (s == null) { return 9; }
    return i32(len(s)) - 2;
}

func main(): i32 {
    let h: H = H { name: stringOf("aa") };
    return f<H>(true, h);
}
"#;
    assert_rejected_once(
        "genericunsafeassign",
        assignment,
        "E10034",
        &["cannot release or store borrowed parameter 'h'"],
    );
}

/// A nested write below a partially moved field keeps the moved prefix.
///
/// `h.name` was moved into `n`, so writing `h.name.x` is a use of the moved
/// value and reports `E10020` at the write; the later read of `h.name.s` is a
/// second use site and reports the same code. The concrete twin reports the
/// same two sites. A whole-field re-initialization stays accepted and clean.
#[test]
fn should_reject_a_nested_write_below_a_moved_field() {
    let generic = r#"module spec.bgi.nestedwritemoved;

import { String, stringOf } from std.text;

struct Inner { x: i32; s: String; }
struct H { name: Inner; }

func f<T>(take h: T): i32 {
    let n: Inner = h.name;
    h.name.x = 7;
    return i32(len(h.name.s)) + n.x;
}

func main(): i32 {
    let h: H = H { name: Inner { x: 1, s: stringOf("ab") } };
    return f<H>(h);
}
"#;
    assert_rejected_count("nestedwritemoved", generic, "E10020", 2, &["was moved"]);

    let concrete = r#"module spec.bgi.nestedwritemovedconc;

import { String, stringOf } from std.text;

struct Inner { x: i32; s: String; }
struct H { name: Inner; }

func f(take h: H): i32 {
    let n: Inner = h.name;
    h.name.x = 7;
    return i32(len(h.name.s)) + n.x;
}

func main(): i32 {
    let h: H = H { name: Inner { x: 1, s: stringOf("ab") } };
    return f(h);
}
"#;
    assert_rejected_count(
        "nestedwritemovedconc",
        concrete,
        "E10020",
        2,
        &["was moved"],
    );

    let control = r#"module spec.bgi.nestedwriteinit;

import { String, stringOf } from std.text;

struct Inner { x: i32; s: String; }
struct H { name: Inner; }

func f(take h: H): i32 {
    let n: Inner = h.name;
    h.name = Inner { x: 7, s: stringOf("cd") };
    return i32(len(h.name.s)) + n.x;
}

func main(): i32 {
    let h: H = H { name: Inner { x: 1, s: stringOf("ab") } };
    return f(h) - 3;
}
"#;
    let output = run_memory("nestedwriteinit", control);
    assert_no_asan("nestedwriteinit", &output);
    assert_exit("nestedwriteinit", &output, 0);

    // A run-time element write clears the null narrowing of every element at
    // that position, so the later constant-index read is no longer narrowed.
    let runtime_index = r#"module spec.bgi.runtimeindexwrite;

import { String, stringOf } from std.text;

struct Cell { v: String?; }
struct H { cells: Cell[2]; }

func main(): i32 {
    let h: H = H { cells: [Cell { v: stringOf("aa") }, Cell { v: null }] };
    let j: i32 = 1;
    if (h.cells[0].v != null) {
        h.cells[j].v = null;
        return i32(len(h.cells[0].v));
    }
    return 0;
}
"#;
    assert_rejected_count(
        "runtimeindexwrite",
        runtime_index,
        "E10010",
        1,
        &["may be null"],
    );

    let control = r#"module spec.bgi.runtimeindexok;

import { String, stringOf } from std.text;

struct Cell { v: String?; }
struct H { cells: Cell[2]; }

func main(): i32 {
    let h: H = H { cells: [Cell { v: stringOf("aa") }, Cell { v: null }] };
    if (h.cells[0].v != null) {
        return i32(len(h.cells[0].v)) - 2;
    }
    return 0;
}
"#;
    let output = run_memory("runtimeindexok", control);
    assert_no_asan("runtimeindexok", &output);
    assert_exit("runtimeindexok", &output, 0);
}
