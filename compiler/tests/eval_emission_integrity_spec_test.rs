//! Runnable specification tests for evaluation and emission integrity.
//!
//! Each case compiles a small Clorraine program with the built `clc` binary,
//! builds the emitted C with the host C compiler under the fixed warning
//! flags, runs the binary, and asserts the process result. Programs that must
//! be rejected assert the diagnostics instead, and no translation unit may be
//! written for them.
//!
//! The module-initializer group pins the six accepted positions for a
//! fixed-array-returning call (whole value, element and member projection,
//! user-function and intrinsic arguments, conditional arm), the
//! single-evaluation rule for an impure guard operand, and the rejection of a
//! slice-typed binding initialized from an array literal.
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
            "clc-eei-{}-{tag}-{sequence}",
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
/// diagnostic, that the message contains every fragment and none of the
/// absent markers, and that no translation unit is written.
fn assert_rejected_output(
    tag: &str,
    output: &Output,
    c_path: &Path,
    code: &str,
    fragments: &[&str],
    absent: &[&str],
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
    for absent_marker in absent {
        assert!(
            !stderr.contains(absent_marker),
            "{tag}: the report {absent_marker:?} must be absent; stderr:\n{stderr}"
        );
    }
    assert!(
        !c_path.exists(),
        "{tag}: a rejected program must not produce a translation unit"
    );
}

/// Compiles `source` and asserts the rejection contract on the output.
fn assert_rejected_once(tag: &str, source: &str, code: &str, fragments: &[&str], absent: &[&str]) {
    let (_scratch, output, c_path) = compile_only(tag, source);
    assert_rejected_output(tag, &output, &c_path, code, fragments, absent);
}

/// Compiles an existing file and asserts the rejection contract on the
/// output.
fn assert_rejected_once_at(
    tag: &str,
    source_path: &Path,
    code: &str,
    fragments: &[&str],
    absent: &[&str],
) {
    let (_scratch, output, c_path) = compile_path(tag, source_path);
    assert_rejected_output(tag, &output, &c_path, code, fragments, absent);
}

/// Emits one host-profile program and asserts the front end accepted it.
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
fn assert_no_asan(tag: &str, output: &Output) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("AddressSanitizer"),
        "{tag}: the run reported a memory error; stderr:\n{stderr}"
    );
}

/// A fixed-array-returning call as a whole module-binding value builds and
/// runs.
///
/// The initializer stores through the callee's hidden destination, so the
/// emitted C passes the array's destination instead of assigning a call
/// result.
#[test]
fn should_build_and_run_the_module_init_whole_value() {
    let source = r#"module spec.eei.initwhole;

func mk(): i32[2] {
    return [4, 2];
}

let a: i32[2] = mk();

func main(): i32 {
    return a[1];
}
"#;
    let output = run_memory("init-whole", source);
    assert_exit("init-whole", &output, 2);
}

/// The element-projection init position builds and runs.
#[test]
fn should_build_and_run_the_module_init_element_projection() {
    let source = r#"module spec.eei.initelement;

func mk(): i32[2] {
    return [7, 2];
}

let b: i32 = mk()[1];

func main(): i32 {
    return b;
}
"#;
    let output = run_memory("init-element", source);
    assert_exit("init-element", &output, 2);
}

/// The member-projection init position builds and runs.
#[test]
fn should_build_and_run_the_module_init_member_projection() {
    let source = r#"module spec.eei.initmember;

struct Box {
    arr: i32[2];
}

func mkBox(): Box {
    return Box { arr: [5, 2] };
}

let c: i32[2] = mkBox().arr;

func main(): i32 {
    return c[1];
}
"#;
    let output = run_memory("init-member", source);
    assert_exit("init-member", &output, 2);
}

/// The user-function and intrinsic argument positions build and run.
#[test]
fn should_build_and_run_the_module_init_argument_positions() {
    let source = r#"module spec.eei.initargs;

import { slice } from std.mem;

func mk(): i32[2] {
    return [3, 9];
}

func eat(arr: i32[2]): i32 {
    return arr[0] + arr[1];
}

let d: i32 = eat(mk());
let e: i32 = i32(len(slice(mk(), 0, 2)));

func main(): i32 {
    return d + e;
}
"#;
    let output = run_memory("init-args", source);
    assert_exit("init-args", &output, 14);
}

