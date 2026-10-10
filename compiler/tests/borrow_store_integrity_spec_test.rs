//! Runnable specification tests for whole-slot and element stores through a
//! lend.
//!
//! Each case compiles a small Clorraine program with the built `clc` binary,
//! builds the emitted C with the host C compiler under the fixed warning
//! flags, runs the binary, and asserts the process result. Programs that must
//! be rejected assert the diagnostics instead, and no translation unit may be
//! written for them.
//!
//! The batch pins the whole-slot and element store rules at two granularities,
//! plus the return semantics of a nullable out-slot:
//!
//! * a whole-slot store of a binding into itself, where the judgment depends on
//!   the instantiation — an owning slot is rejected as a released lend, a
//!   duplicable slot is accepted as a copy, and a nullable slot is judged as a
//!   moved value;
//! * an element store through a nullable view, where two variable indices may
//!   alias and are rejected, two distinct literal indices are accepted, and the
//!   same element written to itself is rejected;
//! * a whole-slot return through a nullable out-slot, where the callee owns the
//!   incoming value and the move-out empties the caller's slot, while a
//!   projected element return and a read-only lend return stay rejected.
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
            "clc-borrowstorespec-{}-{tag}-{sequence}",
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

/// Asserts a run failed and its first stderr line is exactly `expected`.
///
/// The `cl_trap` contract is `error CODE: message` on the first line followed
/// by an abort, so a successful exit is always a failure, and a different first
/// line (a stack-smashing message, a sanitizer banner, or the zero exit of an
/// unchecked write) is a failure too.
fn assert_trap_line(tag: &str, build: &str, output: &Output, expected: &str) {
    assert!(
        !output.status.success(),
        "{tag} ({build}): expected a runtime trap (nonzero exit); stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let first = String::from_utf8_lossy(&output.stderr)
        .lines()
        .next()
        .unwrap_or_default()
        .to_owned();
    assert_eq!(first, expected, "{tag} ({build}): unexpected first stderr line");
}

/// Asserts a program traps with the exact first stderr line `expected` on both
/// a plain strict host build and an AddressSanitizer build.
///
/// The bounds trap is the memory-safety contract for an out-of-range element
/// access: the sanitizer build must reach the same trap and must not report a
/// memory error instead, so the two runs must agree on the first line and the
/// sanitizer run must print no `AddressSanitizer` report.
fn assert_traps(tag: &str, source: &str, expected: &str) {
    let plain = run_host(tag, source);
    assert_trap_line(tag, "plain", &plain, expected);

    let sanitized = run_asan(tag, source);
    assert_trap_line(tag, "asan", &sanitized, expected);
    assert_no_asan(tag, &sanitized);
}

/// A whole-slot self-assignment through an owning generic lend is rejected
/// once.
///
/// The callee receives `slot` as a writable lend. Writing the parameter into
/// itself would first release the caller's value through the lend and then
/// store back the emptied storage, so the caller loses the value. The offense
/// is the target release, not the right side, so exactly one `E10034` names
/// `slot`, and the instantiation with `String` is what makes the slot owning.
#[test]
fn should_reject_a_whole_slot_self_assignment_when_the_generic_slot_owns() {
    let source = r#"module spec.bsi.owning;

import { String, stringOf } from std.text;

func copySelf<T>(borrow slot: T): void {
    slot = slot;
}

func main(): i32 {
    let s: String = stringOf("abc");
    copySelf<String>(borrow s);
    return 0;
}
"#;
    assert_rejected_once("owning-whole-self", source, "E10034", &["'slot'"]);
}

/// A whole-slot self-assignment through a duplicable generic lend is accepted.
///
/// An `i32` slot is duplicable, so the lend passes a copy and the store cannot
/// disturb the caller: writing the copy into itself leaves the caller's value
/// unchanged. The call warns `W10001` (a needless lend), the program compiles,
/// and the caller's value is still one at run time.
#[test]
fn should_accept_a_whole_slot_self_assignment_when_the_generic_slot_is_duplicable() {
    let source = r#"module spec.bsi.duplicable;

func same<T>(borrow p: T): void {
    p = p;
}

func main(): i32 {
    let n: i32 = 1;
    same<i32>(n);
    return n;
}
"#;
    let output = run_memory("duplicable-whole-self", source);
    assert_exit("duplicable-whole-self", &output, 1);
    assert_no_asan("duplicable-whole-self", &output);
}

/// A whole-slot self-assignment through a nullable generic lend is rejected
/// once.
///
/// The `String?` instantiation makes the slot nullable, so the self-store is
/// judged as a value moved out of the parameter and back into itself, which
/// the moved-value rule forbids. Exactly one `E10020` names `slot`, the same
/// diagnostic the concrete nullable out-slot self-assignment receives.
#[test]
fn should_reject_a_whole_slot_self_assignment_when_the_generic_slot_is_nullable_owned() {
    let source = r#"module spec.bsi.nullableowned;

import { String, stringOf } from std.text;

func copySelf<T>(borrow slot: T): void {
    slot = slot;
}

func main(): i32 {
    let s: String? = stringOf("ab");
    copySelf<String?>(borrow s);
    return 0;
}
"#;
    assert_rejected_once("nullable-owned-whole-self", source, "E10020", &["'slot'"]);
}

/// A whole-slot self-assignment through a concrete owning lend is rejected
/// once.
///
/// The concrete `borrow slot: String` spelling is the fully resolved form of
/// the owning generic slot: the store releases the caller's value through the
/// lend before overwriting the emptied storage. The rejection is a single
/// `E10034` naming `slot`; because the store's right side is the lend itself,
/// the target release is the only offense.
#[test]
fn should_reject_a_whole_slot_self_assignment_when_the_slot_is_a_concrete_owned_lend() {
    let source = r#"module spec.bsi.concrete;

import { String, stringOf } from std.text;

func copySelf(borrow slot: String): void {
    slot = slot;
}

func main(): i32 {
    let s: String = stringOf("ab");
    copySelf(borrow s);
    return 0;
}
"#;
    assert_rejected_once("concrete-whole-self", source, "E10034", &["'slot'"]);
}

/// A nullable view element written to itself with a variable index is rejected
/// once.
///
/// A variable index may name the same element as the store target, so the
/// source element would be moved out and back into itself; the two indices
/// cannot be proven distinct. Exactly one `E10020` names the view root `xs`.
#[test]
fn should_reject_a_view_element_self_assignment_when_the_nullable_slot_uses_a_variable_index() {
    let source = r#"module spec.bsi.variableindex;

import { slice } from std.mem;

func bump(borrow xs: i32[]?): i32 {
    let i: i32 = 0;
    if (xs != null) {
        xs[i] = xs[i];
    }
    return 0;
}

func main(): i32 {
    let d: i32[4] = [1, 2, 3, 4];
    let v: i32[]? = slice(d, 0, 4);
    bump(borrow v);
    return 0;
}
"#;
    assert_rejected_once("variable-index-self", source, "E10020", &["'xs'"]);
}

/// A nullable view element assignment between two distinct literal indices is
/// accepted.
///
/// Two different constant indices name different elements, so the source
/// element cannot be the store target and no value is moved onto itself. The
/// program compiles, builds, and runs to a clean exit.
#[test]
fn should_accept_a_view_element_assignment_when_two_literal_indices_are_distinct() {
    let source = r#"module spec.bsi.literalindices;

import { slice } from std.mem;

func bump(borrow xs: i32[]?): i32 {
    if (xs != null) {
        xs[1] = xs[0];
    }
    return 0;
}

func main(): i32 {
    let d: i32[4] = [1, 2, 3, 4];
    let v: i32[]? = slice(d, 0, 4);
    bump(borrow v);
    return 0;
}
"#;
    let output = run_memory("literal-indices", source);
    assert_exit("literal-indices", &output, 0);
    assert_no_asan("literal-indices", &output);
}

/// A nullable view element assignment between two variable indices is rejected
/// once.
///
/// Two variable indices may name the same element, so the compiler cannot rule
/// out a move of an element onto itself. Exactly one `E10020` names the view
/// root `xs`.
#[test]
fn should_reject_a_view_element_assignment_when_two_variable_indices_may_collide() {
    let source = r#"module spec.bsi.twovariables;

import { slice } from std.mem;

func bump(borrow xs: i32[]?): i32 {
    let i: i32 = 0;
    let j: i32 = 1;
    if (xs != null) {
        xs[i] = xs[j];
    }
    return 0;
}

func main(): i32 {
    let d: i32[4] = [1, 2, 3, 4];
    let v: i32[]? = slice(d, 0, 4);
    bump(borrow v);
    return 0;
}
"#;
    assert_rejected_once("two-variables", source, "E10020", &["'xs'"]);
}

/// A nullable view element written to itself with a literal index is rejected
/// once.
///
/// The same constant index on both sides names one element for certain, so the
/// store moves that element onto itself. Exactly one `E10020` names the view
/// root `xs`, matching the variable-index spelling of the same self-store.
#[test]
fn should_reject_a_view_element_self_assignment_when_the_nullable_slot_uses_a_literal_index() {
    let source = r#"module spec.bsi.literalself;

import { slice } from std.mem;

func bump(borrow xs: i32[]?): i32 {
    if (xs != null) {
        xs[0] = xs[0];
    }
    return 0;
}

func main(): i32 {
    let d: i32[4] = [1, 2, 3, 4];
    let v: i32[]? = slice(d, 0, 4);
    bump(borrow v);
    return 0;
}
"#;
    assert_rejected_once("literal-self", source, "E10020", &["'xs'"]);
}

/// A nullable view element swap with one literal and one variable index is
/// accepted.
///
/// The store target is the literal element `0`, so it needs no element read to
/// be moved out; the read side reads the element at the variable index. The
/// reversed store then writes the literal element back. Both stores name a
/// constant destination that cannot be the moved source, so the program
/// compiles, builds, and runs to a clean exit.
#[test]
fn should_accept_a_view_element_swap_when_one_index_is_literal_and_one_is_variable() {
    let source = r#"module spec.bsi.mixedindices;

import { slice } from std.mem;

func main(): i32 {
    let d: i32[4] = [1, 2, 3, 4];
    let xs: i32[]? = slice(d, 0, 4);
    let i: i32 = 1;
    if (xs != null) {
        xs[0] = xs[i];
        xs[i] = xs[0];
    }
    return 0;
}
"#;
    let output = run_memory("mixed-indices", source);
    assert_exit("mixed-indices", &output, 0);
    assert_no_asan("mixed-indices", &output);
}

/// A non-nullable view element written to itself with a variable index is
/// accepted.
///
/// The lend is the non-nullable `i32[]` root, so no null check guards the
/// element store and the element is duplicable. Writing the element back to
/// itself therefore copies rather than moves, and the program compiles,
/// builds, and runs to a clean exit.
#[test]
fn should_accept_a_view_element_self_assignment_when_the_lend_is_non_nullable() {
    let source = r#"module spec.bsi.nonnullableelement;

import { slice } from std.mem;

func bump(borrow xs: i32[]): i32 {
    let i: i32 = 0;
    xs[i] = xs[i];
    return 0;
}

func main(): i32 {
    let d: i32[4] = [1, 2, 3, 4];
    let v: i32[] = slice(d, 0, 4);
    bump(borrow v);
    return 0;
}
"#;
    let output = run_memory("non-nullable-element", source);
    assert_exit("non-nullable-element", &output, 0);
    assert_no_asan("non-nullable-element", &output);
}

/// A whole-slot self-assignment through a non-nullable generic view is
/// accepted.
///
/// A view does not own storage, so writing the view into itself cannot release
/// the caller's value; it copies the view's pointer and length. The `u8[]`
/// instantiation compiles, builds, and runs to a clean exit.
#[test]
fn should_accept_a_whole_slot_self_assignment_when_the_generic_slot_is_a_non_nullable_view() {
    let source = r#"module spec.bsi.nonnullableself;

import { slice } from std.mem;

func copySelf<T>(borrow slot: T): void {
    slot = slot;
}

func main(): i32 {
    let d: u8[2] = [1, 2];
    let v: u8[] = slice(d, 0, 2);
    copySelf<u8[]>(borrow v);
    return 0;
}
"#;
    let output = run_memory("non-nullable-whole-self", source);
    assert_exit("non-nullable-whole-self", &output, 0);
    assert_no_asan("non-nullable-whole-self", &output);
}

/// A whole-slot self-assignment through a nullable generic view is rejected
/// once.
///
/// The `u8[]?` instantiation makes the slot nullable, so the self-store is
/// judged as a value moved out of the parameter and back into itself, which
/// the moved-value rule forbids exactly as it does for the nullable owned
/// slot. Exactly one `E10020` names `slot`.
#[test]
fn should_reject_a_whole_slot_self_assignment_when_the_generic_slot_is_a_nullable_view() {
    let source = r#"module spec.bsi.nullableself;

import { slice } from std.mem;

func copySelf<T>(borrow slot: T): void {
    slot = slot;
}

func main(): i32 {
    let d: u8[2] = [1, 2];
    let v: u8[]? = slice(d, 0, 2);
    copySelf<u8[]?>(borrow v);
    return 0;
}
"#;
    assert_rejected_once("nullable-whole-self", source, "E10020", &["'slot'"]);
}

/// A direct whole-slot return transfers the slot value and empties the caller's
/// slot.
///
/// The callee receives the caller's `String?` slot as an in-out lend, so it
/// owns the incoming value. Returning the slot directly moves that value out:
/// the caller's slot must be `null` after the call, and the returned handle
/// must hold the payload. A non-empty caller slot exits 90 and a null result
/// exits 91, so the pinned success exit 3 proves both the emptying and the
/// payload. The sanitizer run must show no double free and no leak — the two
/// handles must never release one allocation twice.
#[test]
fn should_transfer_and_empty_the_slot_when_a_whole_out_slot_is_returned() {
    let source = r#"module spec.bsi.wholeoutslotreturn;

import { String, stringOf } from std.text;

func cid(borrow v: String?): String? {
    return v;
}

func main(): i32 {
    let s: String? = stringOf("abc");
    let r: String? = cid(borrow s);
    if (s != null) {
        return 90;
    }
    if (r == null) {
        return 91;
    }
    return i32(len(r));
}
"#;
    let output = run_memory("whole-out-slot-return", source);
    assert_exit("whole-out-slot-return", &output, 3);
    assert_no_asan("whole-out-slot-return", &output);
}

/// The guarded-arm return transfers the payload for a non-null slot and yields
/// null for a null slot.
///
/// When the slot holds a value, the guarded arm returns it exactly as the
/// unguarded whole-slot return does: the caller's slot empties (a non-empty
/// slot exits 90) and the result holds the payload (the non-null run exits 3).
/// When the slot is null, the guard is false, the fallback returns null, and
/// the result is null (the null run exits 9). Both runs must be
/// sanitizer-clean.
#[test]
fn should_transfer_the_payload_and_return_null_when_the_guarded_arm_runs() {
    let non_null = r#"module spec.bsi.guardedarmnonnull;

import { String, stringOf } from std.text;

func cid2(borrow v: String?): String? {
    if (v != null) {
        return v;
    }
    return null;
}

func main(): i32 {
    let s: String? = stringOf("abc");
    let r: String? = cid2(borrow s);
    if (s != null) {
        return 90;
    }
    if (r == null) {
        return 91;
    }
    return i32(len(r));
}
"#;
    let output = run_memory("guarded-arm-non-null", non_null);
    assert_exit("guarded-arm-non-null", &output, 3);
    assert_no_asan("guarded-arm-non-null", &output);

    let null_slot = r#"module spec.bsi.guardedarmnull;

import { String } from std.text;

func cid2(borrow v: String?): String? {
    if (v != null) {
        return v;
    }
    return null;
}

func main(): i32 {
    let s: String? = null;
    let r: String? = cid2(borrow s);
    if (r != null) {
        return 1;
    }
    return 9;
}
"#;
    let output = run_memory("guarded-arm-null", null_slot);
    assert_exit("guarded-arm-null", &output, 9);
    assert_no_asan("guarded-arm-null", &output);
}

/// A projected whole-slot return stays rejected by the v1 element-move limit.
///
/// Returning an element of a nullable `String[]` out-slot (`return v[0]` or
/// `return v[i]`) reads the element out of the borrowed view, which the v1
/// element-move limit forbids; the guard only supplies the null check. Each
/// spelling reports exactly one `E10026` and writes no translation unit. The
/// unnarrowed spelling also reports the read path's `E10010`; that behavior is
/// unchanged and is not asserted here.
#[test]
fn should_reject_a_projected_out_slot_return_when_the_slot_is_narrowed() {
    let literal = r#"module spec.bsi.projectedliteral;

import { String, stringOf } from std.text;

func pick(borrow v: String[]?): String {
    if (v != null) {
        return v[0];
    }
    return stringOf("z");
}

func main(): i32 {
    return 0;
}
"#;
    assert_rejected_once("projected-literal-return", literal, "E10026", &["partial moves"]);

    let indexed = r#"module spec.bsi.projectedindexed;

import { String, stringOf } from std.text;

func pick(borrow v: String[]?): String {
    let i: i32 = 0;
    if (v != null) {
        return v[i];
    }
    return stringOf("z");
}

func main(): i32 {
    return 0;
}
"#;
    assert_rejected_once("projected-indexed-return", indexed, "E10026", &["partial moves"]);
}

/// A read-only lend returned as a value stays rejected.
///
/// A `borrow v: String` parameter is a read-only lend the callee may not
/// release or store. Returning it as an owned `String` transfers ownership out
/// of the caller, so the callee would release the caller's value when the
/// result drops. Exactly one `E10034` names `v`.
#[test]
fn should_reject_a_read_only_lend_return_when_the_lend_is_non_nullable() {
    let source = r#"module spec.bsi.readonlylendreturn;

import { String } from std.text;

func gid(borrow v: String): String {
    return v;
}

func main(): i32 {
    return 0;
}
"#;
    assert_rejected_once("read-only-lend-return", source, "E10034", &["'v'"]);
}

/// A generic read-only lend returned as a value stays rejected at a nullable
/// instance.
///
/// The body `gid<T>(borrow v: T): T { return v; }` returns a read-only lend
/// as an owned value — the declaration's mode is read-only whichever type
/// argument is substituted — so the instance is rejected exactly as the
/// concrete non-nullable read-only lend (`borrow v: String`) is. The declared
/// nullable out-slot spelling (`borrow v: String?`) is a different, accepted
/// rule whose transfer is pinned by its own case. The instantiation is judged
/// after substitution, so exactly one `E10034` names `v`.
#[test]
fn should_reject_a_generic_read_only_lend_return_when_the_instance_is_nullable() {
    let source = r#"module spec.bsi.genericreadonlylendreturn;

import { String, stringOf } from std.text;

func gid<T>(borrow v: T): T {
    return v;
}

func main(): i32 {
    let s: String? = stringOf("ab");
    gid<String?>(borrow s);
    return 0;
}
"#;
    assert_rejected_once("generic-read-only-lend-return", source, "E10034", &["'v'"]);
}

/// An element store through an unnarrowed possibly-null view is rejected once.
///
/// A store target's element base obeys the same non-null requirement the read
/// path already enforces: writing `xs[0]` while `xs` may be null is rejected
/// with exactly one `E10010` naming `xs`, and no translation unit is written.
/// The caller passes a null view, so the accepted program would dereference a
/// null pointer.
#[test]
fn should_reject_an_element_store_when_the_possibly_null_view_is_unnarrowed() {
    let source = r#"module spec.bsi.p3.unnarrowedstore;

func f(borrow xs: u8[]?): i32 {
    xs[0] = 1;
    return 0;
}

func main(): i32 {
    let v: u8[]? = null;
    f(borrow v);
    return 0;
}
"#;
    assert_rejected_once("unnarrowed-store", source, "E10010", &["'xs'"]);
}

/// An unnarrowed element store with a run-time index is rejected once.
///
/// The store target's null requirement does not depend on how the index was
/// spelled, so `xs[i] = 1` through an unnarrowed `u8[]?` is rejected with
/// exactly one `E10010` naming `xs`. The caller passes a null view.
#[test]
fn should_reject_an_element_store_when_the_possibly_null_view_uses_a_runtime_index() {
    let source = r#"module spec.bsi.p3.unnarrowedvarstore;

func f(borrow xs: u8[]?): i32 {
    let i: i32 = 0;
    xs[i] = 1;
    return 0;
}

func main(): i32 {
    let v: u8[]? = null;
    f(borrow v);
    return 0;
}
"#;
    assert_rejected_once("unnarrowed-var-store", source, "E10010", &["'xs'"]);
}

/// A narrowed element store into an in-bounds view is accepted.
///
/// The `if (xs != null)` arm narrows the base, so the store is accepted as the
/// matching read already is; the narrowing survives the write, so the read-back
/// returns the stored value (exit 7). The run must be sanitizer-clean.
#[test]
fn should_accept_a_narrowed_element_store_when_the_view_is_in_bounds() {
    let source = r#"module spec.bsi.p3.narrowedinbounds;

import { slice } from std.mem;

func f(borrow xs: u8[]?): i32 {
    if (xs != null) {
        xs[0] = 7;
        return i32(xs[0]);
    }
    return 0;
}

func main(): i32 {
    let d: u8[4] = [1, 2, 3, 4];
    let v: u8[]? = slice(d, 0, 2);
    return f(borrow v);
}
"#;
    let output = run_memory("narrowed-in-bounds", source);
    assert_exit("narrowed-in-bounds", &output, 7);
    assert_no_asan("narrowed-in-bounds", &output);
}

/// An element store through a non-nullable view is accepted.
///
/// A non-nullable `u8[]` lend needs no null narrowing, so the element store is
/// accepted and the read-back returns the stored value (exit 7). The run must
/// be sanitizer-clean.
#[test]
fn should_accept_an_element_store_when_the_lend_is_non_nullable() {
    let source = r#"module spec.bsi.p3.nonnullableinbounds;

import { slice } from std.mem;

func f(borrow xs: u8[]): i32 {
    xs[0] = 7;
    return i32(xs[0]);
}

func main(): i32 {
    let d: u8[4] = [1, 2, 3, 4];
    let v: u8[] = slice(d, 0, 4);
    return f(borrow v);
}
"#;
    let output = run_memory("non-nullable-in-bounds", source);
    assert_exit("non-nullable-in-bounds", &output, 7);
    assert_no_asan("non-nullable-in-bounds", &output);
}

/// An element read through an unnarrowed possibly-null view stays rejected.
///
/// The read path already requires a non-null base, so `return i32(xs[0])` on a
/// possibly-null `u8[]?` view is rejected with exactly one `E10010` naming
/// `xs`. This pins the read rule the store rule now mirrors.
#[test]
fn should_reject_an_element_read_when_the_possibly_null_view_is_unnarrowed() {
    let source = r#"module spec.bsi.p3.unnarrowedread;

func f(borrow xs: u8[]?): i32 {
    return i32(xs[0]);
}

func main(): i32 {
    let v: u8[]? = null;
    return f(borrow v);
}
"#;
    assert_rejected_once("unnarrowed-read", source, "E10010", &["'xs'"]);
}

/// An element update through an unnarrowed possibly-null view is rejected once.
///
/// The update form `xs[0]++` evaluates its operand as a value first, so the
/// read walk reports the null requirement. The store walk must not report it a
/// second time: exactly one `E10010` names `xs` for the whole statement.
#[test]
fn should_reject_an_element_update_when_the_possibly_null_view_is_unnarrowed() {
    let source = r#"module spec.bsi.p3.unnarrowedupdate;

func f(borrow xs: u8[]?): i32 {
    xs[0]++;
    return 0;
}

func main(): i32 {
    let v: u8[]? = null;
    f(borrow v);
    return 0;
}
"#;
    assert_rejected_once("unnarrowed-update", source, "E10010", &["'xs'"]);
}

/// A narrowed constant-index element store past a dynamic bound traps.
///
/// The view `slice(d, 2, 2)` is non-null but empty, so the narrowed store
/// `xs[0] = 1` is in range of no element. The store carries the read path's
/// `E11211` trap, so the plain build aborts with `error E11211: index out of
/// range` as its first stderr line, and the sanitizer build reaches the same
/// trap without reporting a memory error (a baseline unchecked write instead
/// smashes the stack or overflows the buffer).
#[test]
fn should_trap_when_a_narrowed_constant_element_store_is_out_of_bounds() {
    let source = r#"module spec.bsi.p3.oobstore;

import { slice } from std.mem;

func f(borrow xs: i32[]?): i32 {
    if (xs != null) {
        xs[0] = 1;
    }
    return 0;
}

func main(): i32 {
    let d: i32[2] = [1, 2];
    let v: i32[]? = slice(d, 2, 2);
    return f(borrow v);
}
"#;
    assert_traps("oob-store", source, "error E11211: index out of range");
}

/// A narrowed run-time-index element store past a dynamic bound traps.
///
/// The run-time-index store already routes through the pointer path, whose
/// guard is the read path's `E11211` trap, so `xs[i] = 1` on the empty view
/// aborts with `error E11211: index out of range` on both a plain and a
/// sanitizer build, and the sanitizer run reports no memory error. This pins
/// the shape the constant-index fix must match.
#[test]
fn should_trap_when_a_narrowed_runtime_index_element_store_is_out_of_bounds() {
    let source = r#"module spec.bsi.p3.oobvarstore;

import { slice } from std.mem;

func f(borrow xs: i32[]?): i32 {
    let i: i32 = 0;
    if (xs != null) {
        xs[i] = 1;
    }
    return 0;
}

func main(): i32 {
    let d: i32[2] = [1, 2];
    let v: i32[]? = slice(d, 2, 2);
    return f(borrow v);
}
"#;
    assert_traps("oob-var-store", source, "error E11211: index out of range");
}

/// A narrowed constant-index element store inside the dynamic bound is
/// accepted.
///
/// The view `slice(d, 0, 2)` holds the index, so the store's guard passes, the
/// element is written, and the read-back returns the stored value (exit 9).
/// The run must be sanitizer-clean.
#[test]
fn should_accept_a_narrowed_constant_element_store_when_the_view_is_in_bounds() {
    let source = r#"module spec.bsi.p3.inboundsstore;

import { slice } from std.mem;

func f(borrow xs: i32[]?): i32 {
    if (xs != null) {
        xs[0] = 9;
        return i32(xs[0]);
    }
    return 0;
}

func main(): i32 {
    let d: i32[2] = [1, 2];
    let v: i32[]? = slice(d, 0, 2);
    return f(borrow v);
}
"#;
    let output = run_memory("in-bounds-store", source);
    assert_exit("in-bounds-store", &output, 9);
    assert_no_asan("in-bounds-store", &output);
}

/// A narrowed constant-index element read past a dynamic bound traps.
///
/// The read path already carries the `E11211` trap, so `return xs[0]` on the
/// empty view aborts with `error E11211: index out of range` on both a plain
/// and a sanitizer build, and the sanitizer run reports no memory error. This
/// pins the read shape the store fix mirrors.
#[test]
fn should_trap_when_a_narrowed_constant_element_read_is_out_of_bounds() {
    let source = r#"module spec.bsi.p3.oobread;

import { slice } from std.mem;

func f(borrow xs: i32[]?): i32 {
    if (xs != null) {
        return xs[0];
    }
    return 0;
}

func main(): i32 {
    let d: i32[2] = [1, 2];
    let v: i32[]? = slice(d, 2, 2);
    return f(borrow v);
}
"#;
    assert_traps("oob-read", source, "error E11211: index out of range");
}

/// A narrowed compound element store past a dynamic bound traps.
///
/// The compound store `xs[0] += 1` reads and writes one element, so it needs
/// the same bounds trap as the plain store. On the empty view it aborts with
/// `error E11211: index out of range` on both a plain and a sanitizer build,
/// and the sanitizer run reports no memory error (a baseline unchecked compound
/// write instead smashes the stack or overflows the buffer).
#[test]
fn should_trap_when_a_narrowed_compound_element_store_is_out_of_bounds() {
    let source = r#"module spec.bsi.p3.oobcompound;

import { slice } from std.mem;

func f(borrow xs: i32[]?): i32 {
    if (xs != null) {
        xs[0] += 1;
    }
    return 0;
}

func main(): i32 {
    let d: i32[2] = [1, 2];
    let v: i32[]? = slice(d, 2, 2);
    return f(borrow v);
}
"#;
    assert_traps("oob-compound", source, "error E11211: index out of range");
}

/// A narrowed compound store and update inside the dynamic bound are accepted.
///
/// The view `slice(d, 0, 2)` holds the index, so both the compound store
/// (`xs[0] += 1`, giving 2) and the update (`xs[0]++`, giving 3) apply in
/// order and the read-back returns 3. The run must be sanitizer-clean.
#[test]
fn should_accept_narrowed_compound_and_update_element_stores_when_the_view_is_in_bounds() {
    let source = r#"module spec.bsi.p3.inboundscompound;

import { slice } from std.mem;

func f(borrow xs: i32[]?): i32 {
    if (xs != null) {
        xs[0] += 1;
        xs[0]++;
        return i32(xs[0]);
    }
    return 0;
}

func main(): i32 {
    let d: i32[2] = [1, 2];
    let v: i32[]? = slice(d, 0, 2);
    return f(borrow v);
}
"#;
    let output = run_memory("in-bounds-compound", source);
    assert_exit("in-bounds-compound", &output, 3);
    assert_no_asan("in-bounds-compound", &output);
}

/// The accepted programs of the three phases re-emit byte-identically.
///
/// Determinism needs no state carried between emissions: the phase-1
/// self-assign program, the phase-2 out-slot return program, and the phase-3
/// in-bounds store program must each emit the same translation unit twice.
#[test]
fn should_emit_byte_identical_units_when_the_accepted_phase_programs_are_emitted_twice() {
    let phase_1 = r#"module spec.bsi.p3.determinismphase1;

func same<T>(borrow p: T): void {
    p = p;
}

func main(): i32 {
    let n: i32 = 1;
    same<i32>(n);
    return n;
}
"#;
    let phase_2 = r#"module spec.bsi.p3.determinismphase2;

import { String, stringOf } from std.text;

func cid(borrow v: String?): String? {
    return v;
}

func main(): i32 {
    let s: String? = stringOf("abc");
    let r: String? = cid(borrow s);
    if (s != null) {
        return 90;
    }
    if (r == null) {
        return 91;
    }
    return i32(len(r));
}
"#;
    let phase_3 = r#"module spec.bsi.p3.determinismphase3;

import { slice } from std.mem;

func f(borrow xs: i32[]?): i32 {
    if (xs != null) {
        xs[0] = 9;
        return i32(xs[0]);
    }
    return 0;
}

func main(): i32 {
    let d: i32[2] = [1, 2];
    let v: i32[]? = slice(d, 0, 2);
    return f(borrow v);
}
"#;
    for (tag, source) in [
        ("p3-determinism-phase-1", phase_1),
        ("p3-determinism-phase-2", phase_2),
        ("p3-determinism-phase-3", phase_3),
    ] {
        let (first, second) = emit_twice(tag, source);
        assert_eq!(
            first, second,
            "{tag}: two emissions of one program must be byte-identical"
        );
    }
}