/// The conditional-arm init position builds and runs.
#[test]
fn should_build_and_run_the_module_init_conditional_arm() {
    let source = r#"module spec.eei.initcond;

func mk(): i32[2] {
    return [6, 1];
}

func pick(): bool {
    return true;
}

let y: i32[2] = [8, 8];
let f: i32[2] = mk();
let g: i32[2] = pick() ? f : y;

func main(): i32 {
    return g[0];
}
"#;
    let output = run_memory("init-cond", source);
    assert_exit("init-cond", &output, 6);
}

/// An impure module-initializer guard operand is evaluated once.
///
/// The index function counts its calls and goes out of range on a second
/// evaluation; the bounds guard and the read must share one evaluation, so
/// the count is one and the read is in bounds (a second evaluation reads out
/// of bounds, which the sanitizer probe observes at the baseline).
#[test]
fn should_evaluate_the_module_init_guard_operand_once() {
    let source = r#"module spec.eei.guardonce;

let calls: i32 = 0;

func idx(): i32 {
    calls = calls + 1;
    if (calls == 1) {
        return 0;
    }
    return 5;
}

let s: u8[2] = [1, 2];
let v: u8 = s[idx()];

func main(): i32 {
    return calls;
}
"#;
    let output = run_memory("guard-once", source);
    assert_exit("guard-once", &output, 1);
}

/// A slice-typed binding from an array literal is rejected; the slice form
/// stays accepted.
#[test]
fn should_reject_the_literal_slice_binding_and_accept_the_slice_form() {
    let rejected = r#"module spec.eei.literalslice;

let x: i32[] = [1, 2];

func main(): i32 {
    return i32(len(x));
}
"#;
    assert_rejected_once("literal-slice", rejected, "E10200", &[], &[]);

    let rejected_in_body = r#"module spec.eei.literalslicebody;

func main(): i32 {
    let x: i32[] = [1, 2];
    return i32(len(x));
}
"#;
    assert_rejected_once("literal-slice-body", rejected_in_body, "E10200", &[], &[]);

    let rejected_fill = r#"module spec.eei.fillslicetarget;

let x: i32[] = [1, 2; 0];

func main(): i32 {
    return i32(len(x));
}
"#;
    assert_rejected_once(
        "literal-slice-fill",
        rejected_fill,
        "E10215",
        &[],
        &["E10200"],
    );

    let accepted = r#"module spec.eei.slicedform;

import { slice } from std.mem;

let arr: i32[2] = [1, 2];
let y: i32[] = slice(arr, 0, 2);

func main(): i32 {
    return i32(len(y));
}
"#;
    let output = run_host("slice-form", accepted);
    assert_exit("slice-form", &output, 2);
}

/// Emits one freestanding-profile program and asserts the front end accepted
/// it.
fn emit_freestanding(tag: &str, source: &str) -> String {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source)
        .unwrap_or_else(|err| panic!("{tag}: cannot write the program: {err}"));
    let c_path = scratch.path().join("out.c");
    let output = run_clc(&[
        "--profile".to_owned(),
        "freestanding".to_owned(),
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
    std::fs::read_to_string(&c_path).unwrap_or_default()
}

/// Emits a freestanding program, builds it on the host with the no-libc
/// stance, and runs it.
fn run_freestanding_host(tag: &str, source: &str) -> Output {
    let scratch = ScratchDir::new(tag);
    let emitted = emit_freestanding(tag, source);
    let c_path = scratch.path().join("build.c");
    std::fs::write(&c_path, &emitted).expect("write the emitted C");
    let program = scratch.path().join("prog");
    let built = Command::new("cc")
        .args([
            "-std=c99",
            "-ffreestanding",
            "-fno-builtin",
            "-nostdlib",
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
    run_with_timeout(tag, &program, &[])
}

/// Emits a host program, links it with a C stub, and runs it.
fn run_host_with_stub(tag: &str, source: &str, stub: &str) -> Output {
    let scratch = ScratchDir::new(tag);
    let emitted = emit_host(tag, source);
    let c_path = scratch.path().join("build.c");
    std::fs::write(&c_path, &emitted).expect("write the emitted C");
    let stub_path = scratch.path().join("stub.c");
    std::fs::write(&stub_path, stub).expect("write the stub");
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
            stub_path.to_str().expect("stub path"),
        ])
        .output()
        .expect("failed to spawn the host C compiler");
    assert!(
        built.status.success(),
        "{tag}: the host C compiler rejected the linked unit; stderr:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
    run_with_timeout(tag, &program, &[])
}

/// A generic `.len` member of a fixed-array call result builds and runs: the
/// extent is structural, so the length does not need the value.
#[test]
fn should_build_a_generic_structural_len_member() {
    let source = r#"module spec.eei.lengeneric;

func mkArr<T>(x: T): T[2] {
    return [x, x];
}

func g<T>(x: T): i32 {
    return i32(mkArr(x).len);
}

func main(): i32 {
    return g<i32>(5) * 10 + 2;
}
"#;
    let output = run_host("len-generic", source);
    assert_exit("len-generic", &output, 22);
}

/// The `.len` member of a fixed array is the structural extent for a named
/// local and for a call result alike, in all three spellings.
#[test]
fn should_build_the_structural_len_member_spellings() {
    let source = r#"module spec.eei.lenmember;

func mkArr(): i32[2] {
    return [3, 9];
}

func main(): i32 {
    let xs: u8[3] = [1, 2, 3];
    let n: usize = xs.len;
    let m: i32 = i32(len(xs));
    let k: i32 = i32(xs.len());
    return i32(n) * 10 + m + k + i32(mkArr().len);
}
"#;
    let output = run_host("len-member", source);
    assert_exit("len-member", &output, 38);
}

/// A structural `.len` is not a write target: a store, a compound store, an
/// update, a value-position update, and an address-of each report `E10221`
/// instead of emitting nothing; a read-only parameter keeps its own report.
#[test]
fn should_reject_a_structural_len_write_target() {
    for (tag, body) in [
        ("len-store", "    xs.len = 4;\n    return i32(xs.len);"),
        ("len-compound", "    xs.len += 1;\n    return i32(xs.len);"),
        ("len-update", "    xs.len++;\n    return i32(xs.len);"),
        ("len-value-update", "    let y: usize = xs.len++;\n    return i32(y);"),
        (
            "len-address",
            "    unsafe {\n        let p = &xs.len;\n        if (p == null) {\n            return 1;\n        }\n    }\n    return 0;",
        ),
    ] {
        let source = format!(
            "module spec.eei.lentarget;\n\nfunc main(): i32 {{\n    let xs: i32[3] = [1, 2, 3];\n{body}\n}}\n"
        );
        assert_rejected_once(tag, &source, "E10221", &[], &[]);
    }

    let read_only = r#"module spec.eei.lenreadonly;

func f(xs: i32[3]): i32 {
    xs.len = 4;
    return i32(xs.len);
}

func main(): i32 {
    let m: i32[3] = [1, 2, 3];
    return f(m);
}
"#;
    assert_rejected_once("len-readonly", read_only, "E10024", &[], &["E10221"]);
}

/// A struct-returning function that falls off its end returns the zero value
/// without a compound literal, and the fields read as zero.
#[test]
fn should_build_and_run_a_struct_empty_return_without_a_compound_literal() {
    let source = r#"module spec.eei.emptyret;

struct P {
    x: i32;
    y: i32;
}

func maybe(flag: bool): P {
    if (flag) {
        return P { x: 1, y: 2 };
    }
}

func main(): i32 {
    let p: P = maybe(false);
    return p.x + p.y + maybe(true).x;
}
"#;
    let emitted = emit_host("empty-return", source);
    assert!(
        !emitted.contains("(ClP){0}"),
        "empty-return: the C99-subset contract forbids the struct compound literal"
    );
    let output = run_host("empty-return", source);
    assert_exit("empty-return", &output, 1);
}

/// A freestanding program that uses `bool` builds on the host no-libc leg and
/// runs.
#[test]
fn should_build_and_run_a_freestanding_bool_program() {
    let source = r#"module spec.eei.fsboot;

func flag(): bool {
    return true;
}

let g: bool = flag();

func main(): i32 {
    if (g) {
        return 1;
    }
    return 0;
}
"#;
    let output = run_freestanding_host("freestanding-bool", source);
    assert_exit("freestanding-bool", &output, 1);
}

/// A slice passed to an uncontracted extern coerces to its pointer, so the
/// linked stub reads the backing bytes.
#[test]
fn should_build_and_run_a_slice_passed_to_an_uncontracted_extern() {
    let source = r#"module spec.eei.viewextern;

import { slice } from std.mem;

ffi lib {
    func RawSecond(data: u8[]): i32;
}

func main(): i32 {
    let xs: u8[4] = [7, 2, 3, 4];
    let s: u8[] = slice(xs, 0, 4);
    let n: i32 = 0;
    unsafe {
        n = RawSecond(s);
    }
    return n;
}
"#;
    let stub = "typedef void *APTR;\ntypedef int LONG;\nLONG RawSecond(APTR data) { return ((unsigned char *)data)[1]; }\n";
    let output = run_host_with_stub("view-extern", source, stub);
    assert_exit("view-extern", &output, 2);

    let coercions = r#"module spec.eei.viewexterncoerce;

import { slice } from std.mem;
import { String, stringOf } from std.text;

ffi lib {
    func CText(s: cstring): i32;
    func RStr(s: str): i32;
    func RSlice(data: u8[]): i32;
}

func main(): i32 {
    let xs: u8[4] = [7, 2, 3, 4];
    let s: u8[] = slice(xs, 0, 4);
    let t: str = "hi";
    let owned: String = stringOf("z");
    let n: i32 = 0;
    unsafe {
        n = CText("x") + RStr(t) + RStr(owned) + RSlice(s);
    }
    return n;
}
"#;
    let stub = "typedef void *APTR;\ntypedef char *STRPTR;\ntypedef int LONG;\nLONG CText(STRPTR s) { return s != 0; }\nLONG RStr(APTR s) { return s != 0; }\nLONG RSlice(APTR s) { return s != 0; }\n";
    let output = run_host_with_stub("view-extern-coerce", coercions, stub);
    assert_exit("view-extern-coerce", &output, 4);
}

/// Source bindings named like the compiler-generated locals de-duplicate
/// instead of colliding: a `__cl_ret` parameter in a function with cleanup, a
/// `__env` local inside a capturing lambda, and a `__cl_tmp0` binding.
#[test]
fn should_de_duplicate_source_bindings_named_like_generated_locals() {
    let source = r#"module spec.eei.gennames;

import { String, stringOf } from std.text;

func demo(__cl_ret: i32): i32 {
    let s: String = stringOf("x");
    if (__cl_ret > 0) {
        return __cl_ret + i32(len(s));
    }
    return 0;
}

func outer(x: i32): i32 {
    let base: i32 = x;
    return ([base] () => {
        let __env: i32 = base + 1;
        return __env + 1;
    })();
}

func main(): i32 {
    return demo(4) + outer(3);
}
"#;
    let output = run_host("generated-names", source);
    assert_exit("generated-names", &output, 10);
}

/// A multi-extent row slice of a struct builds after the row typedef follows
/// the struct definition, and the rows read correctly.
#[test]
fn should_build_and_run_a_multi_extent_row_slice_of_a_struct() {
    let source = r#"module spec.eei.rowslice;

import { slice } from std.mem;

struct P {
    x: i32;
    y: i32;
}

func total(rows: P[][2]): i32 {
    return rows[0][0].x + rows[1][1].y;
}

func main(): i32 {
    let back: P[3][2] = [[P { x: 1, y: 2 }, P { x: 3, y: 4 }], [P { x: 5, y: 6 }, P { x: 7, y: 8 }], [P { x: 9, y: 10 }, P { x: 11, y: 12 }]];
    let rows: P[][2] = slice(back, 0, 3);
    return total(rows);
}
"#;
    let output = run_host("row-slice", source);
    assert_exit("row-slice", &output, 9);
}

/// The single-extent pointer-element slice control still builds and runs.
#[test]
fn should_build_and_run_a_pointer_element_slice_control() {
    let source = r#"module spec.eei.ptrslice;

import { slice } from std.mem;

struct P {
    x: i32;
    y: i32;
}

func main(): i32 {
    let ps: P[3] = [P { x: 1, y: 2 }, P { x: 3, y: 4 }, P { x: 5, y: 6 }];
    let s: P[] = slice(ps, 0, 3);
    return s[2].x + s[0].y;
}
"#;
    let output = run_host("ptr-slice", source);
    assert_exit("ptr-slice", &output, 7);
}

/// A call of a callable-bound parameter's field builds and runs.
#[test]
fn should_build_and_run_a_callable_bound_field_call() {
    let source = r#"module spec.eei.boundfield;

struct Holder<T> {
    call: T;
}

func use<T: func(i32): i32>(h: Holder<T>): i32 {
    return h.call(5);
}

func addOne(x: i32): i32 {
    return x + 1;
}

func main(): i32 {
    return use<func(i32): i32>(Holder<func(i32): i32> { call: addOne });
}
"#;
    let output = run_host("bound-field", source);
    assert_exit("bound-field", &output, 6);
}

/// A callable field named like the builtin `len` is called through the field,
/// not lowered as the length intrinsic.
#[test]
fn should_call_a_func_field_named_len_through_the_field() {
    let source = r#"module spec.eei.funcfieldlen;

struct Holder {
    len: func(i32): i32;
    tag: i32;
}

func addOne(x: i32): i32 {
    return x + 1;
}

func main(): i32 {
    let h: Holder = Holder { len: addOne, tag: 7 };
    return h.len(5) + h.tag;
}
"#;
    let output = run_host("func-field-len", source);
    assert_exit("func-field-len", &output, 13);
}

/// A generic borrow's struct-field projection reads the instance field's real
/// length, not the fallback.
#[test]
fn should_read_the_instance_field_length_through_a_generic_borrow() {
    let source = r#"module spec.eei.fieldlen;

import { String, stringOf } from std.text;

struct Box {
    s: String;
}

func takeBox<T>(borrow b: T, fallback: i32): i32 {
    if (i32(len(b.s)) > 0) {
        return i32(len(b.s));
    }
    return fallback;
}

func main(): i32 {
    let b: Box = Box { s: stringOf("abcdefg") };
    return takeBox<Box>(b, 3);
}
"#;
    let output = run_host("field-len", source);
    assert_exit("field-len", &output, 7);
}

/// A growable array's projected length through a generic borrow reads the
/// element count, not the folded zero.
#[test]
fn should_read_the_growable_length_through_a_generic_borrow() {
    let source = r#"module spec.eei.growlen;

import { Array, arrayNew } from std.mem;

struct Basket {
    items: Array<i32>;
}

func countOf<T>(borrow b: T): i32 {
    return i32(len(b.items));
}

func main(): i32 {
    let b: Basket = Basket { items: arrayNew<i32>() };
    b.items.push(5);
    b.items.push(7);
    return countOf<Basket>(b);
}
"#;
    let output = run_host("growable-len", source);
    assert_exit("growable-len", &output, 2);
}

/// A local inferred from `null` keeps the nullable type and builds under the
/// strict flags.
#[test]
fn should_build_and_run_an_inferred_null_local() {
    let source = r#"module spec.eei.nullinfer;

func main(): i32 {
    let x = null;
    if (x == null) {
        return 1;
    }
    return 0;
}
"#;
    let output = run_host("null-infer", source);
    assert_exit("null-infer", &output, 1);
}

/// The inferred null local pairs with a view: the pairing is the view's
/// emptiness test, so a non-empty view does not match.
#[test]
fn should_build_and_run_a_null_local_paired_with_a_view() {
    let source = r#"module spec.eei.nullpair;

func main(): i32 {
    let x = null;
    let s: str = "ab";
    if (x == s) {
        return 1;
    }
    return 2;
}
"#;
    let output = run_host("null-pair", source);
    assert_exit("null-pair", &output, 2);
}

/// Two `str` views have no defined equality: the pair reports `E10222` and no
/// translation unit is written.
#[test]
fn should_reject_view_pair_equality() {
    let source = r#"module spec.eei.vieweq;

func main(): i32 {
    let a: str = "ab";
    let b: str = "ab";
    if (a == b) {
        return 1;
    }
    return 0;
}
"#;
    assert_rejected_once("view-eq", source, "E10222", &["operator '=='"], &[]);
}

/// The untyped-operand spelling reports on the view side, exactly like the
/// slice case.
#[test]
fn should_reject_an_untyped_operand_compared_with_a_str() {
    let source = r#"module spec.eei.untypedeq;

func main(): i32 {
    let x;
    x = 5;
    let s: str = "ab";
    if (x == s) {
        return 1;
    }
    return 0;
}
"#;
    assert_rejected_once("untyped-str-eq", source, "E10222", &["operator '=='"], &[]);
}

/// A `str` element loop reports the non-iterable rule; fixed-array and slice
/// loops keep building and run.
#[test]
fn should_reject_a_str_element_loop() {
    let source = r#"module spec.eei.strloop;

func sumStr(s: str): i32 {
    let total: i32 = 0;
    for (borrow c in s) {
        total = total + i32(c);
    }
    return total;
}

func main(): i32 {
    return sumStr("ab");
}
"#;
    assert_rejected_once("str-loop", source, "E10043", &["cannot be iterated"], &[]);

    let control = r#"module spec.eei.loopcontrol;

import { slice } from std.mem;

func sumArray(xs: i32[3]): i32 {
    let total: i32 = 0;
    for (borrow v in xs) {
        total = total + v;
    }
    return total;
}

func sumSlice(xs: i32[]): i32 {
    let total: i32 = 0;
    for (v in xs) {
        total = total + v;
    }
    return total;
}

func countEach<T>(xs: T): i32 {
    let n: i32 = 0;
    for (v in xs) {
        n = n + 1;
    }
    return n;
}

func main(): i32 {
    let m: i32[3] = [1, 2, 3];
    let s: i32[] = slice(m, 0, 3);
    return sumArray(m) + sumSlice(s) + countEach<i32[3]>(m) + countEach<i32[]>(s);
}
"#;
    let output = run_host("loop-control", control);
    assert_exit("loop-control", &output, 18);
}

/// A write through a `str` loop element reports the same non-iterable rule,
/// once for the site.
#[test]
fn should_reject_a_write_through_a_str_element_loop() {
    let source = r#"module spec.eei.strloopwrite;

func fill(s: str): void {
    for (borrow c in s) {
        c = 65;
    }
}

func main(): i32 {
    fill("ab");
    return 0;
}
"#;
    assert_rejected_once("str-loop-write", source, "E10043", &[], &[]);
}

/// A member call on a bare type parameter reports an unknown member; concrete
/// receivers and callable-bound members keep their call paths.
#[test]
fn should_reject_a_member_call_on_a_bare_type_parameter() {
    let source = r#"module spec.eei.bareparam;

struct S {
    x: i32;
}

func g<T>(x: T): i32 {
    return x.foo(5);
}

func main(): i32 {
    return g<i32>(5);
}
"#;
    assert_rejected_once(
        "bare-param-member",
        source,
        "E10212",
        &["'foo' is not a member of 'T'"],
        &["E10307"],
    );

    let control = r#"module spec.eei.dotcontrol;

struct Holder<T> {
    call: T;
}

struct Plain {
    call: func(i32): i32;
    tag: i32;
}

func addOne(x: i32): i32 {
    return x + 1;
}

func use<T: func(i32): i32>(h: Holder<T>): i32 {
    return h.call(5);
}

func main(): i32 {
    let p: Plain = Plain { call: addOne, tag: 7 };
    return p.call(5) + p.tag + use<func(i32): i32>(Holder<func(i32): i32> { call: addOne });
}
"#;
    let output = run_host("dot-control", control);
    assert_exit("dot-control", &output, 19);
}

/// A value-position update on a `const` binding reports the immutable-assign
/// rule, in both the fixed-array and slice spellings.
#[test]
fn should_reject_a_value_update_on_a_const_binding() {
    let fixed = r#"module spec.eei.constupdate;

func main(): i32 {
    const a: u8[2] = [1, 2];
    let y: i32 = i32(a[0]++);
    return y;
}
"#;
    assert_rejected_once("const-update", fixed, "E10023", &[], &[]);

    let slice_source = r#"module spec.eei.constsliceupdate;

import { slice } from std.mem;

func main(): i32 {
    let base: u8[2] = [1, 2];
    const s: u8[] = slice(base, 0, 2);
    let y: i32 = i32(s[0]++);
    return y;
}
"#;
    assert_rejected_once("const-slice-update", slice_source, "E10023", &[], &[]);
}

/// A value-position update through an unmarked parameter reports the
/// read-only rule.
#[test]
fn should_reject_a_value_update_through_a_readonly_parameter() {
    let source = r#"module spec.eei.paramupdate;

import { slice } from std.mem;

func bump(xs: u8[]): i32 {
    let z: i32 = i32(xs[0]++);
    return z;
}

func main(): i32 {
    let a: u8[2] = [1, 2];
    return bump(slice(a, 0, 2));
}
"#;
    assert_rejected_once("readonly-update", source, "E10024", &[], &[]);
}

/// The `str` value update keeps its immutable-view report.
#[test]
fn should_reject_a_value_update_through_a_str_view() {
    let source = r#"module spec.eei.strvalueupdate;

func bump(borrow s: str): i32 {
    let w: i32 = i32(s[0]++);
    return w;
}

func main(): i32 {
    return bump("ab");
}
"#;
    assert_rejected_once(
        "str-value-update",
        source,
        "E10042",
        &["immutable view 's'"],
        &[],
    );
}

/// A `cstring` element write reports the immutable-view rule.
#[test]
fn should_reject_a_cstring_element_write() {
    let source = r#"module spec.eei.cstringwrite;

func fill(borrow c: cstring): i32 {
    c[0] = 65;
    return 0;
}

func main(): i32 {
    return fill("ab");
}
"#;
    assert_rejected_once(
        "cstring-write",
        source,
        "E10042",
        &["immutable view 'c'"],
        &[],
    );
}

/// The statement forms keep their taxonomy: a const element assign reports
/// the immutable-assign rule, and a writable slice write builds and runs.
#[test]
fn should_keep_the_statement_write_taxonomy() {
    let const_assign = r#"module spec.eei.constassign;

func main(): i32 {
    const a: u8[2] = [1, 2];
    a[0] = 5;
    return 0;
}
"#;
    assert_rejected_once("const-assign", const_assign, "E10023", &[], &[]);

    let writable = r#"module spec.eei.slicewrite;

import { slice } from std.mem;

func main(): i32 {
    let a: u8[2] = [1, 2];
    let s: u8[] = slice(a, 0, 2);
    s[0] = 7;
    return i32(s[0]) + i32(a[0]);
}
"#;
    let output = run_host("slice-write", writable);
    assert_exit("slice-write", &output, 14);
}

/// Whole-view rebinds and writable slice writes stay accepted and run.
#[test]
fn should_accept_view_rebinds_and_writable_writes() {
    let source = r#"module spec.eei.writecontrol;

import { slice } from std.mem;
import { stringOf, toCString } from std.text;

func main(): i32 {
    let a: u8[2] = [1, 2];
    let s: u8[] = slice(a, 0, 2);
    s[0] = 7;
    let t: str = "ab";
    t = "cd";
    let c: cstring = toCString(stringOf("ef"));
    c = toCString(stringOf("gh"));
    if (i32(t[0]) != 99) {
        return 1;
    }
    if (i32(c[0]) != 103) {
        return 2;
    }
    return i32(s[0]) + i32(a[0]);
}
"#;
    let output = run_host("write-control", source);
    assert_exit("write-control", &output, 14);
}
