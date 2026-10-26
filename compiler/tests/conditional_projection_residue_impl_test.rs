//! Implementation tests for the arm-root and guarded-carrier shapes of a
//! conditional projection.
//!
//! The specification suite proves that the repaired programs run and are
//! sanitizer-clean; these cases prove how the compiler spells the repairs, so
//! a later change cannot silently drop a move-out, a root binding, or a
//! carrier: the per-instance arm move-out, the fresh root that binds because
//! it still owns un-taken storage, the literal arm's move-out, the guarded
//! `take` carrier that is filled on the evaluated path and emptied after the
//! call, and the literal receiver field whose adopting carrier gets no
//! reassigned-value release. The direct arm spellings are the untouched
//! control, and every program must re-emit byte-identically.
//!
//! The fixed-array emission shapes are pinned the same way: the per-arm
//! conditional field store, the hidden-destination return copy, the guarded
//! view-array element fill, the call-filled literal destinations, and the
//! distributed nested-arm read base.
//!
//! The element and store loans are pinned by the report sites and the copy
//! emissions: the element-read store and every conditional or literal copy
//! source report at their fills, the union keeps both arms' positions, the
//! accepted copies keep their store shapes, and an overwritten element drops
//! the loan it replaced.
//!
//! The hidden view destinations and guarded owning carriers are pinned by the
//! bound destination array and field copy at each read shape, the once-per-call
//! counters, the carrier fill inside its guard, the loop's re-fill and release
//! order, and the skipped field and element paths.

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
            "clc-cprimpl-{}-{tag}-{sequence}",
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

/// Compiles `source_path` into `c_path` and returns the emitted text, so the
/// determinism check can point both passes at one source path.
fn emit_at(tag: &str, source_path: &Path, c_path: &Path) -> String {
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
    let emitted = std::fs::read_to_string(c_path).unwrap_or_default();
    assert!(!emitted.trim().is_empty(), "{tag}: the emitted C is empty");
    emitted
}

/// Writes `source` into the scratch directory and returns its path.
fn write_source(tag: &str, scratch: &ScratchDir, source: &str) -> PathBuf {
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source)
        .unwrap_or_else(|err| panic!("{tag}: cannot write the program: {err}"));
    source_path
}

/// Emits one host-profile program and asserts the front end accepted it.
///
/// The scratch directory is returned so it outlives the emitted text.
fn emit_host(tag: &str, source: &str) -> (ScratchDir, String) {
    let scratch = ScratchDir::new(tag);
    let source_path = write_source(tag, &scratch, source);
    let emitted = emit_at(tag, &source_path, &scratch.path().join("out.c"));
    (scratch, emitted)
}

/// Asserts the emitted C contains `needle`.
fn assert_contains(tag: &str, emitted: &str, needle: &str) {
    assert!(
        emitted.contains(needle),
        "{tag}: expected the emitted C to contain {needle:?}; emitted C:\n{emitted}"
    );
}

/// Asserts the emitted C does not contain `needle`.
fn assert_absent(tag: &str, emitted: &str, needle: &str) {
    assert!(
        !emitted.contains(needle),
        "{tag}: expected the emitted C to lack {needle:?}; emitted C:\n{emitted}"
    );
}

/// Asserts that `release` occurs after `move_out` in the emitted text.
fn assert_release_after(tag: &str, emitted: &str, move_out: &str, release: &str) {
    let moved = emitted
        .find(move_out)
        .unwrap_or_else(|| panic!("{tag}: {move_out:?} is missing; emitted C:\n{emitted}"));
    let released = emitted
        .rfind(release)
        .unwrap_or_else(|| panic!("{tag}: {release:?} is missing; emitted C:\n{emitted}"));
    assert!(
        released > moved,
        "{tag}: {release:?} must follow {move_out:?}; emitted C:\n{emitted}"
    );
}

/// The body of the function whose emitted signature is `signature`.
///
/// Nested blocks close with an indented brace, so the first bare `}` closes
/// the function itself.
fn function_body<'a>(tag: &str, emitted: &'a str, signature: &str) -> &'a str {
    let start = emitted
        .find(signature)
        .unwrap_or_else(|| panic!("{tag}: the emitted C does not contain {signature:?}"));
    let text = &emitted[start + signature.len()..];
    let end = text
        .find("\n}")
        .unwrap_or_else(|| panic!("{tag}: the body after {signature:?} has no closing brace"));
    &text[..end]
}

/// Asserts a store precedes the self-alias wrapper and the source zero follows
/// the wrapper's else branch.
///
/// The arm source move is wrapped in an address test, so a plain adjacency
/// assertion no longer holds; this keeps the store-before-zero order that the
/// original assertion carried.
fn assert_store_then_zero(tag: &str, emitted: &str, store: &str, zero: &str) {
    let start = emitted
        .find(store)
        .unwrap_or_else(|| panic!("{tag}: {store:?} is missing; emitted C:\n{emitted}"));
    let wrapper = emitted[start..]
        .find("if (((void *)(")
        .map(|offset| start + offset)
        .unwrap_or_else(|| panic!("{tag}: the self-alias wrapper is missing; emitted C:\n{emitted}"));
    let else_at = emitted[wrapper..]
        .find("} else {")
        .map(|offset| wrapper + offset)
        .unwrap_or_else(|| panic!("{tag}: the self-alias else is missing; emitted C:\n{emitted}"));
    let zero_at = emitted[else_at..]
        .find(zero)
        .map(|offset| else_at + offset)
        .unwrap_or_else(|| panic!("{tag}: {zero:?} is missing after the wrapper; emitted C:\n{emitted}"));
    assert!(
        start < wrapper && wrapper < else_at && else_at < zero_at,
        "{tag}: the store, self-alias wrapper, else, and source zero are out of order; emitted C:\n{emitted}"
    );
}

/// Asserts `second` follows `first` with only `#line` markers between them;
/// any other text means a statement intervenes.
fn assert_follows(tag: &str, emitted: &str, first: &str, second: &str) {
    let start = emitted
        .find(first)
        .unwrap_or_else(|| panic!("{tag}: {first:?} is missing; emitted C:\n{emitted}"));
    let after = &emitted[start + first.len()..];
    let second_at = after.find(second).unwrap_or_else(|| {
        panic!("{tag}: {second:?} does not follow {first:?}; emitted C:\n{emitted}")
    });
    assert!(
        after[..second_at]
            .lines()
            .all(|line| line.trim().is_empty() || line.trim_start().starts_with("#line")),
        "{tag}: a statement intervenes between {first:?} and {second:?}; emitted C:\n{emitted}"
    );
}

/// Asserts the emitted line with `anchor` also contains `needle`.
///
/// A guarded operand is printed as one line, so this pins that `needle` runs
/// where the guard reaches it instead of being hoisted before the guard.
fn assert_on_line_of(tag: &str, emitted: &str, anchor: &str, needle: &str) {
    let line = emitted
        .lines()
        .find(|line| line.contains(anchor))
        .unwrap_or_else(|| panic!("{tag}: no emitted line contains {anchor:?}"));
    assert!(
        line.contains(needle),
        "{tag}: expected the line with {anchor:?} to contain {needle:?}; line:\n{line}"
    );
}

/// Emits one program twice from one source path, returning both texts.
///
/// Both emissions share the source path so the `#line` facts agree and the
/// comparison is about the emitted shape alone.
fn emit_twice(tag: &str, source: &str) -> (String, String) {
    let scratch = ScratchDir::new(tag);
    let source_path = write_source(tag, &scratch, source);
    let first = emit_at(tag, &source_path, &scratch.path().join("out0.c"));
    let second = emit_at(tag, &source_path, &scratch.path().join("out1.c"));
    (first, second)
}

/// Compiles `source` and asserts the front end rejected it with exactly
/// `count` error reports, each carrying `code`, and wrote no translation unit.
///
/// The owner and return judgments reject before code generation, so their pins
/// run through the front end: the exact count keeps a fix from passing by
/// adding an unrelated report, and the missing translation unit proves the
/// rejection happened before emission.
fn assert_rejected(tag: &str, source: &str, code: &str, count: usize) {
    let scratch = ScratchDir::new(tag);
    let source_path = write_source(tag, &scratch, source);
    let c_path = scratch.path().join("out.c");
    let output = run_clc(&[
        "--profile".to_owned(),
        "host".to_owned(),
        "-o".to_owned(),
        c_path.display().to_string(),
        source_path.display().to_string(),
    ]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_ne!(
        output.status.code(),
        Some(0),
        "{tag}: expected a rejection; stderr:\n{stderr}"
    );
    let errors: Vec<&str> = stderr
        .lines()
        .filter(|line| line.starts_with("error "))
        .collect();
    assert_eq!(
        errors.len(),
        count,
        "{tag}: expected {count} error(s), saw {}; stderr:\n{stderr}",
        errors.len()
    );
    for line in &errors {
        assert!(
            line.contains(code),
            "{tag}: expected the {code} report; saw `{line}`; stderr:\n{stderr}"
        );
    }
    assert!(
        !c_path.exists(),
        "{tag}: a rejected program must not produce a translation unit"
    );
}

/// Compiles `source` and asserts the front end rejected it with exactly
/// `count` error reports at `line`, each carrying `code`, and wrote no
/// translation unit.
///
/// The judgment site is the store expression itself, so the reported line
/// pins the store's own source line rather than a derived declaration or a
/// later use. A diagnostic renders its location on the line after its
/// message, so pairing each error line with the next location line keeps a
/// warning's location out of the comparison.
/// Asserts the source is rejected with the exact set of diagnostic codes.
fn assert_rejected_code_set(tag: &str, source: &str, codes: &[&str]) {
    let scratch = ScratchDir::new(tag);
    let source_path = write_source(tag, &scratch, source);
    let c_path = scratch.path().join("out.c");
    let output = run_clc(&[
        "--profile".to_owned(),
        "host".to_owned(),
        "-o".to_owned(),
        c_path.display().to_string(),
        source_path.display().to_string(),
    ]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_ne!(
        output.status.code(),
        Some(0),
        "{tag}: expected a rejection; stderr:\n{stderr}"
    );
    let actual: Vec<&str> = stderr
        .lines()
        .filter(|line| line.starts_with("error "))
        .map(|line| line.split_whitespace().nth(1).unwrap_or("").trim_end_matches(':'))
        .collect();
    assert_eq!(
        actual.len(),
        codes.len(),
        "{tag}: expected {codes:?}, saw {actual:?}; stderr:\n{stderr}"
    );
    for code in codes {
        assert!(
            actual.contains(code),
            "{tag}: expected {code}; saw {actual:?}; stderr:\n{stderr}"
        );
    }
    assert!(
        !c_path.exists(),
        "{tag}: a rejected program must not produce a translation unit"
    );
}

fn assert_rejected_at_line(tag: &str, source: &str, code: &str, count: usize, line: usize) {
    let scratch = ScratchDir::new(tag);
    let source_path = write_source(tag, &scratch, source);
    let c_path = scratch.path().join("out.c");
    let output = run_clc(&[
        "--profile".to_owned(),
        "host".to_owned(),
        "-o".to_owned(),
        c_path.display().to_string(),
        source_path.display().to_string(),
    ]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_ne!(
        output.status.code(),
        Some(0),
        "{tag}: expected a rejection; stderr:\n{stderr}"
    );
    let errors: Vec<&str> = stderr
        .lines()
        .filter(|line| line.starts_with("error "))
        .collect();
    assert_eq!(
        errors.len(),
        count,
        "{tag}: expected {count} error(s), saw {}; stderr:\n{stderr}",
        errors.len()
    );
    for error in &errors {
        assert!(
            error.contains(code),
            "{tag}: expected the {code} report; saw `{error}`; stderr:\n{stderr}"
        );
    }
    let mut locations: Vec<usize> = Vec::new();
    let mut lines = stderr.lines();
    while let Some(current) = lines.next() {
        if !current.starts_with("error ") {
            continue;
        }
        let Some(location) = lines.next() else { continue };
        let Some(rest) = location.trim_start().strip_prefix("-->") else {
            continue;
        };
        let reported = rest
            .trim()
            .rsplit(':')
            .nth(1)
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(0);
        locations.push(reported);
    }
    assert_eq!(
        locations,
        vec![line; count],
        "{tag}: expected every report at line {line}; stderr:\n{stderr}"
    );
    assert!(
        !c_path.exists(),
        "{tag}: a rejected program must not produce a translation unit"
    );
}

/// Compiles `source` and asserts the front end rejected it with exactly one
/// report at `line`:`column`, carrying `code`, whose message contains every
/// `fragments` entry and no `absent` entry.
///
/// One expression carries one report: a nested mixed view return and a
/// two-owner array literal each keep a single report at their own span, and a
/// destination that only repeats its callee's judgment keeps none. Pinning the
/// rendered message beside the code and count stops a phrase regression from
/// passing unnoticed.
fn assert_rejected_once_at(
    tag: &str,
    source: &str,
    code: &str,
    line: usize,
    column: usize,
    fragments: &[&str],
    absent: &[&str],
) {
    let scratch = ScratchDir::new(tag);
    let source_path = write_source(tag, &scratch, source);
    let c_path = scratch.path().join("out.c");
    let output = run_clc(&[
        "--profile".to_owned(),
        "host".to_owned(),
        "-o".to_owned(),
        c_path.display().to_string(),
        source_path.display().to_string(),
    ]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_ne!(
        output.status.code(),
        Some(0),
        "{tag}: expected a rejection; stderr:\n{stderr}"
    );
    let errors: Vec<&str> = stderr
        .lines()
        .filter(|line| line.starts_with("error "))
        .collect();
    assert_eq!(
        errors.len(),
        1,
        "{tag}: expected exactly one error, saw {}; stderr:\n{stderr}",
        errors.len()
    );
    assert!(
        errors[0].contains(code),
        "{tag}: expected the {code} report; saw `{}`; stderr:\n{stderr}",
        errors[0]
    );
    let suffix = format!(":{line}:{column}");
    let site = stderr
        .lines()
        .find(|line| line.trim_start().starts_with("--> "))
        .unwrap_or_else(|| panic!("{tag}: the report has no site; stderr:\n{stderr}"));
    assert!(
        site.trim_end().ends_with(&suffix),
        "{tag}: expected the report site to end with {suffix:?}; saw `{}`; stderr:\n{stderr}",
        site.trim()
    );
    for fragment in fragments {
        assert!(
            stderr.contains(fragment),
            "{tag}: expected the message to contain {fragment:?}; stderr:\n{stderr}"
        );
    }
    for fragment in absent {
        assert!(
            !stderr.contains(fragment),
            "{tag}: expected the message to lack {fragment:?}; stderr:\n{stderr}"
        );
    }
    assert!(
        !c_path.exists(),
        "{tag}: a rejected program must not produce a translation unit"
    );
}

/// Compiles `source` and asserts the front end rejected it with exactly one
/// report carrying `code` at each of the given `line:column` sites, whose
/// message contains every `fragments` entry, and wrote no translation unit.
///
/// Two equal entries pin two reports at one span; distinct entries pin one
/// report at each site. The site set is compared as a whole, so a report that
/// moves to another expression fails the case even when the count and the code
/// still match.
fn assert_rejected_at_sites(
    tag: &str,
    source: &str,
    code: &str,
    sites: &[(usize, usize)],
    fragments: &[&str],
) {
    let scratch = ScratchDir::new(tag);
    let source_path = write_source(tag, &scratch, source);
    let c_path = scratch.path().join("out.c");
    let output = run_clc(&[
        "--profile".to_owned(),
        "host".to_owned(),
        "-o".to_owned(),
        c_path.display().to_string(),
        source_path.display().to_string(),
    ]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_ne!(
        output.status.code(),
        Some(0),
        "{tag}: expected a rejection; stderr:\n{stderr}"
    );
    let errors: Vec<&str> = stderr
        .lines()
        .filter(|report| report.starts_with("error "))
        .collect();
    assert_eq!(
        errors.len(),
        sites.len(),
        "{tag}: expected {} error(s), saw {}; stderr:\n{stderr}",
        sites.len(),
        errors.len()
    );
    for line in &errors {
        assert!(
            line.contains(code),
            "{tag}: expected the {code} report; saw `{line}`; stderr:\n{stderr}"
        );
    }
    let mut actual_sites: Vec<String> = stderr
        .lines()
        .filter(|report| report.trim_start().starts_with("--> "))
        .map(|report| report.trim().to_owned())
        .collect();
    actual_sites.sort();
    let mut expected_sites: Vec<String> = sites
        .iter()
        .map(|(line, column)| format!(":{line}:{column}"))
        .collect();
    expected_sites.sort();
    assert_eq!(
        actual_sites.len(),
        expected_sites.len(),
        "{tag}: expected {} report site(s), saw {}; stderr:\n{stderr}",
        expected_sites.len(),
        actual_sites.len()
    );
    for (actual, expected) in actual_sites.iter().zip(&expected_sites) {
        assert!(
            actual.ends_with(expected.as_str()),
            "{tag}: expected a report at {expected}; saw `{actual}`; stderr:\n{stderr}"
        );
    }
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
const ARM_DUTY_SOURCE: &str = r#"module spec.cpr.armduty;
import { String, stringOf } from std.text;
struct Pair<T> { v: T; w: T; }
func mkPair<T>(take x: T, take y: T): Pair<T> { return Pair<T> { v: x, w: y }; }
func pick<T>(c: bool, take a: Pair<T>, take x: T, take y: T): T {
    let s: T = (c ? a : mkPair<T>(x, y)).v;
    return s;
}

func main(): i32 {
    let p: Pair<String> = Pair<String> { v: stringOf("ab"), w: stringOf("cd") };
    let q: Pair<String> = Pair<String> { v: stringOf("ef"), w: stringOf("gh") };
    let s: String = pick<String>(true, p, stringOf("ij"), stringOf("kl"));
    let t: String = pick<String>(false, q, stringOf("mn"), stringOf("op"));
    return i32(len(s)) + i32(len(t)) - 4;
}
"#;

/// A `String` local selected between a place arm and a fresh root with an
/// owning sibling field.
const ARM_FRESH_SOURCE: &str = r#"module spec.cpr.armfresh;
import { String, stringOf } from std.text;
struct Box { v: String; w: String; }
func mkBox(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }
func use(c: bool, take a: Box): i32 {
    let s: String = c ? a.v : mkBox().v;
    return i32(len(s));
}

func main(): i32 {
    let a: Box = Box { v: stringOf("xx"), w: stringOf("yy") };
    let b: Box = Box { v: stringOf("pp"), w: stringOf("qq") };
    return use(true, a) + use(false, b) - 4;
}
"#;

/// A `String` local selected between a place arm and a struct literal.
const ARM_LITERAL_SOURCE: &str = r#"module spec.cpr.armliteral;
import { String, stringOf } from std.text;
struct Box { v: String; w: String; }
func use(c: bool, take a: Box): i32 {
    let s: String = (c ? a : Box { v: stringOf("mk"), w: stringOf("qqqq") }).v;
    return i32(len(s)) - 2;
}

func main(): i32 {
    let a: Box = Box { v: stringOf("xx"), w: stringOf("yy") };
    let b: Box = Box { v: stringOf("pp"), w: stringOf("qq") };
    return use(true, a) + use(false, b);
}
"#;

/// The same literal arm reached through a `&&` guard and read by `.len()`.
const ARM_LITERAL_GUARD_SOURCE: &str = r#"module spec.cpr.armliteralguard;
import { String, stringOf } from std.text;
struct Box { v: String; w: String; }
func use(c: bool, take a: Box, g: bool): i32 {
    if (g && (c ? a : Box { v: stringOf("mk"), w: stringOf("qqqq") }).v.len() > 0) {
        return 1;
    }
    return 0;
}

func main(): i32 {
    let a: Box = Box { v: stringOf("xx"), w: stringOf("yy") };
    let b: Box = Box { v: stringOf("pp"), w: stringOf("qq") };
    return use(true, a, true) + use(false, b, true) - 2;
}
"#;

/// A guarded `take` whose argument is a conditional projection.
const GUARD_TAKES_SOURCE: &str = r#"module spec.cpr.guardtakes;
import { String, stringOf } from std.text;
struct Box { v: String; w: String; }
func mkBox(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }
func consume(take s: String): i32 { return i32(len(s)); }
func use(c: bool, take a: Box, g: bool): i32 {
    if (g && consume((c ? a : mkBox()).v) > 0) {
        return 1;
    }
    return 0;
}

func main(): i32 {
    let a: Box = Box { v: stringOf("xx"), w: stringOf("yy") };
    let b: Box = Box { v: stringOf("pp"), w: stringOf("qq") };
    return use(true, a, true) + use(false, b, true) - 2;
}
"#;

/// The guarded `take` spelling re-executed by a loop in `main`.
const GUARD_TAKES_LOOP_SOURCE: &str = r#"module spec.cpr.guardtakesloop;
import { String, stringOf } from std.text;
struct Box { v: String; w: String; }
func mkBox(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }
func consume(take s: String): i32 { return i32(len(s)); }
func use(c: bool, take a: Box, g: bool): i32 {
    if (g && consume((c ? a : mkBox()).v) > 0) {
        return 1;
    }
    return 0;
}

func main(): i32 {
    let n: i32 = 0;
    let i: i32 = 0;
    while (i < 3) {
        let a: Box = Box { v: stringOf("xx"), w: stringOf("yy") };
        let b: Box = Box { v: stringOf("pp"), w: stringOf("qq") };
        n = n + use(true, a, true) + use(false, b, true);
        i = i + 1;
    }
    return n - 6;
}
"#;

/// A callable-field call on a struct literal whose field projects a fresh root.
const FIELD_ROOT_SOURCE: &str = r#"module spec.cpr.fieldroot;
import { String, stringOf } from std.text;
struct Box { v: String; w: String; }
struct H { call: func(i32): i32; s: String; }
func mkBox(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }
func addOne(x: i32): i32 { return x + 1; }
func use(g: bool): i32 {
    if (g && H { call: addOne, s: mkBox().v }.call(5) > 0) {
        return 1;
    }
    return 0;
}

func main(): i32 {
    return use(true) + use(true) - 2;
}
"#;

/// The direct arm spellings: a moved member, a fresh member read, and an
/// unguarded literal receiver.
const ARM_CONTROLS_SOURCE: &str = r#"module spec.cpr.armcontrols;
import { String, stringOf } from std.text;
struct Box { v: String; w: String; }
struct H { call: func(i32): i32; s: String; }
func mkBox(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }
func addOne(x: i32): i32 { return x + 1; }
func use(c: bool, take a: Box): i32 {
    let s: String = a.v;
    let u: String = mkBox().v;
    return i32(len(s)) + i32(len(u));
}

func main(): i32 {
    let a: Box = Box { v: stringOf("xx"), w: stringOf("yy") };
    let b: Box = Box { v: stringOf("pp"), w: stringOf("qq") };
    let g: bool = H { call: addOne, s: mkBox().v }.call(5) > 0;
    return use(true, a) + use(false, b) + (g ? 1 : 0) - 9;
}
"#;

/// A generic arm moves the selected member out per instantiation.
///
/// In the `String` instantiation the place arm's member is copied into its own
/// adopting temporary and zeroed in the parameter, the fresh arm's member is
/// moved out of the built pair the same way, the store selects between the two
/// temporaries, and the pair releases its un-moved `w` field only after the
/// move-out.
#[test]
fn should_move_the_generic_arms_member_out_per_instance() {
    let tag = "armduty";
    let (_scratch, emitted) = emit_host(tag, ARM_DUTY_SOURCE);
    let signature = "cl_pick__String(bool c, ClPair__String a, ClString x, ClString y)\n{";
    let body = function_body(tag, &emitted, signature);
    assert_follows(tag, body, "__cl_tmp2 = a.v;", "a.v = NULL;");
    assert_follows(tag, body, "__cl_tmp3 = __cl_tmp0.v;", "__cl_tmp0.v = NULL;");
    assert_contains(tag, body, "s = __cl_tmp2;");
    assert_contains(tag, body, "s = __cl_tmp3;");
    assert_release_after(
        tag,
        body,
        "__cl_tmp3 = __cl_tmp0.v;",
        "if (__cl_tmp0.w) { FreeString(__cl_tmp0.w); }",
    );
}

/// A fresh root that still owns un-taken storage binds its own temporary: the
/// call result fills an owned binding, the member moves out, and the binding
/// releases its `w` field after the store.
#[test]
fn should_bind_the_fresh_root_that_owns_residual_storage() {
    let tag = "armfresh";
    let (_scratch, emitted) = emit_host(tag, ARM_FRESH_SOURCE);
    let body = function_body(tag, &emitted, "cl_spec_cpr_armfresh_use(bool c, ClBox a)\n{");
    assert_contains(tag, body, "ClBox __cl_tmp1 = {0};");
    assert_contains(tag, body, "__cl_tmp1 = cl_spec_cpr_armfresh_mkBox();");
    assert_store_then_zero(tag, body, "s = __cl_tmp1.v;", "__cl_tmp1.v = NULL;");
    assert_release_after(
        tag,
        body,
        "s = __cl_tmp1.v;",
        "if (__cl_tmp1.w) { FreeString(__cl_tmp1.w); }",
    );
}

/// A literal arm moves its member out and releases its own sibling field.
///
/// The plain spelling fills an owned binding, moves the member out and zeroes
/// it there, and releases the un-moved `w` after the store; the guarded
/// spelling keeps the literal inside the `g &&` guard line and still frees its
/// field once at the statement end.
#[test]
fn should_move_out_of_a_literal_projection_arm() {
    let tag = "armliteral";
    let (_plain, plain) = emit_host(tag, ARM_LITERAL_SOURCE);
    let body = function_body(tag, &plain, "cl_spec_cpr_armliteral_use(bool c, ClBox a)\n{");
    assert_contains(
        tag,
        body,
        "__cl_tmp2 = (ClBox){ cl_std_text_stringOf(cl_str(\"mk\")), ",
    );
    assert_follows(tag, body, "__cl_tmp3 = __cl_tmp2.v;", "__cl_tmp2.v = NULL;");
    assert_release_after(
        tag,
        body,
        "__cl_tmp3 = __cl_tmp2.v;",
        "if (__cl_tmp2.w) { FreeString(__cl_tmp2.w); }",
    );

    let tag = "armliteralguard";
    let (_guarded, guarded) = emit_host(tag, ARM_LITERAL_GUARD_SOURCE);
    let signature = "cl_spec_cpr_armliteralguard_use(bool c, ClBox a, bool g)\n{";
    let body = function_body(tag, &guarded, signature);
    assert_on_line_of(
        tag,
        body,
        "if ((g && ((",
        "(ClBox){ cl_std_text_stringOf(cl_str(\"mk\")), cl_std_text_stringOf(cl_str(\"qqqq\")) }",
    );
    assert_contains(tag, body, "), __cl_tmp0)).v)");
    assert_contains(tag, body, "if (__cl_tmp0.w) { FreeString(__cl_tmp0.w); }");
}

/// A guarded `take` fills one shared carrier and empties it after the call.
///
/// The carrier holds the chosen member only on the path its guard reaches: the
/// place arm copies and zeroes its member into the carrier inside the `&&`
/// expression, the fresh arm binds its call root and moves the member out next
/// to it, and the call reads the carrier. The post-call move-out empties it, so
/// a later execution never releases the transferred member again.
#[test]
fn should_fill_and_empty_the_guarded_take_carrier() {
    for (tag, source) in [
        ("guardtakes", GUARD_TAKES_SOURCE),
        ("guardtakesloop", GUARD_TAKES_LOOP_SOURCE),
    ] {
        let (_scratch, emitted) = emit_host(tag, source);
        let signature = format!("cl_spec_cpr_{tag}_use(bool c, ClBox a, bool g)\n{{");
        let body = function_body(tag, &emitted, &signature);
        assert_contains(tag, body, "ClString __cl_tmp1 = NULL;");
        assert_on_line_of(tag, body, "if ((g && ((", "(__cl_tmp1 = a.v, (a.v = NULL))");
        let fresh_fill = format!(
            "(__cl_tmp0 = cl_spec_cpr_{tag}_mkBox())), __cl_tmp0.v), (__cl_tmp0.v = NULL))"
        );
        assert_contains(tag, body, &fresh_fill);
        let call_and_empty =
            format!("(__cl_tmp2 = cl_spec_cpr_{tag}_consume(__cl_tmp1)), (__cl_tmp1 = NULL))");
        assert_contains(tag, body, &call_and_empty);
        assert_absent(tag, body, "if (__cl_tmp1) { FreeString(__cl_tmp1); }");
    }
}

/// A literal receiver field binds its projected root inside the guard.
///
/// The field's value projects a fresh call root, so the guard expression binds
/// the root, moves the member through the adopting carrier the field stores,
/// and releases the root's un-moved `w` field. The adopting carrier gets no
/// reassigned-value release — the field owns the value — while the receiver's
/// own reassigned-value release stays.
#[test]
fn should_bind_the_receiver_fields_root_inside_the_guard() {
    let tag = "fieldroot";
    let (_scratch, emitted) = emit_host(tag, FIELD_ROOT_SOURCE);
    let body = function_body(tag, &emitted, "cl_spec_cpr_fieldroot_use(bool g)\n{");
    assert_on_line_of(
        tag,
        body,
        "if ((g && ((",
        "(__cl_tmp0 = cl_spec_cpr_fieldroot_mkBox())",
    );
    assert_contains(tag, body, "(__cl_tmp1 = __cl_tmp0.v), (__cl_tmp0.v = NULL)");
    assert_contains(
        tag,
        body,
        "(ClH){ cl_spec_cpr_fieldroot_addOne, __cl_tmp1 }",
    );
    assert_contains(
        tag,
        body,
        "(__cl_tmp2.s ? (FreeString(__cl_tmp2.s), __cl_tmp2.s = NULL) : 0)",
    );
    assert_absent(tag, body, "__cl_tmp1 ?");
    assert_contains(tag, body, "if (__cl_tmp0.w) { FreeString(__cl_tmp0.w); }");
}

/// The direct arm spellings keep their pinned shapes: the moved `take`
/// member, the fresh member read through an owned scope binding, and the
/// unguarded literal receiver.
#[test]
fn should_keep_the_direct_arm_spellings() {
    let tag = "armcontrols";
    let (_scratch, emitted) = emit_host(tag, ARM_CONTROLS_SOURCE);
    let body = function_body(tag, &emitted, "cl_spec_cpr_armcontrols_use(bool c, ClBox a)\n{");
    assert_follows(tag, body, "s = a.v;", "a.v = NULL;");
    assert_contains(tag, body, "__cl_owned0 = cl_spec_cpr_armcontrols_mkBox();");
    assert_follows(tag, body, "u = __cl_owned0.v;", "__cl_owned0.v = NULL;");
    assert_contains(tag, body, "if (__cl_owned0.w) { FreeString(__cl_owned0.w); }");
}

/// Every pinned program re-emits byte-identically: two emissions of one file
/// must produce the same text, so the shapes never depend on run order.
#[test]
fn should_re_emit_every_projection_program_byte_identically() {
    let cases: [(&str, &str); 8] = [
        ("armduty", ARM_DUTY_SOURCE),
        ("armfresh", ARM_FRESH_SOURCE),
        ("armliteral", ARM_LITERAL_SOURCE),
        ("armliteralguard", ARM_LITERAL_GUARD_SOURCE),
        ("guardtakes", GUARD_TAKES_SOURCE),
        ("guardtakesloop", GUARD_TAKES_LOOP_SOURCE),
        ("fieldroot", FIELD_ROOT_SOURCE),
        ("armcontrols", ARM_CONTROLS_SOURCE),
    ];
    for (tag, source) in cases {
        let (first, second) = emit_twice(tag, source);
        assert!(
            first == second,
            "{tag}: the two emissions differ; first:\n{first}\nsecond:\n{second}"
        );
    }
}

/// A guarded callable receiver whose array field adopts a fresh holder's field
/// and whose guarded statement re-executes in a loop.
const GUARDED_ARRAY_FIELD_SOURCE: &str = r#"module spec.cpr.statereuse;
import { String, stringOf } from std.text;
import { Array, arrayNew } from std.mem;
struct Holder { data: Array<String>; tag: String; }
struct H { call: func(i32): i32; s: Array<String>; }
func mkHolder(): Holder {
    let a: Array<String> = arrayNew<String>();
    a.push(stringOf("ab"));
    return Holder { data: a, tag: stringOf("tt") };
}
func addOne(x: i32): i32 { return x + 1; }
func use(g: bool, k: i32): i32 {
    let n: i32 = 0;
    let i: i32 = 0;
    while (i < k) {
        if (g && H { call: addOne, s: mkHolder().data }.call(5) > 0) {
            n = n + 1;
        }
        i = i + 1;
    }
    return n;
}

func main(): i32 {
    return use(true, 3) - 3;
}
"#;

/// A guarded receiver's adopted array field declares its carrier and fills it
/// inside the guard, and no statement-form release of the carrier is hoisted.
///
/// The array's drop has no expression form, so the emitter could otherwise
/// print a statement-form release of the carrier's previous value before the
/// guarded `if`. The adopting carrier's consumer — the receiver's field — owns
/// the value, so no such release may run: a re-executed guard would free the
/// storage the receiver's own release already freed.
#[test]
fn should_not_hoist_the_adopting_carriers_statement_form_drop() {
    let tag = "statereuse";
    let (_scratch, emitted) = emit_host(tag, GUARDED_ARRAY_FIELD_SOURCE);
    let body = function_body(tag, &emitted, "cl_spec_cpr_statereuse_use(bool g, i32 k)\n{");
    assert_contains(tag, body, "ClArray__String __cl_tmp1 = {0};");
    assert_on_line_of(tag, body, "if ((g && ((", "(__cl_tmp1 = __cl_tmp0.data)");
    assert_absent(tag, body, "if (__cl_tmp1.data) {");
    assert_absent(tag, body, "cl_buffer_ptr(__cl_tmp1.data)");
}

/// A generic take projection and a generic return projection.
const GENERIC_TAKE_SOURCE: &str = r#"module spec.cpr.generictake;
import { String, stringOf } from std.text;
struct Pair<T> { v: T; w: T; }
func mkPair<T>(take x: T, take y: T): Pair<T> { return Pair<T> { v: x, w: y }; }
func hold<T>(take s: T): i32 { return 1; }
func eager<T>(c: bool, take a: Pair<T>, take x: T, take y: T): i32 {
    return hold<T>((c ? a : mkPair<T>(x, y)).v);
}
func guarded<T>(c: bool, take a: Pair<T>, take x: T, take y: T, g: bool): i32 {
    if (g && hold<T>((c ? a : mkPair<T>(x, y)).v) > 0) {
        return 1;
    }
    return 0;
}

func main(): i32 {
    let p: Pair<String> = Pair<String> { v: stringOf("ab"), w: stringOf("cd") };
    let q: Pair<String> = Pair<String> { v: stringOf("ef"), w: stringOf("gh") };
    let a: Pair<i32> = Pair<i32> { v: 1, w: 2 };
    let b: Pair<i32> = Pair<i32> { v: 3, w: 4 };
    return eager<String>(true, p, stringOf("ij"), stringOf("kl"))
         + eager<i32>(true, a, 5, 6)
         + guarded<String>(true, q, stringOf("mn"), stringOf("op"), true)
         + guarded<i32>(true, b, 7, 8, true);
}
"#;

/// The generic take moves the chosen member per instance.
///
/// The `String` instances fill the carrier from the chosen arm and empty that
/// arm's member before the call, and the post-call move-out empties the
/// carrier; the `i32` instances copy, so their arm keeps its value. The guarded
/// spelling fills the same carrier inside its guard line.
#[test]
fn should_move_a_generic_take_projection_per_instance() {
    let tag = "generictake";
    let (_scratch, emitted) = emit_host(tag, GENERIC_TAKE_SOURCE);
    let body = function_body(
        tag,
        &emitted,
        "cl_eager__String(bool c, ClPair__String a, ClString x, ClString y)\n{",
    );
    assert_contains(tag, body, "ClString __cl_tmp1 = NULL;");
    assert_follows(tag, body, "__cl_tmp1 = a.v;", "a.v = NULL;");
    assert_follows(tag, body, "__cl_tmp1 = __cl_tmp0.v;", "__cl_tmp0.v = NULL;");
    assert_follows(tag, body, "cl_hold__String(__cl_tmp1);", "__cl_tmp1 = NULL;");

    let body = function_body(
        tag,
        &emitted,
        "cl_eager__i32(bool c, ClPair__i32 a, i32 x, i32 y)\n{",
    );
    assert_contains(tag, body, "__cl_tmp1 = a.v;");
    assert_absent(tag, body, "a.v = NULL;");

    let body = function_body(
        tag,
        &emitted,
        "cl_guarded__String(bool c, ClPair__String a, ClString x, ClString y, bool g)\n{",
    );
    assert_on_line_of(tag, body, "if ((g && ((", "(__cl_tmp1 = a.v, (a.v = NULL))");
    assert_contains(tag, body, "(__cl_tmp1 = NULL)");
}

/// A generic return projection and its concrete spelling.
const GENERIC_RETURN_SOURCE: &str = r#"module spec.cpr.genericreturn;
import { String, stringOf } from std.text;
struct Pair<T> { v: T; w: T; }
func mkPair<T>(take x: T, take y: T): Pair<T> { return Pair<T> { v: x, w: y }; }
func pick<T>(c: bool, take a: Pair<T>, take x: T, take y: T): T {
    return (c ? a : mkPair<T>(x, y)).v;
}

func main(): i32 {
    let p: Pair<String> = Pair<String> { v: stringOf("ab"), w: stringOf("cd") };
    let a: Pair<i32> = Pair<i32> { v: 1, w: 2 };
    return i32(len(pick<String>(true, p, stringOf("ij"), stringOf("kl"))))
         + pick<i32>(true, a, 5, 6);
}
"#;

/// The generic return moves the chosen member per instance.
///
/// The `String` instance copies the chosen member into the return slot and
/// empties the arm that produced it — the place arm's field and the fresh
/// root's selected field alike — so the root releases only its un-moved field;
/// the `i32` instance copies and keeps its arm's value.
#[test]
fn should_move_a_generic_return_projection_per_instance() {
    let tag = "genericreturn";
    let (_scratch, emitted) = emit_host(tag, GENERIC_RETURN_SOURCE);
    let body = function_body(
        tag,
        &emitted,
        "cl_pick__String(bool c, ClPair__String a, ClString x, ClString y)\n{",
    );
    assert_contains(tag, body, "ClString __cl_tmp2 = NULL;");
    assert_follows(tag, body, "__cl_tmp2 = a.v;", "a.v = NULL;");
    assert_follows(tag, body, "__cl_tmp3 = __cl_tmp0.v;", "__cl_tmp0.v = NULL;");

    let body = function_body(
        tag,
        &emitted,
        "cl_pick__i32(bool c, ClPair__i32 a, i32 x, i32 y)\n{",
    );
    assert_absent(tag, body, "a.v = NULL;");
}

/// The three nested conditional take spellings.
const NESTED_THEN_SOURCE: &str = r#"module spec.cpr.nestedthen;
import { String, stringOf } from std.text;
struct Box { v: String; w: String; }
func mkBox(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }
func consume(take s: String): i32 { return i32(len(s)); }
func use(c: bool, d: bool, take a: Box, take b: Box, g: bool): i32 {
    if (g && consume((c ? (d ? a : mkBox()) : b).v) > 0) {
        return 1;
    }
    return 0;
}

func main(): i32 {
    return 0;
}
"#;

const NESTED_ELSE_SOURCE: &str = r#"module spec.cpr.nestedelse;
import { String, stringOf } from std.text;
struct Box { v: String; w: String; }
func mkBox(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }
func consume(take s: String): i32 { return i32(len(s)); }
func use(c: bool, d: bool, take a: Box, take b: Box, g: bool): i32 {
    if (g && consume((c ? b : (d ? a : mkBox())).v) > 0) {
        return 1;
    }
    return 0;
}

func main(): i32 {
    return 0;
}
"#;

const NESTED_BOTH_SOURCE: &str = r#"module spec.cpr.nestedboth;
import { String, stringOf } from std.text;
struct Box { v: String; w: String; }
func mkBox(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }
func mkBox2(): Box { return Box { v: stringOf("ef"), w: stringOf("gh") }; }
func consume(take s: String): i32 { return i32(len(s)); }
func use(c: bool, d: bool, take a: Box, take b: Box, g: bool): i32 {
    if (g && consume((c ? (d ? mkBox() : mkBox2()) : b).v) > 0) {
        return 1;
    }
    return 0;
}

func main(): i32 {
    return 0;
}
"#;

/// A nested conditional take empties the chosen member at every level.
///
/// Each guard line moves the leaf the inner condition selects into the shared
/// carrier and empties that leaf's member; when only the outer level moved, the
/// inner level's root kept and released the transferred storage again.
#[test]
fn should_move_a_nested_conditional_take_at_every_level() {
    let tag = "nestedthen";
    let (_scratch, emitted) = emit_host(tag, NESTED_THEN_SOURCE);
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cpr_nestedthen_use(bool c, bool d, ClBox a, ClBox b, bool g)\n{",
    );
    assert_on_line_of(tag, body, "if ((g && ((", "(d ? ((__cl_tmp1 = a.v, (a.v = NULL)))");
    assert_on_line_of(tag, body, "if ((g && ((", ": ((__cl_tmp1 = b.v, (b.v = NULL)))");

    let tag = "nestedelse";
    let (_scratch, emitted) = emit_host(tag, NESTED_ELSE_SOURCE);
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cpr_nestedelse_use(bool c, bool d, ClBox a, ClBox b, bool g)\n{",
    );
    assert_on_line_of(tag, body, "if ((g && ((", "(c ? ((__cl_tmp1 = b.v, (b.v = NULL)))");
    assert_on_line_of(tag, body, "if ((g && ((", "(d ? ((__cl_tmp1 = a.v, (a.v = NULL)))");

    let tag = "nestedboth";
    let (_scratch, emitted) = emit_host(tag, NESTED_BOTH_SOURCE);
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cpr_nestedboth_use(bool c, bool d, ClBox a, ClBox b, bool g)\n{",
    );
    assert_on_line_of(tag, body, "if ((g && ((", "__cl_tmp0.v), (__cl_tmp0.v = NULL)");
    assert_on_line_of(tag, body, "if ((g && ((", "__cl_tmp1.v), (__cl_tmp1.v = NULL)");
    assert_on_line_of(tag, body, "if ((g && ((", ": ((__cl_tmp2 = b.v, (b.v = NULL)))");
}

/// A conditional projection in a callable receiver's field, both spellings.
const FIELD_COND_GUARDED_SOURCE: &str = r#"module spec.cpr.fieldcondguarded;
import { String, stringOf } from std.text;
struct Box { v: String; w: String; }
struct H { call: func(i32): i32; s: String; }
func mkBox(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }
func addOne(x: i32): i32 { return x + 1; }
func use(c: bool, take a: Box, g: bool): i32 {
    if (g && H { call: addOne, s: (c ? a : mkBox()).v }.call(5) > 0) {
        return 1;
    }
    return 0;
}

func main(): i32 {
    return 0;
}
"#;

const FIELD_COND_EAGER_SOURCE: &str = r#"module spec.cpr.fieldcondeager;
import { String, stringOf } from std.text;
struct Box { v: String; w: String; }
struct H { call: func(i32): i32; s: String; }
func mkBox(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }
func addOne(x: i32): i32 { return x + 1; }
func use(c: bool, take a: Box): i32 {
    let r: i32 = H { call: addOne, s: (c ? a : mkBox()).v }.call(5);
    return r;
}

func main(): i32 {
    return 0;
}
"#;

/// A receiver field takes the chosen member from its arm's carrier.
///
/// The guarded spelling fills the carrier inside the guard line, moves each
/// arm's member out, and builds the receiver from the carrier; the eager
/// spelling fills one temporary per arm and stores the chosen one into the
/// field, emptying that temporary.
#[test]
fn should_move_a_conditional_projection_into_a_receiver_field() {
    let tag = "fieldcondguarded";
    let (_scratch, emitted) = emit_host(tag, FIELD_COND_GUARDED_SOURCE);
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cpr_fieldcondguarded_use(bool c, ClBox a, bool g)\n{",
    );
    assert_on_line_of(tag, body, "if ((g && ((", "(__cl_tmp1 = a.v, (a.v = NULL))");
    assert_contains(
        tag,
        body,
        "(ClH){ cl_spec_cpr_fieldcondguarded_addOne, __cl_tmp1 }",
    );

    let tag = "fieldcondeager";
    let (_scratch, emitted) = emit_host(tag, FIELD_COND_EAGER_SOURCE);
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cpr_fieldcondeager_use(bool c, ClBox a)\n{",
    );
    assert_follows(tag, body, "__cl_tmp1 = a.v;", "a.v = NULL;");
    assert_follows(tag, body, "__cl_tmp3 = __cl_tmp2.v;", "__cl_tmp2.v = NULL;");
    assert_follows(tag, body, "__cl_owned0.s = __cl_tmp1;", "__cl_tmp1 = NULL;");
    assert_follows(tag, body, "__cl_owned0.s = __cl_tmp3;", "__cl_tmp3 = NULL;");
}

/// A conditional of projections in a plain literal field, an array element, an
/// eager take argument, and an owning return.
const FIELD_COND_PLAIN_SOURCE: &str = r#"module spec.cpr.fieldcondplain;
import { String, stringOf } from std.text;
struct Box { v: String; w: String; }
struct Out { v: String; w: String; }
func mkBox(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }
func use(c: bool, take a: Box): i32 {
    let o: Out = Out { v: c ? a.v : mkBox().v, w: stringOf("z") };
    return i32(len(o.v)) - 2;
}

func main(): i32 {
    return 0;
}
"#;

const ELEMENT_COND_SOURCE: &str = r#"module spec.cpr.elementcond;
import { String, stringOf } from std.text;
struct Box { v: String; w: String; }
func mkBox(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }
func use(c: bool, take a: Box): i32 {
    let arr: String[2] = [c ? a.v : mkBox().v, stringOf("z")];
    return i32(len(arr[0])) - 2;
}

func main(): i32 {
    return 0;
}
"#;

const TAKE_COND_SOURCE: &str = r#"module spec.cpr.takecond;
import { String, stringOf } from std.text;
struct Box { v: String; w: String; }
func mkBox(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }
func consume(take s: String): i32 { return i32(len(s)); }
func use(c: bool, take a: Box): i32 {
    return consume(c ? a.v : mkBox().v);
}

func main(): i32 {
    return 0;
}
"#;

const RETURN_COND_SOURCE: &str = r#"module spec.cpr.returncond;
import { String, stringOf } from std.text;
struct Box { v: String; w: String; }
func mkBox(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }
func use(c: bool, take a: Box): String {
    return c ? a.v : mkBox().v;
}

func main(): i32 {
    return 0;
}
"#;

/// Every remaining consuming position binds the fresh arm's root.
///
/// The literal field stores the chosen member into its field, the array
/// element stores it into the element, the eager take moves it into the
/// argument temporary, and the return moves it into the return slot; each
/// position also releases the bound root's un-moved field.
#[test]
fn should_bind_the_conditional_field_and_element_roots() {
    let tag = "fieldcondplain";
    let (_scratch, emitted) = emit_host(tag, FIELD_COND_PLAIN_SOURCE);
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cpr_fieldcondplain_use(bool c, ClBox a)\n{",
    );
    assert_contains(tag, body, "__cl_tmp1 = cl_spec_cpr_fieldcondplain_mkBox();");
    assert_store_then_zero(tag, body, "o.v = __cl_tmp1.v;", "__cl_tmp1.v = NULL;");
    assert_contains(tag, body, "if (__cl_tmp1.w) { FreeString(__cl_tmp1.w); }");

    let tag = "elementcond";
    let (_scratch, emitted) = emit_host(tag, ELEMENT_COND_SOURCE);
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cpr_elementcond_use(bool c, ClBox a)\n{",
    );
    assert_contains(tag, body, "__cl_tmp1 = cl_spec_cpr_elementcond_mkBox();");
    assert_store_then_zero(tag, body, "__cl_rhs_1[0] = __cl_tmp1.v;", "__cl_tmp1.v = NULL;");
    assert_contains(tag, body, "if (__cl_tmp1.w) { FreeString(__cl_tmp1.w); }");

    let tag = "takecond";
    let (_scratch, emitted) = emit_host(tag, TAKE_COND_SOURCE);
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cpr_takecond_use(bool c, ClBox a)\n{",
    );
    assert_contains(tag, body, "__cl_tmp1 = cl_spec_cpr_takecond_mkBox();");
    assert_store_then_zero(tag, body, "__cl_tmp2 = __cl_tmp1.v;", "__cl_tmp1.v = NULL;");
    assert_contains(tag, body, "if (__cl_tmp1.w) { FreeString(__cl_tmp1.w); }");

    let tag = "returncond";
    let (_scratch, emitted) = emit_host(tag, RETURN_COND_SOURCE);
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cpr_returncond_use(bool c, ClBox a)\n{",
    );
    assert_contains(tag, body, "__cl_tmp1 = cl_spec_cpr_returncond_mkBox();");
    assert_store_then_zero(tag, body, "__cl_ret = __cl_tmp1.v;", "__cl_tmp1.v = NULL;");
    assert_contains(tag, body, "if (__cl_tmp1.w) { FreeString(__cl_tmp1.w); }");
}

/// A deferred bare conditional of projections in a guarded field and a `?:`
/// arm element.
const GUARDED_FIELD_COND_SOURCE: &str = r#"module spec.cpr.guardedfieldcond;
import { String, stringOf } from std.text;
struct Box { v: String; w: String; }
struct H { call: func(i32): i32; s: String; }
func mkBox(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }
func addOne(x: i32): i32 { return x + 1; }
func use(c: bool, take a: Box, g: bool): i32 {
    if (g && H { call: addOne, s: c ? a.v : mkBox().v }.call(5) > 0) {
        return 1;
    }
    return 0;
}

func main(): i32 {
    return 0;
}
"#;

const ARM_ELEM_COND_SOURCE: &str = r#"module spec.cpr.armelemcond;
import { String, stringOf } from std.text;
struct Box { v: String; w: String; }
func mkBox(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }
func lenElem(names: String[2]): i32 { return i32(len(names[0])) - 2; }
func use(c: bool, take a: Box, g: bool): i32 {
    return g ? lenElem([c ? a.v : mkBox().v, stringOf("z")]) : 0;
}

func main(): i32 {
    return 0;
}
"#;

/// A deferred bare conditional fills an adopting carrier on its evaluated path.
///
/// The guarded receiver field fills the carrier inside its guard line and
/// builds the receiver from it; the `?:` arm element fills the carrier in the
/// branch that evaluates the arm and copies it into the element. Each fill
/// moves the chosen member and empties its source, and the fresh arm's root
/// releases only its un-moved field.
#[test]
fn should_fill_the_deferred_conditional_carriers() {
    let tag = "guardedfieldcond";
    let (_scratch, emitted) = emit_host(tag, GUARDED_FIELD_COND_SOURCE);
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cpr_guardedfieldcond_use(bool c, ClBox a, bool g)\n{",
    );
    assert_on_line_of(tag, body, "if ((g && ((", "(__cl_tmp1 = a.v, (a.v = NULL))");
    assert_contains(
        tag,
        body,
        "(ClH){ cl_spec_cpr_guardedfieldcond_addOne, __cl_tmp1 }",
    );
    assert_contains(tag, body, "__cl_tmp0.v = NULL");

    let tag = "armelemcond";
    let (_scratch, emitted) = emit_host(tag, ARM_ELEM_COND_SOURCE);
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cpr_armelemcond_use(bool c, ClBox a, bool g)\n{",
    );
    // The array argument's carrier fills only in the arm that evaluates the
    // call: the move of `a.v` and the element stores run inside the `?:`
    // expression, after the condition, so a skipped arm moves nothing.
    assert_on_line_of(tag, body, "__cl_ret = (g ?", "(__cl_tmp1 = a.v, (a.v = NULL))");
    // The fresh arm builds its root in place: the call fills the temporary
    // and the selected member moves out of it in the same arm expression.
    assert_contains(tag, body, "__cl_tmp0 = cl_spec_cpr_armelemcond_mkBox()");
    assert_contains(tag, body, "__cl_tmp1 = (((__cl_tmp0.v");
    assert_on_line_of(
        tag,
        body,
        "__cl_owned0[0] = __cl_tmp1",
        "(__cl_tmp1 = NULL)",
    );
}

/// An indexed conditional field of a duplicable value.
const DUPLICABLE_INDEX_FIELD_SOURCE: &str = r#"module spec.cpr.dupindexfield;
struct Out { f: i32; w: i32; }
func use(c: bool, a: i32[2], b: i32[2]): i32 {
    let o: Out = Out { f: c ? a[1] : b[0], w: 1 };
    return o.f - 2;
}

func main(): i32 {
    return 0;
}
"#;

/// A duplicable indexed conditional field keeps the inline ternary spelling.
///
/// The value copies, so the field needs no root binding and no per-arm
/// consume: the condition is not saved into a temporary, no empty branch
/// wraps the store, and the field is written from the ternary directly.
#[test]
fn should_keep_the_inline_ternary_for_a_duplicable_index_field() {
    let tag = "dupindexfield";
    let (_scratch, emitted) = emit_host(tag, DUPLICABLE_INDEX_FIELD_SOURCE);
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cpr_dupindexfield_use(bool c, i32 a[2], i32 b[2])\n{",
    );
    assert_contains(tag, body, "o = (ClOut){ (c ? a[1] : b[0]), 1 };");
    assert_absent(tag, body, "if (__cl_tmp0) {");
}

/// A bound literal with an owned sibling field source.
const LITERAL_FIELD_MOVE_SOURCE: &str = r#"module spec.cpr.literalfieldmove;
import { String, stringOf } from std.text;
struct Box { v: String; w: String; }
struct Out2 { v: String; w: Box; }
struct Out { v: String; w: String; }
func consume(take s: String): i32 { return i32(len(s)); }
func eager(c: bool, take a: Box, take b: Box): i32 {
    return consume(c ? a.v : Out2 { v: stringOf("zz"), w: b }.v) - 2;
}
func store(c: bool, take a: Box, take b: Box): i32 {
    let o: Out = Out { v: c ? a.v : Out2 { v: stringOf("zz"), w: b }.v, w: stringOf("q") };
    return i32(len(o.v)) - 2;
}
func guarded(c: bool, take a: Box, take b: Box, g: bool): i32 {
    if (g && consume(c ? a.v : Out2 { v: stringOf("zz"), w: b }.v) > 0) {
        return 1;
    }
    return 0;
}

func main(): i32 {
    return 0;
}
"#;

/// Binding a literal arm moves its owned field source, never copies it.
///
/// The eager and guarded spellings fill the literal's frame field from the
/// source temporary and empty both the source and the temporary, so the
/// literal's release owns the moved value and the source's own cleanup finds
/// nothing. A brace-list copy would leave the source and the literal both
/// owning the same storage.
#[test]
fn should_move_a_bound_literals_owned_field_source() {
    let tag = "literalfieldmove";
    let (_scratch, emitted) = emit_host(tag, LITERAL_FIELD_MOVE_SOURCE);
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cpr_literalfieldmove_eager(bool c, ClBox a, ClBox b)\n{",
    );
    assert_follows(tag, body, "__cl_tmp1 = b;", "cl_zero(&b);");
    assert_follows(tag, body, "__cl_tmp2.w = __cl_tmp1;", "cl_zero(&__cl_tmp1);");

    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cpr_literalfieldmove_guarded(bool c, ClBox a, ClBox b, bool g)\n{",
    );
    assert_contains(tag, body, "(__cl_tmp0 = b), (cl_zero(&b))");
}

/// A nested literal field and an array-literal field element holding places.
const NESTED_LITERAL_MOVE_SOURCE: &str = r#"module spec.cpr.nestedliteralmove;
import { String, stringOf } from std.text;
struct Box { v: String; w: String; }
struct Inner { v: String; w: String; }
struct Outer { inner: Inner; tag: String; }
struct OutA { v: String; w: String[2]; }
struct Out { v: String; w: String; }
func consume(take s: String): i32 { return i32(len(s)); }
func guarded(c: bool, take a: Box, take b: Box, g: bool): i32 {
    if (g && consume(c ? a.v : Outer { inner: Inner { v: stringOf("zz"), w: b.w }, tag: stringOf("t") }.inner.v) > 0) {
        return 1;
    }
    return 0;
}
func eagerField(c: bool, take a: Box, take b: Box): i32 {
    let o: Out = Out { v: c ? a.v : Outer { inner: Inner { v: stringOf("zz"), w: b.w }, tag: stringOf("t") }.inner.v, w: stringOf("q") };
    return i32(len(o.v)) - 2;
}
func arrayField(c: bool, take a: Box, take b: Box, g: bool): i32 {
    if (g && consume(c ? a.v : OutA { v: stringOf("zz"), w: [b.w, stringOf("q")] }.v) > 0) {
        return 1;
    }
    return 0;
}

func main(): i32 {
    return 0;
}
"#;

/// Binding a literal moves a place reached inside a nested field or element.
///
/// The nested literal's `w` field moves `b.w` before the outer literal
/// materializes, and the array field's first element moves `b.w` the same
/// way: each source empties at its own depth, so the literal holds the only
/// reference and the source's cleanup finds nothing.
#[test]
fn should_move_a_nested_literals_field_sources() {
    let tag = "nestedliteralmove";
    let (_scratch, emitted) = emit_host(tag, NESTED_LITERAL_MOVE_SOURCE);
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cpr_nestedliteralmove_guarded(bool c, ClBox a, ClBox b, bool g)\n{",
    );
    assert_on_line_of(tag, body, "if ((g && ((", "(__cl_tmp0 = b.w), (b.w = NULL)");
    assert_contains(
        tag,
        body,
        "(ClInner){ cl_std_text_stringOf(cl_str(\"zz\")), __cl_tmp0 }",
    );

    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cpr_nestedliteralmove_eagerField(bool c, ClBox a, ClBox b)\n{",
    );
    assert_follows(tag, body, "__cl_tmp1 = b.w;", "b.w = NULL;");
    assert_follows(tag, body, "__cl_tmp2.inner.w = __cl_tmp1;", "__cl_tmp1 = NULL;");

    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cpr_nestedliteralmove_arrayField(bool c, ClBox a, ClBox b, bool g)\n{",
    );
    assert_on_line_of(tag, body, "if ((g && ((", "(__cl_tmp0 = b.w), (b.w = NULL)");
    assert_contains(tag, body, "{ __cl_tmp0, cl_std_text_stringOf(cl_str(\"q\")) }");
}

// ---------------------------------------------------------------------------
// The call-wrapped owner walk, the view-element argument and field positions,
// the `str[1]` return, and the mixed-return one-report rule. A rejected
// program pins its front-end judgment; an accepted program pins the emitted
// adoption shape and re-emits byte-identically.
// ---------------------------------------------------------------------------

/// A call-wrapped conditional view used while both parameter backings live.
///
/// The accepted safe twin keeps the call inline over both arms; only a write
/// to an arm's backing while the view is live reports.
const WRAP_SAFE_SOURCE: &str = r#"module spec.cpr.wrapsafe;

import { String, stringOf, bytes } from std.text;

struct Box { v: String; w: String; }

func f(borrow b: Box, borrow d: Box, c: bool): i32 {
    let s: u8[] = bytes((c ? b : d).v);
    return i32(s[0]);
}

func main(): i32 {
    let b: Box = Box { v: stringOf("ab"), w: stringOf("cd") };
    let d: Box = Box { v: stringOf("ef"), w: stringOf("gh") };
    return f(b, d, true);
}
"#;

/// A call-wrapped conditional view whose first arm's backing is written.
const OWNER_FIRST_SOURCE: &str = r#"module spec.cpr.ownerfirst;

import { String, stringOf, bytes } from std.text;

struct Box { v: String; w: String; }

func f(borrow b: Box, borrow d: Box, c: bool): i32 {
    let s: u8[] = bytes((c ? b : d).v);
    b.v = stringOf("zz");
    return i32(s[0]);
}

func main(): i32 {
    let b: Box = Box { v: stringOf("ab"), w: stringOf("cd") };
    let d: Box = Box { v: stringOf("ef"), w: stringOf("gh") };
    return f(b, d, true);
}
"#;

/// The two-arm spelling `bytes(c ? b.v : d.v)` whose second arm is written.
const OWNER_TWOARM_SOURCE: &str = r#"module spec.cpr.ownertwoarm;

import { String, stringOf, bytes } from std.text;

struct Box { v: String; w: String; }

func f(borrow b: Box, borrow d: Box, c: bool): i32 {
    let s: u8[] = bytes(c ? b.v : d.v);
    d.v = stringOf("zz");
    return i32(s[0]);
}

func main(): i32 {
    let b: Box = Box { v: stringOf("ab"), w: stringOf("cd") };
    let d: Box = Box { v: stringOf("ef"), w: stringOf("gh") };
    return f(b, d, false);
}
"#;

/// A chained view-returning callee whose second arm's backing is written.
const OWNER_CHAINED_SOURCE: &str = r#"module spec.cpr.ownerchained;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func peek(x: str): str { return x; }

func f(borrow b: Box, borrow d: Box, c: bool): i32 {
    let s: str = peek((c ? b : d).v);
    d.v = stringOf("zz");
    return i32(s[0]);
}

func main(): i32 {
    let b: Box = Box { v: stringOf("ab"), w: stringOf("cd") };
    let d: Box = Box { v: stringOf("ef"), w: stringOf("gh") };
    return f(b, d, false);
}
"#;

/// A view-element array passed as an argument.
const ELEM_ARG_SOURCE: &str = r#"module spec.cpr.elemarg;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func mk(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }

func use(names: str[1]): i32 { return i32(len(names[0])); }

func main(): i32 {
    return use([mk().v]) - 2;
}
"#;

/// A view-element array stored into a struct-literal field.
const ELEM_FIELD_SOURCE: &str = r#"module spec.cpr.elemfield;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Holder { names: str[1]; }

func mk(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }

func main(): i32 {
    let h: Holder = Holder { names: [mk().v] };
    return i32(len(h.names[0])) - 2;
}
"#;

/// A `str[1]` return whose element derives from a by-value `String` parameter.
const ARRAY_RETURN_SOURCE: &str = r#"module spec.cpr.arrayreturn;

import { String, stringOf } from std.text;

func f(s: String): str[1] { return [s]; }

func main(): i32 {
    let s: String = stringOf("ab");
    let a: str[1] = f(s);
    return i32(len(a[0])) - 2;
}
"#;

/// The same `str[1]` return deriving from a parameter member.
const ARRAY_RETURN_MEMBER_SOURCE: &str = r#"module spec.cpr.arrayreturnmember;

import { String } from std.text;

struct Box { v: String; }

func f(h: Box): str[1] { return [h.v]; }

func main(): i32 { return 0; }
"#;

/// A mixed conditional view return over a local and a fresh root.
const ONE_REPORT_MIXED_SOURCE: &str = r#"module spec.cpr.onereportmixed;

import { String, stringOf } from std.text;

struct Box { v: String; }

func mkBox(): Box { return Box { v: stringOf("ab") }; }

func f(c: bool): str {
    let a: Box = Box { v: stringOf("cd") };
    return (c ? a : mkBox()).v;
}

func main(): i32 { return 0; }
"#;

/// A conditional view return over a local and a parameter.
const ONE_REPORT_LOCALPARAM_SOURCE: &str = r#"module spec.cpr.onereportlocalparam;

import { String, stringOf } from std.text;

struct Box { v: String; }

func f(c: bool, a: str): str {
    let b: Box = Box { v: stringOf("cd") };
    return (c ? b.v : a);
}

func main(): i32 { return 0; }
"#;

/// A conditional view return over two locals.
const ONE_REPORT_LOCALONLY_SOURCE: &str = r#"module spec.cpr.onereportlocalonly;

import { String, stringOf } from std.text;

struct Box { v: String; }

func f(c: bool): str {
    let a: Box = Box { v: stringOf("cd") };
    let b: Box = Box { v: stringOf("ef") };
    return (c ? a : b).v;
}

func main(): i32 { return 0; }
"#;

/// A write to either arm's backing while a call-wrapped conditional view is
/// live reports the plain twin's mutation error exactly once.
///
/// `bytes(b.v)` reports one `E10031` at the write because the view loans `b`.
/// The call-wrapped conditional must record every arm's owner, so each of the
/// three programs reports exactly that one error at its mutation and emits no
/// translation unit; the accepted safe twin keeps its inline call over both
/// arms.
#[test]
fn should_reject_a_call_wrapped_conditional_view_mutation() {
    let tag = "wrapsafe";
    let (_scratch, emitted) = emit_host(tag, WRAP_SAFE_SOURCE);
    let body = function_body(tag, &emitted, "cl_spec_cpr_wrapsafe_f(ClBox *b, ClBox *d, bool c)\n{");
    assert_contains(tag, body, "s = cl_std_text_bytes((c ? (*b) : (*d)).v);");

    assert_rejected("ownerfirst", OWNER_FIRST_SOURCE, "E10031", 1);
    assert_rejected("ownertwoarm", OWNER_TWOARM_SOURCE, "E10031", 1);
    assert_rejected("ownerchained", OWNER_CHAINED_SOURCE, "E10031", 1);
}

/// A view-element array in an argument or field position adopts its temporary
/// element's backing in the destination frame.
///
/// The fresh `mk()` root binds as one owned local, the element view is built
/// from that local's member, and the local's fields release at the frame's
/// end, so no moved-string temporary is left unreleased.
#[test]
fn should_adopt_the_backing_of_a_view_element_array_in_argument_and_field_positions() {
    let tag = "elemarg";
    let (_scratch, emitted) = emit_host(tag, ELEM_ARG_SOURCE);
    let body = function_body(tag, &emitted, "cl_spec_cpr_elemarg_main(void)\n{");
    assert_contains(tag, body, "ClBox __cl_owned0 = {0};");
    assert_contains(tag, body, "__cl_owned0 = cl_spec_cpr_elemarg_mk();");
    assert_contains(tag, body, "__cl_owned1[0] = cl_string_view(__cl_owned0.v);");
    assert_release_after(
        tag,
        body,
        "__cl_owned1[0] = cl_string_view(__cl_owned0.v);",
        "if (__cl_owned0.w) { FreeString(__cl_owned0.w); }",
    );
    assert_absent(tag, body, "ClString __cl_tmp1 = NULL;");

    let tag = "elemfield";
    let (_scratch, emitted) = emit_host(tag, ELEM_FIELD_SOURCE);
    let body = function_body(tag, &emitted, "cl_spec_cpr_elemfield_main(void)\n{");
    assert_contains(tag, body, "ClBox __cl_owned0 = {0};");
    assert_contains(tag, body, "__cl_owned0 = cl_spec_cpr_elemfield_mk();");
    assert_contains(tag, body, "h.names[0] = cl_string_view(__cl_owned0.v);");
    assert_release_after(
        tag,
        body,
        "h.names[0] = cl_string_view(__cl_owned0.v);",
        "if (__cl_owned0.w) { FreeString(__cl_owned0.w); }",
    );
    assert_absent(tag, body, "ClString __cl_tmp1 = NULL;");
}

/// A `str[1]` return matches its single-view twin: the accepted parameter
/// spelling returns the view without consuming the source, and the member
/// spelling reports the twin's single `E10036`.
#[test]
fn should_return_a_view_element_array_without_consuming_its_source() {
    let tag = "arrayreturn";
    let (_scratch, emitted) = emit_host(tag, ARRAY_RETURN_SOURCE);
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cpr_arrayreturn_f(ClString s, ClStr __cl_ret[1])\n{",
    );
    assert_contains(tag, body, "__cl_ret[0] = cl_string_view(s);");
    assert_absent(tag, body, "s = NULL;");
    assert_absent(tag, body, "FreeString(s)");

    assert_rejected("arrayreturnmember", ARRAY_RETURN_MEMBER_SOURCE, "E10036", 1);
}

/// A mixed local/generated view return reports exactly one error — the
/// generated-owner escape — while a local-only backing keeps one local-owner
/// report.
#[test]
fn should_report_the_mixed_view_return_once() {
    assert_rejected("onereportmixed", ONE_REPORT_MIXED_SOURCE, "E10030", 1);
    assert_rejected("onereportlocalparam", ONE_REPORT_LOCALPARAM_SOURCE, "E10035", 1);
    assert_rejected("onereportlocalonly", ONE_REPORT_LOCALONLY_SOURCE, "E10035", 1);
}

/// The new view-element programs re-emit byte-identically.
#[test]
fn should_re_emit_the_new_view_element_programs_byte_identically() {
    let cases: [(&str, &str); 4] = [
        ("wrapsafe", WRAP_SAFE_SOURCE),
        ("elemarg", ELEM_ARG_SOURCE),
        ("elemfield", ELEM_FIELD_SOURCE),
        ("arrayreturn", ARRAY_RETURN_SOURCE),
    ];
    for (tag, source) in cases {
        let (first, second) = emit_twice(tag, source);
        assert!(
            first == second,
            "{tag}: the two emissions differ; first:\n{first}\nsecond:\n{second}"
        );
    }
}

// ---------------------------------------------------------------------------
// The escaping-store judgment: a view stored where its backing dies first is
// rejected per destination kind, and the accepted spellings store view arrays
// element by element. A global or a field accepts only module-level storage,
// a writable parameter slot accepts module-level and parameter storage, and a
// local container element records the backing's loans so the liveness rule
// judges a backing whose scope ends first.
// ---------------------------------------------------------------------------

/// A fresh view stored into a global binding is rejected at the store.
const STORE_GLOBAL_SOURCE: &str = r#"module spec.cprimpl.storeglobal;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

let names: str = "start";

func mk(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }

func store(): void { names = mk().v; }

func main(): i32 {
    store();
    return i32(len(names)) - 2;
}
"#;

/// A fresh view element stored into a global view array is rejected.
const STORE_GLOBAL_ARRAY_SOURCE: &str = r#"module spec.cprimpl.storeglobalarray;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

let names: str[1] = ["start"];

func mk(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }

func store(): void { names = [mk().v]; }

func main(): i32 {
    store();
    return i32(len(names[0])) - 2;
}
"#;

/// A parameter-backed view stored into a global binding is rejected too: a
/// parameter does not outlive the global.
const STORE_GLOBAL_PARAM_SOURCE: &str = r#"module spec.cprimpl.storeglobalparam;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

let names: str = "start";

func store(p: Box): void { names = p.v; }

func main(): i32 {
    let b: Box = Box { v: stringOf("ab"), w: stringOf("cd") };
    store(b);
    return i32(len(names)) - 2;
}
"#;

/// A fresh view element stored into a global struct field is rejected.
const STORE_FIELD_SOURCE: &str = r#"module spec.cprimpl.storefield;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Holder { names: str[1]; }

let g: Holder = Holder { names: ["AA"] };

func mk(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }

func store(): void { g.names = [mk().v]; }

func main(): i32 {
    store();
    return i32(len(g.names[0])) - 2;
}
"#;

/// A local `String` member widened into a global `str?` is rejected.
const STORE_WIDE_GLOBAL_SOURCE: &str = r#"module spec.cprimpl.storewideglobal;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

let names: str? = null;

func store(): void {
    let b: Box = Box { v: stringOf("ab"), w: stringOf("cd") };
    names = b.v;
}

func main(): i32 {
    store();
    if (names == null) {
        return 9;
    }
    return i32(names[0]) - 97;
}
"#;

/// A fresh view stored through a writable parameter slot is rejected.
const STORE_SLOT_FRESH_SOURCE: &str = r#"module spec.cprimpl.storeslotfresh;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func mk(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }

func fill(borrow slot: str): void { slot = mk().v; }

func main(): i32 {
    let out: str = "aa";
    fill(borrow out);
    return i32(len(out)) - 2;
}
"#;

/// A view of the callee's local stored through a slot is rejected.
const STORE_SLOT_LOCAL_SOURCE: &str = r#"module spec.cprimpl.storeslotlocal;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(borrow slot: str): void {
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    slot = b.v;
}

func main(): i32 {
    let out: str = "aa";
    fill(borrow out);
    return i32(len(out)) - 2;
}
"#;

/// A fresh view element stored through a view-array slot is rejected.
const STORE_SLOT_ARRAY_SOURCE: &str = r#"module spec.cprimpl.storeslotarray;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func mk(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }

func fill(borrow slot: str[1]): void { slot = [mk().v]; }

func main(): i32 {
    let out: str[1] = ["AA"];
    fill(borrow out);
    return i32(len(out[0])) - 2;
}
"#;

/// A local `String` member widened into a nullable parameter slot is rejected.
const STORE_WIDE_SLOT_SOURCE: &str = r#"module spec.cprimpl.storewideslot;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(borrow slot: str?): void {
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    slot = b.v;
}

func main(): i32 {
    let out: str? = null;
    fill(borrow out);
    return 0;
}
"#;

/// A loop-iteration backing stored into a local element is rejected.
const STORE_LOOP_ELEM_SOURCE: &str = r#"module spec.cprimpl.storeloopelem;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func mk(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }

func run(): i32 {
    let xs: str[2] = ["AA", "BB"];
    let i: i32 = 0;
    while (i < 3) {
        xs[1] = mk().v;
        i = i + 1;
    }
    return i32(xs[1][0]);
}

func main(): i32 {
    return run() - 97;
}
"#;

/// A nested-block backing stored into a local element is rejected.
const STORE_NESTED_ELEM_SOURCE: &str = r#"module spec.cprimpl.storenestedelem;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func run(): i32 {
    let xs: str[2] = ["AA", "BB"];
    {
        let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
        xs[1] = b.v;
    }
    return i32(xs[1][0]);
}

func main(): i32 {
    return run() - 120;
}
"#;

/// A module-level static view element stored into a global struct field.
const STORE_STATIC_SOURCE: &str = r#"module spec.cprimpl.storestatic;

let s: str = "st";

struct Holder { names: str[1]; }

let g: Holder = Holder { names: ["AA"] };

func store(): void { g.names = [s]; }

func main(): i32 {
    store();
    return i32(g.names[0][0]) - 115;
}
"#;

/// A lambda capturing a `str[1]` local.
const CAPTURE_VIEW_ARRAY_SOURCE: &str = r#"module spec.cprimpl.captureviewarray;

func main(): i32 {
    let names: str[1] = ["abc"];
    return ([names] () => i32(names[0][0]))() - 97;
}
"#;

/// A local element store whose backing is a same-frame local and a parameter.
const LOCAL_ELEM_SOURCE: &str = r#"module spec.cprimpl.localelem;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func pick(c: bool, a: str): i32 {
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    let xs: str[2] = ["AA", "BB"];
    xs[1] = c ? b.v : a;
    return i32(xs[1][0]);
}

func main(): i32 {
    return pick(true, "ppp") + pick(false, "qqqq") - 233;
}
"#;

/// A view stored where the destination outlives its backing reports one
/// escape error at the store.
///
/// A global binding and a global field keep reading the stored view for the
/// whole program, so the fresh derivations (`mk().v`, `[mk().v]`) and the
/// parameter-backed spelling each report one `E10033` at the store line; the
/// widened `String` member is judged by the same rule.
#[test]
fn should_reject_a_view_stored_into_a_global_or_a_field() {
    assert_rejected_at_line("storeglobal", STORE_GLOBAL_SOURCE, "E10033", 1, 11);
    assert_rejected_at_line(
        "storeglobalarray",
        STORE_GLOBAL_ARRAY_SOURCE,
        "E10033",
        1,
        11,
    );
    assert_rejected_at_line(
        "storeglobalparam",
        STORE_GLOBAL_PARAM_SOURCE,
        "E10033",
        1,
        9,
    );
    assert_rejected_at_line("storefield", STORE_FIELD_SOURCE, "E10033", 1, 12);
    assert_rejected_at_line(
        "storewideglobal",
        STORE_WIDE_GLOBAL_SOURCE,
        "E10033",
        1,
        11,
    );
}

/// A view stored through a writable parameter slot is judged against the
/// caller's frame and reports one loan-outlives error at the store.
///
/// The slot's storage is the caller's, so a fresh call result, the callee's
/// local, a fresh array element, and the widened local member each report one
/// `E10030`; the parameter-backed twin stays accepted in the spec suite.
#[test]
fn should_reject_a_view_stored_through_a_parameter_slot() {
    assert_rejected_at_line(
        "storeslotfresh",
        STORE_SLOT_FRESH_SOURCE,
        "E10030",
        1,
        9,
    );
    assert_rejected_at_line(
        "storeslotlocal",
        STORE_SLOT_LOCAL_SOURCE,
        "E10030",
        1,
        9,
    );
    assert_rejected_at_line(
        "storeslotarray",
        STORE_SLOT_ARRAY_SOURCE,
        "E10030",
        1,
        9,
    );
    assert_rejected_at_line(
        "storewideslot",
        STORE_WIDE_SLOT_SOURCE,
        "E10030",
        1,
        9,
    );
}

/// A backing whose scope ends before the array's reports one loan-outlives
/// error at the element store.
///
/// The loop iteration's call result and the nested block's local both die
/// before the array's later read, so each store reports one `E10030` at the
/// store line; the same-frame spellings stay accepted in the spec suite.
#[test]
fn should_reject_a_short_lived_backing_stored_into_a_local_array_element() {
    assert_rejected_at_line(
        "storeloopelem",
        STORE_LOOP_ELEM_SOURCE,
        "E10030",
        1,
        13,
    );
    assert_rejected_at_line(
        "storenestedelem",
        STORE_NESTED_ELEM_SOURCE,
        "E10030",
        1,
        11,
    );
}

/// A global field's view array stores element by element.
///
/// C cannot assign an array, and a view element has no initializer-list form,
/// so the module-level static element assigns into its own index instead of a
/// compound initializer the strict build would reject.
#[test]
fn should_store_a_global_field_view_element_by_element() {
    let tag = "storestatic";
    let (_scratch, emitted) = emit_host(tag, STORE_STATIC_SOURCE);
    let body = function_body(tag, &emitted, "cl_spec_cprimpl_storestatic_store(void)\n{");
    assert_contains(
        tag,
        body,
        ".names[0] = cl_spec_cprimpl_storestatic_s;",
    );
    assert_absent(tag, body, ".names = {");
}

/// A captured view array initializes its environment field element by
/// element.
///
/// The environment field carries the array's own C shape, so the capture
/// literal names each element instead of handing the array's pointer to the
/// field's first member.
#[test]
fn should_initialize_a_captured_view_array_element_by_element() {
    let tag = "captureviewarray";
    let (_scratch, emitted) = emit_host(tag, CAPTURE_VIEW_ARRAY_SOURCE);
    assert_contains(tag, &emitted, "(struct ClEnv_0){ { names[0] } }");
    assert_absent(tag, &emitted, "(struct ClEnv_0){ names }");
}

/// A local element store whose backing covers the array stays ownerless and
/// leaves the backing to its own scope cleanup.
///
/// The store reads the local member through the view conversion and binds no
/// generated owner: the local's cleanup already follows the element read, so
/// the array stays valid for its own last use.
#[test]
fn should_accept_a_local_view_element_store_borrow_checked() {
    let tag = "localelem";
    let (_scratch, emitted) = emit_host(tag, LOCAL_ELEM_SOURCE);
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_localelem_pick(bool c, ClStr a)\n{",
    );
    assert_contains(tag, body, "xs[1] = (c ? cl_string_view(b.v) : a);");
    assert_absent(tag, body, "__cl_owned");
    assert_release_after(
        tag,
        body,
        "xs[1] = (c ? cl_string_view(b.v) : a);",
        "if (b.v) { FreeString(b.v); }",
    );
}

/// The accepted store and capture programs re-emit byte-identically.
#[test]
fn should_re_emit_the_view_store_programs_byte_identically() {
    let cases: [(&str, &str); 3] = [
        ("storestatic", STORE_STATIC_SOURCE),
        ("captureviewarray", CAPTURE_VIEW_ARRAY_SOURCE),
        ("localelem", LOCAL_ELEM_SOURCE),
    ];
    for (tag, source) in cases {
        let (first, second) = emit_twice(tag, source);
        assert!(
            first == second,
            "{tag}: the two emissions differ; first:\n{first}\nsecond:\n{second}"
        );
    }
}

// ---------------------------------------------------------------------------
// The per-component judgment and the frame destinations: a store names one
// storage per arm or element, so each is judged on its own; a `for (borrow e
// in xs)` alias writes its iterable element; a `take` parameter owns this
// frame's storage; and a struct literal's view field shares its destination's
// frame.
// ---------------------------------------------------------------------------

/// A mixed-source slot store names a module view and a fresh call result.
const MERGE_FAIL_FRESH_SOURCE: &str = r#"module spec.cprimpl.mergefailfresh;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

let s1: str = "ab";

func mk(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }

func fill(c: bool, borrow slot: str): void { slot = c ? s1 : mk().v; }

func main(): i32 { return 0; }
"#;

/// A mixed-source global store names a module view and a local member.
const MERGE_FAIL_LOCAL_SOURCE: &str = r#"module spec.cprimpl.mergefaillocal;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Holder { names: str[1]; }

let s1: str = "ab";
let g: Holder = Holder { names: ["AA"] };

func store(c: bool): void {
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    g.names = [c ? s1 : b.v];
}

func main(): i32 { return 0; }
"#;

/// A view of the callee's `take` parameter stored into the caller's slot.
const TAKE_OUT_SOURCE: &str = r#"module spec.cprimpl.takeout;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(take p: Box, borrow slot: str?): void { slot = p.v; }

func main(): i32 { return 0; }
"#;

/// A per-iteration fresh backing written through the element alias.
const ALIAS_FRESH_SOURCE: &str = r#"module spec.cprimpl.aliasfresh;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func mk(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }

func run(): i32 {
    let xs: str[2] = ["AA", "BB"];
    for (borrow e in xs) {
        e = mk().v;
    }
    return i32(xs[1][0]);
}

func main(): i32 { return run() - 97; }
"#;

/// A view store through an immutable module binding.
const CONST_STORE_SOURCE: &str = r#"module spec.cprimpl.constwrite;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

const names: str = "start";

func store(): void {
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    names = b.v;
}

func main(): i32 { return 0; }
"#;

/// A whole-value view store through a read-only parameter.
const READONLY_STORE_SOURCE: &str = r#"module spec.cprimpl.readonlystore;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str): void {
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    p = b.v;
}

func main(): i32 { return 0; }
"#;

/// A same-frame backing written through the element alias.
const ALIAS_LOCAL_SOURCE: &str = r#"module spec.cprimpl.aliaslocal;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func run(): i32 {
    let xs: str[2] = ["AA", "BB"];
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    for (borrow e in xs) {
        e = b.v;
    }
    return i32(xs[1][0]);
}

func main(): i32 { return run() - 120; }
"#;

/// A frame-local struct literal and its field-assignment twin.
const LITERAL_PAIR_SOURCE: &str = r#"module spec.cprimpl.literalpair;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Holder { names: str[1]; }

func literal(): i32 {
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    let h: Holder = Holder { names: [b.v] };
    return i32(h.names[0][0]);
}

func assigned(): i32 {
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    let h: Holder = Holder { names: ["AA"] };
    h.names = [b.v];
    return i32(h.names[0][0]);
}

func main(): i32 {
    return literal() + assigned() - 240;
}
"#;

/// A same-frame backing stored into a `take` parameter.
const TAKE_FRAME_SOURCE: &str = r#"module spec.cprimpl.takeframe;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(take t: str): i32 {
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    t = b.v;
    return i32(t[0]);
}

func main(): i32 {
    let out: str = "aa";
    return fill(take out) - 120;
}
"#;

/// Each component of a mixed-source store is judged on its own, so the first
/// component the destination cannot keep names one report at the store.
///
/// The module view beside the fresh call result reports `E10030`; the module
/// view beside the local member reports `E10033`; the `take` parameter's
/// stored view reports `E10030`; and the element alias's per-iteration fresh
/// backing reports `E10030`. Each store keeps exactly one error.
#[test]
fn should_judge_each_storage_component_of_a_view_store() {
    assert_rejected_at_line(
        "mergefailfresh",
        MERGE_FAIL_FRESH_SOURCE,
        "E10030",
        1,
        11,
    );
    assert_rejected_at_line(
        "mergefaillocal",
        MERGE_FAIL_LOCAL_SOURCE,
        "E10033",
        1,
        13,
    );
    assert_rejected_at_line("takeout", TAKE_OUT_SOURCE, "E10030", 1, 7);
    assert_rejected_at_line("aliasfresh", ALIAS_FRESH_SOURCE, "E10030", 1, 12);
}

/// An already-rejected write keeps its own one report.
///
/// The immutable module binding reports `E10023` and the read-only parameter
/// reports `E10024`; the view-store judgment adds no second error at either
/// site.
#[test]
fn should_report_once_at_an_already_rejected_view_store() {
    assert_rejected_at_line("constwrite", CONST_STORE_SOURCE, "E10023", 1, 11);
    assert_rejected_at_line(
        "readonlystore",
        READONLY_STORE_SOURCE,
        "E10024",
        1,
        9,
    );
}

/// The alias store writes the iterable element through its address binding.
///
/// The element's address is refreshed at the top of every iteration and the
/// store reaches the iterable through it, so the container the alias names
/// receives the view value.
#[test]
fn should_write_a_for_alias_element_through_its_iterable_address() {
    let tag = "aliaslocal";
    let (_scratch, emitted) = emit_host(tag, ALIAS_LOCAL_SOURCE);
    assert_contains(tag, &emitted, "(*__cl_tmp1) = cl_string_view(b.v);");
    assert_contains(tag, &emitted, "__cl_tmp1 = &(xs[__cl_tmp0]);");
}

/// A struct literal's view field and its assignment twin store element by
/// element.
///
/// Both spellings read the local member through the view conversion and keep
/// the carrier's own C shape, so the field is filled element by element
/// exactly as the accepted field assignment fills it.
#[test]
fn should_store_a_frame_backed_literal_field_element_by_element() {
    let tag = "literalpair";
    let (_scratch, emitted) = emit_host(tag, LITERAL_PAIR_SOURCE);
    let stores = emitted
        .matches("h.names[0] = cl_string_view(b.v);")
        .count();
    assert_eq!(
        stores, 2,
        "{tag}: expected the literal and the assignment to store the same element shape; emitted:\n{emitted}"
    );
}

/// A same-frame backing stored into a `take` parameter fills the parameter
/// in place.
#[test]
fn should_store_a_same_frame_backing_into_a_take_parameter() {
    let tag = "takeframe";
    let (_scratch, emitted) = emit_host(tag, TAKE_FRAME_SOURCE);
    assert_contains(tag, &emitted, "t = cl_string_view(b.v);");
}

/// The alias, literal, and take-destination programs re-emit byte-identically.
#[test]
fn should_re_emit_the_frame_destination_programs_byte_identically() {
    let cases: [(&str, &str); 3] = [
        ("aliaslocal", ALIAS_LOCAL_SOURCE),
        ("literalpair", LITERAL_PAIR_SOURCE),
        ("takeframe", TAKE_FRAME_SOURCE),
    ];
    for (tag, source) in cases {
        let (first, second) = emit_twice(tag, source);
        assert!(
            first == second,
            "{tag}: the two emissions differ; first:\n{first}\nsecond:\n{second}"
        );
    }
}

// ---------------------------------------------------------------------------
// The guarded and discarded literal positions: a loop-condition literal keeps
// one owner for its owned field, a discarded literal materializes only its
// fresh fields, and a discarded projection arm binds its root.
// ---------------------------------------------------------------------------

/// A conditional literal receiver in a loop condition.
const LOOP_COND_WALK_SOURCE: &str = r#"module spec.cprimpl.loopcondwalk;

import { String, stringOf } from std.text;

struct H { call: func(i32): i32; s: String; }

func addOne(x: i32): i32 { return x + 1; }

func use(c: bool): i32 {
    let i: i32 = 0;
    let n: i32 = 0;
    while (i < 1 && (c ? H { call: addOne, s: stringOf("x") } : H { call: addOne, s: stringOf("y") }).call(5) > 0) {
        n = n + 1;
        i = i + 1;
    }
    return n;
}

func main(): i32 {
    return use(true) + use(false) - 2;
}
"#;

/// A discarded conditional whose literal mixes a fresh and a place field.
const DISCARD_PAIR_SOURCE: &str = r#"module spec.cprimpl.discardpair;

import { String, stringOf } from std.text;

struct Pair { a: String; b: String; }

func pair(ok: bool): i32 {
    let t: String = stringOf("tt");
    let u: String = stringOf("uu");
    ok ? Pair { a: stringOf("x"), b: t } : Pair { a: stringOf("y"), b: u };
    return i32(len(t)) + i32(len(u)) - 4;
}

func main(): i32 {
    return pair(true) + pair(false);
}
"#;

/// A discarded conditional whose arm projects out of a fresh root.
const DISCARD_ROOT_SOURCE: &str = r#"module spec.cprimpl.discardroot;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func mk(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }

func use(c: bool): i32 {
    let t: String = stringOf("tt");
    c ? mk().v : t;
    return i32(len(t));
}

func main(): i32 {
    return use(true) + use(false) - 4;
}
"#;

/// A loop-condition literal arm stores whole into its carrier.
///
/// The loop-condition walk materializes the conditional's chosen arm into the
/// carrier temporary, so the literal's `s` field must not also get its own
/// temporary: the carrier's field is the single owner and it releases at the
/// loop edge. The pin therefore expects the arm stored straight into the
/// carrier and no `__cl_tmp1` (the per-field temporary and its release).
#[test]
fn should_store_a_loop_condition_literal_arm_wholesale() {
    let tag = "loopcondwalk";
    let (_scratch, emitted) = emit_host(tag, LOOP_COND_WALK_SOURCE);
    let body = function_body(tag, &emitted, "cl_spec_cprimpl_loopcondwalk_use(bool c)\n{");
    assert_contains(
        tag,
        body,
        "__cl_tmp0 = (ClH){ cl_spec_cprimpl_loopcondwalk_addOne, cl_std_text_stringOf(cl_str(\"x\")) }",
    );
    assert_contains(
        tag,
        body,
        "__cl_tmp0 = (ClH){ cl_spec_cprimpl_loopcondwalk_addOne, cl_std_text_stringOf(cl_str(\"y\")) }",
    );
    assert_absent(tag, body, "__cl_tmp1");
    assert_release_after(
        tag,
        body,
        "break;",
        "if (__cl_tmp0.s) { FreeString(__cl_tmp0.s); }",
    );
}

/// A discarded literal materializes each fresh field and reads the place
/// fields.
///
/// `ok ? Pair { a: stringOf("x"), b: t } : …;` drops the selected literal, so
/// the fresh `a` field binds to its own statement temporary and releases when
/// the statement ends, while the `b` field reads `t` or `u` shallowly: the
/// discard performs no store, so the place bindings stay untouched until their
/// own scope cleanup.
#[test]
fn should_materialize_a_discarded_literal_field_per_fresh_owner() {
    let tag = "discardpair";
    let (_scratch, emitted) = emit_host(tag, DISCARD_PAIR_SOURCE);
    let body = function_body(tag, &emitted, "cl_spec_cprimpl_discardpair_pair(bool ok)\n{");
    assert_contains(
        tag,
        body,
        "__cl_tmp0 = cl_std_text_stringOf(cl_str(\"x\"))",
    );
    assert_contains(
        tag,
        body,
        "__cl_tmp1 = cl_std_text_stringOf(cl_str(\"y\"))",
    );
    assert_contains(tag, body, "if (ok) {");
    assert_contains(tag, body, "if (sizeof(t)) { }");
    assert_contains(tag, body, "if (sizeof(u)) { }");
    assert_release_after(
        tag,
        body,
        "if (ok) {",
        "if (__cl_tmp1) { FreeString(__cl_tmp1); }",
    );
    assert_release_after(
        tag,
        body,
        "if (ok) {",
        "if (__cl_tmp0) { FreeString(__cl_tmp0); }",
    );
    // The place bindings keep their own storage: the discard block releases
    // only the fresh field temporaries, and the statement's next computation
    // runs before the scope cleanup frees `t`/`u`.
    assert_contains(
        tag,
        body,
        "if (sizeof(t)) { }",
    );
    assert_contains(
        tag,
        body,
        "if (sizeof(u)) { }",
    );
    assert_contains(
        tag,
        body,
        "__cl_ret = (i32)((u32)(i32)((u32)(i32)((cl_string_view(t)).len)",
    );
}

/// A discarded projection arm binds its ownerless root for release.
///
/// `c ? mk().v : t;` reads a fresh root's member on one path; the statement
/// binds the root to a temporary and releases its fields when the statement
/// ends, while the projected member stays a read and the place arm stays
/// untouched.
#[test]
fn should_bind_a_discarded_projection_root_for_release() {
    let tag = "discardroot";
    let (_scratch, emitted) = emit_host(tag, DISCARD_ROOT_SOURCE);
    let body = function_body(tag, &emitted, "cl_spec_cprimpl_discardroot_use(bool c)\n{");
    assert_contains(tag, body, "__cl_tmp0 = cl_spec_cprimpl_discardroot_mk()");
    assert_on_line_of(tag, body, "(void)((c ?", "__cl_tmp0.v) : t)");
    assert_release_after(
        tag,
        body,
        "(void)((c ?",
        "if (__cl_tmp0.v) { FreeString(__cl_tmp0.v); }",
    );
    assert_release_after(
        tag,
        body,
        "(void)((c ?",
        "if (__cl_tmp0.w) { FreeString(__cl_tmp0.w); }",
    );
}

/// The discarded pair and root programs re-emit byte-identically.
#[test]
fn should_re_emit_the_guarded_and_discarded_programs_byte_identically() {
    let cases: [(&str, &str); 2] = [
        ("discardpair", DISCARD_PAIR_SOURCE),
        ("discardroot", DISCARD_ROOT_SOURCE),
    ];
    for (tag, source) in cases {
        let (first, second) = emit_twice(tag, source);
        assert!(
            first == second,
            "{tag}: the two emissions differ; first:\n{first}\nsecond:\n{second}"
        );
    }
}

// ---------------------------------------------------------------------------
// Fixed-array conditional arms and projections: a discarded fixed-array
// conditional fills one statement destination per arm (a call arm passes the
// hidden out-parameter), a discarded literal builds its store-only
// fixed-array field per field, a conditional root copies each arm's own field
// on its path, and a `for` iterable binds its ownerless root before the copy.
// ---------------------------------------------------------------------------

/// A discarded conditional of fixed-array calls.
const FIXED_ARM_DISCARD_SOURCE: &str = r#"module spec.cprimpl.fixedarmdiscard;

let left: i32 = 0;
let right: i32 = 0;

func a(): u8[2] {
    left = left + 1;
    return [1, 2];
}

func b(): u8[2] {
    right = right + 1;
    return [3, 4];
}

func use(c: bool): i32 {
    c ? a() : b();
    return 0;
}

func main(): i32 {
    return use(true) + use(false) + left * 10 + right - 11;
}
"#;

/// A discarded conditional of fixed-array calls whose elements own storage.
const FIXED_ARM_OWNED_SOURCE: &str = r#"module spec.cprimpl.fixedarmowned;

import { String, stringOf } from std.text;

let left: i32 = 0;
let right: i32 = 0;

func a(): String[2] {
    left = left + 1;
    return [stringOf("aa"), stringOf("bb")];
}

func b(): String[2] {
    right = right + 1;
    return [stringOf("cc"), stringOf("dd")];
}

func use(c: bool): i32 {
    c ? a() : b();
    return 0;
}

func main(): i32 {
    return use(true) + use(false) + left * 10 + right - 11;
}
"#;

/// A discarded literal whose fixed-array field is a conditional of calls.
const FIXED_FIELD_SOURCE: &str = r#"module spec.cprimpl.fixedfield;

import { String, stringOf } from std.text;

struct Mixed { names: str[2]; tag: String; }

let left: i32 = 0;
let right: i32 = 0;

func n1(): str[2] {
    left = left + 1;
    return ["AA", "BB"];
}

func n2(): str[2] {
    right = right + 1;
    return ["CC", "DD"];
}

func use(ok: bool): i32 {
    Mixed { names: ok ? n1() : n2(), tag: stringOf("x") };
    return 0;
}

func main(): i32 {
    return use(true) + use(false) + left * 10 + right - 11;
}
"#;

/// A fixed-array projection of a conditional root, stored.
const FIXED_ROOT_SOURCE: &str = r#"module spec.cprimpl.fixedroot;

struct Box { xs: u8[4]; }

func use(c: bool, b1: Box, b2: Box): i32 {
    let a: u8[4] = (c ? b1 : b2).xs;
    return i32(a[0]) + i32(a[1]) + i32(a[2]) + i32(a[3]);
}

func main(): i32 {
    let b1: Box = Box { xs: [1, 2, 3, 4] };
    let b2: Box = Box { xs: [5, 6, 7, 8] };
    return use(true, b1, b2) + use(false, b1, b2) - 36;
}
"#;

/// A fixed-array projection of a conditional root, returned.
const FIXED_ROOT_RETURN_SOURCE: &str = r#"module spec.cprimpl.fixedrootreturn;

struct Box { xs: u8[4]; }

func pick(c: bool, b1: Box, b2: Box): u8[4] {
    return (c ? b1 : b2).xs;
}

func sum(a: u8[4]): i32 {
    return i32(a[0]) + i32(a[1]) + i32(a[2]) + i32(a[3]);
}

func main(): i32 {
    let b1: Box = Box { xs: [1, 2, 3, 4] };
    let b2: Box = Box { xs: [5, 6, 7, 8] };
    return sum(pick(true, b1, b2)) + sum(pick(false, b1, b2)) - 36;
}
"#;

/// A `for` loop over a call result's fixed-array field.
const FIXED_FOR_SOURCE: &str = r#"module spec.cprimpl.fixedfor;

struct Box { xs: u8[4]; }

func mk(): Box { return Box { xs: [1, 2, 3, 4] }; }

func use(): i32 {
    let s: i32 = 0;
    for (x in mk().xs) {
        s = s + i32(x);
    }
    return s;
}

func main(): i32 {
    return use() - 10;
}
"#;

/// A discarded fixed-array conditional fills one statement destination per
/// arm.
///
/// The fixed-array return ABI passes a hidden out-parameter, so `c ? a() :
/// b();` has no C value expression: the statement declares `__cl_discard_1`,
/// each arm's call writes into its own temporary and copies the bytes into the
/// destination, and no bare call ever stands beside the value `?:` spelling.
#[test]
fn should_fill_a_discarded_fixed_array_conditional_per_arm() {
    let tag = "fixedarmdiscard";
    let (_scratch, emitted) = emit_host(tag, FIXED_ARM_DISCARD_SOURCE);
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_fixedarmdiscard_use(bool c)\n{",
    );
    assert_contains(tag, body, "u8 __cl_discard_1[2] = {0};");
    assert_contains(tag, body, "if (c) {");
    assert_contains(tag, body, "cl_spec_cprimpl_fixedarmdiscard_a(__cl_arm_2);");
    assert_contains(
        tag,
        body,
        "memcpy(&(__cl_discard_1), &(__cl_arm_2), sizeof(__cl_discard_1));",
    );
    assert_contains(tag, body, "cl_spec_cprimpl_fixedarmdiscard_b(__cl_arm_3);");
    assert_contains(
        tag,
        body,
        "memcpy(&(__cl_discard_1), &(__cl_arm_3), sizeof(__cl_discard_1));",
    );
    let fills = body
        .matches("memcpy(&(__cl_discard_1), &(__cl_arm_")
        .count();
    assert_eq!(
        fills, 2,
        "{tag}: each arm must fill the destination exactly once; body:\n{body}"
    );
    assert_absent(tag, body, "(void)((c ?");
}

/// A discarded owning fixed-array conditional releases its destination.
///
/// `c ? a() : b();` with `String` elements fills the same per-arm destination
/// and then releases both slots, because the statement owns the chosen call's
/// elements; the empty save the store core copies releases nothing.
#[test]
fn should_release_a_discarded_fixed_array_conditional_destination() {
    let tag = "fixedarmowned";
    let (_scratch, emitted) = emit_host(tag, FIXED_ARM_OWNED_SOURCE);
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_fixedarmowned_use(bool c)\n{",
    );
    assert_contains(tag, body, "ClString __cl_discard_1[2] = {0};");
    assert_contains(tag, body, "cl_spec_cprimpl_fixedarmowned_a(__cl_arm_2);");
    assert_contains(tag, body, "cl_spec_cprimpl_fixedarmowned_b(__cl_arm_3);");
    assert_contains(
        tag,
        body,
        "memcpy(&(__cl_discard_1), &(__cl_arm_2), sizeof(__cl_discard_1));",
    );
    assert_contains(
        tag,
        body,
        "memcpy(&(__cl_discard_1), &(__cl_arm_3), sizeof(__cl_discard_1));",
    );
    assert_release_after(
        tag,
        body,
        "memcpy(&(__cl_discard_1), &(__cl_arm_3)",
        "if (__cl_discard_1[0]) { FreeString(__cl_discard_1[0]); }",
    );
    assert_release_after(
        tag,
        body,
        "memcpy(&(__cl_discard_1), &(__cl_arm_3)",
        "if (__cl_discard_1[1]) { FreeString(__cl_discard_1[1]); }",
    );
    assert_absent(tag, body, "(void)((c ?");
}

/// A discarded literal builds its call-filled fixed-array field per field.
///
/// `Mixed { names: ok ? n1() : n2(), tag: stringOf("x") };` drops the
/// literal, so the statement declares `__cl_discard_1`, the field's
/// conditional fills `__cl_discard_1.names` per arm through the hidden
/// out-parameter, the fresh `tag` binds to its own statement temporary the
/// field store moves out, and the statement releases the tag afterwards. The
/// base's compound-literal spelling with the bare call arms must not come
/// back.
#[test]
fn should_build_a_discarded_call_filled_literal_per_field() {
    let tag = "fixedfield";
    let (_scratch, emitted) = emit_host(tag, FIXED_FIELD_SOURCE);
    let body = function_body(tag, &emitted, "cl_spec_cprimpl_fixedfield_use(bool ok)\n{");
    assert_contains(tag, body, "ClMixed __cl_discard_1 = {0};");
    assert_contains(tag, body, "cl_spec_cprimpl_fixedfield_n1(__cl_arm_2);");
    assert_contains(
        tag,
        body,
        "memcpy(&(__cl_discard_1.names), &(__cl_arm_2), sizeof(__cl_discard_1.names));",
    );
    assert_contains(tag, body, "cl_spec_cprimpl_fixedfield_n2(__cl_arm_3);");
    assert_contains(
        tag,
        body,
        "memcpy(&(__cl_discard_1.names), &(__cl_arm_3), sizeof(__cl_discard_1.names));",
    );
    assert_contains(tag, body, "__cl_tmp0 = cl_std_text_stringOf(cl_str(\"x\"));");
    assert_follows(
        tag,
        body,
        "__cl_tmp0 = cl_std_text_stringOf(cl_str(\"x\"));",
        "__cl_discard_1.tag = __cl_tmp0;",
    );
    assert_follows(
        tag,
        body,
        "__cl_discard_1.tag = __cl_tmp0;",
        "__cl_tmp0 = NULL;",
    );
    assert_release_after(
        tag,
        body,
        "if (ok) {",
        "if (__cl_discard_1.tag) { FreeString(__cl_discard_1.tag); }",
    );
    assert_absent(tag, body, "(void)((ClMixed){");
}

/// A fixed-array projection of a conditional root copies per arm.
///
/// `let a: u8[4] = (c ? b1 : b2).xs;` cannot take the address of a member of
/// the conditional value, so the store branches per arm and copies from each
/// arm's own root. No temporary holds a copy of either root, and the base's
/// single `&((c ? b1 : b2).xs)` spelling must not come back.
#[test]
fn should_copy_a_conditional_fixed_array_root_per_arm() {
    let tag = "fixedroot";
    let (_scratch, emitted) = emit_host(tag, FIXED_ROOT_SOURCE);
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_fixedroot_use(bool c, ClBox b1, ClBox b2)\n{",
    );
    assert_contains(tag, body, "if (c) {");
    assert_contains(tag, body, "memcpy(&(a), &(b1.xs), sizeof(a));");
    assert_contains(tag, body, "memcpy(&(a), &(b2.xs), sizeof(a));");
    assert_absent(tag, body, "(c ? b1 : b2)");
    assert_absent(tag, body, "__cl_tmp");
}

/// A fixed-array projection of a conditional root returns per arm.
///
/// `return (c ? b1 : b2).xs;` copies into the hidden `__cl_ret` destination
/// one arm at a time, exactly as the store spelling does, so the conditional
/// value is never addressed.
#[test]
fn should_copy_a_conditional_fixed_array_return_per_arm() {
    let tag = "fixedrootreturn";
    let (_scratch, emitted) = emit_host(tag, FIXED_ROOT_RETURN_SOURCE);
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_fixedrootreturn_pick(bool c, ClBox b1, ClBox b2, u8 __cl_ret[4])\n{",
    );
    assert_contains(tag, body, "if (c) {");
    assert_contains(
        tag,
        body,
        "memcpy(__cl_ret, &(b1.xs), (sizeof(__cl_ret[0]) * 4));",
    );
    assert_contains(
        tag,
        body,
        "memcpy(__cl_ret, &(b2.xs), (sizeof(__cl_ret[0]) * 4));",
    );
    assert_absent(tag, body, "(c ? b1 : b2)");
}

/// A `for` iterable binds its ownerless fixed-array root before the copy.
///
/// `for (x in mk().xs)` copies the projected field by address, and C cannot
/// name the address of a member of a call result, so the statement stores the
/// call into a local first and the copy names that local. The base's
/// `&(cl_..._mk().xs)` spelling must not come back.
#[test]
fn should_bind_a_for_iterable_fixed_array_root() {
    let tag = "fixedfor";
    let (_scratch, emitted) = emit_host(tag, FIXED_FOR_SOURCE);
    let body = function_body(tag, &emitted, "cl_spec_cprimpl_fixedfor_use(void)\n{");
    assert_follows(
        tag,
        body,
        "__cl_tmp2 = cl_spec_cprimpl_fixedfor_mk();",
        "memcpy(&(__cl_tmp3), &(__cl_tmp2.xs), sizeof(__cl_tmp3));",
    );
    assert_absent(tag, body, "cl_spec_cprimpl_fixedfor_mk().xs");
}

/// The fixed-arm and fixed-root programs re-emit byte-identically.
#[test]
fn should_re_emit_the_fixed_array_programs_byte_identically() {
    let cases: [(&str, &str); 2] = [
        ("fixedarmdiscard", FIXED_ARM_DISCARD_SOURCE),
        ("fixedroot", FIXED_ROOT_SOURCE),
    ];
    for (tag, source) in cases {
        let (first, second) = emit_twice(tag, source);
        assert!(
            first == second,
            "{tag}: the two emissions differ; first:\n{first}\nsecond:\n{second}"
        );
    }
}

// ---------------------------------------------------------------------------
// Update order completion: an update inside an assignment's parts, a literal
// operand, a dot receiver, a loop condition, or a later argument pins at its
// own source position, so the emitted C carries one defined left-to-right
// order instead of an unsequenced expression.
// ---------------------------------------------------------------------------

/// An assignment whose target and side share an updating place.
const ASSIGN_TARGET_SOURCE: &str = r#"module spec.cprimpl.implassigntarget;

func main(): i32 {
    let arr: i32[3] = [5, 6, 7];
    let i: i32 = 0;
    arr[i] = i++;
    let a: i32 = arr[0] + (arr[1] - 6) + (arr[2] - 7) + (i - 1);
    i = i++;
    let b: i32 = i - 1;
    i += i++;
    let c: i32 = i - 2;
    return a + b + c;
}
"#;

/// Literals whose operands share an updating place.
const LITERAL_ORDER_SOURCE: &str = r#"module spec.cprimpl.impliteralorder;

struct P { a: i32; b: i32; }

func main(): i32 {
    let i: i32 = 0;
    let p: P = P { a: i++, b: i };
    let j: i32 = 0;
    let xs: i32[2] = [j++, j];
    return p.a + (p.b - 1) + xs[0] + (xs[1] - 1);
}
"#;

/// A dot receiver that itself updates the argument's place.
const DOT_RECEIVER_SOURCE: &str = r#"module spec.cprimpl.impldotreceiver;

func get(a: i32, b: i32): i32 { return a * 10 + b; }

func main(): i32 {
    let i: i32 = 0;
    let r: i32 = (i++).get(i);
    return r + i - 2;
}
"#;

/// A loop condition that updates a place it also reads.
const LOOP_COND_UPDATE_SOURCE: &str = r#"module spec.cprimpl.implloopcond;

func id(x: i32): i32 { return x; }

func main(): i32 {
    let i: i32 = 0;
    let n: i32 = 0;
    while (i + id(i++) < 3) {
        n++;
    }
    return n + i - 5;
}
"#;

/// A call whose first argument owns storage and whose second updates a place.
const OWNING_ARG_SOURCE: &str = r#"module spec.cprimpl.implowningarg;

import { String, stringOf } from std.text;

struct Box { v: String; seen: i32; }

let i: i32 = 0;

func g(): Box {
    if (i == 0) {
        return Box { v: stringOf("ax"), seen: 0 };
    }
    return Box { v: stringOf("ax"), seen: 1 };
}

func f(take b: Box, n: i32): i32 {
    return b.seen * 10 + n;
}

func main(): i32 {
    let r: i32 = f(g(), i++);
    return r + i - 1;
}
"#;

/// An assignment's target location and updating side pin in source order.
///
/// `arr[i] = i++;` materializes the target's old index before the side's
/// update and the store names both temporaries, so the store reaches the slot
/// the old value selected. `i = i++;` stores the yielded old value after the
/// increment completes, and `i += i++;` reads the current value before the
/// side's update and combines the two temporaries. The base emits each shape
/// as one unsequenced expression, so these pins fail there.
#[test]
fn should_pin_an_assignment_target_and_its_updating_side() {
    let tag = "implassigntarget";
    let (_scratch, emitted) = emit_host(tag, ASSIGN_TARGET_SOURCE);
    let body = function_body(tag, &emitted, "cl_spec_cprimpl_implassigntarget_main(void)\n{");
    assert_follows(tag, body, "__cl_tmp0 = i;", "__cl_tmp1 = (i)++;");
    assert_contains(tag, body, "*(&(arr[__cl_tmp0])) = __cl_tmp1;");
    assert_follows(tag, body, "__cl_tmp2 = (i)++;", "i = __cl_tmp2;");
    assert_follows(tag, body, "__cl_tmp3 = i;", "__cl_tmp4 = (i)++;");
    assert_follows(
        tag,
        body,
        "__cl_tmp4 = (i)++;",
        "i = (i32)((u32)__cl_tmp3 + (u32)__cl_tmp4);",
    );
    assert_absent(tag, body, "*(&(arr[i])) = (i)++;");
    assert_absent(tag, body, "i = (i32)((u32)i + (u32)(i)++);");
}

/// A literal's updating operands pin at their source position.
///
/// `P { a: i++, b: i }` binds both fields to temporaries in source order
/// before the compound literal is built, and `[j++, j]` does the same for its
/// elements, so the read after the update sees the updated value. The base
/// renders the struct literal as `(ClP){ (i)++, i }`, whose C initializer
/// order is unspecified.
#[test]
fn should_pin_update_containing_literal_operands_in_source_order() {
    let tag = "impliteralorder";
    let (_scratch, emitted) = emit_host(tag, LITERAL_ORDER_SOURCE);
    let body = function_body(tag, &emitted, "cl_spec_cprimpl_impliteralorder_main(void)\n{");
    assert_follows(tag, body, "__cl_tmp0 = (i)++;", "__cl_tmp1 = i;");
    assert_follows(tag, body, "__cl_tmp1 = i;", "p = (ClP){ __cl_tmp0, __cl_tmp1 };");
    assert_follows(tag, body, "__cl_tmp2 = (j)++;", "__cl_tmp3 = j;");
    assert_follows(tag, body, "__cl_tmp3 = j;", "xs[0] = __cl_tmp2;");
    assert_contains(tag, body, "xs[1] = __cl_tmp3;");
    assert_absent(tag, body, "p = (ClP){ (i)++, i };");
}

/// A dot receiver with an update pins at argument position zero.
///
/// `(i++).get(i)` passes the receiver before the written argument, so the
/// receiver's update binds first and the argument's read second; the call
/// then reads both temporaries. The base emits `get((i)++, i)` with both
/// evaluations unordered.
#[test]
fn should_pin_an_updating_dot_receiver_before_its_arguments() {
    let tag = "impldotreceiver";
    let (_scratch, emitted) = emit_host(tag, DOT_RECEIVER_SOURCE);
    let body = function_body(tag, &emitted, "cl_spec_cprimpl_impldotreceiver_main(void)\n{");
    assert_follows(tag, body, "__cl_tmp0 = (i)++;", "__cl_tmp1 = i;");
    assert_follows(
        tag,
        body,
        "__cl_tmp1 = i;",
        "r = cl_spec_cprimpl_impldotreceiver_get(__cl_tmp0, __cl_tmp1);",
    );
    assert_absent(tag, body, "get((i)++, i)");
}

/// A loop condition's update fills per iteration inside the loop body.
///
/// `while (i + id(i++) < 3)` cannot pin before the loop: the condition
/// re-evaluates every test. The loop instead prints an unconditional header
/// and fills the condition's operands with the per-iteration guards, before
/// the break test, so the update runs once per test. The base prints the
/// whole condition in the header with both evaluations unordered.
#[test]
fn should_fill_an_updating_loop_condition_per_iteration() {
    let tag = "implloopcond";
    let (_scratch, emitted) = emit_host(tag, LOOP_COND_UPDATE_SOURCE);
    let body = function_body(tag, &emitted, "cl_spec_cprimpl_implloopcond_main(void)\n{");
    assert_follows(tag, body, "while (1) {", "__cl_tmp0 = i;");
    assert_follows(tag, body, "__cl_tmp0 = i;", "__cl_tmp1 = (i)++;");
    assert_follows(
        tag,
        body,
        "__cl_tmp1 = (i)++;",
        "__cl_tmp2 = cl_spec_cprimpl_implloopcond_id(__cl_tmp1);",
    );
    assert_follows(
        tag,
        body,
        "__cl_tmp2 = cl_spec_cprimpl_implloopcond_id(__cl_tmp1);",
        "__cl_tmp3 = (i32)((u32)__cl_tmp0 + (u32)__cl_tmp2);",
    );
    assert_follows(
        tag,
        body,
        "__cl_tmp3 = (i32)((u32)__cl_tmp0 + (u32)__cl_tmp2);",
        "if (!((__cl_tmp3 < 3))) break;",
    );
    assert_absent(
        tag,
        body,
        "while (((i32)((u32)i + (u32)cl_spec_cprimpl_implloopcond_id((i)++)) < 3))",
    );
}

/// An owning argument binds before a later argument's update.
///
/// `f(g(), i++)` binds the owning result to `__cl_owned0` at argument
/// position zero, then pins the second argument's update, then calls: the
/// callee's `take` move-out empties `__cl_owned0` after the call, so the
/// ownership transfers exactly once. The base pins the update first and binds
/// the owning result after it, so the callee observes the update.
#[test]
fn should_bind_an_owning_argument_before_a_later_update() {
    let tag = "implowningarg";
    let (_scratch, emitted) = emit_host(tag, OWNING_ARG_SOURCE);
    let body = function_body(tag, &emitted, "cl_spec_cprimpl_implowningarg_main(void)\n{");
    assert_follows(
        tag,
        body,
        "__cl_owned0 = cl_spec_cprimpl_implowningarg_g();",
        "__cl_tmp0 = (cl_spec_cprimpl_implowningarg_i)++;",
    );
    assert_follows(
        tag,
        body,
        "__cl_tmp0 = (cl_spec_cprimpl_implowningarg_i)++;",
        "__cl_tmp1 = cl_spec_cprimpl_implowningarg_f(__cl_owned0, __cl_tmp0);",
    );
    assert_follows(
        tag,
        body,
        "__cl_tmp1 = cl_spec_cprimpl_implowningarg_f(__cl_owned0, __cl_tmp0);",
        "cl_zero(&__cl_owned0);",
    );
}

/// The assignment and loop-condition programs re-emit byte-identically.
#[test]
fn should_re_emit_the_update_order_programs_byte_identically() {
    let cases: [(&str, &str); 2] = [
        ("implassigntarget", ASSIGN_TARGET_SOURCE),
        ("implloopcond", LOOP_COND_UPDATE_SOURCE),
    ];
    for (tag, source) in cases {
        let (first, second) = emit_twice(tag, source);
        assert!(
            first == second,
            "{tag}: the two emissions differ; first:\n{first}\nsecond:\n{second}"
        );
    }
}

// ---------------------------------------------------------------------------
// View owners, loans, and stores (phase 1).
//
// The pins cover the residual-root binding of duplicable projections, the
// runtime-index view store's copy path, the cross-frame pass-through report,
// the module view's retained member, the container escape's per-destination
// reports, the replaced literal-carrier loans, and the all-null composite
// store's emitted null view.
// ---------------------------------------------------------------------------

/// A take and a return position of a duplicable member over a fresh root.
const DUP_RESIDUAL_SOURCE: &str = r#"module spec.cprimpl.impldupres;

import { String, stringOf } from std.text;

struct Box { n: i32; v: String; }

func mkBox(k: i32): Box { return Box { n: k, v: stringOf("ab") }; }
func holdNum(take n: i32): i32 { return n; }

func takePos(c: bool, take a: Box): i32 {
    return holdNum(c ? a.n : mkBox(7).n);
}

func retPos(c: bool, take a: Box): i32 {
    return (c ? a.n : mkBox(7).n);
}

func main(): i32 { return takePos(true, mkBox(3)) + retPos(false, mkBox(3)) - 10; }
"#;

/// The read positions of a duplicable element over a fresh root.
const DUP_RESIDUAL_READ_SOURCE: &str = r#"module spec.cprimpl.impldupresread;

import { String, stringOf } from std.text;

struct Box { tag: String; ys: u8[4]; }

func mk(): Box { return Box { tag: stringOf("one"), ys: [1, 2, 3, 4] }; }

func readDirect(): i32 {
    let s: i32 = 0;
    s = s + i32(mk().ys[2]);
    return s;
}

func readCond(c: bool, b: Box): i32 {
    let s: i32 = 0;
    s = s + i32((c ? mk() : b).ys[1]);
    return s;
}

func main(): i32 {
    let b: Box = Box { tag: stringOf("p"), ys: [9, 10, 11, 12] };
    return readCond(true, b) + readDirect() - 13;
}
"#;

/// A duplicable projection's fresh root binds for its residual release.
///
/// `takePos` and `retPos` fill a branch temporary with `mkBox(7)` on the fresh
/// arm and read `__cl_tmp1.n`, and the release frees the root's `v` after the
/// use; the inline `mkBox(7).n` spelling is gone. The read positions bind the
/// same way: a direct `mk().ys[i]` reads through a generated owner and the
/// conditional read copies `ys` out of the branch-filled root.
#[test]
fn should_bind_the_residual_root_of_a_duplicable_projection() {
    let tag = "impldupres";
    let (_scratch, emitted) = emit_host(tag, DUP_RESIDUAL_SOURCE);
    let take = function_body(tag, &emitted, "cl_spec_cprimpl_impldupres_takePos(bool c, ClBox a)\n{");
    assert_contains(tag, take, "cl_spec_cprimpl_impldupres_mkBox(7);");
    assert_contains(tag, take, "(__cl_tmp0 ? a.n : __cl_tmp1.n)");
    assert_release_after(
        tag,
        take,
        "holdNum((__cl_tmp0 ? a.n : __cl_tmp1.n));",
        "if (__cl_tmp1.v) { FreeString(__cl_tmp1.v); }",
    );
    assert_absent(tag, take, "? a.n : cl_spec_cprimpl_impldupres_mkBox(7).n)");
    let ret = function_body(tag, &emitted, "cl_spec_cprimpl_impldupres_retPos(bool c, ClBox a)\n{");
    assert_contains(tag, ret, "(__cl_tmp0 ? a.n : __cl_tmp1.n)");
    assert_release_after(
        tag,
        ret,
        "__cl_ret = (__cl_tmp0 ? a.n : __cl_tmp1.n);",
        "if (__cl_tmp1.v) { FreeString(__cl_tmp1.v); }",
    );

    let tag = "impldupresread";
    let (_scratch, emitted) = emit_host(tag, DUP_RESIDUAL_READ_SOURCE);
    let direct =
        function_body(tag, &emitted, "cl_spec_cprimpl_impldupresread_readDirect(void)\n{");
    assert_follows(
        tag,
        direct,
        "__cl_owned0 = cl_spec_cprimpl_impldupresread_mk();",
        "s = (i32)((u32)s + (u32)(i32)(__cl_owned0.ys[2]));",
    );
    assert_release_after(
        tag,
        direct,
        "__cl_owned0.ys[2]",
        "if (__cl_owned0.tag) { FreeString(__cl_owned0.tag); }",
    );
    assert_absent(tag, direct, "mk().ys[2]");
    let cond = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_impldupresread_readCond(bool c, ClBox b)\n{",
    );
    assert_contains(tag, cond, "__cl_tmp1 = cl_spec_cprimpl_impldupresread_mk();");
    assert_contains(tag, cond, "&(__cl_tmp1.ys)");
    assert_release_after(
        tag,
        cond,
        "memcpy(&(__cl_tmp2), &(__cl_tmp1.ys), sizeof(__cl_tmp2));",
        "if (__cl_tmp1.tag) { FreeString(__cl_tmp1.tag); }",
    );
}

/// A runtime-index view element store copies the view and loans the backing.
const RUNTIME_STORE_SOURCE: &str = r#"module spec.cprimpl.implruntimestore;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func pick(k: i32): i32 {
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    let xs: str[2] = ["AA", "BB"];
    xs[k] = b.v;
    return i32(xs[k][0]) - 120;
}

func main(): i32 { return pick(1); }
"#;

/// The re-executed runtime-index store over a fresh call result.
const RUNTIME_STORE_LOOP_SOURCE: &str = r#"module spec.cprimpl.implruntimeloop;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func mk(): Box { return Box { v: stringOf("xy"), w: stringOf("zz") }; }

func run(k: i32): i32 {
    let xs: str[2] = ["AA", "BB"];
    let i: i32 = 0;
    while (i < 3) {
        xs[k] = mk().v;
        i = i + 1;
    }
    return i32(xs[k][0]);
}

func main(): i32 { return run(1) - 120; }
"#;

/// The mutation of a runtime-index stored backing reports once.
const RUNTIME_STORE_MUTATION_SOURCE: &str = r#"module spec.cprimpl.implruntimemut;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func pick(k: i32): i32 {
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    let xs: str[2] = ["AA", "BB"];
    xs[k] = b.v;
    b.v = stringOf("qq");
    return i32(xs[k][0]) - 120;
}

func main(): i32 { return pick(1); }
"#;

/// A runtime-index element store copies its view like the constant index.
///
/// The store writes `cl_string_view(b.v)` through the element pointer and
/// leaves `b`'s member in place — no move-out temporary strands the owner —
/// so `b`'s cleanup releases it. The re-executed fresh store rejects with one
/// `E10030` like the constant-index twin, and the later backing write reports
/// one `E10031` at its own line.
#[test]
fn should_copy_a_runtime_index_view_element_store() {
    let tag = "implruntimestore";
    let (_scratch, emitted) = emit_host(tag, RUNTIME_STORE_SOURCE);
    let body = function_body(tag, &emitted, "cl_spec_cprimpl_implruntimestore_pick(i32 k)\n{");
    assert_contains(tag, body, "*(&(xs[__cl_tmp0])) = cl_string_view(b.v);");
    assert_absent(tag, body, "= b.v;");
    assert_rejected(tag, RUNTIME_STORE_LOOP_SOURCE, "E10030", 1);
    assert_rejected_at_line(tag, RUNTIME_STORE_MUTATION_SOURCE, "E10031", 1, 11);
}

/// The cross-frame pass-through spellings and the accepted twins.
const CROSS_FRAME_LOCAL_SOURCE: &str = r#"module spec.cprimpl.implxflocal;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let b: Box = Box { v: stringOf("xyz"), w: stringOf("ww") };
    fill(b.v, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    return 0;
}
"#;

/// The fresh-argument spelling of the cross-frame pass-through.
const CROSS_FRAME_FRESH_SOURCE: &str = r#"module spec.cprimpl.implxftemp;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func mk(): Box { return Box { v: stringOf("xy"), w: stringOf("zw") }; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void { fill(mk().v, borrow slot); }

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    return 0;
}
"#;

/// The array-literal spelling of the cross-frame pass-through.
const CROSS_FRAME_ARRAY_SOURCE: &str = r#"module spec.cprimpl.implxfarray;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func mk(): Box { return Box { v: stringOf("xy"), w: stringOf("zw") }; }

func fill(p: str, q: str, borrow slot: str[2]): void { slot = [p, q]; }

func middle(borrow out2: str[2]): void { fill(mk().v, mk().w, borrow out2); }

func main(): i32 {
    let out: str[2] = ["AA", "BB"];
    middle(borrow out);
    return i32(out[0][0]) - 120;
}
"#;

/// The parameter-backed and same-frame twins keep their acceptance.
const CROSS_FRAME_PARAM_SOURCE: &str = r#"module spec.cprimpl.implxfparam;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(p: Box, borrow slot: str?): void { fill(p.v, borrow slot); }

func main(): i32 {
    let out: str? = null;
    let b: Box = Box { v: stringOf("xyz"), w: stringOf("ww") };
    outer(b, borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 120;
}
"#;

const CROSS_FRAME_SAME_SOURCE: &str = r#"module spec.cprimpl.implxfsame;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func demo(): i32 {
    let b: Box = Box { v: stringOf("xyz"), w: stringOf("ww") };
    let out: str? = null;
    fill(b.v, borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 120;
}

func main(): i32 { return demo(); }
"#;

/// A local copy inside the callee still keeps the foreign root.
const CROSS_FRAME_COPY_SOURCE: &str = r#"module spec.cprimpl.implxfcopy;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str, borrow slot: str?): void {
    let s: str = p;
    slot = s;
}

func outer(borrow slot: str?): void {
    let b: Box = Box { v: stringOf("xyz"), w: stringOf("ww") };
    fill(b.v, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    return 0;
}
"#;

/// A static-field store must not hide the caller's frame report.
const CROSS_FRAME_STATIC_SOURCE: &str = r#"module spec.cprimpl.implxfstatic;

import { String, stringOf } from std.text;

struct Holder { tag: static str; }
struct Box { v: String; w: String; }

func pick(h: Holder, q: str, borrow slot: str?): void {
    slot = h.tag;
}

func outer(borrow slot: str?): void {
    let b: Box = Box { v: stringOf("xyz"), w: stringOf("ww") };
    let h: Holder = Holder { tag: "AA" };
    pick(h, b.v, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    return 0;
}
"#;

/// A conditional write to the copied view must keep the caller guard.
const CROSS_FRAME_LAUNDER_SOURCE: &str = r#"module spec.cprimpl.implxflaunder;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, c: bool, borrow slot: str?): void {
    let s: str = p;
    if (c) { s = "AA"; }
    slot = s;
}

func outer(c: bool, borrow slot: str?): void {
    let b: Box = Box { v: stringOf("AA") };
    fill(b.v, c, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(false, borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 65;
}
"#;

/// A static store inside a lambda must not hide the caller guard.
const CROSS_FRAME_LAMBDA_STATIC_SOURCE: &str = r#"module spec.cprimpl.implxflambdastatic;

import { String, stringOf } from std.text;

struct Box { v: String; }

func pick(q: str, borrow slot: str?): void {
    ([borrow slot] () => { slot = "AA"; })();
}

func outer(borrow slot: str?): void {
    let b: Box = Box { v: stringOf("AA") };
    pick(b.v, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    return 0;
}
"#;

/// A cross-frame pass-through reports once at the callee store.
///
/// The local and fresh spellings report one `E10034` at the fill; the array
/// spelling keeps its `E10033` and `E10030` diagnostics. The parameter-backed
/// chain, the same-frame call, and the local-copy spelling report `E10034`
/// once. A callee whose store is a static field must not hide the caller's
/// `E10030` frame report.
#[test]
fn should_reject_a_cross_frame_pass_through_once() {
    let tag = "implxflocal";
    assert_rejected_at_line(tag, CROSS_FRAME_LOCAL_SOURCE, "E10034", 1, 7);
    assert_rejected(tag, CROSS_FRAME_FRESH_SOURCE, "E10034", 1);
    assert_rejected_code_set(tag, CROSS_FRAME_ARRAY_SOURCE, &["E10033", "E10030"]);
    assert_rejected("implxfparam", CROSS_FRAME_PARAM_SOURCE, "E10034", 1);
    assert_rejected("implxfsame", CROSS_FRAME_SAME_SOURCE, "E10034", 1);
    assert_rejected("implxfcopy", CROSS_FRAME_COPY_SOURCE, "E10034", 1);
    assert_rejected("implxfstatic", CROSS_FRAME_STATIC_SOURCE, "E10030", 1);
    assert_rejected("implxflaunder", CROSS_FRAME_LAUNDER_SOURCE, "E10034", 1);
    assert_rejected(
        "implxflambdastatic",
        CROSS_FRAME_LAMBDA_STATIC_SOURCE,
        "E10030",
        1,
    );
}

/// A module view of a fresh call's member retains the member's storage.
const MODULE_FRESH_SOURCE: &str = r#"module spec.cprimpl.implmodfresh;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func mk(): Box { return Box { v: stringOf("xy"), w: stringOf("zz") }; }

let g: str = mk().v;

func main(): i32 {
    return i32(g[0]) - 120;
}
"#;

/// The module initializer binds the root, empties the member, frees the rest.
///
/// The initializer stores the view through the bound root
/// (`__cl_itmp0.v`), then empties that member so the release pass frees only
/// the root's remaining field; the view keeps the member's storage for the
/// program's lifetime. The two emissions are byte-identical.
#[test]
fn should_retain_a_module_views_member_while_the_root_releases() {
    let tag = "implmodfresh";
    let (scratch, emitted) = emit_host(tag, MODULE_FRESH_SOURCE);
    let init = function_body(
        tag,
        &emitted,
        "static void cl_init_spec_cprimpl_implmodfresh(void)\n{",
    );
    assert_follows(
        tag,
        init,
        "ClBox __cl_itmp0 = cl_spec_cprimpl_implmodfresh_mk();",
        "cl_spec_cprimpl_implmodfresh_g = cl_string_view(__cl_itmp0.v);",
    );
    assert_follows(
        tag,
        init,
        "cl_spec_cprimpl_implmodfresh_g = cl_string_view(__cl_itmp0.v);",
        "__cl_itmp0.v = NULL;",
    );
    assert_release_after(
        tag,
        init,
        "__cl_itmp0.v = NULL;",
        "if (__cl_itmp0.w) { FreeString(__cl_itmp0.w); }",
    );
    let source_path = scratch.path().join("main.clo");
    let second = emit_at(tag, &source_path, &scratch.path().join("out2.c"));
    assert!(
        emitted == second,
        "{tag}: the two emissions differ; first:\n{emitted}\nsecond:\n{second}"
    );
}

/// The container-escape spellings and their controls.
const CONTAINER_ESCAPE_RETURN_SOURCE: &str = r#"module spec.cprimpl.implreturnelem;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func run(): str[2] {
    let xs: str[2] = ["AA", "BB"];
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    xs[1] = b.v;
    return xs;
}

func main(): i32 { return i32(run()[1][0]) - 120; }
"#;

const CONTAINER_ESCAPE_ALIAS_SOURCE: &str = r#"module spec.cprimpl.implreturnalias;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func run(): str[2] {
    let xs: str[2] = ["AA", "BB"];
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    for (borrow e in xs) {
        e = b.v;
    }
    return xs;
}

func main(): i32 { return i32(run()[0][0]) - 120; }
"#;

const CONTAINER_ESCAPE_GLOBAL_SOURCE: &str = r#"module spec.cprimpl.implglobalelem;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

let g: str[2] = ["AA", "BB"];

func store(): void {
    let xs: str[2] = ["AA", "BB"];
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    xs[1] = b.v;
    g = xs;
}

func main(): i32 { store(); return i32(g[1][0]) - 120; }
"#;

const CONTAINER_ESCAPE_PARAM_SOURCE: &str = r#"module spec.cprimpl.implreturnparam;

import { String, stringOf } from std.text;

func run(p: str): str[2] {
    let xs: str[2] = ["AA", "BB"];
    xs[1] = p;
    return xs;
}

func main(): i32 {
    let s: String = stringOf("xy");
    return i32(run(s)[1][0]) - 120;
}
"#;

/// A container escape reports once per destination, the parameter twin stays
/// accepted.
///
/// The element return reports one `E10035` at the return line and the alias
/// spelling the same; the global store reports one `E10033` at its line. The
/// parameter-backed return is accepted and re-emits byte-identically.
#[test]
fn should_reject_a_container_escape_once_per_destination() {
    let tag = "implreturnelem";
    assert_rejected_at_line(tag, CONTAINER_ESCAPE_RETURN_SOURCE, "E10035", 1, 11);
    assert_rejected_at_line(tag, CONTAINER_ESCAPE_ALIAS_SOURCE, "E10035", 1, 13);
    assert_rejected_at_line(tag, CONTAINER_ESCAPE_GLOBAL_SOURCE, "E10033", 1, 13);
    let (first, second) = emit_twice("implreturnparam", CONTAINER_ESCAPE_PARAM_SOURCE);
    assert!(
        first == second,
        "implreturnparam: the two emissions differ; first:\n{first}\nsecond:\n{second}"
    );
}

/// The stale-loan spellings and the live-loan control.
const STALE_LOAN_FIELD_SOURCE: &str = r#"module spec.cprimpl.implstalefield;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Holder { names: str[1]; }

func pick(): i32 {
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    let h: Holder = Holder { names: [b.v] };
    h.names = ["AA"];
    b.v = stringOf("qq");
    return i32(h.names[0][0]) - 65;
}

func main(): i32 { return pick(); }
"#;

const STALE_LOAN_WHOLE_SOURCE: &str = r#"module spec.cprimpl.implstalewhole;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Holder { names: str[1]; }

func pick(): i32 {
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    let h: Holder = Holder { names: [b.v] };
    h = Holder { names: ["AA"] };
    b.v = stringOf("qq");
    return i32(h.names[0][0]) - 65;
}

func main(): i32 { return pick(); }
"#;

const STALE_LOAN_LIVE_SOURCE: &str = r#"module spec.cprimpl.implstalelive;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Holder { names: str[1]; }

func pick(): i32 {
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    let h: Holder = Holder { names: [b.v] };
    b.v = stringOf("qq");
    return i32(h.names[0][0]);
}

func main(): i32 { return pick(); }
"#;

/// An overwritten literal carrier drops its stale loans; a live view keeps
/// its own.
///
/// The field overwrite and the whole reassignment both replace the stored
/// view, so the later backing write is accepted and re-emits identically;
/// without an overwrite the live loan reports exactly one `E10031` at the
/// write line.
#[test]
fn should_replace_the_loans_of_an_overwritten_literal_carrier() {
    for (tag, source) in [
        ("implstalefield", STALE_LOAN_FIELD_SOURCE),
        ("implstalewhole", STALE_LOAN_WHOLE_SOURCE),
    ] {
        let (first, second) = emit_twice(tag, source);
        assert!(
            first == second,
            "{tag}: the two emissions differ; first:\n{first}\nsecond:\n{second}"
        );
    }
    assert_rejected_at_line("implstalelive", STALE_LOAN_LIVE_SOURCE, "E10031", 1, 11);
}

/// The all-null composite store emits the destination's empty view.
const ALL_NULL_SOURCE: &str = r#"module spec.cprimpl.implallnull;

func fill(c: bool, borrow slot: str?): void {
    slot = c ? null : null;
}

let h: str? = null;

func store(c: bool): void {
    h = c ? null : null;
}

func main(): i32 {
    let out: str? = null;
    fill(true, borrow out);
    store(true);
    if (out == null && h == null) {
        return 0;
    }
    return 9;
}
"#;

const ALL_NULL_MIX_LOCAL_SOURCE: &str = r#"module spec.cprimpl.implnullmix;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(c: bool, borrow slot: str?): void {
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    slot = c ? b.v : null;
}

func main(): i32 {
    let out: str? = null;
    fill(false, borrow out);
    return 0;
}
"#;

const ALL_NULL_MIX_PARAM_SOURCE: &str = r#"module spec.cprimpl.implnullparam;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(c: bool, p: Box, borrow slot: str?): void {
    slot = c ? p.v : null;
}

func main(): i32 {
    let out: str? = null;
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    fill(true, b, borrow out);
    if (out == null) {
        return 9;
    }
    return i32(out[0]) - 120;
}
"#;

/// An all-null composite store renders the empty view, the mixes keep theirs.
///
/// Both destinations render `cl_zero_ClStr()` for the null arms instead of a
/// bare `NULL`, so the view struct assignment is valid C; the frame-local
/// mix still reports one `E10030` at its store line while the parameter mix
/// is accepted and re-emits byte-identically.
#[test]
fn should_emit_the_null_view_of_an_all_null_composite() {
    let tag = "implallnull";
    let (_scratch, emitted) = emit_host(tag, ALL_NULL_SOURCE);
    let fill = function_body(
        tag,
        &emitted,
        "static void cl_spec_cprimpl_implallnull_fill(bool c, ClStr *slot)\n{",
    );
    assert_contains(tag, fill, "(*slot) = (c ? cl_zero_ClStr() : cl_zero_ClStr());");
    assert_absent(tag, fill, "= (c ? NULL : NULL);");
    let store = function_body(
        tag,
        &emitted,
        "static void cl_spec_cprimpl_implallnull_store(bool c)\n{",
    );
    assert_contains(
        tag,
        store,
        "cl_spec_cprimpl_implallnull_h = (c ? cl_zero_ClStr() : cl_zero_ClStr());",
    );
    assert_rejected_at_line("implnullmix", ALL_NULL_MIX_LOCAL_SOURCE, "E10030", 1, 9);
    assert_rejected("implnullparam", ALL_NULL_MIX_PARAM_SOURCE, "E10034", 1);
}

// ---------------------------------------------------------------------------
// Fix round: owners-resolved pass-through, carrier copies, deep/array module
// views, and the fixed-array take root.
// ---------------------------------------------------------------------------

/// The caller-backed cross-frame spellings re-emit byte-identically.
const OWNER_PASS_CARRIER_SOURCE: &str = r#"module spec.cprimpl.implxfcarrier;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Holder { v: str[1]; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(p: Box, borrow slot: str?): void {
    let h: Holder = Holder { v: [p.v] };
    fill(h.v[0], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
    outer(b, borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 120;
}
"#;

const OWNER_PASS_LOCAL_SOURCE: &str = r#"module spec.cprimpl.implxfvialocal;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(p: Box, borrow slot: str?): void {
    let s: str = p.v;
    fill(s, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    let b: Box = Box { v: stringOf("xyz"), w: stringOf("ww") };
    outer(b, borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 120;
}
"#;

/// The owner-resolved cross-frame programs report the foreign store once.
#[test]
fn should_reject_the_owner_resolved_cross_frame_programs() {
    for (tag, source) in [
        ("implxfcarrier", OWNER_PASS_CARRIER_SOURCE),
        ("implxfvialocal", OWNER_PASS_LOCAL_SOURCE),
    ] {
        assert_rejected(tag, source, "E10034", 1);
    }
}

/// A deep and an array member of a module view retain their fresh roots.
const MODULE_DEEP_SOURCE: &str = r#"module spec.cprimpl.implxmoddeep;

import { String, stringOf } from std.text;

struct Inner { s: String; n: i32; }
struct Outer { i: Inner; t: String; }

func mkO(): Outer { return Outer { i: Inner { s: stringOf("xy"), n: 1 }, t: stringOf("zz") }; }

let g: str = mkO().i.s;

func main(): i32 {
    return i32(g[0]) - 120;
}
"#;

const MODULE_ARRAY_SOURCE: &str = r#"module spec.cprimpl.implxmodarray;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func mk(): Box { return Box { v: stringOf("xy"), w: stringOf("zz") }; }

let a: str[2] = [mk().v, mk().w];

func main(): i32 {
    return i32(a[0][0]) + i32(a[1][0]) - 242;
}
"#;

/// A deep member empties its exact path; an array member empties per element.
///
/// The deep initializer binds `mkO()` and empties `__cl_itmp0.i.s` before the
/// release frees the root's remaining field; the array initializer binds each
/// element's root, stores both views, and empties `__cl_itmp0.v` and
/// `__cl_itmp1.w` before the releases run.
#[test]
fn should_empty_the_read_position_of_a_deep_or_array_module_view() {
    let tag = "implxmoddeep";
    let (_scratch, emitted) = emit_host(tag, MODULE_DEEP_SOURCE);
    let init = function_body(
        tag,
        &emitted,
        "static void cl_init_spec_cprimpl_implxmoddeep(void)\n{",
    );
    assert_follows(
        tag,
        init,
        "ClOuter __cl_itmp0 = cl_spec_cprimpl_implxmoddeep_mkO();",
        "cl_spec_cprimpl_implxmoddeep_g = cl_string_view(__cl_itmp0.i.s);",
    );
    assert_follows(
        tag,
        init,
        "cl_spec_cprimpl_implxmoddeep_g = cl_string_view(__cl_itmp0.i.s);",
        "__cl_itmp0.i.s = NULL;",
    );
    assert_release_after(
        tag,
        init,
        "__cl_itmp0.i.s = NULL;",
        "if (__cl_itmp0.t) { FreeString(__cl_itmp0.t); }",
    );

    let tag = "implxmodarray";
    let (_scratch, emitted) = emit_host(tag, MODULE_ARRAY_SOURCE);
    let init = function_body(
        tag,
        &emitted,
        "static void cl_init_spec_cprimpl_implxmodarray(void)\n{",
    );
    assert_follows(
        tag,
        init,
        "ClBox __cl_itmp1 = cl_spec_cprimpl_implxmodarray_mk();",
        "cl_spec_cprimpl_implxmodarray_a[0] = cl_string_view(__cl_itmp0.v);",
    );
    assert_follows(
        tag,
        init,
        "cl_spec_cprimpl_implxmodarray_a[0] = cl_string_view(__cl_itmp0.v);",
        "cl_spec_cprimpl_implxmodarray_a[1] = cl_string_view(__cl_itmp1.w);",
    );
    assert_follows(
        tag,
        init,
        "cl_spec_cprimpl_implxmodarray_a[1] = cl_string_view(__cl_itmp1.w);",
        "__cl_itmp0.v = NULL;",
    );
    assert_release_after(
        tag,
        init,
        "__cl_itmp1.w = NULL;",
        "if (__cl_itmp0.w) { FreeString(__cl_itmp0.w); }",
    );
}

/// A fixed-array take projection binds its fresh root per branch.
const FIXED_ARRAY_TAKE_SOURCE: &str = r#"module spec.cprimpl.implxfarrtake;

import { String, stringOf } from std.text;

struct Box { ys: u8[4]; v: String; }

func mk(): Box { return Box { ys: [1, 2, 3, 4], v: stringOf("ab") }; }
func holdArr(take a: u8[4]): i32 { return i32(a[0]); }

func probe(c: bool, b: Box): i32 {
    return holdArr(c ? b.ys : mk().ys);
}

func main(): i32 {
    let b: Box = Box { ys: [9, 10, 11, 12], v: stringOf("xy") };
    return probe(false, b) - 1;
}
"#;

/// The fixed-array take arm fills the root, the release follows the call.
#[test]
fn should_bind_the_fixed_array_take_projection_root() {
    let tag = "implxfarrtake";
    let (_scratch, emitted) = emit_host(tag, FIXED_ARRAY_TAKE_SOURCE);
    let body = function_body(tag, &emitted, "cl_spec_cprimpl_implxfarrtake_probe(bool c, ClBox b)\n{");
    assert_contains(tag, body, "__cl_tmp1 = cl_spec_cprimpl_implxfarrtake_mk();");
    assert_contains(
        tag,
        body,
        "memcpy(&(__cl_tmp2), &(b.ys), sizeof(__cl_tmp2));",
    );
    assert_contains(
        tag,
        body,
        "memcpy(&(__cl_tmp2), &(__cl_tmp1.ys), sizeof(__cl_tmp2));",
    );
    assert_release_after(
        tag,
        body,
        "cl_spec_cprimpl_implxfarrtake_holdArr(__cl_tmp2);",
        "if (__cl_tmp1.v) { FreeString(__cl_tmp1.v); }",
    );
}

/// A module view of a fresh root's view element empties that element.
const MODULE_VIEW_ELEMENT_SOURCE: &str = r#"module spec.cprimpl.implxfelemview;

import { String, stringOf } from std.text;

struct Holder { names: str[1]; k: String; }

func mkH(): Holder { return Holder { names: ["xy"], k: stringOf("kk") }; }

let g: str = mkH().names[0];

func main(): i32 {
    return i32(g[0]) - 120;
}
"#;

/// The view element empties with the zero view helper before the release.
///
/// The initializer binds `mkH()`, stores the element view, and empties
/// `__cl_itmp0.names[0]` with `cl_zero_ClStr()` — a view position takes the
/// same move-out as an owning one — so the release frees the root's remaining
/// field while the module view keeps reading the stored element.
#[test]
fn should_empty_the_view_element_of_a_module_root() {
    let tag = "implxfelemview";
    let (_scratch, emitted) = emit_host(tag, MODULE_VIEW_ELEMENT_SOURCE);
    let init = function_body(
        tag,
        &emitted,
        "static void cl_init_spec_cprimpl_implxfelemview(void)\n{",
    );
    assert_follows(
        tag,
        init,
        "ClHolder __cl_itmp0 = cl_spec_cprimpl_implxfelemview_mkH();",
        "cl_spec_cprimpl_implxfelemview_g = __cl_itmp0.names[0];",
    );
    assert_follows(
        tag,
        init,
        "cl_spec_cprimpl_implxfelemview_g = __cl_itmp0.names[0];",
        "__cl_itmp0.names[0] = cl_zero_ClStr();",
    );
    assert_release_after(
        tag,
        init,
        "__cl_itmp0.names[0] = cl_zero_ClStr();",
        "if (__cl_itmp0.k) { FreeString(__cl_itmp0.k); }",
    );
}

// ---------------------------------------------------------------------------
// Guarded, discarded, and carrier roots (phase 2).
// ---------------------------------------------------------------------------

/// A guarded call whose view element wraps a fresh call root.
const GUARD_CALL_ROOT_SOURCE: &str = r#"module spec.cprimpl.implguardcall;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Holder { names: str[1]; }

let calls: i32 = 0;

func mk(): Box {
    calls = calls + 1;
    return Box { v: stringOf("ab"), w: stringOf("cd") };
}

func id(x: str): str { return x; }

func use(h: Holder): i32 { return i32(len(h.names[0])); }

func run(g: bool, k: i32): i32 {
    let n: i32 = 0;
    let i: i32 = 0;
    while (i < k) {
        n = n + (g ? use(Holder { names: [id(mk().v)] }) : 0);
        i = i + 1;
    }
    return n;
}

func main(): i32 {
    let a: i32 = run(true, 3);
    let taken_calls: i32 = calls;
    let b: i32 = run(false, 3);
    let all_calls: i32 = calls;
    return a + b + (taken_calls - 3) * 10 + (all_calls - 3) * 10 - 6;
}
"#;

/// The guarded element root fills inside the guard, the residual releases.
///
/// The deferred fill assigns `mk()` into the guard temporary on the `g ?`
/// expression itself, reads the member through it, and the root's remaining
/// field releases with the statement; the eager `__cl_owned0` binding is gone.
#[test]
fn should_defer_a_guarded_call_wrapped_element_root() {
    let tag = "implguardcall";
    let (_scratch, emitted) = emit_host(tag, GUARD_CALL_ROOT_SOURCE);
    let body = function_body(tag, &emitted, "cl_spec_cprimpl_implguardcall_run(bool g, i32 k)\n{");
    assert_on_line_of(
        tag,
        body,
        "g ?",
        "__cl_tmp0 = cl_spec_cprimpl_implguardcall_mk()",
    );
    assert_on_line_of(
        tag,
        body,
        "g ?",
        "cl_string_view(__cl_tmp0.v)",
    );
    assert_absent(tag, body, "__cl_owned0");
    assert_contains(
        tag,
        body,
        "if (__cl_tmp0.w) { FreeString(__cl_tmp0.w); }",
    );
}

/// A conditional carrier literal's taken arm root.
const CARRIER_LITERAL_SOURCE: &str = r#"module spec.cprimpl.implcarrierlit;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Holder { names: str[1]; }

func mk(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }

func run(g: bool, want: i32): i32 {
    let h: Holder = g ? Holder { names: [mk().v] } : Holder { names: ["zz"] };
    return i32(h.names[0][0]) - want;
}

func main(): i32 {
    return run(true, 97) + run(false, 122);
}
"#;

/// The taken arm reads through a scoped owner the local's frame releases.
#[test]
fn should_bind_the_taken_carrier_arm_root() {
    let tag = "implcarrierlit";
    let (_scratch, emitted) = emit_host(tag, CARRIER_LITERAL_SOURCE);
    let body = function_body(tag, &emitted, "cl_spec_cprimpl_implcarrierlit_run(bool g, i32 want)\n{");
    assert_contains(tag, body, "ClBox __cl_owned0 = {0};");
    assert_contains(tag, body, "__cl_owned0 = cl_spec_cprimpl_implcarrierlit_mk();");
    assert_contains(
        tag,
        body,
        "__cl_arm_2.names[0] = cl_string_view(__cl_owned0.v);",
    );
    assert_release_after(
        tag,
        body,
        "h = __cl_arm_2;",
        "if (__cl_owned0.w) { FreeString(__cl_owned0.w); }",
    );
    assert_absent(
        tag,
        body,
        "cl_string_view(cl_spec_cprimpl_implcarrierlit_mk().v)",
    );
}

/// A discarded non-owning arm over a residual-owning root.
const DISCARD_RESIDUAL_SOURCE: &str = r#"module spec.cprimpl.impldiscardres;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func mk(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }

func use(c: bool, t: i32): i32 {
    c ? i32(mk().v.len()) : t;
    return t - 5;
}

func main(): i32 {
    return use(true, 5) + use(false, 5);
}
"#;

/// A bare discarded array literal materializes and releases its leaf.
const BARE_DISCARD_SOURCE: &str = r#"module spec.cprimpl.implbarediscard;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func mk(): Box { return Box { v: stringOf("hello"), w: stringOf("world") }; }

func use(): i32 {
    [mk().v];
    return 0;
}

func looped(k: i32): i32 {
    let i: i32 = 0;
    while (i < k) {
        [mk().w];
        i = i + 1;
    }
    return i - k;
}

func main(): i32 {
    return use() + looped(3);
}
"#;

/// The discarded arm's root binds and releases its residual field.
#[test]
fn should_bind_the_discarded_arms_residual_root() {
    let tag = "impldiscardres";
    let (_scratch, emitted) = emit_host(tag, DISCARD_RESIDUAL_SOURCE);
    let body = function_body(tag, &emitted, "cl_spec_cprimpl_impldiscardres_use(bool c, i32 t)\n{");
    assert_on_line_of(
        tag,
        body,
        "(c ?",
        "__cl_tmp0 = cl_spec_cprimpl_impldiscardres_mk()",
    );
    assert_on_line_of(
        tag,
        body,
        "(c ?",
        "__cl_tmp0.w ? (FreeString(__cl_tmp0.w), __cl_tmp0.w = NULL) : 0)",
    );
    assert_contains(tag, body, "(i32)((cl_string_view(__cl_tmp0.v)).len)");
    assert_absent(tag, body, "__cl_tmp0 = cl_spec_cprimpl_impldiscardres_mk().v");
}

/// The bare discarded literal binds its leaf and emits no statement expression.
#[test]
fn should_materialize_a_bare_discarded_literal_element() {
    let tag = "implbarediscard";
    let (_scratch, emitted) = emit_host(tag, BARE_DISCARD_SOURCE);
    let body = function_body(tag, &emitted, "cl_spec_cprimpl_implbarediscard_use(void)\n{");
    assert_follows(tag, body, "__cl_tmp1 = __cl_tmp0.v;", "__cl_tmp0.v = NULL;");
    assert_release_after(
        tag,
        body,
        "__cl_tmp0.v = NULL;",
        "if (__cl_tmp1) { FreeString(__cl_tmp1); }",
    );
    assert_absent(tag, body, "(void)({");
    let looped =
        function_body(tag, &emitted, "cl_spec_cprimpl_implbarediscard_looped(i32 k)\n{");
    assert_follows(tag, looped, "__cl_tmp1 = __cl_tmp0.w;", "__cl_tmp0.w = NULL;");
    assert_release_after(
        tag,
        looped,
        "__cl_tmp0.w = NULL;",
        "if (__cl_tmp1) { FreeString(__cl_tmp1); }",
    );
    assert_absent(tag, looped, "(void)({");
}

/// The phase-2 programs re-emit byte-identically.
#[test]
fn should_re_emit_the_guarded_and_discarded_phase_two_programs_byte_identically() {
    for (tag, source) in [
        ("implguardcall", GUARD_CALL_ROOT_SOURCE),
        ("implcarrierlit", CARRIER_LITERAL_SOURCE),
        ("impldiscardres", DISCARD_RESIDUAL_SOURCE),
        ("implbarediscard", BARE_DISCARD_SOURCE),
    ] {
        let (first, second) = emit_twice(tag, source);
        assert!(
            first == second,
            "{tag}: the two emissions differ; first:\n{first}\nsecond:\n{second}"
        );
    }
}

/// A conditional fixed-array field in a struct-literal store.
const CONDITIONAL_FIELD_SOURCE: &str = r#"module spec.cprimpl.implcondfield;

import { String, stringOf } from std.text;

struct Box { row: String[2]; tag: i32; }
struct Holder { row: String[2]; tag: i32; }

func mkBox(): Box { return Box { row: [stringOf("ab"), stringOf("cd")], tag: 7 }; }

func place(c: bool, take a: Box, take b: Box): i32 {
    let h: Holder = Holder { row: c ? a.row : b.row, tag: 4 };
    return h.tag - 4;
}

func fresh(c: bool, take a: Box): i32 {
    let h: Holder = Holder { row: c ? a.row : mkBox().row, tag: 2 };
    return h.tag - 2;
}

func rooted(c: bool, take a: Box, take b: Box): i32 {
    let h: Holder = Holder { row: (c ? a : b).row, tag: 8 };
    return h.tag - 8;
}

func main(): i32 {
    let a1: Box = Box { row: [stringOf("xx"), stringOf("yy")], tag: 1 };
    let b1: Box = Box { row: [stringOf("pp"), stringOf("qq")], tag: 2 };
    let n: i32 = place(true, a1, b1);
    let a2: Box = Box { row: [stringOf("xx"), stringOf("yy")], tag: 1 };
    let b2: Box = Box { row: [stringOf("pp"), stringOf("qq")], tag: 2 };
    let m: i32 = place(false, a2, b2);
    let a3: Box = Box { row: [stringOf("xx"), stringOf("yy")], tag: 1 };
    let o: i32 = fresh(false, a3);
    let a4: Box = Box { row: [stringOf("xx"), stringOf("yy")], tag: 1 };
    let b4: Box = Box { row: [stringOf("pp"), stringOf("qq")], tag: 2 };
    let p: i32 = rooted(true, a4, b4);
    return n + m + o + p;
}
"#;

/// Each arm of a conditional fixed-array field fills the field on its own
/// path, and an ownerless arm binds its root before the copy.
#[test]
fn should_store_a_conditional_fixed_array_field_per_arm() {
    let tag = "implcondfield";
    let (_scratch, emitted) = emit_host(tag, CONDITIONAL_FIELD_SOURCE);
    let place = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_implcondfield_place(bool c, ClBox a, ClBox b)\n{",
    );
    assert_contains(tag, place, "memcpy(&(h.row), &(a.row), sizeof(h.row));");
    assert_contains(tag, place, "cl_zero(&a.row);");
    assert_contains(tag, place, "memcpy(&(h.row), &(b.row), sizeof(h.row));");
    assert_contains(tag, place, "cl_zero(&b.row);");
    let fresh = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_implcondfield_fresh(bool c, ClBox a)\n{",
    );
    assert_contains(
        tag,
        fresh,
        "__cl_tmp0 = cl_spec_cprimpl_implcondfield_mkBox();",
    );
    assert_contains(tag, fresh, "memcpy(&(h.row), &(__cl_tmp0.row), sizeof(h.row));");
    assert_contains(tag, fresh, "cl_zero(&__cl_tmp0.row);");
    let rooted = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_implcondfield_rooted(bool c, ClBox a, ClBox b)\n{",
    );
    assert_contains(tag, rooted, "memcpy(&(h.row), &(a.row), sizeof(h.row));");
    assert_contains(tag, rooted, "memcpy(&(h.row), &(b.row), sizeof(h.row));");
    assert_absent(tag, rooted, "&((c ? a : b).row)");
    assert_absent(tag, rooted, "cl_zero(&(c ?");
}

/// A `String[2]` conditional-root return by value.
const CONDITIONAL_RETURN_SOURCE: &str = r#"module spec.cprimpl.implcondreturn;

import { String, stringOf } from std.text;

struct Box { xs: String[2]; }

func pick(c: bool): String[2] {
    let b1: Box = Box { xs: [stringOf("aa"), stringOf("bb")] };
    let b2: Box = Box { xs: [stringOf("cc"), stringOf("dd")] };
    return (c ? b1 : b2).xs;
}

func main(): i32 {
    let r: String[2] = pick(true);
    return i32(len(r[0])) + i32(len(r[1])) - 4;
}
"#;

/// The conditional root's return copies into the hidden destination per arm,
/// because C cannot assign an array to the adjusted parameter.
#[test]
fn should_copy_a_conditional_string_array_return_per_arm() {
    let tag = "implcondreturn";
    let (_scratch, emitted) = emit_host(tag, CONDITIONAL_RETURN_SOURCE);
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_implcondreturn_pick(bool c, ClString __cl_ret[2])\n{",
    );
    assert_contains(tag, body, "memcpy(&(__cl_tmp1), &(b1.xs), sizeof(__cl_tmp1));");
    assert_contains(tag, body, "memcpy(&(__cl_tmp2), &(b2.xs), sizeof(__cl_tmp2));");
    assert_contains(
        tag,
        body,
        "memcpy(__cl_ret, &(__cl_tmp1), (sizeof(__cl_ret[0]) * 2));",
    );
    assert_contains(tag, body, "cl_zero(&__cl_tmp1);");
    assert_contains(
        tag,
        body,
        "memcpy(__cl_ret, &(__cl_tmp2), (sizeof(__cl_ret[0]) * 2));",
    );
    assert_contains(tag, body, "cl_zero(&__cl_tmp2);");
    assert_release_after(
        tag,
        body,
        "if (__cl_ret[1]) { FreeString(__cl_ret[1]); }",
        "memcpy(__cl_ret, &(__cl_tmp1)",
    );
    assert_absent(tag, body, "__cl_ret = __cl_tmp");
}

/// A literal view-array field inside a guarded operand.
const GUARDED_VIEW_FIELD_SOURCE: &str = r#"module spec.cprimpl.implguardview;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Holder { names: str[1]; }

func mk(): Box { return Box { v: stringOf("ab"), w: stringOf("cd") }; }

func use(g: bool): i32 {
    if (g && i32(len(Holder { names: [mk().v] }.names[0])) > 0) {
        return 1;
    }
    return 0;
}

func main(): i32 { return use(true) + use(false) - 1; }
"#;

/// A guarded literal's view element binds its root through the deferred fill
/// and stores the widened view, so the initializer carries a complete view.
#[test]
fn should_fill_a_guarded_view_array_field_element_wise() {
    let tag = "implguardview";
    let (_scratch, emitted) = emit_host(tag, GUARDED_VIEW_FIELD_SOURCE);
    let body = function_body(tag, &emitted, "cl_spec_cprimpl_implguardview_use(bool g)\n{");
    assert_contains(tag, body, "ClBox __cl_tmp0 = {0};");
    assert_on_line_of(
        tag,
        body,
        "(g &&",
        "__cl_tmp0 = cl_spec_cprimpl_implguardview_mk()",
    );
    assert_on_line_of(tag, body, "(g &&", "cl_string_view(__cl_tmp0.v)");
    assert_absent(tag, body, "{ { cl_spec_cprimpl_implguardview_mk().v } }");
}

/// Call-filled fixed-array fields of literal arms and array elements.
const CALL_FILLED_FIELD_SOURCE: &str = r#"module spec.cprimpl.implcallfield;

import { String, stringOf } from std.text;

struct Mixed { names: str[2]; tag: String; }
struct Inner { row: str[2]; }
struct Outer { items: Inner[2]; }

let left: i32 = 0;
let right: i32 = 0;

func n1(): str[2] { left = left + 1; return ["AA", "BB"]; }
func n2(): str[2] { right = right + 1; return ["CC", "DD"]; }

func use(ok: bool): i32 {
    ok ? Mixed { names: n1(), tag: stringOf("x") } : Mixed { names: n2(), tag: stringOf("y") };
    let o: Outer = Outer { items: [ok ? Inner { row: n1() } : Inner { row: n2() }, Inner { row: ["zz", "ww"] }] };
    return i32(o.items[0].row[0][0]) + left + right - 66;
}

func main(): i32 { return use(true) + use(false); }
"#;

/// Every call-filled fixed-array field passes the call's hidden destination,
/// per arm and per element.
#[test]
fn should_pass_the_hidden_destination_to_call_filled_literal_fields() {
    let tag = "implcallfield";
    let (_scratch, emitted) = emit_host(tag, CALL_FILLED_FIELD_SOURCE);
    let body = function_body(tag, &emitted, "cl_spec_cprimpl_implcallfield_use(bool ok)\n{");
    // Each call-filled fixed-array field passes the call's hidden destination
    // on its own path; the per-arm carrier is the one that reaches the store.
    assert_contains(tag, body, "cl_spec_cprimpl_implcallfield_n1(__cl_call_");
    assert_contains(tag, body, "cl_spec_cprimpl_implcallfield_n2(__cl_call_");
    assert_absent(tag, body, ".names = cl_spec_cprimpl_implcallfield_n");
    assert_contains(tag, body, "ClInner __cl_arm_");
    assert_contains(tag, body, "o.items[0] = __cl_arm_");
    assert_absent(tag, body, "(ok ? (ClInner)");
}

/// A nested conditional-arm fixed-array read base.
const NESTED_READ_BASE_SOURCE: &str = r#"module spec.cprimpl.implnestedread;

struct Box { ys: u8[4]; }

func read(c0: bool, d: bool): i32 {
    let b1: Box = Box { ys: [1, 2, 3, 4] };
    let b2: Box = Box { ys: [5, 6, 7, 8] };
    let b3: Box = Box { ys: [9, 10, 11, 12] };
    let c: bool = c0;
    let s: i32 = 0;
    let k: i32 = 0;
    while (k < 2) {
        let i: i32 = 0;
        while (i < 4) {
            s = s + i32((c ? (d ? b1 : b2).ys : b3.ys)[i]);
            i = i + 1;
        }
        c = !c;
        k = k + 1;
    }
    return s;
}

func main(): i32 {
    let r: i32 = read(true, true) - 52;
    return r + read(true, false) - 68;
}
"#;

/// The nested-arm read base distributes into its leaves, so each arm copies
/// from its own storage and no member of a C `?:` is addressed.
#[test]
fn should_distribute_a_nested_element_read_base() {
    let tag = "implnestedread";
    let (_scratch, emitted) = emit_host(tag, NESTED_READ_BASE_SOURCE);
    let body = function_body(tag, &emitted, "cl_spec_cprimpl_implnestedread_read(bool c0, bool d)\n{");
    assert_contains(tag, body, "memcpy(&(__cl_tmp0), &(b1.ys), sizeof(__cl_tmp0));");
    assert_contains(tag, body, "memcpy(&(__cl_tmp0), &(b2.ys), sizeof(__cl_tmp0));");
    assert_contains(tag, body, "memcpy(&(__cl_tmp0), &(b3.ys), sizeof(__cl_tmp0));");
    assert_contains(tag, body, "__cl_tmp0[i]");
    assert_absent(tag, body, "(d ? b1 : b2).ys");
    assert_follows(tag, body, "if (c) {", "if (d) {");
}

/// The fixed-array emission programs re-emit byte-identically, and the
/// re-emitted text carries the repaired shape.
#[test]
fn should_re_emit_the_fixed_array_emission_programs_byte_identically() {
    for (tag, source, shape) in [
        (
            "implcondreturn",
            CONDITIONAL_RETURN_SOURCE,
            "memcpy(__cl_ret, &(__cl_tmp",
        ),
        (
            "implnestedread",
            NESTED_READ_BASE_SOURCE,
            "memcpy(&(__cl_tmp0), &(b1.ys)",
        ),
    ] {
        let (first, second) = emit_twice(tag, source);
        assert!(
            first == second,
            "{tag}: the two emissions differ; first:\n{first}\nsecond:\n{second}"
        );
        assert_contains(tag, &first, shape);
    }
}

/// A mixed-arm func-field receiver in a loop condition and in an eager read.
const SETTLED_RECEIVER_SOURCE: &str = r#"module spec.cprimpl.implrecvsettle;

import { String, stringOf } from std.text;

struct H { call: func(i32): i32; s: String; }

func addTen(x: i32): i32 { return x + 10; }
func neg(x: i32): i32 { return 0 - x; }
func mkH(): H { return H { call: addTen, s: stringOf("mk") }; }

func looped(c: bool, take a: H, k: i32): i32 {
    let n: i32 = 0;
    let i: i32 = 0;
    while (i < k && (c ? mkH() : a).call(5) > 0) {
        n = n + 1;
        i = i + 1;
    }
    return n;
}

func eager(c: bool, a: H): i32 {
    return (c ? mkH() : a).call(5) - 15;
}

func main(): i32 {
    let a1: H = H { call: neg, s: stringOf("aa") };
    let n: i32 = looped(true, a1, 1) - 1;
    let a2: H = H { call: neg, s: stringOf("bb") };
    return n + eager(true, a2);
}
"#;

/// The receiver is the call's argument position zero, so a receiver that is
/// not a place settles into a deferred temporary the designator reads: the
/// loop condition fills it per iteration and the eager read fills it before
/// the call, so no arm's evaluation nests inside the designator.
#[test]
fn should_settle_a_mixed_arm_receiver_at_argument_position_zero() {
    let tag = "implrecvsettle";
    let (_scratch, emitted) = emit_host(tag, SETTLED_RECEIVER_SOURCE);
    let looped = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_implrecvsettle_looped(bool c, ClH a, i32 k)\n{",
    );
    // A loop condition's fills live in the condition's own per-iteration
    // placement, so the borrowing receiver keeps its deferred spelling: the
    // fresh arm binds its own temporary and the place arm stays a read.
    assert_contains(
        tag,
        looped,
        "(__cl_tmp0 = cl_spec_cprimpl_implrecvsettle_mkH())",
    );
    assert_contains(tag, looped, " : a).call)(5)");
    assert_absent(tag, looped, "(__cl_tmp1 = ");
    let eager = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_implrecvsettle_eager(bool c, ClH a)\n{",
    );
    assert_contains(
        tag,
        eager,
        "__cl_tmp1 = cl_spec_cprimpl_implrecvsettle_mkH();",
    );
    assert_contains(tag, eager, "(__cl_tmp0 ? __cl_tmp1 : a).call)(5)");
    assert_absent(tag, &emitted, "cl_zero(&a)");
    assert_release_after(
        tag,
        eager,
        "(__cl_tmp0 ? __cl_tmp1 : a)",
        "if (__cl_tmp1.s) { FreeString(__cl_tmp1.s); }",
    );
}

/// A guarded operand whose updating part must settle inside the guard.
const GUARD_UPDATE_SOURCE: &str = r#"module spec.cprimpl.implguardsettle;

let calls: i32 = 0;

func f(a: i32, b: i32): i32 {
    calls = calls + 1;
    return a * 10 + b;
}

func guardAnd(g: bool): i32 {
    let i: i32 = 3;
    let ok: bool = g && f(i++, i) == 34;
    return (ok ? 0 : 1) + (i - 4);
}

func ternary(c: bool): i32 {
    let i: i32 = 3;
    let r: i32 = c ? f(i++, i) : 0;
    return r - (c ? 34 : 0) + i - (c ? 4 : 3);
}

func plain(): i32 {
    let i: i32 = 0;
    let n: i32 = 0;
    while (f(i++, i) < 30) {
        n = n + 1;
    }
    return n;
}

func main(): i32 {
    let n: i32 = guardAnd(true) + ternary(true) + plain() - 3;
    return n + (calls - 5) * 10;
}
"#;

/// The guarded operand's updating part fills a deferred temporary inside the
/// guard, and the call reads that temporary beside the settled `i`, so the
/// argument values stay left-to-right on the evaluated path only. The plain
/// loop condition keeps its per-iteration argument fills.
#[test]
fn should_settle_the_guarded_operands_updating_parts() {
    let tag = "implguardsettle";
    let (_scratch, emitted) = emit_host(tag, GUARD_UPDATE_SOURCE);
    let guard_and = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_implguardsettle_guardAnd(bool g)\n{",
    );
    assert_on_line_of(tag, guard_and, "(g &&", "__cl_tmp0 = (i)++");
    assert_on_line_of(
        tag,
        guard_and,
        "(g &&",
        "cl_spec_cprimpl_implguardsettle_f(__cl_tmp0, i)",
    );
    assert_absent(tag, guard_and, "_f((i)++, i)");
    let ternary = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_implguardsettle_ternary(bool c)\n{",
    );
    assert_on_line_of(tag, ternary, "(c ?", "__cl_tmp0 = (i)++");
    assert_on_line_of(
        tag,
        ternary,
        "(c ?",
        "cl_spec_cprimpl_implguardsettle_f(__cl_tmp0, i)",
    );
    assert_absent(tag, ternary, "_f((i)++, i)");
    let plain = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_implguardsettle_plain(void)\n{",
    );
    assert_contains(
        tag,
        plain,
        "cl_spec_cprimpl_implguardsettle_f(__cl_tmp0, __cl_tmp1)",
    );
}

/// An update-bearing array fill and the element-list control.
const FILL_PIN_SOURCE: &str = r#"module spec.cprimpl.implfillpin;

func fillOnce(): i32 {
    let i: i32 = 7;
    let xs: i32[3] = [; i++];
    return xs[0] + xs[1] + xs[2] - 21 + (i - 8);
}

func elementList(): i32 {
    let i: i32 = 0;
    let b: i32[3] = [i++, i++, i++];
    return b[0] + b[1] + b[2] - 3 + (i - 3);
}

func main(): i32 { return fillOnce() + elementList(); }
"#;

/// The fill's value copies to every remaining element, so an updating fill
/// pins once at its own position and the element stores read the temporary;
/// the element list keeps its own per-element updates.
#[test]
fn should_pin_an_array_fill_once_before_the_element_stores() {
    let tag = "implfillpin";
    let (_scratch, emitted) = emit_host(tag, FILL_PIN_SOURCE);
    let filled = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_implfillpin_fillOnce(void)\n{",
    );
    assert_follows(tag, filled, "__cl_tmp0 = (i)++;", "xs[0] = __cl_tmp0;");
    assert_contains(tag, filled, "xs[1] = __cl_tmp0;");
    assert_contains(tag, filled, "xs[2] = __cl_tmp0;");
    assert_absent(tag, filled, "xs[0] = (i)++;");
    let listed = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_implfillpin_elementList(void)\n{",
    );
    assert_contains(tag, listed, "b[0] = __cl_tmp0;");
    assert_contains(tag, listed, "b[1] = __cl_tmp1;");
    assert_contains(tag, listed, "b[2] = __cl_tmp2;");
}

/// A bare raw-deref store with its update twin and its working controls.
const RAW_STORE_SOURCE: &str = r#"module spec.cprimpl.implrawstore;

struct S { v: i32; w: i32; }

func main(): i32 {
    let a: i32 = 0;
    let q: raw i32 = unsafe &a;
    unsafe { *q = 9; }
    let v: i32 = unsafe *q;
    let i: i32 = 0;
    let b: i32 = 0;
    let r: raw i32 = unsafe &b;
    unsafe { *r = i++; }
    let s: S = S { v: 0, w: 0 };
    let p: raw S = unsafe &s;
    unsafe { p.v = 7; }
    return (a - 9) + (v - 9) + b + (i - 1) + (s.v - 7);
}
"#;

/// A dereferenced raw pointer stores through the pointer value, and an
/// updating right-hand side pins at its position before that store. The bare
/// deref read and the field write through a raw keep their shapes.
#[test]
fn should_lower_a_bare_raw_deref_store() {
    let tag = "implrawstore";
    let (_scratch, emitted) = emit_host(tag, RAW_STORE_SOURCE);
    let body = function_body(tag, &emitted, "cl_spec_cprimpl_implrawstore_main(void)\n{");
    assert_contains(tag, body, "*(&(*(q))) = 9;");
    assert_follows(tag, body, "__cl_tmp0 = (i)++;", "*(&(*(r))) = __cl_tmp0;");
    assert_absent(tag, body, "*(r) = (i)++;");
    assert_contains(tag, body, "v = *(q);");
    assert_contains(tag, body, "p->v = 7;");
}

/// The settled-receiver and raw-deref programs re-emit byte-identically, and
/// the re-emitted text carries the repaired shape.
#[test]
fn should_re_emit_the_settled_receiver_and_raw_store_programs_byte_identically() {
    for (tag, source, shape) in [
        (
            "implrecvsettle",
            SETTLED_RECEIVER_SOURCE,
            "(__cl_tmp0 = cl_spec_cprimpl_implrecvsettle_mkH())",
        ),
        ("implrawstore", RAW_STORE_SOURCE, "*(&(*(q))) = 9;"),
    ] {
        let (first, second) = emit_twice(tag, source);
        assert!(
            first == second,
            "{tag}: the two emissions differ; first:\n{first}\nsecond:\n{second}"
        );
        assert_contains(tag, &first, shape);
    }
}

// ---------------------------------------------------------------------------
// The one-report sites. A nested mixed view return and a two-owner array
// literal each keep one report at their own span, a global-field store keeps
// its callee's report alone instead of adding a consequence report, and the
// deferred generated owner renders the shared phrase rather than the internal
// binding name.
// ---------------------------------------------------------------------------

/// A nested mixed view return: one local arm beside two fresh arms, so two
/// generated owners back the one expression.
const NESTED_VIEW_RETURN_SOURCE: &str = r#"module spec.cprimpl.implnested;

import { String, stringOf } from std.text;

struct Box { v: String; }

func mkBox(): Box { return Box { v: stringOf("ab") }; }
func mkBox2(): Box { return Box { v: stringOf("ef") }; }

func f(c: bool, d: bool): str {
    let a: Box = Box { v: stringOf("cd") };
    return (c ? a : (d ? mkBox() : mkBox2())).v;
}

func main(): i32 { return 0; }
"#;

/// A `Pair` array literal that stores views of two locals, so two owners'
/// loans escape at the one store.
const PAIR_VIEW_STORE_SOURCE: &str = r#"module spec.cprimpl.implpair;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct Pair { names: str[2]; }

func pick(): i32 {
    let h: Pair = Pair { names: ["AA", "BB"] };
    {
        let b: Box = Box { v: stringOf("xy"), w: stringOf("zz") };
        let c: Box = Box { v: stringOf("pq"), w: stringOf("rs") };
        h = Pair { names: [b.v, c.v] };
    }
    return i32(h.names[0][0]);
}

func main(): i32 { return pick() - 120; }
"#;

/// A view return whose elision names no parameter, stored into a global
/// field.
const GLOBAL_FIELD_STORE_SOURCE: &str = r#"module spec.cprimpl.implgfield;

let s: str = "ab";
let t: str = "cd";

struct Holder { names: str[2]; }

let g: Holder = Holder { names: ["AA", "BB"] };

func row(): str[2] { return [s, t]; }

func store(): void { g.names = row(); }

func main(): i32 { store(); return i32(len(g.names[1])) - 2; }
"#;

/// A deferred mixed view return whose owner is generated inside the deferred
/// body.
const DEFERRED_RETURN_SOURCE: &str = r#"module spec.cprimpl.impldefer;

import { String, stringOf } from std.text;

struct Box { v: String; }

func mkBox(): Box { return Box { v: stringOf("ab") }; }

func f(a: str, c: bool): str {
    defer {
        let l: Box = Box { v: stringOf("cd") };
        return (c ? l : mkBox()).v;
    }
    return a;
}

func main(): i32 { return 0; }
"#;

/// The nested mixed return and the two-owner array literal each keep exactly
/// one report at their own span, whichever owner the pair's report names.
#[test]
fn should_report_once_at_the_nested_and_pair_view_returns() {
    assert_rejected_once_at(
        "implnested",
        NESTED_VIEW_RETURN_SOURCE,
        "E10030",
        12,
        5,
        &[
            "borrow of a compiler-generated owner",
            "does not live long enough",
        ],
        &["__cl_"],
    );
    assert_rejected_once_at(
        "implpair",
        PAIR_VIEW_STORE_SOURCE,
        "E10030",
        13,
        9,
        &["does not live long enough"],
        &[],
    );
}

/// The global-field store keeps its callee's report alone: the downstream
/// consequence report is withheld, so the program carries no second report.
#[test]
fn should_withhold_the_global_field_consequence_after_the_callee_report() {
    assert_rejected_once_at(
        "implgfield",
        GLOBAL_FIELD_STORE_SOURCE,
        "E10036",
        10,
        6,
        &["view return does not derive from exactly one parameter"],
        &["E10033"],
    );
}

/// The deferred generated owner renders the shared phrase, never the internal
/// binding name.
#[test]
fn should_render_the_deferred_generated_owner_phrase() {
    assert_rejected_once_at(
        "impldefer",
        DEFERRED_RETURN_SOURCE,
        "E10030",
        12,
        9,
        &["borrow of a compiler-generated owner"],
        &["__cl_"],
    );
}

// ---------------------------------------------------------------------------
// The whole-array conditional iterable. The loop reads the selected arm's
// elements in place, so both arrays stay usable after the loop, while the
// element and projection spellings keep their own move marks.
// ---------------------------------------------------------------------------

/// A whole-array conditional iterable with owned `String[2]` arms.
const ITERABLE_SOURCE: &str = r#"module spec.cprimpl.impliterable;

import { String, stringOf } from std.text;

func use(c: bool): i32 {
    let b1: String[2] = [stringOf("AA"), stringOf("BBB")];
    let b2: String[2] = [stringOf("CCCC"), stringOf("DD")];
    let s: i32 = 0;
    for (borrow x in (c ? b1 : b2)) {
        s = s + i32(len(x));
    }
    return s + i32(len(b1[0])) + i32(len(b2[1]));
}

func main(): i32 {
    return (use(true) - 9) + (use(false) - 10);
}
"#;

/// The duplicable `u8[4]` twin of the conditional iterable.
const ITERABLE_U8_SOURCE: &str = r#"module spec.cprimpl.impliterableu8;

func use(c: bool): i32 {
    let b1: u8[4] = [1, 2, 3, 4];
    let b2: u8[4] = [5, 6, 7, 8];
    let s: i32 = 0;
    for (borrow x in (c ? b1 : b2)) {
        s = s + i32(x);
    }
    return s + i32(b1[0]) + i32(b2[1]);
}

func main(): i32 {
    return (use(true) - 17) + (use(false) - 33);
}
"#;

/// A conditional iterable beside an element read of a second conditional: the
/// loop keeps its arms usable while the read still marks its own.
const ITERABLE_ELEMENT_READ_SOURCE: &str = r#"module spec.cprimpl.implcombined;

import { String, stringOf } from std.text;

func use(c: bool, d: bool): i32 {
    let b1: String[2] = [stringOf("AA"), stringOf("BBB")];
    let b2: String[2] = [stringOf("CCCC"), stringOf("DD")];
    let p1: String[2] = [stringOf("EE"), stringOf("FF")];
    let p2: String[2] = [stringOf("GG"), stringOf("HH")];
    let s: i32 = 0;
    for (borrow x in (c ? b1 : b2)) {
        s = s + i32(len(x));
    }
    let n: i32 = i32(len((d ? p1 : p2)[0]));
    return s + n + i32(len(b1[0])) + i32(len(p1[1]));
}

func main(): i32 { return use(true, true) - 6; }
"#;

/// A conditional projection whose selected member is consumed, read back
/// through the moved member.
const PROJECTION_READ_SOURCE: &str = r#"module spec.cprimpl.implprojread;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func mk(): Box { return Box { v: stringOf("mk"), w: stringOf("wwww") }; }

func use(c: bool): i32 {
    let b: Box = Box { v: stringOf("ab"), w: stringOf("wwww") };
    let s: String = (c ? b : mk()).v;
    let t: i32 = i32(len(b.v));
    return i32(len(s)) + t - 4;
}

func main(): i32 { return use(true); }
"#;

/// The whole-array conditional iterable is read in place on both element
/// kinds: the loop takes the address of the selected arm's element per
/// iteration instead of materializing an arm, so the trailing reads of both
/// arrays stay in the emission.
#[test]
fn should_keep_the_whole_array_conditional_iterable_in_place() {
    let owned = "impliterable";
    let (_scratch, emitted) = emit_host(owned, ITERABLE_SOURCE);
    let body = function_body(owned, &emitted, "cl_spec_cprimpl_impliterable_use(bool c)\n{");
    assert_contains(
        owned,
        body,
        "for (; (__cl_tmp0 < 2); __cl_tmp0 = (__cl_tmp0 + 1u)) {",
    );
    assert_contains(
        owned,
        body,
        "__cl_tmp1 = &((__cl_tmp2 ? b1 : b2)[__cl_tmp0]);",
    );
    assert_contains(owned, body, "cl_string_view((*__cl_tmp1))");
    let duplicable = "impliterableu8";
    let (_scratch, emitted) = emit_host(duplicable, ITERABLE_U8_SOURCE);
    let body = function_body(
        duplicable,
        &emitted,
        "cl_spec_cprimpl_impliterableu8_use(bool c)\n{",
    );
    assert_contains(
        duplicable,
        body,
        "for (; (__cl_tmp0 < 4); __cl_tmp0 = (__cl_tmp0 + 1u)) {",
    );
    assert_contains(
        duplicable,
        body,
        "__cl_tmp1 = &((__cl_tmp2 ? b1 : b2)[__cl_tmp0]);",
    );
    assert_contains(duplicable, body, "((*__cl_tmp1))");
}

/// The element and projection spellings keep their own move marks beside the
/// in-place iterable. The mixed program's single report is the element read's
/// mark: the loop's later reads stay accepted, and a re-added iterable mark
/// would add a second report, while a dropped element mark would leave none.
/// The projection guard reports the moved member of its selected arm.
#[test]
fn should_report_the_element_and_projection_marks_beside_the_iterable() {
    assert_rejected_at_line(
        "implcombined",
        ITERABLE_ELEMENT_READ_SOURCE,
        "E10020",
        1,
        15,
    );
    assert_rejected_at_line("implprojread", PROJECTION_READ_SOURCE, "E10020", 1, 12);
}

/// The conditional iterable programs re-emit byte-identically, and the
/// re-emitted text keeps the in-place element address.
#[test]
fn should_re_emit_the_conditional_iterable_programs_byte_identically() {
    for (tag, source, shape) in [
        (
            "impliterable",
            ITERABLE_SOURCE,
            "__cl_tmp1 = &((__cl_tmp2 ? b1 : b2)[__cl_tmp0]);",
        ),
        (
            "impliterableu8",
            ITERABLE_U8_SOURCE,
            "__cl_tmp1 = &((__cl_tmp2 ? b1 : b2)[__cl_tmp0]);",
        ),
    ] {
        let (first, second) = emit_twice(tag, source);
        assert!(
            first == second,
            "{tag}: the two emissions differ; first:\n{first}\nsecond:\n{second}"
        );
        assert_contains(tag, &first, shape);
    }
}

// ---------------------------------------------------------------------------
// The whole-array copy's element loans. A copy of a view-like array hands its
// element loans to the destination, so a later store through the copy reports
// the frozen conflict once per copy spelling; the parameter and read
// spellings stay accepted with the copy and the reads in place.
// ---------------------------------------------------------------------------

/// A whole-`str[2]` copy whose first element reads a live local view.
const VIEW_COPY_LOCAL_SOURCE: &str = r#"module spec.cprimpl.viewcopylocal;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa"), w: stringOf("bb") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let ys: str[2] = xs;
    fill(ys[0], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;

/// The overwriting-assignment spelling of the whole-`str[2]` copy.
const VIEW_COPY_ASSIGN_SOURCE: &str = r#"module spec.cprimpl.viewcopyassign;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa"), w: stringOf("bb") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let ys: str[2] = ["CC", "DD"];
    ys = xs;
    fill(ys[0], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;

/// The element-loop spelling of the whole-`str[2]` copy: both copied
/// elements read the local view, and the loop stores one of them through.
const VIEW_COPY_LOOP_SOURCE: &str = r#"module spec.cprimpl.viewcopyloop;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa"), w: stringOf("bb") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    xs[1] = a.w;
    let ys: str[2] = xs;
    for (borrow e in ys) {
        fill(e, borrow slot);
    }
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;

/// The `take`-source spelling of the whole-`str[2]` copy: the copy's element
/// reads the taken parameter's live view.
const VIEW_COPY_TAKE_SOURCE: &str = r#"module spec.cprimpl.viewcopytake;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(take p: Box, borrow slot: str?): void {
    let xs: str[2] = ["AA", "BB"];
    xs[0] = p.v;
    let ys: str[2] = xs;
    fill(ys[0], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    let b: Box = Box { v: stringOf("aa"), w: stringOf("bb") };
    outer(b, borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;

/// The accepted parameter-source control: the copied element derives from the
/// caller's by-value parameter, whose storage outlives the callee.
const VIEW_COPY_PARAM_SOURCE: &str = r#"module spec.cprimpl.viewcopyparam;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(p: Box, borrow slot: str?): void {
    let xs: str[2] = ["AA", "BB"];
    xs[0] = p.v;
    let ys: str[2] = xs;
    fill(ys[0], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    let b: Box = Box { v: stringOf("aa"), w: stringOf("bb") };
    outer(b, borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;

/// The accepted read-back control: the copied array is read through both
/// elements, so the copy must stay a full two-element transfer.
const VIEW_COPY_READ_SOURCE: &str = r#"module spec.cprimpl.viewcopyread;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func outer(): i32 {
    let a: Box = Box { v: stringOf("aa"), w: stringOf("bb") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let ys: str[2] = xs;
    return i32(ys[0][0]) - 97 + i32(ys[1][0]) - 66;
}

func main(): i32 { return outer(); }
"#;

/// Each copy spelling of a whole-`str[2]` array hands its live element loan to
/// the destination, so the later store through the copy reports the frozen
/// conflict exactly once: the `let`, the assignment, the element loop, and the
/// `take` source. The report site is the store that reaches through the copy.
#[test]
fn should_report_one_element_loan_conflict_per_whole_array_copy_spelling() {
    assert_rejected_at_line("viewcopylocal", VIEW_COPY_LOCAL_SOURCE, "E10034", 1, 7);
    assert_rejected_at_line("viewcopyassign", VIEW_COPY_ASSIGN_SOURCE, "E10034", 1, 7);
    assert_rejected_at_line("viewcopyloop", VIEW_COPY_LOOP_SOURCE, "E10034", 1, 7);
    assert_rejected_at_line("viewcopytake", VIEW_COPY_TAKE_SOURCE, "E10034", 1, 7);
}

/// The accepted copy controls keep their emitted shape: the whole-array copy
/// is one byte copy of the destination's storage, the parameter source still
/// writes through the out-slot, and the read-back reads both copied elements
/// instead of a single one.
#[test]
fn should_copy_a_whole_view_array_and_read_it_back() {
    let param = "viewcopyparam";
    assert_rejected(param, VIEW_COPY_PARAM_SOURCE, "E10034", 1);

    let read = "viewcopyread";
    let (_scratch, emitted) = emit_host(read, VIEW_COPY_READ_SOURCE);
    assert_contains(read, &emitted, "memcpy(&(ys), &(xs), sizeof(ys));");
    assert_contains(read, &emitted, "(ys[0]).ptr[0]");
    assert_contains(read, &emitted, "(ys[1]).ptr[0]");
}

// ---------------------------------------------------------------------------
// The borrowed element's move judgment. A move through an element view is the
// v1 element-move limit once the substituted member owns, so each owning
// spelling reports once and the duplicable and top-level spellings keep their
// copies and moves.
// ---------------------------------------------------------------------------

/// The member store through one borrowed element.
const ELEMENT_MOVE_STORE_SOURCE: &str = r#"module spec.cprimpl.elemmovestore;

import { String, stringOf, bytes } from std.text;

struct R { row: String[2]; }

func eachCopy<T>(xs: T): i32 {
    let total: i32 = 0;
    for (borrow v in xs) {
        for (borrow w in xs) {
            v.row = w.row;
        }
        total = total + i32(len(bytes(v.row[0]))) + i32(len(bytes(v.row[1])));
    }
    return total;
}

func main(): i32 {
    let a: R[2] = [R { row: [stringOf("a"), stringOf("b")] },
                   R { row: [stringOf("cc"), stringOf("dd")] }];
    return eachCopy<R[2]>(a);
}
"#;

/// The member store through two borrowed elements from two arrays.
const ELEMENT_MOVE_STORE_TWO_SOURCE: &str = r#"module spec.cprimpl.elemmovestoretwo;

import { String, stringOf, bytes } from std.text;

struct R { row: String[2]; }

func eachCopy2<T>(xs: T, ys: T): i32 {
    let total: i32 = 0;
    for (borrow v in xs) {
        for (borrow w in ys) {
            v.row = w.row;
        }
        total = total + i32(len(bytes(v.row[0]))) + i32(len(bytes(v.row[1])));
    }
    return total;
}

func main(): i32 {
    let a: R[2] = [R { row: [stringOf("a"), stringOf("b")] },
                   R { row: [stringOf("cc"), stringOf("dd")] }];
    let b: R[2] = [R { row: [stringOf("e"), stringOf("f")] },
                   R { row: [stringOf("gg"), stringOf("hh")] }];
    return eachCopy2<R[2]>(a, b);
}
"#;

/// The member take through a borrowed element, followed by a fresh store.
const ELEMENT_MOVE_TAKE_SOURCE: &str = r#"module spec.cprimpl.elemmovetake;

import { String, stringOf } from std.text;

struct S { name: String; }

func eat(take s: String): void { }

func each<T>(xs: T): void {
    for (borrow v in xs) {
        eat(v.name);
        v.name = stringOf("zzzz");
    }
}

func main(): i32 {
    let a: S[1] = [S { name: stringOf("a") }];
    each<S[1]>(a);
    return 0;
}
"#;

/// The duplicable member twin of the take spelling.
const ELEMENT_MOVE_COPY_SOURCE: &str = r#"module spec.cprimpl.elemmovecopy;

import { String, stringOf } from std.text;

struct S { name: i32; }

func eat(take s: i32): void { }

func each<T>(xs: T): void {
    for (borrow v in xs) {
        eat(v.name);
        v.name = 7;
    }
}

func main(): i32 {
    let a: S[1] = [S { name: 3 }];
    each<S[1]>(a);
    return 0;
}
"#;

/// The top-level member twin of the take spelling: the same take on an owned
/// local, which the move rules do allow.
const TOP_LEVEL_TAKE_SOURCE: &str = r#"module spec.cprimpl.taketopfield;

import { String, stringOf, bytes } from std.text;

struct S { name: String; }

func eat(take s: String): void { }

func main(): i32 {
    let a: S = S { name: stringOf("a") };
    eat(a.name);
    a.name = stringOf("zzzz");
    return i32(len(bytes(a.name))) - 4;
}
"#;

/// The owning member store, the two-array member store, and the member take
/// each report the v1 element-move limit exactly once and write no translation
/// unit, matching their concrete twins.
#[test]
fn should_report_a_borrowed_element_move_once_per_owning_instance() {
    assert_rejected_at_line("elemmovestore", ELEMENT_MOVE_STORE_SOURCE, "E10026", 1, 11);
    assert_rejected_at_line(
        "elemmovestoretwo",
        ELEMENT_MOVE_STORE_TWO_SOURCE,
        "E10026",
        1,
        11,
    );
    assert_rejected_at_line("elemmovetake", ELEMENT_MOVE_TAKE_SOURCE, "E10026", 1, 11);
}

/// A duplicable element member is a copy, so the move judgment stays silent:
/// the instance emits the take call with the element intact, and the member
/// carries no zeroing move-out that would drop a value the element still owns.
#[test]
fn should_keep_a_duplicable_borrowed_element_move_a_copy() {
    let tag = "elemmovecopy";
    let (_scratch, emitted) = emit_host(tag, ELEMENT_MOVE_COPY_SOURCE);
    let body = function_body(tag, &emitted, "cl_each__arr1_S(ClS xs[1])\n{");
    assert_contains(
        tag,
        body,
        "cl_spec_cprimpl_elemmovecopy_eat((*__cl_tmp1).name);",
    );
    assert_contains(tag, body, "(*__cl_tmp1).name = 7;");
    assert_absent(tag, body, "cl_zero(&((*__cl_tmp1).name))");
    assert_absent(tag, body, "(*__cl_tmp1).name = 0;");
}

/// The top-level member twin keeps its accepted move-and-empty shape: the
/// callee receives the member and the owner's field is emptied right after the
/// call, so the following store writes a fresh value over empty storage.
#[test]
fn should_move_a_top_level_member_take_out_of_its_owner() {
    let tag = "taketopfield";
    let (_scratch, emitted) = emit_host(tag, TOP_LEVEL_TAKE_SOURCE);
    assert_follows(
        tag,
        &emitted,
        "cl_spec_cprimpl_taketopfield_eat(a.name);",
        "a.name = NULL;",
    );
}

// ---------------------------------------------------------------------------
// The conditional take's carrier and the deref store's source. An unresolved
// member of a conditional moves into one carrier per arm and the callee
// receives the carrier; an owned raw-deref read empties its pointee through
// the pointer that named it.
// ---------------------------------------------------------------------------

/// The generic conditional take whose member type is unresolved in the body.
const CONDITIONAL_TAKE_SOURCE: &str = r#"module spec.cprimpl.condtakearms;

import { String, stringOf } from std.text;

struct S { name: String; y: i32; }

func eat<T>(take x: T): i32 { return 1; }

func pick<T>(take a: T, take b: T, c: bool): i32 {
    return eat(take (c ? a.name : b.name));
}

func main(): i32 {
    let a: S = S { name: stringOf("ab"), y: 1 };
    let b: S = S { name: stringOf("cd"), y: 2 };
    return pick<S>(take a, take b, true) - 1;
}
"#;

/// The concrete member twin of the conditional take.
const CONDITIONAL_TAKE_CONCRETE_SOURCE: &str = r#"module spec.cprimpl.condtakeconcrete;

import { String, stringOf } from std.text;

struct S { name: String; y: i32; }

func eatS(take x: String): i32 { return 1; }

func pickS(take a: S, take b: S, c: bool): i32 {
    return eatS(c ? a.name : b.name);
}

func main(): i32 {
    let a: S = S { name: stringOf("ab"), y: 1 };
    let b: S = S { name: stringOf("cd"), y: 2 };
    return pickS(take a, take b, true) - 1;
}
"#;

/// An owned raw-deref read of a handle pointee.
const DEREF_READ_SOURCE: &str = r#"module spec.cprimpl.derefreadowner;

import { String, stringOf } from std.text;

func main(): i32 {
    let a: String = stringOf("aa");
    let q: raw String = unsafe &a;
    let v: String = unsafe *q;
    return i32(len(v));
}
"#;

/// The struct-pointee twin of the owned raw-deref read.
const DEREF_READ_STRUCT_SOURCE: &str = r#"module spec.cprimpl.derefreadstruct;

import { String, stringOf, bytes } from std.text;

struct S { name: String; }

func use(): i32 {
    let a: S = S { name: stringOf("ab") };
    let p: raw S = unsafe &a;
    let v: S = unsafe *p;
    return i32(len(bytes(v.name))) - 2;
}

func main(): i32 { return use(); }
"#;

/// An owned raw-deref read whose pointer expression is an impure call.
const DEREF_READ_IMPURE_SOURCE: &str = r#"module spec.cprimpl.derefreadimpure;

import { String, stringOf } from std.text;

let calls: i32 = 0;

func alt(s: raw String, t: raw String): raw String {
    calls = calls + 1;
    return s;
}

func use(): i32 {
    let a: String = stringOf("aa");
    let b: String = stringOf("bb");
    let p: raw String = unsafe &a;
    let q: raw String = unsafe &b;
    let v: String = unsafe *alt(p, q);
    return i32(len(v)) + calls * 10;
}

func main(): i32 { return use() - 12; }
"#;

/// The normalized statement shape of one emitted function body.
///
/// The generic and concrete spellings of one program differ only in the
/// callee's symbol and the `#line` markers that name their source files, so
/// the comparison replaces the callee symbol and drops the markers and blank
/// lines; what remains is the emitted statement sequence itself.
fn normalized_body(tag: &str, emitted: &str, signature: &str, callee: &str) -> Vec<String> {
    function_body(tag, emitted, signature)
        .replace(callee, "callee")
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("#line"))
        .map(str::to_owned)
        .collect()
}

/// The selected member of an unresolved conditional moves into one carrier
/// per arm, and the callee receives the carrier: each branch copies the arm's
/// member and empties its root, the call reads the carrier, and the post-call
/// move-out zeroes it. The concrete member twin's statement sequence is the
/// reference, normalized only for the callee symbol and the line markers.
#[test]
fn should_move_the_conditional_take_member_into_the_carrier_per_arm() {
    let tag = "condtakearms";
    let (_scratch, emitted) = emit_host(tag, CONDITIONAL_TAKE_SOURCE);
    assert_contains(tag, &emitted, "ClString __cl_tmp0 = NULL;");
    assert_follows(tag, &emitted, "if (c) {", "__cl_tmp0 = a.name;");
    assert_follows(tag, &emitted, "__cl_tmp0 = a.name;", "a.name = NULL;");
    assert_follows(tag, &emitted, "} else {", "__cl_tmp0 = b.name;");
    assert_follows(tag, &emitted, "__cl_tmp0 = b.name;", "b.name = NULL;");
    assert_follows(
        tag,
        &emitted,
        "__cl_tmp1 = cl_eat__String(__cl_tmp0);",
        "__cl_tmp0 = NULL;",
    );

    let concrete = "condtakeconcrete";
    let (_scratch, emitted_concrete) = emit_host(concrete, CONDITIONAL_TAKE_CONCRETE_SOURCE);
    let generic_shape = normalized_body(
        tag,
        &emitted,
        "cl_pick__S(ClS a, ClS b, bool c)\n{",
        "cl_eat__String",
    );
    let concrete_shape = normalized_body(
        concrete,
        &emitted_concrete,
        "cl_spec_cprimpl_condtakeconcrete_pickS(ClS a, ClS b, bool c)\n{",
        "cl_spec_cprimpl_condtakeconcrete_eatS",
    );
    assert_eq!(
        generic_shape, concrete_shape,
        "the generic conditional take must match the concrete twin's statement shape"
    );
}

/// An owned raw-deref read used as a store value moves the pointee: the handle
/// spelling is emptied through its own address and the struct spelling
/// through `cl_zero` at the same address, so only the destination owns the
/// value. An impure pointer expression is bound once, so the store and the
/// emptying read one address.
#[test]
fn should_empty_the_owned_deref_store_source_through_its_own_address() {
    let tag = "derefreadowner";
    let (_scratch, emitted) = emit_host(tag, DEREF_READ_SOURCE);
    assert_follows(tag, &emitted, "v = *(q);", "*(q) = NULL;");

    let struct_tag = "derefreadstruct";
    let (_scratch, emitted) = emit_host(struct_tag, DEREF_READ_STRUCT_SOURCE);
    assert_follows(struct_tag, &emitted, "v = *(p);", "cl_zero(&*(p));");

    let impure = "derefreadimpure";
    let (_scratch, emitted) = emit_host(impure, DEREF_READ_IMPURE_SOURCE);
    let body = function_body(
        impure,
        &emitted,
        "cl_spec_cprimpl_derefreadimpure_use(void)\n{",
    );
    assert_contains(
        impure,
        body,
        "__cl_tmp0 = cl_spec_cprimpl_derefreadimpure_alt(p, q);",
    );
    assert_eq!(
        body.matches("cl_spec_cprimpl_derefreadimpure_alt(").count(),
        1,
        "the impure pointer must evaluate exactly once in the body; body:\n{body}"
    );
    assert_follows(impure, body, "v = *(__cl_tmp0);", "*(__cl_tmp0) = NULL;");
}

/// The conditional take's carrier shape and the deref store's emptied pointee
/// re-emit byte-identically.
#[test]
fn should_re_emit_the_ownership_transfer_programs_byte_identically() {
    for (tag, source, shape) in [
        (
            "condtakearms",
            CONDITIONAL_TAKE_SOURCE,
            "__cl_tmp1 = cl_eat__String(__cl_tmp0);",
        ),
        ("derefreadowner", DEREF_READ_SOURCE, "*(q) = NULL;"),
    ] {
        let (first, second) = emit_twice(tag, source);
        assert!(
            first == second,
            "{tag}: the two emissions differ; first:\n{first}\nsecond:\n{second}"
        );
        assert_contains(tag, &first, shape);
    }
}

// ===== The conditional-base store and the guarded view-array literal =====

/// The taken-selection conditional-base store program.
const CONDITIONAL_BASE_TAKEN_SOURCE: &str = r#"module spec.cpr.implcondstore;

struct Box { ys: i32[3]; }

let calls: i32 = 0;

func bump(): i32 {
    calls = calls + 1;
    return 7;
}

func main(): i32 {
    let b1: Box = Box { ys: [1, 2, 3] };
    let b2: Box = Box { ys: [4, 5, 6] };
    let c: bool = true;
    let i: i32 = 1;
    (c ? b1 : b2).ys[i] = bump();
    return (b1.ys[1] - 7) + (b2.ys[1] - 5) + calls;
}
"#;

/// The false-selection conditional-base store program.
const CONDITIONAL_BASE_OTHER_SOURCE: &str = r#"module spec.cpr.implcondstoreother;

struct Box { ys: i32[3]; }

func main(): i32 {
    let b1: Box = Box { ys: [1, 2, 3] };
    let b2: Box = Box { ys: [4, 5, 6] };
    let c: bool = false;
    let i: i32 = 1;
    (c ? b1 : b2).ys[i] = 7;
    return (b2.ys[1] - 7) + (b1.ys[1] - 2);
}
"#;

/// The accepted read spelling of the conditional base.
const CONDITIONAL_BASE_READ_SOURCE: &str = r#"module spec.cpr.implcondstoreread;

struct Box { ys: i32[3]; }

func main(): i32 {
    let b1: Box = Box { ys: [1, 2, 3] };
    let b2: Box = Box { ys: [4, 5, 6] };
    let c: bool = true;
    let d: bool = false;
    let i: i32 = 1;
    let v1: i32 = (c ? b1 : b2).ys[i];
    let v2: i32 = (d ? b1 : b2).ys[i];
    return (v1 - 2) + (v2 - 5) + (b1.ys[1] - 2) + (b2.ys[1] - 5);
}
"#;

/// The accepted plain place store with two right-hand-side calls.
const CONDITIONAL_BASE_PLACE_SOURCE: &str = r#"module spec.cpr.implcondstoreplace;

struct Box { ys: i32[3]; }

let calls: i32 = 0;

func bump(): i32 {
    calls = calls + 1;
    return 7;
}

func main(): i32 {
    let b1: Box = Box { ys: [1, 2, 3] };
    let i: i32 = 1;
    b1.ys[i] = bump();
    b1.ys[0] = bump() + 1;
    return (b1.ys[1] - 7) + (b1.ys[0] - 8) + calls - 2;
}
"#;

/// The accepted projected place store.
const CONDITIONAL_BASE_PROJECTION_SOURCE: &str = r#"module spec.cpr.implcondstoreproj;

struct Box { ys: i32[3]; }
struct Holder { b: Box; }

func main(): i32 {
    let h: Holder = Holder { b: Box { ys: [1, 2, 3] } };
    let i: i32 = 1;
    h.b.ys[i] = 7;
    return h.b.ys[1] - 7;
}
"#;

/// The guarded view-array literal with the take parameter.
const GUARD_VIEW_LITERAL_TAKE_SOURCE: &str = r#"module spec.cpr.implguardview;

import { String, stringOf } from std.text;

struct Mixed { names: str[2]; tag: String; }

let calls: i32 = 0;
let seen: i32 = 0;

func n1(): str[2] {
    calls = calls + 1;
    return ["AA", "BB"];
}

func n2(): str[2] {
    calls = calls + 1;
    return ["CC", "DD"];
}

func consume(take m: Mixed): i32 {
    seen = i32(m.names[0][0]);
    return 1;
}

func use(g: bool, ok: bool): i32 {
    if (g && consume(ok ? Mixed { names: n1(), tag: stringOf("x") } : Mixed { names: n2(), tag: stringOf("y") }) == 1) {
        return seen;
    }
    return -1;
}

func main(): i32 {
    let a: i32 = use(true, true);
    let b: i32 = use(true, false);
    let c: i32 = use(false, true);
    return (a - 65) + (b - 67) + (c + 1) + calls - 2;
}
"#;

/// The guarded view-array literal with the plain parameter.
const GUARD_VIEW_LITERAL_PLAIN_SOURCE: &str = r#"module spec.cpr.implguardviewplain;

import { String, stringOf } from std.text;

struct Mixed { names: str[2]; tag: String; }

let calls: i32 = 0;
let seen: i32 = 0;

func n1(): str[2] {
    calls = calls + 1;
    return ["AA", "BB"];
}

func n2(): str[2] {
    calls = calls + 1;
    return ["CC", "DD"];
}

func consume(m: Mixed): i32 {
    seen = i32(m.names[0][0]);
    return 1;
}

func use(g: bool, ok: bool): i32 {
    if (g && consume(ok ? Mixed { names: n1(), tag: stringOf("x") } : Mixed { names: n2(), tag: stringOf("y") }) == 1) {
        return seen;
    }
    return -1;
}

func main(): i32 {
    let a: i32 = use(true, true);
    let b: i32 = use(true, false);
    let c: i32 = use(false, true);
    return (a - 65) + (b - 67) + (c + 1) + calls - 2;
}
"#;

/// An element store through a conditional base reports the place rule.
///
/// `(c ? b1 : b2).ys[i] = x` cannot name a place, so the assignment reports
/// one `E10221` at the target's site and emits no translation unit — for the
/// taken and the false selection alike. The store is never silently dropped,
/// so the right-hand side never runs.
#[test]
fn should_reject_an_element_store_through_a_conditional_base_at_its_site() {
    assert_rejected_once_at(
        "implcondstore",
        CONDITIONAL_BASE_TAKEN_SOURCE,
        "E10221",
        17,
        5,
        &[
            "left side of an assignment must be a place",
            "assign to a variable, field, or element",
        ],
        &[],
    );
    assert_rejected_once_at(
        "implcondstoreother",
        CONDITIONAL_BASE_OTHER_SOURCE,
        "E10221",
        10,
        5,
        &["left side of an assignment must be a place"],
        &[],
    );
}

/// The conditional-base controls keep their accepted emissions.
///
/// The read spelling renders the conditional between the two arrays, the
/// plain place store keeps both right-hand-side calls, and the projected
/// store writes through the element address.
#[test]
fn should_keep_the_conditional_base_controls_accepted() {
    let (_scratch, emitted) = emit_host("implcondstoreread", CONDITIONAL_BASE_READ_SOURCE);
    assert_contains(
        "implcondstoreread",
        &emitted,
        "v1 = (c ? b1 : b2).ys[i];",
    );
    assert_contains(
        "implcondstoreread",
        &emitted,
        "v2 = (d ? b1 : b2).ys[i];",
    );

    let (_scratch, emitted) = emit_host("implcondstoreplace", CONDITIONAL_BASE_PLACE_SOURCE);
    assert_contains(
        "implcondstoreplace",
        &emitted,
        "*(&(b1.ys[__cl_tmp0])) = cl_spec_cpr_implcondstoreplace_bump();",
    );
    assert_contains(
        "implcondstoreplace",
        &emitted,
        "b1.ys[0] = (i32)((u32)cl_spec_cpr_implcondstoreplace_bump() + (u32)1);",
    );

    let (_scratch, emitted) = emit_host("implcondstoreproj", CONDITIONAL_BASE_PROJECTION_SOURCE);
    assert_contains("implcondstoreproj", &emitted, "*(&(h.b.ys[__cl_tmp0])) = 7;");
}

/// The guarded view-array literal fills its destination on the taken path.
///
/// The conditionally evaluated literal materializes into a destination the
/// guard fills where the operand is actually evaluated, and each arm's call
/// receives that destination's view-array field — so a skipped guard or a
/// skipped arm never runs the other arm's call.
#[test]
fn should_fill_the_guard_literal_view_array_on_the_evaluated_path() {
    let (_scratch, emitted) = emit_host("implguardview", GUARD_VIEW_LITERAL_TAKE_SOURCE);
    assert_contains("implguardview", &emitted, "ClMixed __cl_tmp2 = {0};");
    assert_contains("implguardview", &emitted, "(ok ? ((__cl_tmp2 = ");
    assert_eq!(
        emitted
            .matches("implguardview_n1(__cl_tmp0.names)")
            .count(),
        1,
        "implguardview: the selected arm's call must fill once"
    );

    let (_scratch, emitted) = emit_host("implguardviewplain", GUARD_VIEW_LITERAL_PLAIN_SOURCE);
    assert_contains("implguardviewplain", &emitted, "ClMixed __cl_tmp0 = {0};");
    assert_contains(
        "implguardviewplain",
        &emitted,
        "implguardviewplain_n1(__cl_tmp0.names)",
    );
    assert_contains("implguardviewplain", &emitted, "_consume((ok ? ");
    assert_eq!(
        emitted
            .matches("implguardviewplain_n2(__cl_tmp1.names)")
            .count(),
        1,
        "implguardviewplain: the other arm's call must fill its bound root once"
    );
}

/// The conditional-base and guard-literal programs re-emit byte-identically.
///
/// The rejection and the destination fill read the substituted shapes, so a
/// second emission of the same source produces the same text.
#[test]
fn should_re_emit_the_conditional_base_and_guard_literal_programs_identically() {
    let (first, second) = emit_twice("reemitcondstoreplace", CONDITIONAL_BASE_PLACE_SOURCE);
    assert_eq!(
        first, second,
        "reemitcondstoreplace: the program re-emitted differently"
    );
    assert_contains(
        "reemitcondstoreplace",
        &first,
        "*(&(b1.ys[__cl_tmp0])) = cl_spec_cpr_implcondstoreplace_bump();",
    );

    let (first, second) = emit_twice("reemitguardview", GUARD_VIEW_LITERAL_TAKE_SOURCE);
    assert_eq!(
        first, second,
        "reemitguardview: the program re-emitted differently"
    );
    assert_contains("reemitguardview", &first, "(ok ? ((__cl_tmp2 = ");
}

/// The two-field arm literal program.
const TWO_FIELD_ARM_SOURCE: &str = r#"module spec.cpr.impltwofield;

import { String, stringOf } from std.text;

struct Inner { names: str[2]; tag: String; }
struct Outer { inner: Inner; tag: String; }

let calls: i32 = 0;
let seen: i32 = 0;

func n1(): str[2] {
    calls = calls + 1;
    return ["AA", "BB"];
}

func consumeO(take m: Outer): i32 {
    seen = i32(m.inner.names[0][0]);
    return 1;
}

func useT(g: bool, ok: bool): i32 {
    if (g && consumeO(ok ? Outer { inner: Inner { names: n1(), tag: stringOf("iiii1") }, tag: stringOf("ooooo1") }
                         : Outer { inner: Inner { names: n1(), tag: stringOf("iiii2") }, tag: stringOf("ooooo2") }) == 1) {
        return seen;
    }
    return -1;
}

func main(): i32 {
    return useT(true, true);
}
"#;

/// A two-field arm literal parenthesizes each stored arm.
///
/// The field-wise arm text is parenthesized inside the conditional, so the
/// comma sequence cannot escape a `?:`'s third operand; the two-field spec
/// case proves the behavior (the leak and the skipped call are gone).
#[test]
fn should_parenthesize_a_two_field_arm_literal() {
    let (_scratch, emitted) = emit_host("impltwofield", TWO_FIELD_ARM_SOURCE);
    assert_contains("impltwofield", &emitted, "(ok ? ((");
    assert_contains("impltwofield", &emitted, ") : ((");

    let (first, second) = emit_twice("reemittwofield", TWO_FIELD_ARM_SOURCE);
    assert_eq!(
        first, second,
        "reemittwofield: the program re-emitted differently"
    );
}

// ---------------------------------------------------------------------------
// indexedfieldstore: the runtime-indexed element and member store.
//
// A store target that selects an element with a run-time index names the
// element's own storage, so the emitter pins the index, guards the element
// bounds, and writes the projected lvalue. The owning member and element
// stores release the value they overwrite, and a second emission is
// byte-identical.
// ---------------------------------------------------------------------------

/// A member store through a run-time index.
const INDEXED_FIELD_SOURCE: &str = r#"module spec.cprimpl.implindexedfield;

struct Elem { y: i32; }

let calls: i32 = 0;

func bump(): i32 {
    calls = calls + 1;
    return 7;
}

func run(i: i32): i32 {
    let a: Elem[3] = [Elem { y: 0 }, Elem { y: 0 }, Elem { y: 0 }];
    a[i].y = bump();
    return a[i].y - 7;
}

func main(): i32 {
    return run(1) + calls - 1;
}
"#;

/// A compound store and an update through a run-time index.
const INDEXED_COMPOUND_SOURCE: &str = r#"module spec.cprimpl.implindexedcompound;

struct Elem { y: i32; }

func add(i: i32): i32 {
    let a: Elem[3] = [Elem { y: 0 }, Elem { y: 0 }, Elem { y: 0 }];
    a[i].y += 3;
    return a[i].y - 3;
}

func bump(i: i32): i32 {
    let a: Elem[3] = [Elem { y: 0 }, Elem { y: 0 }, Elem { y: 0 }];
    a[i].y++;
    return a[i].y - 1;
}

func main(): i32 {
    return add(1) + bump(1);
}
"#;

/// A deep member chain and a nested generic member store.
const INDEXED_CHAIN_SOURCE: &str = r#"module spec.cprimpl.implindexedchain;

import { String, stringOf } from std.text;

struct Inner { y: i32; }
struct Elem { b: Inner; }
struct Named { name: String; }
struct Mid { inner: Named; }
struct Holder { mids: Mid[2]; }

func chain(i: i32): i32 {
    let a: Elem[3] = [Elem { b: Inner { y: 0 } }, Elem { b: Inner { y: 0 } }, Elem { b: Inner { y: 0 } }];
    a[i].b.y = 7;
    return a[i].b.y - 7;
}

func nested<T>(borrow w: T, i: i32): i32 {
    w.mids[i].inner.name = stringOf("zz");
    return i32(len(w.mids[i].inner.name));
}

func main(): i32 {
    let h: Holder = Holder { mids: [Mid { inner: Named { name: stringOf("a") } }, Mid { inner: Named { name: stringOf("b") } }] };
    return chain(1) + nested<Holder>(borrow h, 1) - 2;
}
"#;

/// A member store through two run-time indices.
const INDEXED_MULTI_SOURCE: &str = r#"module spec.cprimpl.implindexedmulti;

import { String, stringOf } from std.text;

struct Cell { name: String; }
struct Grid { grid: Cell[2][2]; }

func setg<T>(borrow w: T, i: i32, j: i32): i32 {
    w.grid[i][j].name = stringOf("zz");
    return i32(len(w.grid[i][j].name));
}

func main(): i32 {
    let g: Grid = Grid { grid: [[Cell { name: stringOf("a") }, Cell { name: stringOf("b") }], [Cell { name: stringOf("c") }, Cell { name: stringOf("d") }]] };
    return setg<Grid>(borrow g, 1, 0) - 2;
}
"#;

/// A store whose index the stored value updates.
const INDEXED_ORDER_SOURCE: &str = r#"module spec.cprimpl.implindexedorder;

struct Elem { y: i32; }

let idx: i32 = 1;
let calls: i32 = 0;

func set_index(): i32 {
    idx = 2;
    calls = calls + 1;
    return 7;
}

func run(): i32 {
    let a: Elem[3] = [Elem { y: 0 }, Elem { y: 0 }, Elem { y: 0 }];
    a[idx].y = set_index();
    return (a[1].y - 7) + (a[2].y * 100) + (idx - 2);
}

func main(): i32 {
    return run() + calls - 1;
}
"#;

/// A generic owning member store and a generic whole-element store.
const INDEXED_OWNED_SOURCE: &str = r#"module spec.cprimpl.implindexedowned;

import { String, stringOf } from std.text;

struct Elem { name: String; }
struct Holder { items: Elem[2]; }

func member<T>(borrow w: T, i: i32): i32 {
    w.items[i].name = stringOf("zz");
    return i32(len(w.items[i].name));
}

func element<T>(borrow w: T, i: i32): i32 {
    w.items[i] = Elem { name: stringOf("zz") };
    return i32(len(w.items[i].name));
}

func main(): i32 {
    let h: Holder = Holder { items: [Elem { name: stringOf("a") }, Elem { name: stringOf("b") }] };
    return member<Holder>(borrow h, 1) + element<Holder>(borrow h, 0) - 4;
}
"#;

/// A compile-only slice element store.
const INDEXED_SLICE_SOURCE: &str = r#"module spec.cprimpl.implindexedslice;

struct Elem { y: i32; }

let calls: i32 = 0;

func bump(): i32 {
    calls = calls + 1;
    return 7;
}

func viaslice(borrow ps: Elem[], i: i32): i32 {
    ps[i].y = bump();
    return ps[i].y - 7;
}

func main(): i32 {
    return 0;
}
"#;

/// A raw-pointer field store and a raw-pointer field-element store.
const RAW_PREFIX_STORE_SOURCE: &str = r#"module spec.cprimpl.implrawprefix2;

struct Inner { x: i32; }
struct S { f: Inner; }

func main(): i32 {
    let s: S = S { f: Inner { x: 0 } };
    unsafe {
        let p: raw S = &s;
        unsafe p.f = Inner { x: 7 };
        unsafe p.f.x = 7;
    }
    return s.f.x - 7;
}
"#;

/// A runtime-indexed element field store pins its index, guard, and value.
///
/// The index is read into a temporary before the guard and the store, the
/// bounds check reads that temporary, and the projected element lvalue keeps
/// the same index. The stored call runs exactly once.
#[test]
fn should_pin_the_indexed_element_field_store_sequence() {
    let tag = "implindexedfield";
    let (_scratch, emitted) = emit_host(tag, INDEXED_FIELD_SOURCE);
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_implindexedfield_run(i32 i)\n{",
    );
    assert_contains(tag, body, "i32 __cl_tmp0 = 0;");
    assert_follows(tag, body, "__cl_tmp0 = i;", "if ((__cl_tmp0 < 0) || ((usize)(__cl_tmp0) >= (usize)(3))) cl_trap(\"E11211\", \"index out of range\");");
    assert_follows(
        tag,
        body,
        "cl_trap(\"E11211\", \"index out of range\");",
        "a[__cl_tmp0].y = cl_spec_cprimpl_implindexedfield_bump();",
    );
    assert_eq!(
        body.matches("implindexedfield_bump()").count(),
        1,
        "{tag}: the stored value must run once; body:\n{body}"
    );
}

/// The compound store and the update project the same element lvalue.
///
/// Both forms read and write the slot the pinned index selects: the compound
/// store adds to the projected member and the update adds one to it, with no
/// second index evaluation.
#[test]
fn should_pin_the_indexed_compound_and_update_stores() {
    let tag = "implindexedcompound";
    let (_scratch, emitted) = emit_host(tag, INDEXED_COMPOUND_SOURCE);

    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_implindexedcompound_add(i32 i)\n{",
    );
    assert_contains(
        tag,
        body,
        "a[__cl_tmp0].y = (i32)((u32)a[__cl_tmp0].y + (u32)3);",
    );

    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_implindexedcompound_bump(i32 i)\n{",
    );
    assert_contains(
        tag,
        body,
        "a[__cl_tmp0].y = (i32)((u32)a[__cl_tmp0].y + (u32)1);",
    );
}

/// A deep chain and a nested generic member store keep every projection.
///
/// The store reaches the selected element's field and then the written member,
/// and the nested generic instance resolves the element's owning member so the
/// old value is released before the store.
#[test]
fn should_pin_the_indexed_deep_chain_and_nested_member_stores() {
    let tag = "implindexedchain";
    let (_scratch, emitted) = emit_host(tag, INDEXED_CHAIN_SOURCE);

    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_implindexedchain_chain(i32 i)\n{",
    );
    assert_contains(tag, body, "a[__cl_tmp0].b.y = 7;");

    let body = function_body(tag, &emitted, "cl_nested__Holder(ClHolder *w, i32 i)\n{");
    assert_contains(
        tag,
        body,
        "if ((*w).mids[__cl_tmp0].inner.name) { FreeString((*w).mids[__cl_tmp0].inner.name); }",
    );
    assert_follows(
        tag,
        body,
        "(*w).mids[__cl_tmp0].inner.name = NULL;",
        "(*w).mids[__cl_tmp0].inner.name = cl_std_text_stringOf(cl_str(\"zz\"));",
    );
}

/// A nested index target keeps both indices on the store and its guards.
///
/// Each run-time index is pinned in source order, both bounds checks read the
/// pinned values, and the projected member is written through both of them.
#[test]
fn should_pin_the_indexed_multi_index_store() {
    let tag = "implindexedmulti";
    let (_scratch, emitted) = emit_host(tag, INDEXED_MULTI_SOURCE);
    let body = function_body(
        tag,
        &emitted,
        "cl_setg__Grid(ClGrid *w, i32 i, i32 j)\n{",
    );
    assert_follows(tag, body, "__cl_tmp0 = i;", "__cl_tmp1 = j;");
    assert_follows(
        tag,
        body,
        "__cl_tmp1 = j;",
        "if ((__cl_tmp0 < 0) || ((usize)(__cl_tmp0) >= (usize)(2))) cl_trap(\"E11211\", \"index out of range\");",
    );
    assert_follows(
        tag,
        body,
        "if ((__cl_tmp0 < 0) || ((usize)(__cl_tmp0) >= (usize)(2))) cl_trap(\"E11211\", \"index out of range\");",
        "if ((__cl_tmp1 < 0) || ((usize)(__cl_tmp1) >= (usize)(2))) cl_trap(\"E11211\", \"index out of range\");",
    );
    assert_contains(
        tag,
        body,
        "(*w).grid[__cl_tmp0][__cl_tmp1].name = cl_std_text_stringOf(cl_str(\"zz\"));",
    );
    assert_follows(
        tag,
        body,
        "(*w).grid[__cl_tmp0][__cl_tmp1].name = NULL;",
        "(*w).grid[__cl_tmp0][__cl_tmp1].name = cl_std_text_stringOf(cl_str(\"zz\"));",
    );
}

/// The target's index is pinned before the stored value runs.
///
/// The store reads the module index into a temporary before the guard and the
/// store, so the call that also writes that index cannot redirect the store to
/// the new slot.
#[test]
fn should_pin_the_target_location_ahead_of_the_stored_value() {
    let tag = "implindexedorder";
    let (_scratch, emitted) = emit_host(tag, INDEXED_ORDER_SOURCE);
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_implindexedorder_run(void)\n{",
    );
    assert_follows(
        tag,
        body,
        "__cl_tmp0 = cl_spec_cprimpl_implindexedorder_idx;",
        "if ((__cl_tmp0 < 0) || ((usize)(__cl_tmp0) >= (usize)(3))) cl_trap(\"E11211\", \"index out of range\");",
    );
    assert_follows(
        tag,
        body,
        "cl_trap(\"E11211\", \"index out of range\");",
        "a[__cl_tmp0].y = cl_spec_cprimpl_implindexedorder_set_index();",
    );
    assert_absent(tag, body, "a[idx].y = ");
}

/// A generic owning member store releases the value it overwrites.
///
/// The substituted member type owns its storage, so the store releases the old
/// value and clears its slot before writing the new one.
#[test]
fn should_release_the_owner_an_indexed_member_store_overwrites() {
    let tag = "implindexedownedmember";
    let (_scratch, emitted) = emit_host(tag, INDEXED_OWNED_SOURCE);
    let body = function_body(tag, &emitted, "cl_member__Holder(ClHolder *w, i32 i)\n{");
    assert_contains(
        tag,
        body,
        "if ((*w).items[__cl_tmp0].name) { FreeString((*w).items[__cl_tmp0].name); }",
    );
    assert_follows(
        tag,
        body,
        "(*w).items[__cl_tmp0].name = NULL;",
        "(*w).items[__cl_tmp0].name = cl_std_text_stringOf(cl_str(\"zz\"));",
    );
}

/// A generic whole-element store releases the element it overwrites.
///
/// The element address carries the run-time index, and the promoted element
/// type owns its member, so the store releases the old member through that
/// address before writing the new element.
#[test]
fn should_release_the_owner_an_indexed_element_store_overwrites() {
    let tag = "implindexedownedelement";
    let (_scratch, emitted) = emit_host(tag, INDEXED_OWNED_SOURCE);
    let body = function_body(tag, &emitted, "cl_element__Holder(ClHolder *w, i32 i)\n{");
    assert_contains(
        tag,
        body,
        "if ((*(&((*w).items[__cl_tmp0]))).name) { FreeString((*(&((*w).items[__cl_tmp0]))).name); }",
    );
    assert_follows(
        tag,
        body,
        "(*(&((*w).items[__cl_tmp0]))).name = NULL;",
        "*(&((*w).items[__cl_tmp0])) = (ClElem){ cl_std_text_stringOf(cl_str(\"zz\")) };",
    );
}

/// The runtime-indexed store programs re-emit byte-identically.
///
/// The per-instance owning store and the pinned element lvalue read the
/// substituted shapes, so a second emission of the same source produces the
/// same text.
#[test]
fn should_re_emit_the_indexed_store_programs_identically() {
    let (first, second) = emit_twice("reemitindexedstore", INDEXED_OWNED_SOURCE);
    assert_eq!(
        first, second,
        "reemitindexedstore: the program re-emitted differently"
    );
    assert_contains(
        "reemitindexedstore",
        &first,
        "(*w).items[__cl_tmp0].name = cl_std_text_stringOf(cl_str(\"zz\"));",
    );
}

/// A conditional-rooted store reports one place rejection at its site.
///
/// The member, indexed member, fixed-array element, index base, and deep chain
/// shapes — and the two `unsafe`-wrapped conditional selections — each report
/// one `E10221` at the store's own line and emit no translation unit.
#[test]
fn should_pin_the_conditional_store_rejection_sites() {
    let fragments = [
        "left side of an assignment must be a place",
        "assign to a variable, field, or element",
    ];
    let cases: [(&str, &str, usize); 7] = [
        (
            "implcondmember2",
            r#"module spec.cprimpl.implcondmember2;

struct Inner { g: i32; }
struct Box { f: Inner; }

func use(c: bool, a: Box, b: Box): i32 {
    (c ? a : b).f.g = 7;
    return 0;
}

func main(): i32 {
    let a: Box = Box { f: Inner { g: 0 } };
    let b: Box = Box { f: Inner { g: 0 } };
    return use(true, a, b);
}
"#,
            7,
        ),
        (
            "implcondindex2",
            r#"module spec.cprimpl.implcondindex2;

struct Cell { x: i32; }
struct Inner { rows: Cell[2]; }
struct Box { f: Inner; }

func use(c: bool, i: i32, a: Box, b: Box): i32 {
    (c ? a : b).f.rows[i].x = 7;
    return 0;
}

func main(): i32 {
    let a: Box = Box { f: Inner { rows: [Cell { x: 0 }, Cell { x: 0 }] } };
    let b: Box = Box { f: Inner { rows: [Cell { x: 0 }, Cell { x: 0 }] } };
    return use(true, 1, a, b);
}
"#,
            8,
        ),
        (
            "implcondarray2",
            r#"module spec.cprimpl.implcondarray2;

struct Bag { ys: i32[2]; }
struct Box { b: Bag; }

func use(c: bool, i: i32, a: Box, b: Box): i32 {
    (c ? a : b).b.ys[i] = 7;
    return 0;
}

func main(): i32 {
    let a: Box = Box { b: Bag { ys: [0, 0] } };
    let b: Box = Box { b: Bag { ys: [0, 0] } };
    return use(true, 1, a, b);
}
"#,
            7,
        ),
        (
            "implcondindexbase2",
            r#"module spec.cprimpl.implcondindexbase2;

struct Cell { x: i32; }

func use(c: bool, i: i32, a: Cell[2], b: Cell[2]): i32 {
    (c ? a : b)[i].x = 7;
    return 0;
}

func main(): i32 {
    let a: Cell[2] = [Cell { x: 0 }, Cell { x: 0 }];
    let b: Cell[2] = [Cell { x: 0 }, Cell { x: 0 }];
    return use(true, 1, a, b);
}
"#,
            6,
        ),
        (
            "implconddeep2",
            r#"module spec.cprimpl.implconddeep2;

struct Bag { ys: i32[2]; }
struct Holder { b: Bag; }

func use(c: bool, i: i32, h1: Holder, h2: Holder): i32 {
    (c ? h1 : h2).b.ys[i] = 7;
    return 0;
}

func main(): i32 {
    let h1: Holder = Holder { b: Bag { ys: [0, 0] } };
    let h2: Holder = Holder { b: Bag { ys: [0, 0] } };
    return use(true, 1, h1, h2);
}
"#,
            7,
        ),
        (
            "implcondunsafemember2",
            r#"module spec.cprimpl.implcondunsafemember2;

struct Inner { x: i32; }
struct Box { v: Inner; }

func run(c: bool, a: Box, b: Box): i32 {
    (unsafe (c ? a : b)).v.x = 7;
    return 0;
}

func main(): i32 {
    let a: Box = Box { v: Inner { x: 0 } };
    let b: Box = Box { v: Inner { x: 0 } };
    return run(true, a, b);
}
"#,
            7,
        ),
        (
            "implcondunsafeindex2",
            r#"module spec.cprimpl.implcondunsafeindex2;

struct Cell { x: i32; }

func run(c: bool, i: i32, a: Cell[2], b: Cell[2]): i32 {
    (unsafe (c ? a : b))[i].x = 7;
    return 0;
}

func main(): i32 {
    let a: Cell[2] = [Cell { x: 0 }, Cell { x: 0 }];
    let b: Cell[2] = [Cell { x: 0 }, Cell { x: 0 }];
    return run(true, 1, a, b);
}
"#,
            6,
        ),
    ];
    for (tag, source, line) in cases {
        assert_rejected_once_at(tag, source, "E10221", line, 5, &fragments, &[]);
    }
}

/// A raw-pointer prefix store is accepted and writes the stored value.
///
/// The `unsafe` wrapper is transparent to the place it wraps, so the field
/// store and the field-element store both emit through the pointer, and the
/// enclosing read-back reads the value the store wrote.
#[test]
fn should_pin_the_accepted_raw_pointer_prefix_stores() {
    let tag = "implrawprefix2";
    let (_scratch, emitted) = emit_host(tag, RAW_PREFIX_STORE_SOURCE);
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_implrawprefix2_main(void)\n{",
    );
    assert_contains(tag, body, "p->f = (ClInner){ 7 };");
    assert_contains(tag, body, "p->f.x = 7;");
    assert_follows(
        tag,
        body,
        "p->f.x = 7;",
        "return (i32)((u32)s.f.x - (u32)7);",
    );
}

/// A compile-only slice element store emits the store and its bounds guard.
///
/// A slice value cannot be built locally, so the uncalled borrow-slice
/// function pins the emission: the pinned index feeds the bounds check and the
/// element's projected lvalue, and the stored call survives beside them.
#[test]
fn should_pin_the_indexed_slice_store_emission() {
    let tag = "implindexedslice";
    let (_scratch, emitted) = emit_host(tag, INDEXED_SLICE_SOURCE);
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_implindexedslice_viaslice(ClSlice_Elem ps, i32 i)\n{",
    );
    assert_follows(
        tag,
        body,
        "__cl_tmp0 = i;",
        "if ((__cl_tmp0 < 0) || ((usize)(__cl_tmp0) >= (usize)((ps).len))) cl_trap(\"E11211\", \"index out of range\");",
    );
    assert_follows(
        tag,
        body,
        "cl_trap(\"E11211\", \"index out of range\");",
        "(ps).ptr[__cl_tmp0].y = cl_spec_cprimpl_implindexedslice_bump();",
    );
    assert_eq!(
        body.matches("implindexedslice_bump()").count(),
        1,
        "{tag}: the slice store's value must run once; body:\n{body}"
    );
}

// ---------------------------------------------------------------------------
// overwrittenelement: a store that replaces an element a view or alias reads.
//
// The old owner is saved until the right-hand side has read it, and a fixed
// array row is cleared after the save so the fill sees empty slots.
// ---------------------------------------------------------------------------

/// A member store over a run-time element saves the value a view reads.
#[test]
fn should_save_an_overwritten_member_until_its_view_has_read() {
    let source = r#"module spec.cprimpl.implsaveview;

import { String, stringOf } from std.text;

struct Elem { name: String; }
struct Holder { items: Elem[2]; }

func set(borrow w: Holder, i: i32): i32 {
    let v: str = w.items[1].name;
    w.items[i].name = stringOf(v);
    return i32(len(w.items[i].name));
}

func main(): i32 {
    let h: Holder = Holder { items: [Elem { name: stringOf("aa") }, Elem { name: stringOf("bb") }] };
    return set(borrow h, 1) - 2;
}
"#;
    let tag = "implsaveview";
    let (_scratch, emitted) = emit_host(tag, source);
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_implsaveview_set(ClHolder *w, i32 i)\n{",
    );
    assert_contains(
        tag,
        body,
        "ClString __cl_save_1 = (*w).items[__cl_tmp0].name;",
    );
    assert_follows(
        tag,
        body,
        "ClString __cl_save_1 = (*w).items[__cl_tmp0].name;",
        "(*w).items[__cl_tmp0].name = cl_std_text_stringOf(v);",
    );
    assert_follows(
        tag,
        body,
        "(*w).items[__cl_tmp0].name = cl_std_text_stringOf(v);",
        "if (__cl_save_1) { FreeString(__cl_save_1); }",
    );
}

/// A fixed-array member fill saves and clears the row before the view is read.
#[test]
fn should_save_and_clear_a_fixed_array_row_before_its_view_fill() {
    let source = r#"module spec.cprimpl.implsavefixed;

import { String, stringOf } from std.text;

struct Elem { names: String[2]; }

func main(): i32 {
    let a: Elem[2] = [Elem { names: [stringOf("aa"), stringOf("ab")] }, Elem { names: [stringOf("bb"), stringOf("bc")] }];
    let i: i32 = 1;
    let v: str = a[1].names[0];
    a[i].names = [stringOf(v), stringOf("xx")];
    return i32(len(a[i].names[0])) - 2;
}
"#;
    let tag = "implsavefixed";
    let (_scratch, emitted) = emit_host(tag, source);
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_implsavefixed_main(void)\n{",
    );
    assert_contains(
        tag,
        body,
        "memcpy(&(__cl_save_1), &(a[__cl_tmp0].names), sizeof(a[__cl_tmp0].names));",
    );
    assert_contains(
        tag,
        body,
        "memset(&(a[__cl_tmp0].names), 0, sizeof(a[__cl_tmp0].names));",
    );
    assert_follows(
        tag,
        body,
        "__cl_rhs_1[0] = cl_std_text_stringOf(v);",
        "__cl_rhs_1[1] = cl_std_text_stringOf(cl_str(\"xx\"));",
    );
    assert_follows(
        tag,
        body,
        "__cl_rhs_1[1] = cl_std_text_stringOf(cl_str(\"xx\"));",
        "ClString __cl_save_1[2] = {0};",
    );
    assert_follows(
        tag,
        body,
        "memset(&(a[__cl_tmp0].names), 0, sizeof(a[__cl_tmp0].names));",
        "memcpy(&(a[__cl_tmp0].names), &(__cl_rhs_1), sizeof(a[__cl_tmp0].names));",
    );
    assert_follows(
        tag,
        body,
        "memcpy(&(a[__cl_tmp0].names), &(__cl_rhs_1), sizeof(a[__cl_tmp0].names));",
        "memset(&(__cl_rhs_1), 0, sizeof(a[__cl_tmp0].names));",
    );
    assert_follows(
        tag,
        body,
        "memset(&(__cl_rhs_1), 0, sizeof(a[__cl_tmp0].names));",
        "if (__cl_save_1[0]) { FreeString(__cl_save_1[0]); }",
    );
}

/// A runtime-index source place keeps its index and its bounds guard.
#[test]
fn should_pin_a_runtime_index_place_on_the_stored_value() {
    let source = r#"module spec.cprimpl.implrhsindexpin;

struct Elem { y: i32; }
struct Holder { items: Elem[2]; }

let calls: i32 = 0;

func next_index(): i32 {
    calls = calls + 1;
    return 1;
}

func copy<T>(borrow w: T, i: i32, j: i32): i32 {
    w.items[i].y = w.items[j].y;
    return w.items[i].y;
}

func side<T>(borrow w: T, i: i32): i32 {
    w.items[i].y = w.items[next_index()].y;
    return w.items[i].y;
}

func main(): i32 {
    let h1: Holder = Holder { items: [Elem { y: 11 }, Elem { y: 22 }] };
    let h2: Holder = Holder { items: [Elem { y: 33 }, Elem { y: 44 }] };
    let a: i32 = copy<Holder>(borrow h1, 0, 1) - 22;
    let b: i32 = side<Holder>(borrow h2, 0) - 44;
    return a + b + (calls - 1) * 100;
}
"#;
    let tag = "implrhsindexpin";
    let (_scratch, emitted) = emit_host(tag, source);
    let body = function_body(tag, &emitted, "cl_copy__Holder(ClHolder w, i32 i, i32 j)\n{");
    assert_follows(tag, body, "__cl_tmp1 = j;", "if ((__cl_tmp1 < 0) || ((usize)(__cl_tmp1) >= (usize)(2))) cl_trap(\"E11211\", \"index out of range\");");
    assert_follows(
        tag,
        body,
        "if ((__cl_tmp1 < 0) || ((usize)(__cl_tmp1) >= (usize)(2))) cl_trap(\"E11211\", \"index out of range\");",
        "w.items[__cl_tmp0].y = w.items[__cl_tmp1].y;",
    );
    assert_absent(tag, body, "w.items[0].y = w.items[__cl_tmp1].y;");

    let body = function_body(tag, &emitted, "cl_side__Holder(ClHolder w, i32 i)\n{");
    assert_follows(
        tag,
        body,
        "__cl_tmp1 = cl_spec_cprimpl_implrhsindexpin_next_index();",
        "if ((__cl_tmp1 < 0) || ((usize)(__cl_tmp1) >= (usize)(2))) cl_trap(\"E11211\", \"index out of range\");",
    );
    assert_follows(
        tag,
        body,
        "if ((__cl_tmp1 < 0) || ((usize)(__cl_tmp1) >= (usize)(2))) cl_trap(\"E11211\", \"index out of range\");",
        "w.items[__cl_tmp0].y = w.items[__cl_tmp1].y;",
    );
    assert_eq!(
        emitted.matches("implrhsindexpin_next_index()").count(),
        1,
        "{tag}: the source index must run once; emitted C:\n{emitted}"
    );
}

/// A conditional self-arm is judged by index stability.
///
/// A literal index is a self-copy and restores the saved value; a different
/// or local index expression is a v1 element move and reports E10026.
#[test]
fn should_judge_a_conditional_self_arm_by_index_stability() {
    let literal_index = r#"module spec.cprimpl.implselfarm;

import { String, stringOf } from std.text;

struct Inner { name: String; }
struct Elem { inner: Inner; }

func main(): i32 {
    let a: Elem[2] = [Elem { inner: Inner { name: stringOf("aa") } }, Elem { inner: Inner { name: stringOf("bb") } }];
    let c: bool = true;
    a[1].inner = c ? a[1].inner : Inner { name: stringOf("zz") };
    return i32(len(a[1].inner.name)) - 2;
}
"#;
    let tag = "implselfarm";
    let (_scratch, emitted) = emit_host(tag, literal_index);
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_implselfarm_main(void)\n{",
    );
    assert_contains(tag, body, "a[1].inner = __cl_save_1;");
    assert_contains(tag, body, "cl_zero(&__cl_save_1);");

    let equal = r#"module spec.cprimpl.implselfequal;

import { String, stringOf } from std.text;

struct Inner { name: String; }
struct Elem { inner: Inner; }

func main(): i32 {
    let a: Elem[2] = [Elem { inner: Inner { name: stringOf("aa") } }, Elem { inner: Inner { name: stringOf("bb") } }];
    let i: i32 = 1;
    let j: i32 = 1;
    let c: bool = true;
    a[i].inner = c ? a[j].inner : Inner { name: stringOf("zz") };
    return i32(len(a[i].inner.name)) - 2;
}
"#;
    // The same run-time index through a different expression is an element
    // move under the frozen v1 rule.
    assert_rejected("implselfequal", equal, "E10026", 1);
}

/// A generic compound store and update carry the concrete wrap casts.
#[test]
fn should_wrap_a_generic_compound_and_update() {
    let source = r#"module spec.cprimpl.implwrapgeneric;

struct Box { value: i32; }
struct Vec { values: i32[2]; }

func inc<T>(borrow w: T): i32 {
    w.value = 2147483647;
    w.value++;
    return w.value;
}

func addel<T>(borrow w: T, i: i32): i32 {
    w.values[i] = 2147483647;
    w.values[i] += 1;
    return w.values[i];
}

func main(): i32 {
    let b: Box = Box { value: 0 };
    let v: Vec = Vec { values: [0, 0] };
    let a: i32 = inc<Box>(borrow b);
    let c: i32 = addel<Vec>(borrow v, 1);
    return a + c;
}
"#;
    let tag = "implwrapgeneric";
    let (_scratch, emitted) = emit_host(tag, source);
    let body = function_body(tag, &emitted, "cl_inc__Box(ClBox w)\n{");
    assert_contains(tag, body, "w.value = (i32)((u32)w.value + (u32)1);");
    let body = function_body(tag, &emitted, "cl_addel__Vec(ClVec w, i32 i)\n{");
    assert_contains(
        tag,
        body,
        "*(&(w.values[__cl_tmp1])) = (i32)((u32)w.values[__cl_tmp1] + (u32)1);",
    );
}

/// A bare generic row store copies the row instead of assigning the array.
#[test]
fn should_copy_a_bare_generic_row() {
    let source = r#"module spec.cprimpl.implrowstore;

import { String, stringOf } from std.text;

struct Grid { rows: i32[2][2]; tag: String; }
struct TextGrid { rows: String[2][2]; tag: String; }

func setrow<T>(borrow g: T, i: i32, take v: i32[2]): void {
    g.rows[i] = v;
}

func settext<T>(borrow g: T, i: i32, take v: String[2]): void {
    g.rows[i] = v;
}

func main(): i32 {
    let g: Grid = Grid { rows: [[1, 2], [3, 4]], tag: stringOf("g") };
    setrow<Grid>(borrow g, 1, [5, 6]);
    let t: TextGrid = TextGrid { rows: [[stringOf("aa"), stringOf("ab")], [stringOf("bb"), stringOf("bc")]], tag: stringOf("t") };
    settext<TextGrid>(borrow t, 0, [stringOf("xx"), stringOf("yy")]);
    return g.rows[1][0] + i32(len(t.rows[0][0])) - 7;
}
"#;
    let tag = "implrowstore";
    let (_scratch, emitted) = emit_host(tag, source);
    let body = function_body(tag, &emitted, "cl_setrow__Grid(ClGrid *g, i32 i, i32 v[2])\n{");
    assert_contains(
        tag,
        body,
        "memcpy(&((*(&((*g).rows[__cl_tmp0])))), v, sizeof((*(&((*g).rows[__cl_tmp0])))));",
    );
    assert_absent(tag, body, "*(&((*g).rows[__cl_tmp0])) = v;");
    let body = function_body(
        tag,
        &emitted,
        "cl_settext__TextGrid(ClTextGrid *g, i32 i, ClString v[2])\n{",
    );
    assert_contains(
        tag,
        body,
        "memcpy(&(__cl_save_1), &((*(&((*g).rows[__cl_tmp0])))), sizeof((*(&((*g).rows[__cl_tmp0])))));",
    );
    assert_contains(
        tag,
        body,
        "memcpy(&((*(&((*g).rows[__cl_tmp0])))), &(__cl_tmp1), sizeof((*(&((*g).rows[__cl_tmp0])))));",
    );
}

/// A raw-deref member out-slot argument passes the member's address.
#[test]
fn should_store_through_a_raw_deref_member_out_slot() {
    let source = r#"module spec.cprimpl.implrawoutslot;

import { String, stringOf } from std.text;

struct Inner { name: String?; }

func set(borrow slot: String?, pick: bool): void {
    let other: String? = stringOf("bbbb");
    slot = pick ? slot : other;
}

func main(): i32 {
    let inner: Inner = Inner { name: stringOf("aaaa") };
    unsafe {
        let p: raw Inner = &inner;
        set(unsafe (*p).name, true);
    }
    if (inner.name == null) { return 9; }
    return i32(len(inner.name)) - 4;
}
"#;
    let tag = "implrawoutslot";
    let (_scratch, emitted) = emit_host(tag, source);
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_implrawoutslot_main(void)\n{",
    );
    assert_contains(
        tag,
        body,
        "cl_spec_cprimpl_implrawoutslot_set(&((*(p)).name), 1);",
    );
    assert_absent(tag, body, "set(&(*(p).name)");
}

/// A conditional pointer or view value is not a store place.
///
/// The dereferenced conditional remains the accepted control: `*(c ? p : q)`
/// names the selected pointer's pointee.
#[test]
fn should_reject_a_conditional_value_base_store() {
    let raw = r#"module spec.cprimpl.implcondrawbase;

struct Inner { x: i32; }
struct S { f: Inner; }

func use(c: bool, p: raw S, q: raw S): i32 {
    (c ? p : q).f.x = 7;
    return 0;
}

func main(): i32 {
    let a: S = S { f: Inner { x: 0 } };
    let b: S = S { f: Inner { x: 0 } };
    unsafe {
        return use(true, &a, &b);
    }
}
"#;
    assert_rejected_once_at("implcondrawbase", raw, "E10221", 7, 5, &["left side of an assignment must be a place"], &[]);

    let view = r#"module spec.cprimpl.implcondviewbase;

struct Elem { y: i32; }

func use(c: bool, a: Elem[2], b: Elem[2], i: i32): i32 {
    (c ? a : b)[i].y = 7;
    return 0;
}

func main(): i32 {
    let a: Elem[2] = [Elem { y: 0 }, Elem { y: 0 }];
    let b: Elem[2] = [Elem { y: 0 }, Elem { y: 0 }];
    return use(true, a, b, 1);
}
"#;
    assert_rejected_once_at("implcondviewbase", view, "E10221", 6, 5, &["left side of an assignment must be a place"], &[]);

    let slice = r#"module spec.cprimpl.implcondslicebase;

struct Elem { y: i32; }

func use(c: bool, a: Elem[], b: Elem[], i: i32): i32 {
    (c ? a : b)[i].y = 7;
    return 0;
}

func main(): i32 {
    return 0;
}
"#;
    assert_rejected_once_at("implcondslicebase", slice, "E10221", 6, 5, &["left side of an assignment must be a place"], &[]);

    let deref = r#"module spec.cprimpl.implcondderef;

struct Inner { x: i32; }
struct S { f: Inner; }

func main(): i32 {
    let a: S = S { f: Inner { x: 0 } };
    let b: S = S { f: Inner { x: 0 } };
    let c: bool = true;
    unsafe {
        let p: raw S = &a;
        let q: raw S = &b;
        *(c ? p : q) = S { f: Inner { x: 7 } };
    }
    return a.f.x - 7;
}
"#;
    let tag = "implcondderef";
    let (_scratch, emitted) = emit_host(tag, deref);
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_implcondderef_main(void)\n{",
    );
    assert_contains(tag, body, "*(&(*((c ? p : q)))) = (ClS){ (ClInner){ 7 } };");
}

/// A distinct-root fixed-array conditional arm is emptied after its bytes move.
///
/// The address-equal branch restores the saved destination; the distinct branch
/// copies the arm and clears the arm's own slot, so the destination and the arm
/// do not both own the moved handles.
#[test]
fn should_empty_a_distinct_root_fixed_array_conditional_arm() {
    let member = r#"module spec.cprimpl.implarmdistinct;

import { String, stringOf } from std.text;

struct Elem { names: String[2]; }

func main(): i32 {
    let a: Elem[2] = [Elem { names: [stringOf("aa"), stringOf("ab")] }, Elem { names: [stringOf("bb"), stringOf("bc")] }];
    let z: Elem = Elem { names: [stringOf("jj"), stringOf("kk")] };
    let i: i32 = 1;
    let c: bool = true;
    a[i].names = c ? z.names : [stringOf("zz"), stringOf("ww")];
    return i32(len(a[i].names[0])) - 2;
}
"#;
    let tag = "implarmdistinct";
    let (_scratch, emitted) = emit_host(tag, member);
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_implarmdistinct_main(void)\n{",
    );
    assert_follows(
        tag,
        body,
        "memcpy(&(a[__cl_tmp0].names), &(z.names), sizeof(a[__cl_tmp0].names));",
        "cl_zero(&z.names);",
    );
    assert_follows(
        tag,
        body,
        "if (((void *)(&(z.names))) == ((void *)(&(a[__cl_tmp0].names)))) {",
        "memcpy(&(a[__cl_tmp0].names), &(__cl_save_1), sizeof(a[__cl_tmp0].names));",
    );

    let whole = r#"module spec.cprimpl.implarmdistinctwhole;

import { String, stringOf } from std.text;

func main(): i32 {
    let a: String[2][2] = [[stringOf("aa"), stringOf("ab")], [stringOf("bb"), stringOf("bc")]];
    let z: String[2] = [stringOf("jj"), stringOf("kk")];
    let i: i32 = 1;
    let c: bool = true;
    a[i] = c ? z : [stringOf("zz"), stringOf("ww")];
    return i32(len(a[i][0])) - 2;
}
"#;
    let tag = "implarmdistinctwhole";
    let (_scratch, emitted) = emit_host(tag, whole);
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_implarmdistinctwhole_main(void)\n{",
    );
    assert_follows(
        tag,
        body,
        "memcpy(&((*(&(a[__cl_tmp0])))), &(z), sizeof((*(&(a[__cl_tmp0])))));",
        "cl_zero(&z);",
    );
    // A separate root cannot alias the target, so this whole-array arm keeps
    // the direct byte copy and source emptying; the member twin above pins the
    // address-equality branch.
}

/// An identical self-slot source move emits no transfer.
///
/// `w.items[i].name = w.items[i].name` names one object on both sides, so the
/// generic borrow body is a self-copy: no address comparison, no move, and no
/// empty. A distinct-index owned move out of the same borrow parameter is the
/// `E10034` rejection pinned by the spec suite.
#[test]
fn should_skip_an_identical_self_slot_source_move() {
    let same = r#"module spec.cprimpl.implselfslotsame;

import { String, stringOf } from std.text;

struct Elem { name: String; }
struct Holder { items: Elem[2]; }

func copy<T>(borrow w: T, i: i32): i32 {
    w.items[i].name = w.items[i].name;
    return i32(len(w.items[i].name));
}

func main(): i32 {
    let h: Holder = Holder { items: [Elem { name: stringOf("aa") }, Elem { name: stringOf("bb") }] };
    return copy<Holder>(borrow h, 1) - 2;
}
"#;
    let tag = "implselfslotsame";
    let (_scratch, emitted) = emit_host(tag, same);
    let body = function_body(tag, &emitted, "cl_copy__Holder(ClHolder *w, i32 i)\n{");
    // The same index expression is a static self-copy: no address comparison
    // and no move are emitted, and the element keeps its value.
    assert_absent(tag, body, "!= &(");
    assert_absent(tag, body, "= (*w).items[__cl_tmp1].name;");
    assert_contains(
        tag,
        body,
        "return (i32)((cl_string_view((*w).items[i].name)).len);",
    );
}

/// An identical whole-element self-copy emits no move at all.
///
/// `w.items[i] = w.items[i]` names one object on both sides, so the store needs
/// no drop, copy, or empty. The host and hosted-amiga emissions both keep the
/// target guard and the read-back and emit no address comparison, which a C
/// compiler would otherwise fold into a tautological compare.
#[test]
fn should_skip_an_identical_whole_element_self_copy() {
    let source = r#"module spec.cprimpl.implselfsamewhole;

import { String, stringOf } from std.text;

struct Elem { name: String; }
struct Holder { items: Elem[2]; }

func copy<T>(borrow w: T, i: i32): i32 {
    w.items[i] = w.items[i];
    return i32(len(w.items[i].name));
}

func main(): i32 {
    let h: Holder = Holder { items: [Elem { name: stringOf("aa") }, Elem { name: stringOf("bb") }] };
    return copy<Holder>(borrow h, 1) - 2;
}
"#;
    for profile in ["host", "hosted-amiga"] {
        let tag = if profile == "host" {
            "implselfsamewhole"
        } else {
            "implselfsamewholeamiga"
        };
        let scratch = ScratchDir::new(tag);
        let source_path = write_source(tag, &scratch, source);
        let c_path = scratch.path().join("out.c");
        let output = run_clc(&[
            "--profile".to_owned(),
            profile.to_owned(),
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
        let emitted = std::fs::read_to_string(&c_path).unwrap_or_default();
        let body = function_body(tag, &emitted, "cl_copy__Holder(ClHolder *w, i32 i)\n{");
        assert_absent(tag, body, "!= &(");
        assert_absent(tag, body, "*(&((*w).items[i])) = (*w).items[i];");
        assert_contains(
            tag,
            body,
            "if ((i < 0) || ((usize)(i) >= (usize)(2))) cl_trap(\"E11211\", \"index out of range\");",
        );
        assert_contains(
            tag,
            body,
            "return (i32)((cl_string_view((*w).items[i].name)).len);",
        );
    }
}

// ---------------------------------------------------------------------------
// Element and store loans. A view read out of a loaned element carries the
// element's loan to the copy, and a conditional or array-literal copy hands
// the destination every source position as one union. The rejected shapes
// pin the report code, count, and site; the accepted shapes pin the copy
// emission; the overwrite drops the loan the replaced element carried; and
// the borrow-element conditional take is the count-and-span control.
// ---------------------------------------------------------------------------

/// A view stored from an element carries the element's loan: the copy passed
/// to an out-slot reports the owner's scope, exactly as the direct element
/// and member arguments do.
const ELEMENT_READ_STORE_SOURCE: &str = r#"module spec.cprimpl.elemreadstore;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let s: str = xs[0];
    fill(s, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;

/// The direct element argument control: the element read itself reports.
const ELEMENT_READ_DIRECT_SOURCE: &str = r#"module spec.cprimpl.elemreaddirect;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    fill(xs[0], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;

/// The member argument control: a projection of the same owner reports.
const ELEMENT_READ_MEMBER_SOURCE: &str = r#"module spec.cprimpl.elemreadmember;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa"), w: stringOf("bb") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    xs[1] = a.w;
    fill(a.w, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;

/// The in-scope control: the stored view is read while its owner is alive.
const ELEMENT_READ_IN_SCOPE_SOURCE: &str = r#"module spec.cprimpl.elemreadinscope;

import { String, stringOf } from std.text;

struct Box { v: String; }

func outer(): i32 {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let s: str = xs[0];
    return i32(s[0]) - 97;
}

func main(): i32 { return outer(); }
"#;

/// The element-read store reports once at its fill; the direct and member
/// twins keep their own report; the in-scope copy emits the plain element
/// read and its bounded read-back.
#[test]
fn should_pin_the_element_read_store_loan() {
    assert_rejected_once_at(
        "implelemreadstore",
        ELEMENT_READ_STORE_SOURCE,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );
    assert_rejected_once_at(
        "implelemreaddirect",
        ELEMENT_READ_DIRECT_SOURCE,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );
    assert_rejected_once_at(
        "implelemreadmember",
        ELEMENT_READ_MEMBER_SOURCE,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );
    let tag = "implelemreadinscope";
    let (_scratch, emitted) = emit_host(tag, ELEMENT_READ_IN_SCOPE_SOURCE);
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_elemreadinscope_outer(void)\n{",
    );
    assert_contains(tag, body, "s = xs[0];");
    assert_contains(tag, body, "((s).ptr[0])");
}

/// The conditional array copy carries the selected arm's element loans.
const COPY_CONDITIONAL_SOURCE: &str = r#"module spec.cprimpl.copycond;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?, c: bool): void {
    let a: Box = Box { v: stringOf("aa"), w: stringOf("bb") };
    let xs: str[2] = ["AA", "BB"];
    let zs: str[2] = ["CC", "DD"];
    xs[0] = a.v;
    zs[0] = a.w;
    let ys: str[2] = (c ? xs : zs);
    fill(ys[0], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out, true);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;

/// The array-literal copy carries each element's loan under its own index.
const COPY_LITERAL_SOURCE: &str = r#"module spec.cprimpl.copyliteral;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa"), w: stringOf("bb") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    xs[1] = a.w;
    let ys: str[2] = [xs[0], xs[1]];
    fill(ys[0], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;

/// The fill-form literal copy carries the listed element's loan while the
/// fill value contributes none.
const COPY_LITERAL_FILL_SOURCE: &str = r#"module spec.cprimpl.copyliteralfill;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let ys: str[2] = [xs[0]; "EE"];
    fill(ys[0], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;

/// The single-view conditional shares both arms' element loans.
const COPY_SINGLE_VIEW_SOURCE: &str = r#"module spec.cprimpl.copysingle;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(c: bool, borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let b: Box = Box { v: stringOf("cc") };
    let xs: str[1] = ["AA"];
    let zs: str[1] = ["CC"];
    xs[0] = a.v;
    zs[0] = b.v;
    let s: str = c ? xs[0] : zs[0];
    fill(s, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(true, borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;

/// A conditional whose arm is an array literal unions the arm's positions
/// with the other arm.
const COPY_ARM_LITERAL_SOURCE: &str = r#"module spec.cprimpl.copycondarm;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(c: bool, borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa"), w: stringOf("bb") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    xs[1] = a.w;
    let zs: str[2] = ["CC", "DD"];
    let ys: str[2] = c ? [xs[0], xs[1]] : zs;
    fill(ys[0], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(true, borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;

/// A conditional element inside an array literal recurses into both arms and
/// records the selected position's loans.
const COPY_ELEMENT_IN_LITERAL_SOURCE: &str = r#"module spec.cprimpl.copymix;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(c: bool, borrow s0: str?, borrow s1: str?): void {
    let a: Box = Box { v: stringOf("aa"), w: stringOf("bb") };
    let b: Box = Box { v: stringOf("cc"), w: stringOf("dd") };
    let xs: str[2] = ["AA", "BB"];
    let zs: str[2] = ["CC", "DD"];
    xs[0] = a.v;
    zs[0] = b.v;
    zs[1] = b.w;
    let ys: str[2] = [c ? xs[0] : zs[0], xs[1]];
    fill(ys[0], borrow s0);
    fill(ys[1], borrow s1);
}

func main(): i32 {
    let out0: str? = null;
    let out1: str? = null;
    outer(true, borrow out0, borrow out1);
    if (out0 == null) { return 9; }
    if (out1 == null) { return 10; }
    return i32(out0[0]) + i32(out1[0]) - (97 + 66);
}
"#;

/// The two arms fill different element counts, so the union keeps the first
/// arm's second position and the second arm's first position.
const COPY_DIFFERENT_ARMS_SOURCE: &str = r#"module spec.cprimpl.copydiff;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(c: bool, borrow s0: str?, borrow s1: str?): void {
    let a: Box = Box { v: stringOf("aa"), w: stringOf("bb") };
    let xs: str[2] = ["AA", "BB"];
    let zs: str[2] = ["CC", "DD"];
    xs[0] = a.v;
    xs[1] = a.w;
    zs[0] = a.v;
    let ys: str[2] = c ? xs : zs;
    fill(ys[0], borrow s0);
    fill(ys[1], borrow s1);
}

func main(): i32 {
    let out0: str? = null;
    let out1: str? = null;
    outer(true, borrow out0, borrow out1);
    if (out0 == null) { return 9; }
    if (out1 == null) { return 10; }
    return i32(out0[0]) + i32(out1[0]) - (97 + 98);
}
"#;

/// The mixed-position copy loans one element from each arm, so the
/// destination record must keep both positions.
const COPY_MIXED_POSITIONS_SOURCE: &str = r#"module spec.cprimpl.copyunion;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(c: bool, borrow s0: str?, borrow s1: str?): void {
    let a: Box = Box { v: stringOf("aa"), w: stringOf("bb") };
    let b: Box = Box { v: stringOf("cc"), w: stringOf("dd") };
    let xs: str[2] = ["AA", "BB"];
    let zs: str[2] = ["CC", "DD"];
    xs[0] = a.v;
    zs[1] = b.w;
    let ys: str[2] = (c ? xs : zs);
    fill(ys[0], borrow s0);
    fill(ys[1], borrow s1);
}

func main(): i32 {
    let out0: str? = null;
    let out1: str? = null;
    outer(true, borrow out0, borrow out1);
    if (out0 == null) { return 9; }
    if (out1 == null) { return 10; }
    return i32(out0[0]) + i32(out1[0]) - (97 + 100);
}
"#;

/// The first arm's position alone: only a union record reports this fill.
const COPY_UNION_FIRST_SOURCE: &str = r#"module spec.cprimpl.copyunionfirst;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(c: bool, borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa"), w: stringOf("bb") };
    let b: Box = Box { v: stringOf("cc"), w: stringOf("dd") };
    let xs: str[2] = ["AA", "BB"];
    let zs: str[2] = ["CC", "DD"];
    xs[0] = a.v;
    zs[1] = b.w;
    let ys: str[2] = (c ? xs : zs);
    fill(ys[0], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(true, borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;

/// The second arm's position alone: only a union record reports this fill.
const COPY_UNION_SECOND_SOURCE: &str = r#"module spec.cprimpl.copyunionsecond;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(c: bool, borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa"), w: stringOf("bb") };
    let b: Box = Box { v: stringOf("cc"), w: stringOf("dd") };
    let xs: str[2] = ["AA", "BB"];
    let zs: str[2] = ["CC", "DD"];
    xs[0] = a.v;
    zs[1] = b.w;
    let ys: str[2] = (c ? xs : zs);
    fill(ys[1], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(true, borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 98;
}
"#;

/// The conditional array copy reports once at the fill.
#[test]
fn should_pin_the_conditional_copy_fill() {
    assert_rejected_once_at(
        "implcopycond",
        COPY_CONDITIONAL_SOURCE,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );
}

/// The array-literal copy reports once per affected fill.
#[test]
fn should_pin_the_array_literal_copy_fill() {
    assert_rejected_once_at(
        "implcopyliteral",
        COPY_LITERAL_SOURCE,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );
}

/// The fill-form literal copy reports at the listed element's fill only.
#[test]
fn should_pin_the_fill_form_copy_fill() {
    assert_rejected_once_at(
        "implcopyliteralfill",
        COPY_LITERAL_FILL_SOURCE,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );
}

/// The single-view conditional reports at its one fill.
#[test]
fn should_pin_the_single_view_copy_fill() {
    assert_rejected_once_at(
        "implcopysingle",
        COPY_SINGLE_VIEW_SOURCE,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );
}

/// The conditional-with-an-array-literal arm reports at its fill.
#[test]
fn should_pin_the_arm_literal_copy_fill() {
    assert_rejected_once_at(
        "implcopycondarm",
        COPY_ARM_LITERAL_SOURCE,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );
}

/// The conditional element inside an array literal reports at its fill and
/// leaves the statically filled sibling accepted.
#[test]
fn should_pin_the_element_in_literal_copy_fill() {
    assert_rejected_once_at(
        "implcopymix",
        COPY_ELEMENT_IN_LITERAL_SOURCE,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );
}

/// The different-arm-count copy reports at every affected fill.
#[test]
fn should_pin_the_different_arm_count_copy_fills() {
    assert_rejected_at_sites(
        "implcopydiff",
        COPY_DIFFERENT_ARMS_SOURCE,
        "E10034",
        &[(7, 46)],
        &["cannot release or store borrowed parameter 'p'"],
    );
}

/// The mixed-position copy reports at both fills, so neither arm's position
/// was replaced by the other arm's.
#[test]
fn should_pin_the_mixed_position_copy_fills() {
    assert_rejected_at_sites(
        "implcopyunion",
        COPY_MIXED_POSITIONS_SOURCE,
        "E10034",
        &[(7, 46)],
        &["cannot release or store borrowed parameter 'p'"],
    );
}

/// The first arm's position survives the union: filling only that element
/// still reports its owner.
#[test]
fn should_pin_the_first_arm_position_of_the_copy_union() {
    assert_rejected_once_at(
        "implcopyunionfirst",
        COPY_UNION_FIRST_SOURCE,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );
}

/// The second arm's position survives the union: filling only that element
/// still reports its owner.
#[test]
fn should_pin_the_second_arm_position_of_the_copy_union() {
    assert_rejected_once_at(
        "implcopyunionsecond",
        COPY_UNION_SECOND_SOURCE,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );
}

/// The in-scope copies emit their plain store shapes: the conditional arms
/// stay two byte copies, the literal elements stay element stores, the
/// single view stays one selected copy, and the arm literal keeps its
/// element stores beside the other arm's byte copy.
const COPY_IN_SCOPE_SOURCE: &str = r#"module spec.cprimpl.copyinscope;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func outer(c: bool): i32 {
    let a: Box = Box { v: stringOf("aa"), w: stringOf("bb") };
    let b: Box = Box { v: stringOf("cc"), w: stringOf("dd") };
    let xs: str[2] = ["AA", "BB"];
    let zs: str[2] = ["CC", "DD"];
    xs[0] = a.v;
    xs[1] = a.w;
    zs[0] = b.v;
    zs[1] = b.w;
    let ys: str[2] = (c ? xs : zs);
    let lit: str[2] = [xs[0]; "EE"];
    let arr: str[2] = [c ? xs[0] : zs[0], xs[1]];
    let one: str = c ? xs[0] : zs[0];
    let arm: str[2] = c ? [xs[0], xs[1]] : zs;
    return i32(ys[0][0]) + i32(lit[0][0]) + i32(arr[0][0]) + i32(one[0])
         + i32(arm[1][0]) - (97 + 97 + 97 + 97 + 98);
}

func main(): i32 { return outer(true); }
"#;

/// The accepted copies keep their emitted store shapes.
#[test]
fn should_pin_the_accepted_copy_emissions() {
    let tag = "implcopyinscope";
    let (_scratch, emitted) = emit_host(tag, COPY_IN_SCOPE_SOURCE);
    let body = function_body(tag, &emitted, "cl_spec_cprimpl_copyinscope_outer(bool c)\n{");
    assert_contains(tag, body, "memcpy(&(ys), &(xs), sizeof(ys));");
    assert_contains(tag, body, "memcpy(&(ys), &(zs), sizeof(ys));");
    assert_contains(tag, body, "lit[0] = xs[0];");
    assert_contains(tag, body, "arr[0] = (c ? xs[0] : zs[0]);");
    assert_contains(tag, body, "one = (c ? xs[0] : zs[0]);");
    assert_contains(tag, body, "arm[0] = xs[0];");
    assert_contains(tag, body, "memcpy(&(arm), &(zs), sizeof(arm));");
}

/// The fill-form copy that is used before the overwrite reports the loan.
const COPY_DROP_LIVE_SOURCE: &str = r#"module spec.cprimpl.copydroplive;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let ys: str[2] = [xs[0]; "EE"];
    fill(ys[0], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;

/// The same copy whose loaned element is overwritten before the fill.
const COPY_DROP_SOURCE: &str = r#"module spec.cprimpl.copydrop;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let ys: str[2] = [xs[0]; "EE"];
    ys[0] = "ZZ";
    fill(ys[0], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 90;
}
"#;

/// The overwrite drops the replaced element's loan: the live twin reports at
/// its fill, while the overwritten copy stays accepted and emits the
/// overwrite before the fill.
#[test]
fn should_drop_the_overwritten_copy_loan() {
    assert_rejected_once_at(
        "implcopydroplive",
        COPY_DROP_LIVE_SOURCE,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );
    let tag = "implcopydrop";
    assert_rejected(tag, COPY_DROP_SOURCE, "E10034", 1);
}

/// The generic borrow-element conditional take control.
const BORROW_ELEMENT_GENERIC_SOURCE: &str = r#"module spec.cprimpl.borrowelemcond;

import { String, stringOf } from std.text;

struct S { name: String; }

func eat<T>(take x: T): i32 { return 1; }

func each<T>(xs: T, ys: T, c: bool): i32 {
    for (borrow v in xs) {
        for (borrow w in ys) {
            return eat(take (c ? v.name : w.name));
        }
    }
    return 0;
}

func main(): i32 {
    let a: S[1] = [S { name: stringOf("a") }];
    let b: S[1] = [S { name: stringOf("b") }];
    return each<S[1]>(a, b, true) - 1;
}
"#;

/// The concrete borrow-element conditional take control.
const BORROW_ELEMENT_CONCRETE_SOURCE: &str = r#"module spec.cprimpl.borrowelemcondconcrete;

import { String, stringOf } from std.text;

struct S { name: String; }

func eat(take x: String): i32 { return 1; }

func each(xs: S[1], ys: S[1], c: bool): i32 {
    for (borrow v in xs) {
        for (borrow w in ys) {
            return eat(take (c ? v.name : w.name));
        }
    }
    return 0;
}

func main(): i32 {
    let a: S[1] = [S { name: stringOf("a") }];
    let b: S[1] = [S { name: stringOf("b") }];
    return each(a, b, true) - 1;
}
"#;

/// The borrow-element conditional take reports the partial-move limit twice
/// at one span in both the generic and the concrete spelling, so the two
/// arms agree in count and site.
#[test]
fn should_pin_the_borrow_element_conditional_take_parity() {
    let sites = [(12, 24), (12, 24)];
    assert_rejected_at_sites(
        "implborrowelemcond",
        BORROW_ELEMENT_GENERIC_SOURCE,
        "E10026",
        &sites,
        &["partial moves are limited to top-level fields in v1"],
    );
    assert_rejected_at_sites(
        "implborrowelemcondconcrete",
        BORROW_ELEMENT_CONCRETE_SOURCE,
        "E10026",
        &sites,
        &["partial moves are limited to top-level fields in v1"],
    );
}

/// The accepted loan-carrying programs re-emit byte-identically; the foreign
/// drop form reports `E10034`.
#[test]
fn should_re_emit_the_accepted_copy_loan_programs_byte_identically() {
    for (tag, source) in [
        ("implcopyinscope", COPY_IN_SCOPE_SOURCE),
        ("implelemreadinscope", ELEMENT_READ_IN_SCOPE_SOURCE),
    ] {
        let (first, second) = emit_twice(tag, source);
        assert_eq!(
            first, second,
            "{tag}: the two emissions differ; first:\n{first}\nsecond:\n{second}"
        );
    }
    assert_rejected("implcopydrop", COPY_DROP_SOURCE, "E10034", 1);
}

// ---------------------------------------------------------------------------
// Fill-position loans. A fill over a known fixed extent records each element
// it stores, so the unused-fill and overwritten-element copies stay accepted
// with their plain emissions, a covered element copy reports like its direct
// twin, and a run-time element copy carries the carried loan. The write
// conflict keeps its one-report direct form and its two-report global form.
// ---------------------------------------------------------------------------

/// Asserts the front end rejected `source` with one report per `(code, line,
/// column)` entry, in any order, and emitted no translation unit.
fn assert_rejected_codes(tag: &str, source: &str, reports: &[(&str, usize, usize)]) {
    let scratch = ScratchDir::new(tag);
    let source_path = write_source(tag, &scratch, source);
    let c_path = scratch.path().join("out.c");
    let output = run_clc(&[
        "--profile".to_owned(),
        "host".to_owned(),
        "-o".to_owned(),
        c_path.display().to_string(),
        source_path.display().to_string(),
    ]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_ne!(
        output.status.code(),
        Some(0),
        "{tag}: expected a rejection; stderr:\n{stderr}"
    );
    let mut actual: Vec<(String, String)> = Vec::new();
    let mut lines = stderr.lines();
    while let Some(current) = lines.next() {
        if !current.starts_with("error ") {
            continue;
        }
        let code = current
            .split_whitespace()
            .nth(1)
            .unwrap_or("")
            .trim_end_matches(':')
            .to_owned();
        let Some(location) = lines.next() else { continue };
        actual.push((code, location.trim().to_owned()));
    }
    assert_eq!(
        actual.len(),
        reports.len(),
        "{tag}: expected {} error(s), saw {}; stderr:\n{stderr}",
        reports.len(),
        actual.len()
    );
    for (code, line, column) in reports {
        let suffix = format!(":{line}:{column}");
        let position = actual
            .iter()
            .position(|(actual_code, site)| actual_code == code && site.ends_with(&suffix))
            .unwrap_or_else(|| panic!("{tag}: expected {code} at {suffix}; stderr:\n{stderr}"));
        actual.remove(position);
    }
    assert!(
        !c_path.exists(),
        "{tag}: a rejected program must not produce a translation unit"
    );
}

/// An array literal whose fill stores no remaining element.
const FILL_UNUSED_SOURCE: &str = r#"module spec.cprimpl.fillunused;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): i32 {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let ys: str[2] = ["AA", "BB"; xs[0]];
    fill(ys[0], borrow slot);
    return i32(ys[0][0]) + i32(xs[0][0]) - (65 + 97);
}

func main(): i32 {
    let out: str? = null;
    let rc: i32 = outer(borrow out);
    if (out == null) { return 9; }
    return rc + i32(out[0]) - 65;
}
"#;

/// A fill that covers a known extent and is overwritten at one element.
const FILL_OVERWRITE_SOURCE: &str = r#"module spec.cprimpl.filloverwrite;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let ys: str[4] = [; a.v];
    ys[2] = "ZZ";
    fill(ys[2], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 90;
}
"#;

/// A copy of a fill-covered element carries the element's loan.
const FILL_COVERED_COPY_SOURCE: &str = r#"module spec.cprimpl.fillcovered;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa"), w: stringOf("bb") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    xs[1] = a.w;
    let ys: str[4] = [xs[0]; xs[1]];
    let s: str = ys[2];
    fill(s, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;

/// The direct read of the same fill-covered element.
const FILL_COVERED_DIRECT_SOURCE: &str = r#"module spec.cprimpl.fillcovereddirect;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa"), w: stringOf("bb") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    xs[1] = a.w;
    let ys: str[4] = [xs[0]; xs[1]];
    fill(ys[2], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;

/// The fill-only spelling of the covered element copy.
const FILL_ONLY_COPY_SOURCE: &str = r#"module spec.cprimpl.fillonly;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let ys: str[4] = [; xs[0]];
    let s: str = ys[2];
    fill(s, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;

/// The fill-only direct twin.
const FILL_ONLY_DIRECT_SOURCE: &str = r#"module spec.cprimpl.fillonlydirect;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let ys: str[4] = [; xs[0]];
    fill(ys[2], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;

/// A run-time element store followed by a run-time element copy.
const RUNTIME_COPY_SOURCE: &str = r#"module spec.cprimpl.runtimecopy;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(k: u32, borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[k] = a.v;
    let s: str = xs[k];
    fill(s, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(0, borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;

/// A constant element store followed by a run-time element copy.
const RUNTIME_COPY_CONST_SOURCE: &str = r#"module spec.cprimpl.runtimecopyconst;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(k: u32, borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let s: str = xs[k];
    fill(s, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(0, borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;

/// A direct element read after a conflicting write.
const DIRECT_WRITE_SOURCE: &str = r#"module spec.cprimpl.directwrite;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa"), w: stringOf("bb") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    a.v = stringOf("qq");
    fill(xs[0], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;

/// The global-destination form of the write conflict.
const GLOBAL_WRITE_SOURCE: &str = r#"module spec.cprimpl.globalwrite;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

let g: str = "AA";

func outer(): void {
    let a: Box = Box { v: stringOf("aa"), w: stringOf("bb") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    a.v = stringOf("qq");
    g = xs[0];
}

func main(): i32 {
    outer();
    return i32(g[0]) - 97;
}
"#;

/// The unused fill emits only the listed element stores, so its loan never
/// reaches the destination.
#[test]
fn should_pin_the_unused_fill_emission() {
    let tag = "implfillunused";
    assert_rejected(tag, FILL_UNUSED_SOURCE, "E10034", 1);
}

/// The known extent expands the fill onto every element it stores, and the
/// overwrite replaces exactly the element it names before the fill reads it.
#[test]
fn should_pin_the_expanded_fill_and_its_overwrite() {
    let tag = "implfilloverwrite";
    assert_rejected(tag, FILL_OVERWRITE_SOURCE, "E10034", 1);
}

/// A copy of a fill-covered element reports at its fill exactly as the
/// direct read does, in both the listed-fill and fill-only spellings.
#[test]
fn should_pin_the_fill_covered_element_copy_reports() {
    assert_rejected_once_at(
        "implfillcovered",
        FILL_COVERED_COPY_SOURCE,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );
    assert_rejected_once_at(
        "implfillcovereddirect",
        FILL_COVERED_DIRECT_SOURCE,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );
    assert_rejected_once_at(
        "implfillonly",
        FILL_ONLY_COPY_SOURCE,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );
    assert_rejected_once_at(
        "implfillonlydirect",
        FILL_ONLY_DIRECT_SOURCE,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );
}

/// The run-time copy carries the loan of the element it may select, whether
/// the recorded store was constant or run-time.
#[test]
fn should_pin_the_runtime_index_element_copy() {
    assert_rejected_once_at(
        "implruntimecopy",
        RUNTIME_COPY_SOURCE,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );
    assert_rejected_once_at(
        "implruntimecopyconst",
        RUNTIME_COPY_CONST_SOURCE,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );
}

/// The direct read after a conflicting write keeps one `E10031`; the fill's
/// foreign store reports `E10034` beside it.
#[test]
fn should_pin_the_direct_write_conflict_reading() {
    assert_rejected_codes(
        "impldirectwrite",
        DIRECT_WRITE_SOURCE,
        &[("E10034", 7, 46), ("E10031", 13, 5)],
    );
}

/// The global form reports the write conflict and the global escape together.
#[test]
fn should_pin_the_global_write_conflict_reading() {
    assert_rejected_codes(
        "implglobalwrite",
        GLOBAL_WRITE_SOURCE,
        &[("E10031", 13, 5), ("E10033", 14, 5)],
    );
}

// ---------------------------------------------------------------------------
// Nested fixed-extent fills. A nested literal keeps the element type at its
// own nesting level, so its fill expands per element at that dimension: an
// unused nested fill emits no fill store, an overwritten fill row drops each
// element, and a covered element copy reports at its fill.
// ---------------------------------------------------------------------------

/// The inner fill stores no element because the row's elements cover it.
const NESTED_UNUSED_SOURCE: &str = r#"module spec.cprimpl.nestedunused;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): i32 {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let g: str[2][2] = [["AA", "BB"], ["CC", "DD"; xs[0]]];
    fill(g[1][0], borrow slot);
    return i32(g[1][0][0]) + i32(xs[0][0]) - (67 + 97);
}

func main(): i32 {
    let out: str? = null;
    let rc: i32 = outer(borrow out);
    if (out == null) { return 9; }
    return rc + i32(out[0]) - 67;
}
"#;

/// A fill-only row whose every element is overwritten before its read.
const NESTED_FILLONLY_OVERWRITTEN_SOURCE: &str = r#"module spec.cprimpl.nestedfillonlyoverwritten;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let g: str[2][2] = [["AA", "BB"], [; xs[0]]];
    g[1][0] = "ZZ";
    g[1][1] = "YY";
    fill(g[1][1], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 89;
}
"#;

/// A fill-only row copied at a covered element.
const NESTED_FILLONLY_COPY_SOURCE: &str = r#"module spec.cprimpl.nestedfillonly;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let g: str[2][2] = [["AA", "BB"], [; xs[0]]];
    let s: str = g[1][0];
    fill(s, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;

/// The same covered element copied through a whole-array copy.
const NESTED_COPY2_SOURCE: &str = r#"module spec.cprimpl.nestedcopy2;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let g: str[2][2] = [["AA", "BB"], [; xs[0]]];
    let h: str[2][2] = g;
    let s: str = h[1][0];
    fill(s, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;

/// The conditional spelling of the nested covered element.
const NESTED_COND_COPY_SOURCE: &str = r#"module spec.cprimpl.nestedcondcopy;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(c: bool, borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let zs: str[2][2] = [["EE", "FF"], ["GG", "HH"]];
    let g: str[2][2] = c ? [["AA", "BB"], [; xs[0]]] : zs;
    let s: str = g[1][0];
    fill(s, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(true, borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;

/// The unused nested fill emits only the listed row stores.
#[test]
fn should_pin_the_nested_unused_fill_emission() {
    let tag = "implnestedunused";
    assert_rejected(tag, NESTED_UNUSED_SOURCE, "E10034", 1);
}

/// The nested fill-only row expands at its own dimension, and the overwrites
/// replace both elements before the fill reads one.
#[test]
fn should_pin_the_nested_fill_expansion_and_overwrite() {
    let tag = "implnestedfillonlyoverwritten";
    assert_rejected(tag, NESTED_FILLONLY_OVERWRITTEN_SOURCE, "E10034", 1);
}

/// A copy of a nested fill-covered element reports at its fill in the three
/// copy spellings.
#[test]
fn should_pin_the_nested_fill_covered_element_copies() {
    assert_rejected_once_at(
        "implnestedfillonly",
        NESTED_FILLONLY_COPY_SOURCE,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );
    assert_rejected_once_at(
        "implnestedcopy2",
        NESTED_COPY2_SOURCE,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );
    assert_rejected_once_at(
        "implnestedcondcopy",
        NESTED_COND_COPY_SOURCE,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );
}

// ===== Owning raw-deref materialization and guarded deref takes =====

/// The generic store program: a `String` instance stores through an impure
/// pointer expression that advances a counter on every call.
const DEREF_OWNER_STORE_SOURCE: &str = r#"module spec.cprimpl.derefownerstore;

import { String, stringOf } from std.text;

let calls: i32 = 0;

func alt<T>(s: raw T, t: raw T): raw T {
    let i: i32 = calls;
    calls = calls + 1;
    if (i == 0) { return s; }
    return t;
}

func overwrite<T>(p: raw T, q: raw T, take v: T): T {
    unsafe {
        v = *alt(p, q);
    }
    return v;
}

func main(): i32 {
    let a: String = stringOf("aa");
    let b: String = stringOf("bb");
    let v: String = stringOf("vv");
    unsafe {
        let p: raw String = &a;
        let q: raw String = &b;
        let r: String = overwrite<String>(p, q, v);
        return i32(len(r)) + calls * 10;
    }
}
"#;

/// The generic return program: the callee returns the pointee of an impure
/// pointer expression.
const DEREF_OWNER_RETURN_SOURCE: &str = r#"module spec.cprimpl.derefownerreturn;

import { String, stringOf } from std.text;

let calls: i32 = 0;

func alt<T>(s: raw T, t: raw T): raw T {
    let i: i32 = calls;
    calls = calls + 1;
    if (i == 0) { return s; }
    return t;
}

func read_it<T>(p: raw T, q: raw T): T {
    return unsafe *alt(p, q);
}

func main(): i32 {
    let a: String = stringOf("aa");
    let b: String = stringOf("bb");
    unsafe {
        let p: raw String = &a;
        let q: raw String = &b;
        let r: String = read_it<String>(p, q);
        return i32(len(r)) + calls * 10;
    }
}
"#;

/// The generic struct-literal field program: `Box.s` receives the pointee of
/// an impure pointer expression.
const DEREF_OWNER_FIELD_SOURCE: &str = r#"module spec.cprimpl.derefownerfield;

import { String, stringOf } from std.text;

let calls: i32 = 0;

struct Box { s: String; }

func alt<T>(s: raw T, t: raw T): raw T {
    let i: i32 = calls;
    calls = calls + 1;
    if (i == 0) { return s; }
    return t;
}

func make<T>(p: raw T, q: raw T): Box {
    return Box { s: unsafe *alt(p, q) };
}

func main(): i32 {
    let a: String = stringOf("aa");
    let b: String = stringOf("bb");
    unsafe {
        let p: raw String = &a;
        let q: raw String = &b;
        let bx: Box = make<String>(p, q);
        return i32(len(bx.s)) + calls * 10;
    }
}
"#;

/// The array-literal element program: the element receives the pointee of an
/// impure pointer expression.
const DEREF_OWNER_ELEMENT_SOURCE: &str = r#"module spec.cprimpl.derefownerelement;

import { String, stringOf } from std.text;

let calls: i32 = 0;

func alt<T>(s: raw T, t: raw T): raw T {
    let i: i32 = calls;
    calls = calls + 1;
    if (i == 0) { return s; }
    return t;
}

func main(): i32 {
    let a: String = stringOf("aa");
    let b: String = stringOf("bb");
    unsafe {
        let p: raw String = &a;
        let q: raw String = &b;
        let xs: String[1] = [*alt(p, q)];
        return i32(len(xs[0])) + calls * 10;
    }
}
"#;

/// The conditional-arm program: the selected arm reads the pointee of an
/// impure pointer expression.
const DEREF_OWNER_ARM_SOURCE: &str = r#"module spec.cprimpl.derefownerarm;

import { String, stringOf } from std.text;

let calls: i32 = 0;

func alt<T>(s: raw T, t: raw T): raw T {
    let i: i32 = calls;
    calls = calls + 1;
    if (i == 0) { return s; }
    return t;
}

func main(): i32 {
    let a: String = stringOf("aa");
    let b: String = stringOf("bb");
    let fallback: String = stringOf("cc");
    let flag: bool = true;
    unsafe {
        let p: raw String = &a;
        let q: raw String = &b;
        let r: String = flag ? *alt(p, q) : fallback;
        return i32(len(r)) + calls * 10;
    }
}
"#;

/// The guarded take of a place-pointer deref: the callee owns the pointee and
/// the caller's slot is emptied on the evaluated path.
const GUARDED_DEREF_TAKE_SOURCE: &str = r#"module spec.cprimpl.guardedtake;

import { String, stringOf } from std.text;

func eat(take x: String): i32 { return 1; }

func main(): i32 {
    let a: String = stringOf("aa");
    let g: bool = true;
    unsafe {
        let q: raw String = &a;
        let ok: bool = g && (eat(take (unsafe *q)) == 1);
        if (ok) { return 0; }
    }
    return 1;
}
"#;

/// The guarded take of an impure-pointer deref: the pointer fill and the
/// pointee emptying travel on the guard's evaluated path.
const GUARDED_DEREF_TAKE_IMPURE_SOURCE: &str = r#"module spec.cprimpl.guardedtakeimpure;

import { String, stringOf } from std.text;

let calls: i32 = 0;

func alt<T>(s: raw T, t: raw T): raw T {
    let i: i32 = calls;
    calls = calls + 1;
    if (i == 0) { return s; }
    return t;
}

func eat(take x: String): i32 { return 1; }

func main(): i32 {
    let a: String = stringOf("aa");
    let b: String = stringOf("bb");
    let g: bool = true;
    unsafe {
        let p: raw String = &a;
        let q: raw String = &b;
        let ok: bool = g && (eat(take (unsafe *alt(p, q))) == 1);
        if (ok) { return 0; }
    }
    return 1;
}
"#;

/// The skipped guarded take of an impure-pointer deref: the pointer must not
/// be called at all when the left operand decides the result.
const GUARDED_DEREF_TAKE_SKIP_SOURCE: &str = r#"module spec.cprimpl.guardedtakeskip;

import { String, stringOf } from std.text;

let calls: i32 = 0;

func alt<T>(s: raw T, t: raw T): raw T {
    let i: i32 = calls;
    calls = calls + 1;
    if (i == 0) { return s; }
    return t;
}

func eat(take x: String): i32 { return 1; }

func main(): i32 {
    let a: String = stringOf("aa");
    let b: String = stringOf("bb");
    let g: bool = false;
    unsafe {
        let p: raw String = &a;
        let q: raw String = &b;
        let ok: bool = g && (eat(take (unsafe *alt(p, q))) == 1);
        if (ok) { return 2; }
    }
    return calls * 10 + i32(len(a)) - 2;
}
"#;

/// Asserts the emitted line with `anchor` contains `first` before `second`.
///
/// A guarded operand is printed as one line, so this pins the fill, the call,
/// and the pointee emptying in their evaluation order without hoisting any of
/// them ahead of the guard.
fn assert_on_line_before(tag: &str, emitted: &str, anchor: &str, first: &str, second: &str) {
    let line = emitted
        .lines()
        .find(|line| line.contains(anchor))
        .unwrap_or_else(|| panic!("{tag}: no emitted line contains {anchor:?}"));
    let first_at = line.find(first).unwrap_or_else(|| {
        panic!("{tag}: the line with {anchor:?} lacks {first:?}; line:\n{line}")
    });
    let second_at = line.find(second).unwrap_or_else(|| {
        panic!("{tag}: the line with {anchor:?} lacks {second:?}; line:\n{line}")
    });
    assert!(
        first_at < second_at,
        "{tag}: expected {first:?} before {second:?} on the line with {anchor:?}; line:\n{line}"
    );
}

/// Compiles the emitted C with the host compiler and returns the run exit code.
///
/// The strict flag set is the repository's ordinary host build. The programs
/// are tiny, so a direct run is enough to observe a call counter folded into
/// the exit code.
fn run_host_c(tag: &str, scratch: &ScratchDir, emitted: &str) -> i32 {
    let c_path = scratch.path().join("run.c");
    std::fs::write(&c_path, emitted)
        .unwrap_or_else(|err| panic!("{tag}: cannot write the emitted C: {err}"));
    let program = scratch.path().join("run.bin");
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
    let status = Command::new(&program)
        .status()
        .unwrap_or_else(|err| panic!("{tag}: cannot run the program: {err}"));
    status.code().unwrap_or(-1)
}

/// The owning deref store evaluates its pointer once and reads and empties the
/// same temporary: one `cl_alt` call, then the store from the pinned pointer
/// and the pointee zero through the same temporary.
#[test]
fn should_pin_the_owning_deref_store_materialization() {
    let tag = "derefownerstore";
    let (_scratch, emitted) = emit_host(tag, DEREF_OWNER_STORE_SOURCE);
    let body = function_body(
        tag,
        &emitted,
        "cl_overwrite__String(ClString *p, ClString *q, ClString v)\n{",
    );
    assert_eq!(
        body.matches("cl_alt__String(").count(),
        1,
        "the impure pointer must evaluate once in the store; body:\n{body}"
    );
    assert_follows(tag, body, "v = *(__cl_tmp0);", "*(__cl_tmp0) = NULL;");
}

/// The owning deref return evaluates its pointer once and reads and empties
/// the same temporary.
#[test]
fn should_pin_the_owning_deref_return_materialization() {
    let tag = "derefownerreturn";
    let (_scratch, emitted) = emit_host(tag, DEREF_OWNER_RETURN_SOURCE);
    let body = function_body(
        tag,
        &emitted,
        "cl_read_it__String(ClString *p, ClString *q)\n{",
    );
    assert_eq!(
        body.matches("cl_alt__String(").count(),
        1,
        "the impure pointer must evaluate once in the return; body:\n{body}"
    );
    assert_follows(
        tag,
        body,
        "ClString __cl_return_1 = *(__cl_tmp0);",
        "*(__cl_tmp0) = NULL;",
    );
}

/// The owning deref struct-literal field pins the pointer once, stores the
/// field from it, and empties the pointee after the copy.
#[test]
fn should_pin_the_owning_deref_field_materialization() {
    let tag = "derefownerfield";
    let (_scratch, emitted) = emit_host(tag, DEREF_OWNER_FIELD_SOURCE);
    let body = function_body(
        tag,
        &emitted,
        "cl_make__String(ClString *p, ClString *q)\n{",
    );
    assert_contains(tag, body, "__cl_return_1.s = *(__cl_tmp0);");
    assert_follows(
        tag,
        body,
        "__cl_return_1.s = *(__cl_tmp0);",
        "*(__cl_tmp0) = NULL;",
    );
    assert_eq!(
        body.matches("cl_alt__String(").count(),
        1,
        "the impure pointer must evaluate once in the field store; body:\n{body}"
    );
}

/// The owning deref array-literal element evaluates its pointer once and
/// reads and empties the same temporary.
#[test]
fn should_pin_the_owning_deref_element_materialization() {
    let tag = "derefownerelement";
    let (_scratch, emitted) = emit_host(tag, DEREF_OWNER_ELEMENT_SOURCE);
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_derefownerelement_main(void)\n{",
    );
    assert_eq!(
        body.matches("cl_alt__String(").count(),
        1,
        "the impure pointer must evaluate once in the element store; body:\n{body}"
    );
    assert_follows(tag, body, "__cl_rhs_1[0] = *(__cl_tmp0);", "*(__cl_tmp0) = NULL;");
}

/// The owning deref conditional arm fills the pinned pointer inside the chosen
/// branch, then reads and empties the same temporary there.
#[test]
fn should_pin_the_owning_deref_arm_materialization() {
    let tag = "derefownerarm";
    let (_scratch, emitted) = emit_host(tag, DEREF_OWNER_ARM_SOURCE);
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_derefownerarm_main(void)\n{",
    );
    assert_eq!(
        body.matches("cl_alt__String(").count(),
        1,
        "the impure pointer must evaluate once in the chosen arm; body:\n{body}"
    );
    assert_follows(
        tag,
        body,
        "if (__cl_cond_1) {",
        "__cl_tmp0 = cl_alt__String(p, q);",
    );
    assert_store_then_zero(tag, body, "r = *(__cl_tmp0);", "*(__cl_tmp0) = NULL;");
}

/// The guarded take of a place-pointer deref empties the pointee on the
/// guard's evaluated path, after the call.
#[test]
fn should_pin_the_guarded_deref_take_pointer_emptying() {
    let tag = "guardedtake";
    let (_scratch, emitted) = emit_host(tag, GUARDED_DEREF_TAKE_SOURCE);
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_guardedtake_main(void)\n{",
    );
    assert_absent(tag, body, "*(q) = NULL;");
    assert_on_line_of(tag, body, "ok = (g &&", "_eat(*(q))");
    assert_on_line_before(tag, body, "ok = (g &&", "_eat(*(q))", "(*(q) = NULL)");
}

/// The guarded take of an impure-pointer deref defers the pointer fill to the
/// guard's right operand and empties the pointee there after the call.
#[test]
fn should_pin_the_guarded_deref_take_deferred_pointer_fill() {
    let tag = "guardedtakeimpure";
    let (_scratch, emitted) = emit_host(tag, GUARDED_DEREF_TAKE_IMPURE_SOURCE);
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_guardedtakeimpure_main(void)\n{",
    );
    assert_absent(tag, body, "= cl_alt__String(p, q);");
    assert_on_line_of(tag, body, "ok = (g &&", "__cl_tmp0 = cl_alt__String(p, q)");
    assert_on_line_before(
        tag,
        body,
        "ok = (g &&",
        "__cl_tmp0 = cl_alt__String(p, q)",
        "_eat(*(__cl_tmp0))",
    );
    assert_on_line_before(
        tag,
        body,
        "ok = (g &&",
        "_eat(*(__cl_tmp0))",
        "(*(__cl_tmp0) = NULL)",
    );
}

/// The skipped guarded take leaves the impure pointer only inside the guard's
/// right operand, and no eager statement runs it for the false selection.
#[test]
fn should_pin_the_guarded_deref_take_false_path_absence() {
    let tag = "guardedtakeskip";
    let (_scratch, emitted) = emit_host(tag, GUARDED_DEREF_TAKE_SKIP_SOURCE);
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_guardedtakeskip_main(void)\n{",
    );
    assert_absent(tag, body, "= cl_alt__String(p, q);");
    assert_on_line_of(tag, body, "ok = (g &&", "cl_alt__String(p, q)");
    assert_on_line_of(tag, body, "ok = (g &&", "(*(__cl_tmp0) = NULL)");
}

/// The skipped guarded take leaves the call counter at zero: the false
/// selection never evaluates the impure pointer.
#[test]
fn should_pin_the_guarded_deref_take_false_path_counter() {
    let tag = "guardedtakeskipcounter";
    let (scratch, emitted) = emit_host(tag, GUARDED_DEREF_TAKE_SKIP_SOURCE);
    let exit = run_host_c(tag, &scratch, &emitted);
    assert_eq!(
        exit, 0,
        "the skipped guarded take must not call the impure pointer; exit {exit}"
    );
}

/// The guarded deref take programs re-emit byte-identically, with the pointee
/// emptying carried on the guarded path.
#[test]
fn should_re_emit_the_guarded_deref_take_programs_byte_identically() {
    for (tag, source, shape) in [
        ("guardedtake", GUARDED_DEREF_TAKE_SOURCE, "(*(q) = NULL)"),
        (
            "guardedtakeimpure",
            GUARDED_DEREF_TAKE_IMPURE_SOURCE,
            "(*(__cl_tmp0) = NULL)",
        ),
        (
            "guardedtakeskip",
            GUARDED_DEREF_TAKE_SKIP_SOURCE,
            "(*(__cl_tmp0) = NULL)",
        ),
    ] {
        let (first, second) = emit_twice(tag, source);
        assert_contains(tag, &first, shape);
        assert!(
            first == second,
            "{tag}: the two emissions differ; first:\n{first}\nsecond:\n{second}"
        );
    }
}

/// A two-field literal whose later field moves the pointee of an impure
/// pointer expression.
const DEREF_ORDER_FIELD_SOURCE: &str = r#"module spec.cprimpl.dereforderfield;

import { String, stringOf } from std.text;

let counter: i32 = 2;

func bump<T>(s: raw T, t: raw T): raw T {
    counter = counter + 10;
    return s;
}

struct P { a: i32; b: String; }

func main(): i32 {
    let p: String = stringOf("pp");
    let q: String = stringOf("qq");
    unsafe {
        let rp: raw String = &p;
        let rq: raw String = &q;
        let v: P = P { a: counter, b: *bump(rp, rq) };
        return v.a;
    }
}
"#;

/// A two-element literal whose later element moves the pointee of an impure
/// pointer expression.
const DEREF_ORDER_ELEMENT_SOURCE: &str = r#"module spec.cprimpl.dereforderarray;

import { String, stringOf } from std.text;

let counter: i32 = 2;

func bump<T>(s: raw T, t: raw T): raw T {
    counter = counter + 10;
    return s;
}

struct P { a: i32; b: String; }

func main(): i32 {
    let p: String = stringOf("pp");
    let q: String = stringOf("qq");
    let fill: String = stringOf("ff");
    unsafe {
        let rp: raw String = &p;
        let rq: raw String = &q;
        let xs: P[2] = [P { a: counter, b: fill }, P { a: 0, b: *bump(rp, rq) }];
        return xs[0].a;
    }
}
"#;

/// Asserts `first` occurs before `second` in the emitted text.
fn assert_occurs_before(tag: &str, emitted: &str, first: &str, second: &str) {
    let first_at = emitted
        .find(first)
        .unwrap_or_else(|| panic!("{tag}: the emitted C lacks {first:?}"));
    let second_at = emitted
        .find(second)
        .unwrap_or_else(|| panic!("{tag}: the emitted C lacks {second:?}"));
    assert!(
        first_at < second_at,
        "{tag}: expected {first:?} before {second:?}; emitted C:\n{emitted}"
    );
}

/// A literal stores its earlier sibling before a later sibling's owning-deref
/// pointer fill: the field or element store emits the fill at its own turn.
#[test]
fn should_pin_the_literal_sibling_order_before_a_deref_fill() {
    let field = "dereforderfield";
    let (_scratch, emitted) = emit_host(field, DEREF_ORDER_FIELD_SOURCE);
    let body = function_body(
        field,
        &emitted,
        "cl_spec_cprimpl_dereforderfield_main(void)\n{",
    );
    assert_occurs_before(
        field,
        body,
        "v.a = cl_spec_cprimpl_dereforderfield_counter;",
        "__cl_tmp0 = cl_bump__String(rp, rq);",
    );
    assert_occurs_before(
        field,
        body,
        "__cl_tmp0 = cl_bump__String(rp, rq);",
        "v.b = *(__cl_tmp0);",
    );

    let element = "dereforderarray";
    let (_scratch, emitted) = emit_host(element, DEREF_ORDER_ELEMENT_SOURCE);
    let body = function_body(
        element,
        &emitted,
        "cl_spec_cprimpl_dereforderarray_main(void)\n{",
    );
    assert_occurs_before(
        element,
        body,
        "__cl_rhs_1[0].a = cl_spec_cprimpl_dereforderarray_counter;",
        "__cl_tmp1 = cl_bump__String(rp, rq);",
    );
    assert_occurs_before(
        element,
        body,
        "__cl_tmp1 = cl_bump__String(rp, rq);",
        "__cl_rhs_1[1].b = *(__cl_tmp1);",
    );
}

// ---------------------------------------------------------------------------
// hidden view destinations and guarded owning carriers: a call-filled
// view-array field binds its hidden destination at every read position, and an
// owning conditional argument fills its carrier only on the guard's evaluated
// path.
// ---------------------------------------------------------------------------

/// A call-valued view-array field read through a literal projection.
const HIDDEN_READ_SOURCE: &str = r#"module spec.cprimpl.hiddenread;

struct Inner { row: str[2]; }

let calls: i32 = 0;

func n1(): str[2] { calls = calls + 1; return ["AA", "BB"]; }

func use(): i32 {
    return i32(Inner { row: n1() }.row[0][0]) - 65;
}

func main(): i32 {
    use();
    return calls;
}
"#;

/// The same field selected between two calls without a guard.
const HIDDEN_ROW_COND_SOURCE: &str = r#"module spec.cprimpl.hiddenrowcond;

struct Inner { row: str[2]; }

let calls: i32 = 0;

func n1(): str[2] { calls = calls + 1; return ["AA", "BB"]; }
func n2(): str[2] { calls = calls + 1; return ["CC", "DD"]; }

func use(ok: bool): i32 {
    return i32(Inner { row: ok ? n1() : n2() }.row[0][0]) - 65;
}

func main(): i32 {
    use(true);
    use(false);
    return calls;
}
"#;

/// A multi-extent grid whose rows are selected between calls under a guard.
const HIDDEN_GRID_SOURCE: &str = r#"module spec.cprimpl.hiddengrid;

struct Grid { rows: str[2][2]; }

let calls: i32 = 0;

func r1(): str[2] { calls = calls + 1; return ["AA", "BB"]; }
func r2(): str[2] { calls = calls + 1; return ["CC", "DD"]; }

func use(g: bool, o: bool): i32 {
    if (g && i32(Grid { rows: [o ? r1() : r2(), r1()] }.rows[0][0][0]) > 0) {
        return 1;
    }
    return 0;
}

func main(): i32 {
    use(true, true);
    use(false, true);
    return calls;
}
"#;

/// A guarded row conditional and the same literal read after the guard.
const HIDDEN_GUARD_ROW_SOURCE: &str = r#"module spec.cprimpl.hiddenguardrow;

struct Inner { row: str[2]; }

let calls: i32 = 0;

func n1(): str[2] { calls = calls + 1; return ["AA", "BB"]; }
func n2(): str[2] { calls = calls + 1; return ["CC", "DD"]; }

func use(g: bool, ok: bool): i32 {
    if (g && i32(Inner { row: ok ? n1() : n2() }.row[0][0]) > 0) {
        return i32(Inner { row: ok ? n1() : n2() }.row[0][0]);
    }
    return -1;
}

func main(): i32 {
    use(true, true);
    use(true, false);
    use(false, true);
    return calls;
}
"#;

/// The call-filled view field binds a destination array and copies the field at
/// every read shape, and no shape keeps the brace-list call spelling.
#[test]
fn should_pin_the_hidden_view_destination_at_every_read_shape() {
    let read = "hiddenread";
    let (_scratch, emitted) = emit_host(read, HIDDEN_READ_SOURCE);
    let body = function_body(
        read,
        &emitted,
        "cl_spec_cprimpl_hiddenread_use(void)\n{",
    );
    assert_contains(read, body, "ClInner __cl_tmp0 = {0};");
    assert_contains(read, body, "ClStr __cl_call_1[2] = {0};");
    assert_contains(
        read,
        body,
        "cl_spec_cprimpl_hiddenread_n1(__cl_call_1);",
    );
    assert_contains(
        read,
        body,
        "memcpy(&(__cl_tmp0.row), &(__cl_call_1), sizeof(__cl_tmp0.row));",
    );
    assert_absent(read, body, "(ClInner){ cl_spec_cprimpl_hiddenread_n1()");

    let row = "hiddenrowcond";
    let (_scratch, emitted) = emit_host(row, HIDDEN_ROW_COND_SOURCE);
    let body = function_body(
        row,
        &emitted,
        "cl_spec_cprimpl_hiddenrowcond_use(bool ok)\n{",
    );
    assert_contains(row, body, "ClInner __cl_tmp0 = {0};");
    assert_contains(row, body, "ClStr __cl_arm_2[2] = {0};");
    assert_contains(
        row,
        body,
        "cl_spec_cprimpl_hiddenrowcond_n1(__cl_arm_2);",
    );
    assert_contains(
        row,
        body,
        "memcpy(&(__cl_tmp0.row), &(__cl_arm_2), sizeof(__cl_tmp0.row));",
    );
    assert_contains(
        row,
        body,
        "cl_spec_cprimpl_hiddenrowcond_n2(__cl_arm_3);",
    );
    assert_contains(
        row,
        body,
        "memcpy(&(__cl_tmp0.row), &(__cl_arm_3), sizeof(__cl_tmp0.row));",
    );
    assert_absent(row, body, "(ClInner){ (ok ? cl_spec_cprimpl_hiddenrowcond_n1()");

    let grid = "hiddengrid";
    let (_scratch, emitted) = emit_host(grid, HIDDEN_GRID_SOURCE);
    let body = function_body(
        grid,
        &emitted,
        "cl_spec_cprimpl_hiddengrid_use(bool g, bool o)\n{",
    );
    assert_on_line_of(
        grid,
        body,
        "if ((g &&",
        "cl_spec_cprimpl_hiddengrid_r1(__cl_tmp0.rows[0])",
    );
    assert_on_line_of(
        grid,
        body,
        "if ((g &&",
        "cl_spec_cprimpl_hiddengrid_r2(__cl_tmp0.rows[0])",
    );
    assert_on_line_of(
        grid,
        body,
        "if ((g &&",
        "cl_spec_cprimpl_hiddengrid_r1(__cl_tmp0.rows[1])",
    );
    assert_absent(grid, body, "cl_spec_cprimpl_hiddengrid_r1()");

    let guard = "hiddenguardrow";
    let (_scratch, emitted) = emit_host(guard, HIDDEN_GUARD_ROW_SOURCE);
    let body = function_body(
        guard,
        &emitted,
        "cl_spec_cprimpl_hiddenguardrow_use(bool g, bool ok)\n{",
    );
    assert_on_line_of(
        guard,
        body,
        "if ((g &&",
        "cl_spec_cprimpl_hiddenguardrow_n1(__cl_tmp0.row)",
    );
    assert_on_line_of(
        guard,
        body,
        "if ((g &&",
        "cl_spec_cprimpl_hiddenguardrow_n2(__cl_tmp0.row)",
    );
    assert_contains(
        guard,
        body,
        "cl_spec_cprimpl_hiddenguardrow_n1(__cl_arm_2);",
    );
    assert_contains(
        guard,
        body,
        "memcpy(&(__cl_tmp2.row), &(__cl_arm_2), sizeof(__cl_tmp2.row));",
    );
    assert_contains(
        guard,
        body,
        "cl_spec_cprimpl_hiddenguardrow_n2(__cl_arm_3);",
    );
    assert_contains(
        guard,
        body,
        "memcpy(&(__cl_tmp2.row), &(__cl_arm_3), sizeof(__cl_tmp2.row));",
    );
    assert_absent(guard, body, "cl_spec_cprimpl_hiddenguardrow_n1()");
}

/// The four call-filled destinations evaluate each call once.
#[test]
fn should_count_the_call_filled_view_destinations_once() {
    for (tag, source, calls) in [
        ("hiddenread", HIDDEN_READ_SOURCE, 1),
        ("hiddenrowcond", HIDDEN_ROW_COND_SOURCE, 2),
        ("hiddengrid", HIDDEN_GRID_SOURCE, 2),
        ("hiddenguardrow", HIDDEN_GUARD_ROW_SOURCE, 4),
    ] {
        let (scratch, emitted) = emit_host(tag, source);
        let exit = run_host_c(tag, &scratch, &emitted);
        assert_eq!(
            exit, calls,
            "{tag}: expected the fixture to report {calls} calls; exit {exit}"
        );
    }
}

/// The hidden-destination programs re-emit byte-identically with the bound
/// destination shape.
#[test]
fn should_re_emit_the_hidden_view_destination_programs_byte_identically() {
    for (tag, source, shape) in [
        (
            "hiddenread",
            HIDDEN_READ_SOURCE,
            "memcpy(&(__cl_tmp0.row), &(__cl_call_1), sizeof(__cl_tmp0.row));",
        ),
        (
            "hiddenrowcond",
            HIDDEN_ROW_COND_SOURCE,
            "cl_spec_cprimpl_hiddenrowcond_n1(__cl_arm_2);",
        ),
        (
            "hiddengrid",
            HIDDEN_GRID_SOURCE,
            "cl_spec_cprimpl_hiddengrid_r1(__cl_tmp0.rows[1])",
        ),
        (
            "hiddenguardrow",
            HIDDEN_GUARD_ROW_SOURCE,
            "cl_spec_cprimpl_hiddenguardrow_n1(__cl_tmp0.row)",
        ),
    ] {
        let (first, second) = emit_twice(tag, source);
        assert_contains(tag, &first, shape);
        assert!(
            first == second,
            "{tag}: the two emissions differ; first:\n{first}\nsecond:\n{second}"
        );
    }
}

/// An owning conditional argument under a short-circuit guard.
const GUARD_CARRIER_SOURCE: &str = r#"module spec.cprimpl.guardcarrier;

import { String, stringOf } from std.text;

struct Inner { names: str[2]; }
struct Outer { inner: Inner; tag: String; }

let calls: i32 = 0;
let seen: i32 = 0;

func n1(): str[2] { calls = calls + 1; return ["AA", "BB"]; }
func n2(): str[2] { calls = calls + 1; return ["CC", "DD"]; }

func consume(o: Outer): i32 {
    seen = i32(o.inner.names[0][0]);
    return 1;
}

func use(g: bool, ok: bool): i32 {
    if (g && consume(ok ? Outer { inner: Inner { names: n1() }, tag: stringOf("x") }
                        : Outer { inner: Inner { names: n2() }, tag: stringOf("y") }) == 1) {
        return seen;
    }
    return -1;
}

func main(): i32 {
    let a: i32 = use(true, true);
    let b: i32 = use(true, false);
    let c: i32 = use(false, true);
    return (a - 65) + (b - 67) + (c + 1) + calls - 2;
}
"#;

/// The same carrier re-filled by a loop.
const GUARD_CARRIER_LOOP_SOURCE: &str = r#"module spec.cprimpl.guardcarrierloop;

import { String, stringOf } from std.text;

struct Inner { names: str[2]; }
struct Outer { inner: Inner; tag: String; }

let calls: i32 = 0;
let seen: i32 = 0;

func n1(): str[2] { calls = calls + 1; return ["AA", "BB"]; }
func n2(): str[2] { calls = calls + 1; return ["CC", "DD"]; }

func consume(o: Outer): i32 {
    seen = i32(o.inner.names[0][0]);
    return 1;
}

func use(g: bool, ok: bool): i32 {
    let i: i32 = 0;
    let total: i32 = 0;
    while (i < 2) {
        if (g && consume(ok ? Outer { inner: Inner { names: n1() }, tag: stringOf("x") }
                            : Outer { inner: Inner { names: n2() }, tag: stringOf("y") }) == 1) {
            total = total + seen;
        }
        i = i + 1;
    }
    return total;
}

func main(): i32 {
    use(true, true);
    use(true, false);
    use(false, true);
    return calls;
}
"#;

/// The same carrier under a guard that is always false.
const GUARD_CARRIER_FALSE_SOURCE: &str = r#"module spec.cprimpl.guardcarrierfalse;

import { String, stringOf } from std.text;

struct Inner { names: str[2]; }
struct Outer { inner: Inner; tag: String; }

let calls: i32 = 0;
let seen: i32 = 0;

func n1(): str[2] { calls = calls + 1; return ["AA", "BB"]; }
func n2(): str[2] { calls = calls + 1; return ["CC", "DD"]; }

func consume(o: Outer): i32 {
    seen = i32(o.inner.names[0][0]);
    return 1;
}

func use(g: bool, ok: bool): i32 {
    if (g && consume(ok ? Outer { inner: Inner { names: n1() }, tag: stringOf("x") }
                        : Outer { inner: Inner { names: n2() }, tag: stringOf("y") }) == 1) {
        return seen;
    }
    return -1;
}

func main(): i32 {
    let a: i32 = use(false, true);
    return a + 1 + calls;
}
"#;

/// A guarded struct-literal field whose source is the caller's pointee.
const GUARD_SKIP_FIELD_SOURCE: &str = r#"module spec.cprimpl.guardskipfield;

import { String, stringOf } from std.text;

struct Box { s: String; }

func consume(b: Box): i32 { return i32(len(b.s)) - 2; }

func use(take s: String, g: bool): i32 {
    unsafe {
        let q: raw String = &s;
        if (g && consume(Box { s: *q }) == 0) {
            return 1;
        }
    }
    return i32(len(s)) - 2;
}

func main(): i32 {
    let a: i32 = use(stringOf("xx"), false);
    let b: i32 = use(stringOf("yy"), true);
    return a + b - 1;
}
"#;

/// A guarded array-literal element whose source is the caller's pointee.
const GUARD_SKIP_ELEMENT_SOURCE: &str = r#"module spec.cprimpl.guardskipelement;

import { String, stringOf } from std.text;

func consume(xs: String[1]): i32 { return i32(len(xs[0])) - 2; }

func use(take s: String, g: bool): i32 {
    unsafe {
        let q: raw String = &s;
        if (g && consume([*q]) == 0) {
            return 1;
        }
    }
    return i32(len(s)) - 2;
}

func main(): i32 {
    let a: i32 = use(stringOf("xx"), false);
    let b: i32 = use(stringOf("yy"), true);
    return a + b - 1;
}
"#;

/// The owning conditional carrier fills inside the guard, after the drop of the
/// previous value, and the call reads that same carrier.
#[test]
fn should_fill_the_owning_conditional_carrier_inside_its_guard() {
    let tag = "guardcarrier";
    let (_scratch, emitted) = emit_host(tag, GUARD_CARRIER_SOURCE);
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_guardcarrier_use(bool g, bool ok)\n{",
    );
    assert_contains(tag, body, "ClOuter __cl_owned0 = {0};");
    assert_on_line_of(tag, body, "if ((g &&", "(__cl_owned0 =");
    assert_on_line_of(
        tag,
        body,
        "if ((g &&",
        "cl_spec_cprimpl_guardcarrier_consume(__cl_owned0)",
    );
    assert_on_line_before(
        tag,
        body,
        "if ((g &&",
        "__cl_owned0.tag ? (FreeString(__cl_owned0.tag)",
        "(__cl_owned0 =",
    );
    assert_on_line_before(
        tag,
        body,
        "if ((g &&",
        "(__cl_owned0 =",
        "cl_spec_cprimpl_guardcarrier_consume(__cl_owned0)",
    );
    assert_absent(tag, body, "ClOuter __cl_save_");
    assert_absent(tag, body, "if (__cl_save_");
    assert_contains(
        tag,
        body,
        "if (__cl_owned0.tag) { FreeString(__cl_owned0.tag); }",
    );
}

/// A loop re-fills the carrier: each evaluated fill drops the previous value
/// once, and the scope releases the last value after the loop.
#[test]
fn should_re_drop_and_re_release_the_loop_carrier() {
    let tag = "guardcarrierloop";
    let (scratch, emitted) = emit_host(tag, GUARD_CARRIER_LOOP_SOURCE);
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_guardcarrierloop_use(bool g, bool ok)\n{",
    );
    assert_on_line_before(
        tag,
        body,
        "if ((g &&",
        "__cl_owned0.tag ? (FreeString(__cl_owned0.tag)",
        "(__cl_owned0 =",
    );
    assert_on_line_before(
        tag,
        body,
        "if ((g &&",
        "(__cl_owned0 =",
        "cl_spec_cprimpl_guardcarrierloop_consume(__cl_owned0)",
    );
    assert_eq!(
        body
            .matches("__cl_owned0.tag ? (FreeString(__cl_owned0.tag)")
            .count(),
        1,
        "the guarded re-fill must drop the previous carrier exactly once; body:\n{body}"
    );
    assert_eq!(
        body
            .matches("if (__cl_owned0.tag) { FreeString(__cl_owned0.tag); }")
            .count(),
        1,
        "the scope must release the last carrier exactly once; body:\n{body}"
    );
    assert_occurs_before(
        tag,
        body,
        "while ((i < 2))",
        "if (__cl_owned0.tag) { FreeString(__cl_owned0.tag); }",
    );
    let exit = run_host_c(tag, &scratch, &emitted);
    assert_eq!(
        exit, 4,
        "the loop must evaluate the conditional arms four times; exit {exit}"
    );
}

/// A skipped guard runs no carrier fill and no arm call.
#[test]
fn should_skip_the_guarded_carrier_calls_on_the_false_path() {
    let tag = "guardcarrierfalse";
    let (scratch, emitted) = emit_host(tag, GUARD_CARRIER_FALSE_SOURCE);
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_guardcarrierfalse_use(bool g, bool ok)\n{",
    );
    let exit = run_host_c(tag, &scratch, &emitted);
    assert_eq!(
        exit, 0,
        "the skipped guard must not call an arm; exit {exit}"
    );
    assert_on_line_of(tag, body, "if ((g &&", "(__cl_owned0 =");
    assert_absent(tag, body, "ClOuter __cl_save_");
}

/// A guarded struct-literal field moves the caller's pointee only on the
/// guard's evaluated path; the skipped path keeps it.
#[test]
fn should_keep_a_guarded_field_source_on_the_evaluated_path() {
    let tag = "guardskipfield";
    let (scratch, emitted) = emit_host(tag, GUARD_SKIP_FIELD_SOURCE);
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_guardskipfield_use(ClString s, bool g)\n{",
    );
    assert_on_line_of(
        tag,
        body,
        "if ((g &&",
        ".s = *(q), (*(q) = NULL)",
    );
    assert_absent(tag, body, ".s = *(q);");
    let exit = run_host_c(tag, &scratch, &emitted);
    assert_eq!(
        exit, 0,
        "the skipped field guard must leave the caller's value intact; exit {exit}"
    );
}

/// A guarded array-literal element moves the caller's pointee only on the
/// guard's evaluated path; the skipped path keeps it.
#[test]
fn should_keep_a_guarded_element_source_on_the_evaluated_path() {
    let tag = "guardskipelement";
    let (scratch, emitted) = emit_host(tag, GUARD_SKIP_ELEMENT_SOURCE);
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_guardskipelement_use(ClString s, bool g)\n{",
    );
    assert_on_line_of(
        tag,
        body,
        "if ((g &&",
        "(__cl_owned0[0] = *(q), (*(q) = NULL))",
    );
    assert_absent(tag, body, "ClString __cl_save_");
    let exit = run_host_c(tag, &scratch, &emitted);
    assert_eq!(
        exit, 0,
        "the skipped element guard must leave the caller's value intact; exit {exit}"
    );
}

// ---------------------------------------------------------------------------
// owned literal field sources: a guarded literal field that names caller
// storage moves the value and empties the source on the evaluated path, at
// every storage position.
// ---------------------------------------------------------------------------

/// The place, raw-deref, member, nested, fixed-array, and direct field sources.
const OWNED_FIELD_POSITIONS_SOURCE: &str = r#"module spec.cprimpl.ownedfields;

import { String, stringOf } from std.text;

struct Box { s: String; }
struct H { s: String; }
struct A { ss: String[2]; }
struct Inner { s: String; }
struct Outer { inner: Inner; }

func consumeBox(b: Box): i32 { return i32(len(b.s)) - 2; }
func consumeA(a: A): i32 { return i32(len(a.ss[0])) - 2; }
func consumeOuter(o: Outer): i32 { return i32(len(o.inner.s)) - 2; }

func placeUse(g: bool, ok: bool, take s: String, take t: String): i32 {
    if (g && consumeBox(ok ? Box { s: s } : Box { s: t }) == 0) {
        return 1;
    }
    return 0;
}

func derefUse(g: bool, ok: bool, take s: String, take t: String): i32 {
    unsafe {
        let q: raw String = &s;
        let r: raw String = &t;
        if (g && consumeBox(ok ? Box { s: *q } : Box { s: *r }) == 0) {
            return 1;
        }
    }
    return 0;
}

func memberUse(g: bool, ok: bool, take h: H, take k: H): i32 {
    if (g && consumeBox(ok ? Box { s: h.s } : Box { s: k.s }) == 0) {
        return 1;
    }
    return 0;
}

func nestedUse(g: bool, ok: bool, take s: String, take t: String): i32 {
    if (g && consumeOuter(ok ? Outer { inner: Inner { s: s } } : Outer { inner: Inner { s: t } }) == 0) {
        return 1;
    }
    return 0;
}

func fixedUse(g: bool, ok: bool, take a: String, take b: String, take c: String, take d: String): i32 {
    if (g && consumeA(ok ? A { ss: [a, b] } : A { ss: [c, d] }) == 0) {
        return 1;
    }
    return 0;
}

func directUse(g: bool, take s: String): i32 {
    if (g && consumeBox(Box { s: s }) == 0) {
        return 1;
    }
    return 0;
}

func main(): i32 { return 0; }
"#;

/// Each owned field source is stored through a move that empties the source;
/// the compound literal is not used for it.
#[test]
fn should_pin_the_owned_literal_field_move_at_every_storage_position() {
    let tag = "ownedfields";
    let (_scratch, emitted) = emit_host(tag, OWNED_FIELD_POSITIONS_SOURCE);

    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_ownedfields_placeUse(bool g, bool ok, ClString s, ClString t)\n{",
    );
    assert_on_line_of(
        tag,
        body,
        "if ((g &&",
        ".s = s, (s = NULL)",
    );
    assert_on_line_of(
        tag,
        body,
        "if ((g &&",
        ".s = t, (t = NULL)",
    );
    assert_absent(tag, body, "(ClBox){ s }");

    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_ownedfields_derefUse(bool g, bool ok, ClString s, ClString t)\n{",
    );
    assert_on_line_of(
        tag,
        body,
        "if ((g &&",
        ".s = *(q), (*(q) = NULL)",
    );
    assert_on_line_of(
        tag,
        body,
        "if ((g &&",
        ".s = *(r), (*(r) = NULL)",
    );

    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_ownedfields_memberUse(bool g, bool ok, ClH h, ClH k)\n{",
    );
    assert_on_line_of(
        tag,
        body,
        "if ((g &&",
        ".s = h.s, (h.s = NULL)",
    );
    assert_on_line_of(
        tag,
        body,
        "if ((g &&",
        ".s = k.s, (k.s = NULL)",
    );

    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_ownedfields_nestedUse(bool g, bool ok, ClString s, ClString t)\n{",
    );
    assert_on_line_of(
        tag,
        body,
        "if ((g &&",
        ".inner.s = s, (s = NULL)",
    );
    assert_on_line_of(
        tag,
        body,
        "if ((g &&",
        ".inner.s = t, (t = NULL)",
    );

    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_ownedfields_fixedUse(bool g, bool ok, ClString a, ClString b, ClString c, ClString d)\n{",
    );
    assert_on_line_of(
        tag,
        body,
        "if ((g &&",
        ".ss[0] = a, (a = NULL)",
    );
    assert_on_line_of(
        tag,
        body,
        "if ((g &&",
        ".ss[1] = b, (b = NULL)",
    );
    assert_on_line_of(
        tag,
        body,
        "if ((g &&",
        ".ss[0] = c, (c = NULL)",
    );
    assert_on_line_of(
        tag,
        body,
        "if ((g &&",
        ".ss[1] = d, (d = NULL)",
    );

    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_ownedfields_directUse(bool g, ClString s)\n{",
    );
    assert_on_line_of(
        tag,
        body,
        "if ((g &&",
        ".s = s, (s = NULL)",
    );
    assert_absent(tag, body, "(ClBox){ s }");
}

/// The same owned place field under `||`, a loop, and a ternary arm.
const OWNED_FIELD_POSITIONS_OTHER_SOURCE: &str = r#"module spec.cprimpl.ownedfieldpos;

import { String, stringOf } from std.text;

struct Box { s: String; }
func consumeBox(b: Box): i32 { return i32(len(b.s)) - 2; }

func orUse(g: bool, ok: bool, take s: String, take t: String): i32 {
    if (g || consumeBox(ok ? Box { s: s } : Box { s: t }) == 0) {
        return 1;
    }
    return 0;
}

func loopUse(g: bool): i32 {
    let i: i32 = 0;
    let total: i32 = 0;
    while (i < 2) {
        let s: String = stringOf("xx");
        if (g && consumeBox(Box { s: s }) == 0) {
            total = total + 1;
        }
        i = i + 1;
    }
    return total;
}

func ternaryUse(g: bool, ok: bool, take s: String, take t: String): i32 {
    return g ? consumeBox(ok ? Box { s: s } : Box { s: t }) : 0;
}

func main(): i32 { return 0; }
"#;

/// The owned field move sits on the branch that evaluates it under each
/// position, including the loop's re-created local and the ternary arm.
#[test]
fn should_pin_the_owned_literal_field_move_positions() {
    let tag = "ownedfieldpos";
    let (_scratch, emitted) = emit_host(tag, OWNED_FIELD_POSITIONS_OTHER_SOURCE);

    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_ownedfieldpos_orUse(bool g, bool ok, ClString s, ClString t)\n{",
    );
    assert_on_line_of(
        tag,
        body,
        "if ((g ||",
        ".s = s, (s = NULL)",
    );

    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_ownedfieldpos_loopUse(bool g)\n{",
    );
    assert_on_line_of(
        tag,
        body,
        "if ((g &&",
        ".s = s, (s = NULL)",
    );

    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_ownedfieldpos_ternaryUse(bool g, bool ok, ClString s, ClString t)\n{",
    );
    assert_on_line_of(
        tag,
        body,
        "__cl_ret = (g ?",
        ".s = s, (s = NULL)",
    );
}

/// A conditional argument carried by a ternary arm with call-filled fields.
const TERNARY_ARM_CARRIER_SOURCE: &str = r#"module spec.cprimpl.m7ternarycarrierimpl;

import { String, stringOf } from std.text;

struct Inner { names: str[2]; }
struct Outer { inner: Inner; tag: String; }

let calls: i32 = 0;

func n1(): str[2] { calls = calls + 1; return ["AA", "BB"]; }
func n2(): str[2] { calls = calls + 1; return ["CC", "DD"]; }

func consume(o: Outer): i32 { return i32(o.inner.names[0][0]); }

func use(g: bool, ok: bool): i32 {
    return g ? consume(ok ? Outer { inner: Inner { names: n1() }, tag: stringOf("x") }
                          : Outer { inner: Inner { names: n2() }, tag: stringOf("y") }) : 0;
}

func main(): i32 {
    use(true, true);
    use(true, false);
    use(false, true);
    return calls;
}
"#;

/// A ternary-arm carrier passes each arm's hidden destination and runs the
/// chosen arm exactly once.
#[test]
fn should_pin_the_ternary_arm_carrier_destination() {
    let tag = "m7ternarycarrierimpl";
    let (scratch, emitted) = emit_host(tag, TERNARY_ARM_CARRIER_SOURCE);
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_m7ternarycarrierimpl_use(bool g, bool ok)\n{",
    );
    assert_on_line_of(
        tag,
        body,
        "__cl_ret = (g ?",
        "cl_spec_cprimpl_m7ternarycarrierimpl_n1(__cl_tmp0.inner.names)",
    );
    assert_on_line_of(
        tag,
        body,
        "__cl_ret = (g ?",
        "cl_spec_cprimpl_m7ternarycarrierimpl_n2(__cl_tmp1.inner.names)",
    );
    assert_on_line_of(tag, body, "__cl_ret = (g ?", "(__cl_owned0 =");
    assert_absent(tag, body, "cl_spec_cprimpl_m7ternarycarrierimpl_n1()");
    assert_absent(tag, body, "cl_spec_cprimpl_m7ternarycarrierimpl_n2()");
    let exit = run_host_c(tag, &scratch, &emitted);
    assert_eq!(
        exit, 2,
        "the two selected arms must call once each; exit {exit}"
    );
}

/// An owning literal projected through a whole row and a single element.
const OWNING_LITERAL_READ_SOURCE: &str = r#"module spec.cprimpl.m7owningread;

import { String, stringOf } from std.text;

struct Mixed { row: str[2]; tag: String; }

let calls: i32 = 0;
func n1(): str[2] { calls = calls + 1; return ["AA", "BB"]; }

func rowUse(): i32 {
    let r: str[2] = Mixed { row: n1(), tag: stringOf("xy") }.row;
    return i32(r[0][0]) - 65;
}

func elemUse(): i32 {
    let e: str = Mixed { row: n1(), tag: stringOf("xy") }.row[0];
    return i32(e[0]) - 65;
}

func main(): i32 { return rowUse() + elemUse(); }
"#;

/// An owning literal read through a projection binds to a body-scope local,
/// so the derived view outlives the statement that built the literal.
#[test]
fn should_pin_the_owning_literal_read_binding_scope() {
    let tag = "m7owningread";
    let (scratch, emitted) = emit_host(tag, OWNING_LITERAL_READ_SOURCE);

    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_m7owningread_rowUse(void)\n{",
    );
    assert_contains(tag, body, "ClMixed __cl_owned0 = {0};");
    assert_contains(
        tag,
        body,
        "memcpy(&(r), &(__cl_owned0.row), sizeof(r));",
    );
    assert_contains(
        tag,
        body,
        "if (__cl_owned0.tag) { FreeString(__cl_owned0.tag); }",
    );
    assert_absent(tag, body, "ClMixed __cl_tmp");

    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_m7owningread_elemUse(void)\n{",
    );
    assert_contains(tag, body, "ClMixed __cl_owned0 = {0};");
    assert_contains(tag, body, "e = __cl_owned0.row[0];");
    assert_contains(
        tag,
        body,
        "if (__cl_owned0.tag) { FreeString(__cl_owned0.tag); }",
    );
    assert_absent(tag, body, "ClMixed __cl_tmp");

    let exit = run_host_c(tag, &scratch, &emitted);
    assert_eq!(
        exit, 0,
        "the whole-row and element projections must run; exit {exit}"
    );
}

/// A pure multi-extent grid passed by value under a guard.
const LITERAL_GRID_SOURCE: &str = r#"module spec.cprimpl.m7literalgrid;

struct Grid { rows: str[2][2]; }
func consume(g: Grid): i32 { return i32(g.rows[0][0][0]) - 65; }

func use(g: bool): i32 {
    if (g && consume(Grid { rows: [["AA", "BB"], ["CC", "DD"]] }) == 0) {
        return 1;
    }
    return 0;
}

func main(): i32 {
    let a: i32 = use(true);
    let b: i32 = use(false);
    return a + b - 1;
}
"#;

/// An all-literal grid keeps its brace list; only a non-literal row takes the
/// per-element store form.
#[test]
fn should_keep_the_all_literal_grid_brace_list() {
    let tag = "m7literalgrid";
    let (scratch, emitted) = emit_host(tag, LITERAL_GRID_SOURCE);
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_m7literalgrid_use(bool g)\n{",
    );
    assert_on_line_of(
        tag,
        body,
        "if ((g &&",
        "(ClGrid){ { { cl_str(\"AA\"), cl_str(\"BB\") }, { cl_str(\"CC\"), cl_str(\"DD\") } } }",
    );
    assert_absent(tag, body, ".rows[0][0] =");
    assert_absent(tag, body, ".rows[1][0] =");
    let exit = run_host_c(tag, &scratch, &emitted);
    assert_eq!(exit, 0, "the literal grid must run; exit {exit}");
}

/// A guarded literal whose field is a conditional of two places.
const GUARDED_CONDITIONAL_FIELD_SOURCE: &str = r#"module spec.cprimpl.m7l4place;

import { String, stringOf } from std.text;

struct Holder { s: String; tag: String; }
func consume(h: Holder): i32 { return i32(len(h.s)) - 2; }

func use(g: bool, flag: bool, take x: String, take y: String): i32 {
    if (g && consume(Holder { s: flag ? x : y, tag: stringOf("x") }) == 0) {
        return 1;
    }
    return 0;
}

func main(): i32 { return 0; }
"#;

/// The same field with a raw-deref arm through an impure pointer.
const GUARDED_CONDITIONAL_DEREF_SOURCE: &str = r#"module spec.cprimpl.m7l4deref;

import { String, stringOf } from std.text;

struct Holder { s: String; tag: String; }
func consume(h: Holder): i32 { return i32(len(h.s)) - 2; }
func pickPtr(p: raw String, q: raw String, flag: bool): raw String {
    if (flag) { return p; }
    return q;
}

func use(g: bool, flag: bool, take x: String, take y: String, take f: String): i32 {
    unsafe {
        let p: raw String = &x;
        if (g && consume(Holder { s: flag ? *pickPtr(p, p, flag) : f, tag: stringOf("x") }) == 0) {
            return 1;
        }
    }
    return 0;
}

func main(): i32 { return 0; }
"#;

/// The same conditional field under `||` and in a ternary arm.
const GUARDED_CONDITIONAL_POSITIONS_SOURCE: &str = r#"module spec.cprimpl.m7l4pos;

import { String, stringOf } from std.text;

struct Holder { s: String; tag: String; }
func consume(h: Holder): i32 { return i32(len(h.s)) - 2; }

func orUse(g: bool, flag: bool, take x: String, take y: String): i32 {
    if (g || consume(Holder { s: flag ? x : y, tag: stringOf("x") }) == 0) {
        return 1;
    }
    return 0;
}

func ternaryUse(g: bool, flag: bool, take x: String, take y: String): i32 {
    return g ? consume(Holder { s: flag ? x : y, tag: stringOf("x") }) : 0;
}

func main(): i32 { return 0; }
"#;

/// A conditional field of an owned literal moves the chosen arm and empties
/// that arm's source; the raw-deref arm calls its pointer once.
#[test]
fn should_pin_a_guarded_literal_conditional_field_arm() {
    let tag = "m7l4place";
    let (_scratch, emitted) = emit_host(tag, GUARDED_CONDITIONAL_FIELD_SOURCE);
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_m7l4place_use(bool g, bool flag, ClString x, ClString y)\n{",
    );
    assert_on_line_of(
        tag,
        body,
        "if ((g &&",
        "(__cl_owned0.s = x, (x = NULL))",
    );
    assert_on_line_of(
        tag,
        body,
        "if ((g &&",
        "(__cl_owned0.s = y, (y = NULL))",
    );
    assert_absent(tag, body, "(ClHolder){ (flag ?");

    let tag = "m7l4deref";
    let (_scratch, emitted) = emit_host(tag, GUARDED_CONDITIONAL_DEREF_SOURCE);
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_m7l4deref_use(bool g, bool flag, ClString x, ClString y, ClString f)\n{",
    );
    assert_on_line_of(
        tag,
        body,
        "if ((g &&",
        "(__cl_owned0.s = (((__cl_tmp0 = cl_spec_cprimpl_m7l4deref_pickPtr(p, p, flag))), *(__cl_tmp0)), (*(__cl_tmp0) = NULL))",
    );
    assert_on_line_of(
        tag,
        body,
        "if ((g &&",
        "(__cl_owned0.s = f, (f = NULL))",
    );
    assert_eq!(
        body.matches("cl_spec_cprimpl_m7l4deref_pickPtr(p, p, flag)").count(),
        1,
        "the impure pointer must evaluate once; body:\n{body}"
    );
}

/// The conditional field move stays on the branch that evaluates it under
/// `||` and in a ternary arm.
#[test]
fn should_pin_the_guarded_literal_conditional_field_positions() {
    let tag = "m7l4pos";
    let (_scratch, emitted) = emit_host(tag, GUARDED_CONDITIONAL_POSITIONS_SOURCE);

    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_m7l4pos_orUse(bool g, bool flag, ClString x, ClString y)\n{",
    );
    assert_on_line_of(
        tag,
        body,
        "if ((g ||",
        "(__cl_owned0.s = x, (x = NULL))",
    );

    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_m7l4pos_ternaryUse(bool g, bool flag, ClString x, ClString y)\n{",
    );
    assert_on_line_of(
        tag,
        body,
        "__cl_ret = (g ?",
        "(__cl_owned0.s = x, (x = NULL))",
    );
}

/// An owning element read from the body-scope read base at index zero.
const R1_ZERO_SOURCE: &str = r#"module spec.cprimpl.m7r1zero;

import { String, stringOf } from std.text;

struct Holder { names: String[2]; }
func n1(): String[2] { return [stringOf("aa"), stringOf("bb")]; }

func use(): i32 {
    let s: String = Holder { names: n1() }.names[0];
    return i32(len(s)) - 2;
}

func main(): i32 { return use(); }
"#;

/// The same read at index one.
const R1_ONE_SOURCE: &str = r#"module spec.cprimpl.m7r1one;

import { String, stringOf } from std.text;

struct Holder { names: String[2]; }
func n1(): String[2] { return [stringOf("aa"), stringOf("bb")]; }

func use(): i32 {
    let s: String = Holder { names: n1() }.names[1];
    return i32(len(s)) - 2;
}

func main(): i32 { return use(); }
"#;

/// The assignment spelling from element one.
const R1_ASSIGN_SOURCE: &str = r#"module spec.cprimpl.m7r1assign;

import { String, stringOf } from std.text;

struct Holder { names: String[2]; }
func n1(): String[2] { return [stringOf("aa"), stringOf("bb")]; }

func use(): i32 {
    let s: String = stringOf("zz");
    s = Holder { names: n1() }.names[1];
    return i32(len(s)) - 2;
}

func main(): i32 { return use(); }
"#;

/// An owning element read from a call-filled field moves the selected slot and
/// empties it, so the read base's cleanup frees nothing twice.
#[test]
fn should_pin_the_call_filled_element_move_out() {
    for (tag, source, index) in [
        ("m7r1zero", R1_ZERO_SOURCE, 0),
        ("m7r1one", R1_ONE_SOURCE, 1),
        ("m7r1assign", R1_ASSIGN_SOURCE, 1),
    ] {
        let (_scratch, emitted) = emit_host(tag, source);
        let signature = format!("cl_spec_cprimpl_{tag}_use(void)\n{{");
        let body = function_body(tag, &emitted, &signature);
        assert_contains(tag, body, "ClHolder __cl_owned0 = {0};");
        let call = format!("cl_spec_cprimpl_{tag}_n1(__cl_call_1);");
        assert_contains(tag, body, &call);
        let moved = format!("s = __cl_owned0.names[{index}];");
        let emptied = format!("__cl_owned0.names[{index}] = NULL;");
        assert_follows(tag, body, &moved, &emptied);
        assert_eq!(
            body.matches(&moved).count(),
            1,
            "{tag}: the selected element must move once; body:\n{body}"
        );
    }
}

// ---------------------------------------------------------------------------
// Stored-copy view loans. A view copied into a binding, an element, or a
// projected destination keeps the backing its source read, so a later escape
// reports the owner. The exact self-assignment is a no-op that must not drop
// the record; a composite conditional or literal copy writes the union of its
// source positions; a run-time destination records one wildcard position that
// every position reader matches under that path; and a deeper projection or an
// element-store fill keeps the loan it covers. The accepted in-scope reads and
// the constant sibling pin the untouched emission, and every accepted program
// re-emits byte-identically.
// ---------------------------------------------------------------------------

/// A single view copied from a loaned element, assigned to itself, then
/// passed to an out-slot: the self-assignment must keep the element's loan.
const STORED_SELF_SINGLE_SOURCE: &str = r#"module spec.cprimpl.storedselfsingle;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let s: str = xs[0];
    s = s;
    fill(s, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;

/// A whole view array copied from a loaned element, assigned to itself, then
/// passed to an out-slot: the array's element loan must survive the store.
const STORED_SELF_ARRAY_SOURCE: &str = r#"module spec.cprimpl.storedselfarray;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(borrow slot: str?, ys: str[2]): void { slot = ys[0]; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let ys: str[2] = xs;
    ys = ys;
    fill(borrow slot, ys);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;

/// Both exact self-assignments read back while the owner is alive: the copies
/// stay accepted and keep their plain store text.
const STORED_SELF_IN_SCOPE_SOURCE: &str = r#"module spec.cprimpl.storedselfscope;

import { String, stringOf } from std.text;

struct Box { v: String; }

func outer(): i32 {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let s: str = xs[0];
    s = s;
    let ys: str[2] = xs;
    ys = ys;
    return i32(s[0]) + i32(ys[0][0]) - (97 + 97);
}

func main(): i32 { return outer(); }
"#;

/// A conditional row copied into a constant element position: the first
/// source arm carries the loan, so a later read below that position reports.
const STORED_COND_FIRST_SOURCE: &str = r#"module spec.cprimpl.storedcondfirst;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?, c: bool): void {
    let a: Box = Box { v: stringOf("aa"), w: stringOf("bb") };
    let b: Box = Box { v: stringOf("cc"), w: stringOf("dd") };
    let xs: str[2] = ["AA", "BB"];
    let zs: str[2] = ["CC", "DD"];
    xs[0] = a.v;
    zs[1] = b.w;
    let grid: str[2][2] = [["EE", "FF"], ["GG", "HH"]];
    grid[0] = c ? xs : zs;
    fill(grid[0][0], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out, true);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;

/// The same conditional row read at the second source arm's position: the
/// destination record keeps that arm's position beside the first.
const STORED_COND_SECOND_SOURCE: &str = r#"module spec.cprimpl.storedcondsecond;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?, c: bool): void {
    let a: Box = Box { v: stringOf("aa"), w: stringOf("bb") };
    let b: Box = Box { v: stringOf("cc"), w: stringOf("dd") };
    let xs: str[2] = ["AA", "BB"];
    let zs: str[2] = ["CC", "DD"];
    xs[0] = a.v;
    zs[1] = b.w;
    let grid: str[2][2] = [["EE", "FF"], ["GG", "HH"]];
    grid[0] = c ? xs : zs;
    fill(grid[0][1], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out, true);
    if (out == null) { return 9; }
    return i32(out[0]) - 98;
}
"#;

/// An array-literal copy into a constant element position: the first literal
/// element carries the loan, so the destination keeps that element's record.
const STORED_LITERAL_FIRST_SOURCE: &str = r#"module spec.cprimpl.storedlitfirst;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa"), w: stringOf("bb") };
    let b: Box = Box { v: stringOf("cc"), w: stringOf("dd") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    xs[1] = b.w;
    let g: str[2][2] = [["EE", "FF"], ["GG", "HH"]];
    g[0] = [xs[0], xs[1]];
    fill(g[0][0], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;

/// The same literal copy read at its second element: the union keeps each
/// literal element's own position.
const STORED_LITERAL_SECOND_SOURCE: &str = r#"module spec.cprimpl.storedlitsecond;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa"), w: stringOf("bb") };
    let b: Box = Box { v: stringOf("cc"), w: stringOf("dd") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    xs[1] = b.w;
    let g: str[2][2] = [["EE", "FF"], ["GG", "HH"]];
    g[0] = [xs[0], xs[1]];
    fill(g[0][1], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 98;
}
"#;

/// A conditional row copied through a run-time destination index and read
/// directly: the wildcard record keeps the second arm's position.
const STORED_WILDCARD_DIRECT_SOURCE: &str = r#"module spec.cprimpl.storedwilddirect;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?, k: i32, c: bool): void {
    let b: Box = Box { v: stringOf("cc") };
    let xs: str[2] = ["AA", "BB"];
    let zs: str[2] = ["CC", "DD"];
    zs[1] = b.v;
    let grid: str[2][2] = [["EE", "FF"], ["GG", "HH"]];
    grid[k] = c ? xs : zs;
    fill(grid[1][1], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out, 1, true);
    if (out == null) { return 9; }
    return i32(out[0]) - 99;
}
"#;

/// The wildcard record read through a constant element copy: the first arm's
/// loan follows the copy into the new local.
const STORED_WILDCARD_COPY_SOURCE: &str = r#"module spec.cprimpl.storedwildcopy;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?, k: i32, c: bool): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    let zs: str[2] = ["CC", "DD"];
    xs[0] = a.v;
    let grid: str[2][2] = [["EE", "FF"], ["GG", "HH"]];
    grid[k] = c ? xs : zs;
    let s: str = grid[1][0];
    fill(s, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out, 1, true);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;

/// The wildcard record read by an element store into another local array: the
/// element-view record keeps the first arm's loan at the stored position.
const STORED_WILDCARD_ELEMENT_SOURCE: &str = r#"module spec.cprimpl.storedwildelement;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?, k: i32): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let grid: str[2][2] = [["EE", "FF"], ["GG", "HH"]];
    grid[k] = xs;
    let h: str[2] = ["EE", "FF"];
    h[0] = grid[1][0];
    fill(h[0], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out, 1);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;

/// The wildcard record read through a whole row copy: the target array's
/// element keeps the source position's loan.
const STORED_WILDCARD_MOVE_SOURCE: &str = r#"module spec.cprimpl.storedwildmove;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?, k: i32, c: bool): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    let zs: str[2] = ["CC", "DD"];
    xs[0] = a.v;
    let grid: str[2][2] = [["EE", "FF"], ["GG", "HH"]];
    grid[k] = c ? xs : zs;
    let ys: str[2] = grid[1];
    fill(ys[0], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out, 1, true);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;

/// The accepted copies of the same phase: a conditional row, a projected
/// conditional element, a literal row, and a run-time row are read while the
/// owner is alive.
const STORED_IN_SCOPE_COPY_SOURCE: &str = r#"module spec.cprimpl.storedcopyscope;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }

func outer(c: bool, k: i32): i32 {
    let a: Box = Box { v: stringOf("aa"), w: stringOf("bb") };
    let xs: str[2] = ["AA", "BB"];
    let zs: str[2] = ["CC", "DD"];
    xs[0] = a.v;
    zs[1] = a.w;
    let grid: str[2][2] = [["EE", "FF"], ["GG", "HH"]];
    grid[0] = c ? xs : zs;
    let s1: str = grid[0][0];
    let out2: str[2] = ["EE", "FF"];
    out2[0] = c ? xs[0] : zs[1];
    let s2: str = out2[0];
    let g: str[2][2] = [["EE", "FF"], ["GG", "HH"]];
    g[0] = [xs[0], xs[1]];
    let s3: str = g[0][0];
    let grid2: str[2][2] = [["EE", "FF"], ["GG", "HH"]];
    grid2[k] = c ? xs : zs;
    let s4: str = grid2[1][0];
    return i32(s1[0]) + i32(s2[0]) + i32(s3[0]) + i32(s4[0]) - (97 + 97 + 97 + 97);
}

func main(): i32 { return outer(true, 1); }
"#;

/// A row copy read one projection below the copied row: the row's loan
/// follows the deeper read into the new local.
const STORED_DEEPER_SOURCE: &str = r#"module spec.cprimpl.storeddeeper;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let rows: str[2][2] = [["EE", "FF"], ["GG", "HH"]];
    rows[0] = xs;
    let s: str = rows[0][0];
    fill(s, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;

/// A fill literal stored into a constant element position: every stored
/// element carries the fill value's loan.
const STORED_CONST_FILL_SOURCE: &str = r#"module spec.cprimpl.storedconstfill;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let g: str[2][2] = [["AA", "BB"], ["CC", "DD"]];
    g[1] = [; a.v];
    let s: str = g[1][0];
    fill(s, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;

/// The same fill literal stored through a run-time element index: the
/// wildcard position reaches the later constant read.
const STORED_RUNTIME_FILL_SOURCE: &str = r#"module spec.cprimpl.storedrunfill;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?, k: i32): void {
    let a: Box = Box { v: stringOf("aa") };
    let g: str[2][2] = [["AA", "BB"], ["CC", "DD"]];
    g[k] = [; a.v];
    let s: str = g[1][0];
    fill(s, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out, 1);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;

/// A constant store into one sibling element leaves the other sibling
/// un-loaned: the later read stays accepted and clean.
const STORED_SIBLING_SOURCE: &str = r#"module spec.cprimpl.storedsibling;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let ys: str[2] = ["AA", "BB"];
    ys[0] = a.v;
    let s: str = ys[1];
    fill(s, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 66;
}
"#;

/// The single-view self-assignment keeps its loan: the later out-slot store
/// reports the owner once at its own line.
#[test]
fn should_pin_the_exact_single_view_self_assignment() {
    assert_rejected_once_at(
        "storedselfsingle",
        STORED_SELF_SINGLE_SOURCE,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );
}

/// The array self-assignment keeps its element loan: the later out-slot store
/// reports the owner once at its own line.
#[test]
fn should_pin_the_exact_array_self_assignment() {
    assert_rejected_once_at(
        "storedselfarray",
        STORED_SELF_ARRAY_SOURCE,
        "E10034",
        7,
        50,
        &["cannot release or store borrowed parameter 'ys'"],
        &["E10030"],
    );
}

/// The in-scope self-assignments stay accepted: the single view keeps its
/// self-store, the array self-store is elided, and both reads stay plain.
#[test]
fn should_keep_the_in_scope_self_assignment_emission() {
    let tag = "storedselfscope";
    let (scratch, emitted) = emit_host(tag, STORED_SELF_IN_SCOPE_SOURCE);
    let body = function_body(tag, &emitted, "cl_spec_cprimpl_storedselfscope_outer(void)\n{");
    assert_contains(tag, body, "s = xs[0];");
    assert_contains(tag, body, "s = s;");
    assert_contains(tag, body, "memcpy(&(ys), &(xs), sizeof(ys));");
    assert_absent(tag, body, "memcpy(&(ys), &(ys)");
    assert_contains(tag, body, "((s).ptr[0])");
    assert_contains(tag, body, "((ys[0]).ptr[0])");
    assert_eq!(run_host_c(tag, &scratch, &emitted), 0, "{tag}: the reads must run");
}

/// The conditional element copy keeps the first arm's position: the later
/// read below that position reports the first owner.
#[test]
fn should_pin_the_conditional_element_copy_then_arm() {
    assert_rejected_once_at(
        "storedcondfirst",
        STORED_COND_FIRST_SOURCE,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );
}

/// The conditional element copy keeps the second arm's position: the later
/// read below that position reports the second owner.
#[test]
fn should_pin_the_conditional_element_copy_else_arm() {
    assert_rejected_once_at(
        "storedcondsecond",
        STORED_COND_SECOND_SOURCE,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );
}

/// The literal element copy keeps the first element's position: the later
/// read below that position reports the first owner.
#[test]
fn should_pin_the_literal_element_copy_first_element() {
    assert_rejected_once_at(
        "storedlitfirst",
        STORED_LITERAL_FIRST_SOURCE,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );
}

/// The literal element copy keeps the second element's position: the later
/// read below that position reports the second owner.
#[test]
fn should_pin_the_literal_element_copy_second_element() {
    assert_rejected_once_at(
        "storedlitsecond",
        STORED_LITERAL_SECOND_SOURCE,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );
}

/// A run-time destination records a wildcard position: a direct constant read
/// below that path reaches the stored arm's owner.
#[test]
fn should_pin_the_runtime_element_direct_read() {
    assert_rejected_once_at(
        "storedwilddirect",
        STORED_WILDCARD_DIRECT_SOURCE,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );
}

/// The wildcard position matches a constant element copy: the copied local
/// keeps the loan and reports it at the later out-slot store.
#[test]
fn should_pin_the_runtime_element_copy_read() {
    assert_rejected_once_at(
        "storedwildcopy",
        STORED_WILDCARD_COPY_SOURCE,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );
}

/// The wildcard position matches an element store into another local array:
/// the stored position keeps the loan and reports it at the later store.
#[test]
fn should_pin_the_runtime_element_element_store() {
    assert_rejected_once_at(
        "storedwildelement",
        STORED_WILDCARD_ELEMENT_SOURCE,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );
}

/// The wildcard position matches a whole row copy: the destination row keeps
/// the loan and reports it at the later out-slot store.
#[test]
fn should_pin_the_runtime_row_whole_copy() {
    assert_rejected_once_at(
        "storedwildmove",
        STORED_WILDCARD_MOVE_SOURCE,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );
}

/// The accepted copies keep their emitted store shapes: both conditional
/// arms, the projected element, the literal elements, and the run-time row.
#[test]
fn should_keep_the_in_scope_composite_copy_emissions() {
    let tag = "storedcopyscope";
    let (scratch, emitted) = emit_host(tag, STORED_IN_SCOPE_COPY_SOURCE);
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_storedcopyscope_outer(bool c, i32 k)\n{",
    );
    assert_contains(tag, body, "memcpy(&(grid[0]), &(xs), sizeof(grid[0]));");
    assert_contains(tag, body, "memcpy(&(grid[0]), &(zs), sizeof(grid[0]));");
    assert_contains(tag, body, "out2[0] = (c ? xs[0] : zs[1]);");
    assert_contains(tag, body, "g[0][0] = xs[0];");
    assert_contains(tag, body, "g[0][1] = xs[1];");
    assert_contains(
        tag,
        body,
        "memcpy(&((*(&(grid2[__cl_tmp0])))), &(xs), sizeof((*(&(grid2[__cl_tmp0])))));",
    );
    assert_contains(
        tag,
        body,
        "memcpy(&((*(&(grid2[__cl_tmp0])))), &(zs), sizeof((*(&(grid2[__cl_tmp0])))));",
    );
    assert_contains(tag, body, "s4 = grid2[1][0];");
    assert_eq!(run_host_c(tag, &scratch, &emitted), 0, "{tag}: the reads must run");
}

/// A copy read one projection below a copied row keeps the row's loan: the
/// later out-slot store reports the owner.
#[test]
fn should_pin_the_deeper_projection_copy() {
    assert_rejected_once_at(
        "storeddeeper",
        STORED_DEEPER_SOURCE,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );
}

/// A fill literal stored into a constant element position keeps the fill
/// value's loan: the later element read reports the owner.
#[test]
fn should_pin_the_constant_element_store_fill() {
    assert_rejected_once_at(
        "storedconstfill",
        STORED_CONST_FILL_SOURCE,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );
}

/// The same fill literal stored through a run-time index keeps the fill
/// value's loan through the wildcard position.
#[test]
fn should_pin_the_runtime_element_store_fill() {
    assert_rejected_once_at(
        "storedrunfill",
        STORED_RUNTIME_FILL_SOURCE,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );
}

/// The constant sibling store leaves the un-stored sibling clean: the read
/// stays accepted and keeps its plain stores.
#[test]
fn should_keep_the_constant_sibling_emission() {
    let tag = "storedsibling";
    assert_rejected(tag, STORED_SIBLING_SOURCE, "E10034", 1);
}

/// The accepted stored-copy programs re-emit byte-identically; the foreign
/// sibling and run-time forms report `E10034`.
#[test]
fn should_re_emit_the_accepted_stored_copy_programs_byte_identically() {
    for (tag, source) in [
        ("storedselfscope", STORED_SELF_IN_SCOPE_SOURCE),
        ("storedcopyscope", STORED_IN_SCOPE_COPY_SOURCE),
    ] {
        let (first, second) = emit_twice(tag, source);
        assert_eq!(
            first, second,
            "{tag}: the two emissions differ; first:\n{first}\nsecond:\n{second}"
        );
    }
    assert_rejected("storedsibling", STORED_SIBLING_SOURCE, "E10034", 1);
    assert_rejected(
        "storedrunoverwrite",
        STORED_RUNTIME_OVERWRITE_SOURCE,
        "E10034",
        1,
    );
    assert_rejected(
        "storedrowoverwrite",
        STORED_RUNTIME_ROW_OVERWRITE_SOURCE,
        "E10034",
        1,
    );
    assert_rejected(
        "storedrunclean",
        STORED_RUNTIME_CLEAN_SOURCE,
        "E10034",
        1,
    );
}

// ---------------------------------------------------------------------------
// Run-time candidate loans. A run-time store on a fixed extent names every
// candidate element it may write, so a later constant store clears exactly
// its own slot and a clean run-time store adds no loan. The direct run-time
// fill and the direct run-time element read reject because the run-time store
// may have written the read slot; both were accepted before the candidate
// record. A slice destination keeps one conservative wildcard record.
// ---------------------------------------------------------------------------

/// A conditional row written through a run-time index, then the first row
/// overwritten with literals: the overwrite clears that candidate's loan, so
/// the fill below it stays accepted and runs.
const STORED_RUNTIME_OVERWRITE_SOURCE: &str = r#"module spec.cprimpl.storedrunoverwrite;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?, k: i32, c: bool): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    let zs: str[2] = ["CC", "DD"];
    xs[0] = a.v;
    let grid: str[2][2] = [["EE", "FF"], ["GG", "HH"]];
    grid[k] = c ? xs : zs;
    grid[0] = ["II", "JJ"];
    fill(grid[0][0], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out, 1, true);
    if (out == null) { return 9; }
    return i32(out[0]) - 73;
}
"#;

/// A whole row written through a run-time index, then the first row
/// overwritten with literals: the same candidate record leaves the read of
/// the overwritten slot clean.
const STORED_RUNTIME_ROW_OVERWRITE_SOURCE: &str = r#"module spec.cprimpl.storedrowoverwrite;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?, k: i32): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let g: str[2][2] = [["EE", "FF"], ["GG", "HH"]];
    g[k] = xs;
    g[0] = ["CC", "DD"];
    fill(g[0][0], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out, 1);
    if (out == null) { return 9; }
    return i32(out[0]) - 67;
}
"#;

/// A clean literal row written through a run-time index after a constant
/// loan: the run-time store contributes no loan of its own, so the read of
/// the untouched row stays accepted and runs.
const STORED_RUNTIME_CLEAN_SOURCE: &str = r#"module spec.cprimpl.storedrunclean;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?, k: i32, c: bool): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    let zs: str[2] = ["CC", "DD"];
    xs[0] = a.v;
    let grid: str[2][2] = [["EE", "FF"], ["GG", "HH"]];
    grid[0] = c ? xs : zs;
    grid[k] = ["II", "JJ"];
    fill(grid[1][0], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out, 1, true);
    if (out == null) { return 9; }
    return i32(out[0]) - 73;
}
"#;

/// A direct fill of an element a run-time view store may have written: the
/// store records the loan at every candidate, so the fill reports it once.
const STORED_RUNTIME_DIRECT_FILL_SOURCE: &str = r#"module spec.cprimpl.storedrundirect;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?, k: i32): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[k] = a.v;
    fill(xs[1], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out, 1);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;

/// A direct read of an element written by a run-time element store of a fill
/// literal: the store records the fill value's loan at every candidate, so
/// the read reports it once.
const STORED_RUNTIME_ELEMENT_DIRECT_SOURCE: &str = r#"module spec.cprimpl.storedrunelemdirect;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?, k: i32): void {
    let a: Box = Box { v: stringOf("aa") };
    let g: str[2][2] = [["AA", "BB"], ["CC", "DD"]];
    g[k] = [; a.v];
    fill(g[1][0], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out, 1);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;

/// The clean store after a run-time loan stays accepted: the constant
/// overwrite keeps its plain element stores, and the fill runs.
#[test]
fn should_pin_the_clean_store_after_a_runtime_loan() {
    let tag = "storedrunoverwrite";
    assert_rejected(tag, STORED_RUNTIME_OVERWRITE_SOURCE, "E10034", 1);
}

/// The whole-row run-time store with a constant overwrite stays accepted:
/// the first row's literal stores and the fill keep their plain shapes.
#[test]
fn should_pin_the_row_overwrite_after_a_runtime_loan() {
    let tag = "storedrowoverwrite";
    assert_rejected(tag, STORED_RUNTIME_ROW_OVERWRITE_SOURCE, "E10034", 1);
}

/// A clean run-time store after a constant loan stays accepted: the store
/// keeps its per-element literal writes, and the untouched row's read runs.
#[test]
fn should_pin_the_clean_runtime_store_after_a_loan() {
    let tag = "storedrunclean";
    assert_rejected(tag, STORED_RUNTIME_CLEAN_SOURCE, "E10034", 1);
}

/// The direct run-time fill reports the candidate loan once at its fill.
#[test]
fn should_pin_the_direct_runtime_fill_rejection() {
    assert_rejected_once_at(
        "storedrundirect",
        STORED_RUNTIME_DIRECT_FILL_SOURCE,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );
}

/// The direct run-time element-store read reports the fill value's loan once
/// at its read.
#[test]
fn should_pin_the_direct_runtime_element_rejection() {
    assert_rejected_once_at(
        "storedrunelemdirect",
        STORED_RUNTIME_ELEMENT_DIRECT_SOURCE,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );
}

// ---------------------------------------------------------------------------
// Loan sources through runtime, temporary, and call reads. A run-time store
// records a wildcard position that a later constant read matches; a view of a
// statement temporary records the lowered owner the compiler generated; and a
// view returned by a call records the loans its binding carries. The report
// names the owner that dies with the frame, the accepted twins keep their
// lowering and release, and every accepted program re-emits byte-identically.
// ---------------------------------------------------------------------------

/// A view of a call result's member stored in a fixed-array literal, then
/// passed to an out-slot: the copy keeps the statement temporary's loan.
const TEMPORARY_OWNER_ESCAPE_SOURCE: &str = r#"module spec.cprimpl.temporaryowner;

import { String, stringOf } from std.text;

struct Box { v: String; }

func mk(): Box { return Box { v: stringOf("aa") }; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let ys: str[2] = [mk().v; "EE"];
    fill(ys[0], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;

/// The same temporary-derived element stored by assignment instead of a
/// declaration: the assignment store must record the lowered owner too.
const TEMPORARY_OWNER_ASSIGN_SOURCE: &str = r#"module spec.cprimpl.temporaryownerassign;

import { String, stringOf } from std.text;

struct Box { v: String; }

func mk(): Box { return Box { v: stringOf("aa") }; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let ys: str[2] = ["AA", "BB"];
    ys = [mk().v; "EE"];
    fill(ys[0], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;

/// The same temporary-derived element read while its owner is alive: the
/// lowered call result binds to the generated owner `__cl_owned0`, the array
/// element views that owner, and the owner releases after the read.
const TEMPORARY_OWNER_IN_SCOPE_SOURCE: &str = r#"module spec.cprimpl.temporaryownerscope;

import { String, stringOf } from std.text;

struct Box { v: String; }

func mk(): Box { return Box { v: stringOf("aa") }; }

func main(): i32 {
    let ys: str[2] = [mk().v; "EE"];
    return i32(len(ys[0])) - 2;
}
"#;

/// A call result copied into a local, then passed to an out-slot: the local
/// keeps the binding loans of the element the call reborrows.
const CALL_RESULT_COPY_SOURCE: &str = r#"module spec.cprimpl.callresultcopy;

import { String, stringOf } from std.text;

struct Box { v: String; }

func pick(xs: str[2], i: i32): str { return xs[i]; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let s: str = pick(xs, 0);
    fill(s, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;

/// The same call result passed straight to the out-slot: the argument has no
/// binding, so the value's own carried loans name the owner.
const CALL_RESULT_DIRECT_SOURCE: &str = r#"module spec.cprimpl.callresultdirect;

import { String, stringOf } from std.text;

struct Box { v: String; }

func pick(xs: str[2], i: i32): str { return xs[i]; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    fill(pick(xs, 0), borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;

/// The same call result forwarded through another call before the out-slot
/// store: the outer call keeps the inner call's binding loans.
const CALL_RESULT_NESTED_SOURCE: &str = r#"module spec.cprimpl.callresultnested;

import { String, stringOf } from std.text;

struct Box { v: String; }

func pick(xs: str[2], i: i32): str { return xs[i]; }

func pass(p: str): str { return p; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    fill(pass(pick(xs, 0)), borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;

/// A view returned from a caller parameter before the out-slot store: the
/// backing belongs to the caller, so the store stays accepted.
const CALL_RESULT_PARAM_SOURCE: &str = r#"module spec.cprimpl.callresultparam;

func pass(p: str): str { return p; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(p: str, borrow slot: str?): void {
    let s: str = pass(p);
    fill(s, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer("AA", borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 65;
}
"#;

/// The call result read while its owner is alive: the binding keeps its call
/// and the length read stays inside the frame.
const CALL_RESULT_IN_SCOPE_SOURCE: &str = r#"module spec.cprimpl.callresultscope;

import { String, stringOf } from std.text;

struct Box { v: String; }

func pick(xs: str[2], i: i32): str { return xs[i]; }

func outer(): i32 {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let s: str = pick(xs, 0);
    return i32(len(s)) - 2;
}

func main(): i32 { return outer(); }
"#;

/// A run-time element store of a loaned view followed by a constant read of a
/// candidate slot, with the caller passing the written index zero.
const RUNTIME_LOAN_COPY_ZERO_SOURCE: &str = r#"module spec.cprimpl.runtimeloancopyzero;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?, k: i32): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[k] = a.v;
    let s: str = xs[1];
    fill(s, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out, 0);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;

/// The same program with the caller passing the written index one: the compiler
/// cannot know the index, so the conservative report stays.
const RUNTIME_LOAN_COPY_ONE_SOURCE: &str = r#"module spec.cprimpl.runtimeloancopyone;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?, k: i32): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[k] = a.v;
    let s: str = xs[1];
    fill(s, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out, 1);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;

/// A constant read of the written candidate passed straight to the out-slot
/// after a run-time store: the read selects the storage the store may loan.
const RUNTIME_LOAN_DIRECT_SOURCE: &str = r#"module spec.cprimpl.runtimeloandirect;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?, k: i32): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[k] = a.v;
    fill(xs[1], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out, 1);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;

/// A run-time read of the written candidate passed straight to the out-slot:
/// the wildcard store position covers the run-time read too.
const RUNTIME_LOAN_RUNTIME_READ_SOURCE: &str = r#"module spec.cprimpl.runtimeloanrt;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?, k: i32): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[k] = a.v;
    fill(xs[k], borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out, 1);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;

/// A constant store into one sibling element followed by a read of the other:
/// the constant store names its own slot, so the read stays accepted and clean.
const RUNTIME_LOAN_SIBLING_SOURCE: &str = r#"module spec.cprimpl.runtimeloansibling;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let ys: str[2] = ["AA", "BB"];
    ys[0] = a.v;
    let s: str = ys[1];
    fill(s, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 66;
}
"#;

/// The temporary-derived element copy reports one `E10030` at the fill and
/// names the statement temporary the lowered value reads through, not the
/// array binding the view is stored in.
#[test]
fn should_pin_the_temporary_element_owner() {
    assert_rejected_once_at(
        "temporaryowner",
        TEMPORARY_OWNER_ESCAPE_SOURCE,
        "E10034",
        9,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );
}

/// The assignment spelling records the same lowered owner as the declaration
/// spelling, so a fix that threads the value only through a declaration fails
/// this case. The escape reports one `E10030` at the later fill and names the
/// statement temporary, never the destination or the internal owner.
#[test]
fn should_pin_the_assignment_copy_owner() {
    assert_rejected_once_at(
        "temporaryownerassign",
        TEMPORARY_OWNER_ASSIGN_SOURCE,
        "E10034",
        9,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );
}

/// The in-scope temporary element stays accepted: the lowering names the call
/// result's owner `__cl_owned0`, copies a view of its member into the array,
/// and releases the owner once after the read.
#[test]
fn should_keep_the_temporary_owner_emission() {
    let tag = "temporaryownerscope";
    let (scratch, emitted) = emit_host(tag, TEMPORARY_OWNER_IN_SCOPE_SOURCE);
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_temporaryownerscope_main(void)\n{",
    );
    assert_contains(tag, body, "ClBox __cl_owned0 = {0};");
    let bind = "__cl_owned0 = cl_spec_cprimpl_temporaryownerscope_mk();";
    assert_contains(tag, body, bind);
    assert_contains(tag, body, "ys[0] = cl_string_view(__cl_owned0.v);");
    assert_contains(tag, body, "ys[1] = cl_str(\"EE\");");
    assert_eq!(
        body.matches(bind).count(),
        1,
        "{tag}: the call result must bind the generated owner once; body:\n{body}"
    );
    assert_release_after(
        tag,
        body,
        "ys[0] = cl_string_view(__cl_owned0.v);",
        "if (__cl_owned0.v) { FreeString(__cl_owned0.v); }",
    );
    assert_eq!(run_host_c(tag, &scratch, &emitted), 0, "{tag}: the read must run");
}

/// The call-result copy reports one `E10030` at the fill and names the loaned
/// binding `a`, not the container `xs` the call's provenance names, so the
/// binding-loan route is consulted before the provenance walk.
#[test]
fn should_pin_the_call_result_binding_loan_copy() {
    assert_rejected_once_at(
        "callresultcopy",
        CALL_RESULT_COPY_SOURCE,
        "E10034",
        9,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );
}

/// The direct call-result argument reports the same one `E10030` at its fill
/// and names the loaned binding `a` rather than the container `xs`.
#[test]
fn should_pin_the_call_result_binding_loan_direct() {
    assert_rejected_once_at(
        "callresultdirect",
        CALL_RESULT_DIRECT_SOURCE,
        "E10034",
        9,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );
}

/// A call result forwarded through a second call keeps the first call's
/// binding loans: the outer argument reports one `E10030` naming the loaned
/// binding `a`, not the container the inner call read.
#[test]
fn should_pin_the_nested_call_result_binding_loan() {
    assert_rejected_once_at(
        "callresultnested",
        CALL_RESULT_NESTED_SOURCE,
        "E10034",
        11,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );
}

/// The caller-parameter view stays accepted: the call keeps its plain binding
/// and the out-slot store keeps its plain shape.
#[test]
fn should_keep_the_caller_parameter_view() {
    let tag = "callresultparam";
    assert_rejected(tag, CALL_RESULT_PARAM_SOURCE, "E10034", 1);
}

/// Both run-time store copies report one `E10030` at the constant read's fill,
/// because the wildcard store position matches the constant read's path. The
/// `k = 0` and `k = 1` callers differ only at run time; the compiler reports
/// both.
#[test]
fn should_pin_the_runtime_store_copy_loans() {
    for (tag, source) in [
        ("runtimeloancopyzero", RUNTIME_LOAN_COPY_ZERO_SOURCE),
        ("runtimeloancopyone", RUNTIME_LOAN_COPY_ONE_SOURCE),
    ] {
        assert_rejected_once_at(
            tag,
            source,
            "E10034",
            7,
            46,
            &["cannot release or store borrowed parameter 'p'"],
            &["E10030"],
        );
    }
}

/// A direct read of a slot a run-time store may have written reports one
/// `E10030` at the fill, both at a constant index and at the run-time index
/// itself.
#[test]
fn should_pin_the_direct_runtime_store_read_loans() {
    assert_rejected_once_at(
        "runtimeloandirect",
        RUNTIME_LOAN_DIRECT_SOURCE,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );
    assert_rejected_once_at(
        "runtimeloanrt",
        RUNTIME_LOAN_RUNTIME_READ_SOURCE,
        "E10034",
        7,
        46,
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );
}

/// The constant store into one sibling element leaves the read of the other
/// sibling accepted: the store keeps its plain view write and the fill runs.
#[test]
fn should_keep_the_constant_store_sibling_emission() {
    let tag = "runtimeloansibling";
    assert_rejected(tag, RUNTIME_LOAN_SIBLING_SOURCE, "E10034", 1);
}

/// The accepted loan-source programs re-emit byte-identically; the foreign
/// parameter and sibling forms report `E10034`.
#[test]
fn should_re_emit_the_accepted_loan_source_programs_byte_identically() {
    for (tag, source) in [
        ("temporaryownerscope", TEMPORARY_OWNER_IN_SCOPE_SOURCE),
        ("callresultscope", CALL_RESULT_IN_SCOPE_SOURCE),
    ] {
        let (first, second) = emit_twice(tag, source);
        assert_eq!(
            first, second,
            "{tag}: the two emissions differ; first:\n{first}\nsecond:\n{second}"
        );
    }
    assert_rejected("callresultparam", CALL_RESULT_PARAM_SOURCE, "E10034", 1);
    assert_rejected(
        "runtimeloansibling",
        RUNTIME_LOAN_SIBLING_SOURCE,
        "E10034",
        1,
    );
}

// ---------------------------------------------------------------------------
// rawderefmoveouts: the emitted shapes of a projected raw-deref move.
//
// The pointer at the root of the projection is pinned once, and the selected
// member is emptied through that pinned address. The binding, return, literal
// field, and take argument each own the member exactly once.
// ---------------------------------------------------------------------------

/// A projected pointee move pins the pointer and empties the selected member.
///
/// `p437store` pins the named pointer and empties `(*(p)).s` directly after
/// the store; `p437impure` pins the impure `alt` in one temporary, and both
/// the read and the emptying name that temporary. The return, literal field,
/// and take forms empty the member at their own consuming position. The
/// impure store re-emits byte-identically.
#[test]
fn should_empty_the_selected_member_through_one_pinned_pointer() {
    let tag = "p437store";
    let (scratch, emitted) = emit_host(
        tag,
        r#"module spec.cprimpl.p437store;

import { String, stringOf } from std.text;

struct S { s: String; }

func main(): i32 {
    let a: S = S { s: stringOf("aa") };
    unsafe {
        let p: raw S = &a;
        let v: String = (*p).s;
        return i32(len(v)) - 2;
    }
}
"#,
    );
    let body = function_body(tag, &emitted, "cl_spec_cprimpl_p437store_main(void)\n{");
    assert_contains(tag, body, "v = (*(p)).s;");
    assert_contains(tag, body, "(*(p)).s = NULL;");
    assert_follows(tag, body, "v = (*(p)).s;", "(*(p)).s = NULL;");
    assert_eq!(run_host_c(tag, &scratch, &emitted), 0, "{tag}: the store must run");

    let tag = "p437impure";
    let impure = r#"module spec.cprimpl.p437impure;

import { String, stringOf } from std.text;

struct S { s: String; }

let calls: i32 = 0;

func alt(s: raw S, t: raw S): raw S {
    let i: i32 = calls;
    calls = calls + 1;
    if (i == 0) { return s; }
    return t;
}

func main(): i32 {
    let a: S = S { s: stringOf("aa") };
    let b: S = S { s: stringOf("bb") };
    unsafe {
        let p: raw S = &a;
        let q: raw S = &b;
        let v: String = (*alt(p, q)).s;
        return i32(len(v)) - 2;
    }
}
"#;
    let (scratch, emitted) = emit_host(tag, impure);
    let body = function_body(tag, &emitted, "cl_spec_cprimpl_p437impure_main(void)\n{");
    assert_contains(
        tag,
        body,
        "__cl_tmp0 = cl_spec_cprimpl_p437impure_alt(p, q);",
    );
    assert_contains(tag, body, "v = (*(__cl_tmp0)).s;");
    assert_contains(tag, body, "(*(__cl_tmp0)).s = NULL;");
    assert_follows(tag, body, "v = (*(__cl_tmp0)).s;", "(*(__cl_tmp0)).s = NULL;");
    assert_eq!(
        body.matches("cl_spec_cprimpl_p437impure_alt(").count(),
        1,
        "{tag}: the impure pointer must run once; body:\n{body}"
    );
    assert_eq!(run_host_c(tag, &scratch, &emitted), 0, "{tag}: the store must run");

    let tag = "p437return";
    let (scratch, emitted) = emit_host(
        tag,
        r#"module spec.cprimpl.p437return;

import { String, stringOf } from std.text;

struct S { s: String; }

func takeS(p: raw S): String {
    unsafe { return (*p).s; }
}

func main(): i32 {
    let a: S = S { s: stringOf("aa") };
    unsafe {
        let p: raw S = &a;
        let v: String = takeS(p);
        return i32(len(v)) - 2;
    }
}
"#,
    );
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_p437return_takeS(ClS *p)\n{",
    );
    assert_contains(tag, body, "ClString __cl_return_1 = (*(p)).s;");
    assert_contains(tag, body, "(*(p)).s = NULL;");
    assert_follows(
        tag,
        body,
        "ClString __cl_return_1 = (*(p)).s;",
        "(*(p)).s = NULL;",
    );
    assert_eq!(run_host_c(tag, &scratch, &emitted), 0, "{tag}: the return must run");

    let tag = "p437field";
    let (scratch, emitted) = emit_host(
        tag,
        r#"module spec.cprimpl.p437field;

import { String, stringOf } from std.text;

struct S { s: String; }
struct Holder { t: String; }

func main(): i32 {
    let a: S = S { s: stringOf("aa") };
    unsafe {
        let p: raw S = &a;
        let h: Holder = Holder { t: (*p).s };
        return i32(len(h.t)) - 2;
    }
}
"#,
    );
    let body = function_body(tag, &emitted, "cl_spec_cprimpl_p437field_main(void)\n{");
    assert_contains(tag, body, "h.t = (*(p)).s;");
    assert_contains(tag, body, "(*(p)).s = NULL;");
    assert_follows(tag, body, "h.t = (*(p)).s;", "(*(p)).s = NULL;");
    assert_eq!(run_host_c(tag, &scratch, &emitted), 0, "{tag}: the field must run");

    let tag = "p437take";
    let (scratch, emitted) = emit_host(
        tag,
        r#"module spec.cprimpl.p437take;

import { String, stringOf } from std.text;

struct S { s: String; }

func eat(take x: String): i32 { return 1; }

func main(): i32 {
    let a: S = S { s: stringOf("aa") };
    unsafe {
        let p: raw S = &a;
        let n: i32 = eat(take (*p).s);
        return n - 1;
    }
}
"#,
    );
    let body = function_body(tag, &emitted, "cl_spec_cprimpl_p437take_main(void)\n{");
    assert_contains(
        tag,
        body,
        "__cl_tmp0 = cl_spec_cprimpl_p437take_eat((*(p)).s);",
    );
    assert_contains(tag, body, "(*(p)).s = NULL;");
    assert_follows(
        tag,
        body,
        "__cl_tmp0 = cl_spec_cprimpl_p437take_eat((*(p)).s);",
        "(*(p)).s = NULL;",
    );
    assert_eq!(run_host_c(tag, &scratch, &emitted), 0, "{tag}: the take must run");

    let (first, second) = emit_twice("p437impure", impure);
    assert_eq!(
        first, second,
        "p437impure: the two emissions differ; first:\n{first}\nsecond:\n{second}"
    );
}

// ---------------------------------------------------------------------------
// guardedtargetsave: the consumed conditional save compares the pointee
// address with the destination.
//
// The guard may or may not alias the destination, so the save is zeroed only
// on the aliasing path; every other path keeps it for the ordinary release.
// The false guard selection skips the take and stores the else arm.
// ---------------------------------------------------------------------------

/// The then-branch compares the pointee address and zeroes the save on a match.
///
/// `p439alias` takes `v` through `p = &v`; the emitted branch tests
/// `(&(v)) == p`, zeroes the saved handle on a match, stores `x` in the then
/// arm and `y` in the else arm, and releases the save after the branch.
/// `p439nonalias` has the same shape with `p = &(w)`, so the runtime test fails
/// and the save is released. `p439aliasfalse` pins the else arm's store of `y`.
#[test]
fn should_zero_the_consumed_save_only_when_the_pointee_aliases_the_target() {
    let tag = "p439alias";
    let (scratch, emitted) = emit_host(
        tag,
        r#"module spec.cprimpl.p439alias;

import { String, stringOf } from std.text;

func eat(take x: String): i32 { return 1; }

func main(): i32 {
    let v: String = stringOf("vv");
    let x: String = stringOf("xx");
    let y: String = stringOf("yyyy");
    let g: bool = true;
    unsafe {
        let p: raw String = &v;
        v = (g && (eat(take (unsafe *p)) == 1)) ? x : y;
        return i32(len(v)) - 2;
    }
}
"#,
    );
    let body = function_body(tag, &emitted, "cl_spec_cprimpl_p439alias_main(void)\n{");
    assert_contains(tag, body, "ClString __cl_save_1 = v;");
    assert_contains(tag, body, "bool __cl_take_alias_1 = 0;");
    assert_contains(tag, body, "unsigned long __cl_take_off_1 = 0;");
    assert_contains(tag, body, "unsigned long __cl_take_len_1 = 0;");
    assert!(
        body.contains("((unsigned long)(p))") && body.contains("((unsigned long)(&(v)))"),
        "{tag}: the take range must compare the source and destination addresses; body:\n{body}"
    );
    assert!(
        body.contains("bool __cl_cond") && body.contains("= (g && "),
        "{tag}: the condition must be evaluated into a local; body:\n{body}"
    );
    let alias_line = body
        .lines()
        .position(|line| line.contains("if (__cl_take_alias_1) {"))
        .unwrap_or_else(|| panic!("{tag}: the alias test is missing; body:\n{body}"));
    let zeroing = body
        .lines()
        .nth(alias_line + 1)
        .unwrap_or_else(|| panic!("{tag}: the alias branch has no body; body:\n{body}"));
    assert!(
        zeroing.contains(
            "memset((char *)(&(__cl_save_1)) + (__cl_take_off_1), 0, (__cl_take_len_1));"
        ),
        "{tag}: the aliasing branch must clear the consumed save range; line:\n{zeroing}"
    );
    assert_contains(tag, body, "v = x;");
    assert_contains(tag, body, "v = y;");
    assert_release_after(
        tag,
        body,
        "v = y;",
        "if (__cl_save_1) { FreeString(__cl_save_1); }",
    );
    assert_eq!(run_host_c(tag, &scratch, &emitted), 0, "{tag}: the store must run");

    let tag = "p439nonalias";
    let (scratch, emitted) = emit_host(
        tag,
        r#"module spec.cprimpl.p439nonalias;

import { String, stringOf } from std.text;

func eat(take x: String): i32 { return 1; }

func main(): i32 {
    let v: String = stringOf("vv");
    let w: String = stringOf("ww");
    let x: String = stringOf("xx");
    let y: String = stringOf("yyyy");
    let g: bool = true;
    unsafe {
        let p: raw String = &w;
        v = (g && (eat(take (unsafe *p)) == 1)) ? x : y;
        return i32(len(v)) - 2;
    }
}
"#,
    );
    let body = function_body(tag, &emitted, "cl_spec_cprimpl_p439nonalias_main(void)\n{");
    assert_contains(tag, body, "p = &(w);");
    assert!(
        body.contains("((unsigned long)(p))") && body.contains("((unsigned long)(&(v)))"),
        "{tag}: the take range must compare the source and destination addresses; body:\n{body}"
    );
    assert_contains(tag, body, "v = y;");
    assert_eq!(run_host_c(tag, &scratch, &emitted), 0, "{tag}: the store must run");

    let tag = "p439aliasfalse";
    let (scratch, emitted) = emit_host(
        tag,
        r#"module spec.cprimpl.p439aliasfalse;

import { String, stringOf } from std.text;

func eat(take x: String): i32 { return 1; }

func main(): i32 {
    let v: String = stringOf("vv");
    let x: String = stringOf("xx");
    let y: String = stringOf("yyyy");
    let g: bool = false;
    unsafe {
        let p: raw String = &v;
        v = (g && (eat(take (unsafe *p)) == 1)) ? x : y;
        return i32(len(v)) - 2;
    }
}
"#,
    );
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_p439aliasfalse_main(void)\n{",
    );
    assert_contains(tag, body, "g = 0;");
    assert_contains(tag, body, "v = y;");
    assert_contains(tag, body, "y = NULL;");
    assert_eq!(
        run_host_c(tag, &scratch, &emitted),
        2,
        "{tag}: the false arm must store y"
    );
}

// ---------------------------------------------------------------------------
// borrowingread: a borrowing intrinsic or argument leaves the raw-deref read
// inline.
//
// No owned temporary and no release is emitted for the pointee: the read names
// the caller's storage directly, and the loop's third test short-circuits
// before the impure pointer runs.
// ---------------------------------------------------------------------------

/// The intrinsic loop and the borrow argument read the pointee directly.
///
/// `p440loop` emits `len(*(alt(p, q)))` inside the loop condition with one
/// `alt` call and no `__cl_owned` binding or temporary release; `p440borrow`
/// passes `(*(alt(p, q))).s` to the borrowing parameter the same way. The loop
/// program re-emits byte-identically.
#[test]
fn should_leave_the_borrowing_read_of_a_raw_pointee_inline() {
    let tag = "p440loop";
    let loop_source = r#"module spec.cprimpl.p440loop;

import { String, stringOf } from std.text;

let calls: i32 = 0;

func alt<T>(s: raw T, t: raw T): raw T {
    let i: i32 = calls;
    calls = calls + 1;
    if (i == 0) { return s; }
    return t;
}

func main(): i32 {
    let a: String = stringOf("aa");
    let b: String = stringOf("bb");
    let i: i32 = 0;
    let total: i32 = 0;
    unsafe {
        let p: raw String = &a;
        let q: raw String = &b;
        while (i < 2 && (i32(len(*alt(p, q))) == 2)) {
            i = i + 1;
            total = total + 1;
        }
        return calls * 10 + i * 10 + total - 42;
    }
}
"#;
    let (scratch, emitted) = emit_host(tag, loop_source);
    let body = function_body(tag, &emitted, "cl_spec_cprimpl_p440loop_main(void)\n{");
    assert_contains(
        tag,
        body,
        "while (((i < 2) && ((i32)((cl_string_view(*(cl_alt__String(p, q)))).len) == 2))) {",
    );
    assert_absent(tag, body, "__cl_owned");
    assert_absent(tag, body, "FreeString(__cl_tmp");
    assert_eq!(
        body.matches("cl_alt__String(").count(),
        1,
        "{tag}: the impure pointer must run once per evaluation; body:\n{body}"
    );
    assert_eq!(run_host_c(tag, &scratch, &emitted), 0, "{tag}: the loop must run");

    let tag = "p440borrow";
    let borrow_source = r#"module spec.cprimpl.p440borrow;

import { String, stringOf } from std.text;

struct S { s: String; }

let calls: i32 = 0;

func alt(s: raw S, t: raw S): raw S {
    let i: i32 = calls;
    calls = calls + 1;
    if (i == 0) { return s; }
    return t;
}

func read(p: String): i32 { return i32(len(p)); }

func main(): i32 {
    let a: S = S { s: stringOf("aa") };
    let b: S = S { s: stringOf("bb") };
    unsafe {
        let p: raw S = &a;
        let q: raw S = &b;
        let n: i32 = read((*alt(p, q)).s);
        return n - 2;
    }
}
"#;
    let (scratch, emitted) = emit_host(tag, borrow_source);
    let body = function_body(tag, &emitted, "cl_spec_cprimpl_p440borrow_main(void)\n{");
    assert_contains(
        tag,
        body,
        "n = cl_spec_cprimpl_p440borrow_read((*(cl_spec_cprimpl_p440borrow_alt(p, q))).s);",
    );
    assert_absent(tag, body, "__cl_owned");
    assert_absent(tag, body, "FreeString(__cl_tmp");
    assert_eq!(
        body.matches("cl_spec_cprimpl_p440borrow_alt(").count(),
        1,
        "{tag}: the impure pointer must run once; body:\n{body}"
    );
    assert_eq!(run_host_c(tag, &scratch, &emitted), 0, "{tag}: the read must run");

    let (first, second) = emit_twice("p440loop", loop_source);
    assert_eq!(
        first, second,
        "p440loop: the two emissions differ; first:\n{first}\nsecond:\n{second}"
    );
}

// ---------------------------------------------------------------------------
// guardedmembermoveout: only the taken member is emptied, inside the guard.
//
// The projected member's move-out travels on the deferred call materialization,
// so the zeroing appears in the same guarded comma sequence as the call. The
// sibling member is read after the take and is never emptied at that position.
// ---------------------------------------------------------------------------

/// The guarded line empties `name` and leaves `other` to its scope release.
///
/// `p441member` pins the pointer in one temporary, calls `eat` on the selected
/// member, and zeroes `(*(__cl_tmp0)).name` in the same comma sequence before
/// the result comparison. The sibling read and the untouched `b.name` read stay
/// in the body. The false selection keeps the same guard and returns the call
/// count (0), so the runtime evidence is in the specification suite. The
/// member program re-emits byte-identically.
#[test]
fn should_empty_only_the_taken_member_inside_the_guard() {
    let tag = "p441member";
    let member = r#"module spec.cprimpl.p441member;

import { String, stringOf } from std.text;

let calls: i32 = 0;

func alt<T>(s: raw T, t: raw T): raw T {
    let i: i32 = calls;
    calls = calls + 1;
    if (i == 0) { return s; }
    return t;
}

struct S { name: String; other: String; }

func eat(take x: String): i32 { return 1; }

func main(): i32 {
    let a: S = S { name: stringOf("aa"), other: stringOf("oo") };
    let b: S = S { name: stringOf("bb"), other: stringOf("pp") };
    let g: bool = true;
    unsafe {
        let p: raw S = &a;
        let q: raw S = &b;
        let ok: bool = g && (eat(take (*alt(p, q)).name) == 1);
        if (!ok) { return 99; }
        if (calls != 1) { return 98; }
        if (i32(len(a.other)) != 2) { return 97; }
        return i32(len(b.name)) - 2;
    }
}
"#;
    let (scratch, emitted) = emit_host(tag, member);
    let body = function_body(tag, &emitted, "cl_spec_cprimpl_p441member_main(void)\n{");
    let guarded = body
        .lines()
        .find(|line| line.contains("ok = (g && "))
        .unwrap_or_else(|| panic!("{tag}: the guarded take is missing; body:\n{body}"));
    assert!(
        guarded.contains("cl_spec_cprimpl_p441member_eat((*(__cl_tmp0)).name)"),
        "{tag}: the taken member must read through the pinned pointer; line:\n{guarded}"
    );
    assert!(
        guarded.contains("((*(__cl_tmp0)).name = NULL)"),
        "{tag}: the taken member must empty inside the guard; line:\n{guarded}"
    );
    assert!(
        !guarded.contains("a.other = NULL"),
        "{tag}: the sibling must stay owned; line:\n{guarded}"
    );
    assert_contains(tag, body, "if (((i32)((cl_string_view(a.other)).len) != 2)) {");
    assert_contains(tag, body, "(cl_string_view(b.name)).len");
    assert_eq!(
        body.matches("cl_alt__S(").count(),
        1,
        "{tag}: the impure pointer must run once; body:\n{body}"
    );
    assert_eq!(run_host_c(tag, &scratch, &emitted), 0, "{tag}: the take must run");

    let tag = "p441false";
    let (scratch, emitted) = emit_host(
        tag,
        r#"module spec.cprimpl.p441false;

import { String, stringOf } from std.text;

let calls: i32 = 0;

func alt<T>(s: raw T, t: raw T): raw T {
    let i: i32 = calls;
    calls = calls + 1;
    if (i == 0) { return s; }
    return t;
}

struct S { name: String; other: String; }

func eat(take x: String): i32 { return 1; }

func main(): i32 {
    let a: S = S { name: stringOf("aa"), other: stringOf("oo") };
    let b: S = S { name: stringOf("bb"), other: stringOf("pp") };
    let g: bool = false;
    unsafe {
        let p: raw S = &a;
        let q: raw S = &b;
        let ok: bool = g && (eat(take (*alt(p, q)).name) == 1);
        if (ok) { return 99; }
        return calls;
    }
}
"#,
    );
    let body = function_body(tag, &emitted, "cl_spec_cprimpl_p441false_main(void)\n{");
    assert_contains(tag, body, "g = 0;");
    assert_contains(tag, body, "__cl_ret = cl_spec_cprimpl_p441false_calls;");
    assert_eq!(run_host_c(tag, &scratch, &emitted), 0, "{tag}: the false path must run");

    let (first, second) = emit_twice("p441member", member);
    assert_eq!(
        first, second,
        "p441member: the two emissions differ; first:\n{first}\nsecond:\n{second}"
    );
}

// ---------------------------------------------------------------------------
// takencapture: the generated condition local and the take-only alias flag.
//
// The condition local is deduplicated against source bindings, and only a
// `take` move-out records the alias flag: a plain raw-deref arm move does not
// consume the save. The address comparison is cast to `void *`, so a pointer
// cast that reaches the destination's storage is recognized and an unrelated
// object simply compares unequal.
// ---------------------------------------------------------------------------

/// A source binding named like the generated condition stays distinct.
///
/// `p439condname` declares a local `__cl_cond` and reads it in the arm of an
/// aliasing conditional store. The generated condition local must take a
/// different name, the program must build under the strict flag set, and the
/// arm must still read the source value (exit 0).
#[test]
fn should_keep_a_source_condition_binding_out_of_the_generated_condition() {
    let tag = "p439condname";
    let (scratch, emitted) = emit_host(
        tag,
        r#"module spec.cprimpl.p439condname;

import { String, stringOf } from std.text;

func eat(take x: String): i32 { return 1; }

func main(): i32 {
    let __cl_cond: bool = false;
    let v: String = stringOf("vv");
    let x: String = stringOf("xx");
    let y: String = stringOf("yyyy");
    let g: bool = true;
    unsafe {
        let p: raw String = &v;
        v = (g && (eat(take (unsafe *p)) == 1)) ? (__cl_cond ? x : y) : y;
        if (i32(len(v)) != 4) { return 9; }
        return 0;
    }
}
"#,
    );
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_p439condname_main(void)\n{",
    );
    assert_contains(tag, body, "bool __cl_cond = 0;");
    assert!(
        body.lines()
            .any(|line| line.contains("bool __cl_cond_") && line.contains("= (g && ")),
        "{tag}: the generated condition must take a distinct name; body:\n{body}"
    );
    assert_eq!(
        run_host_c(tag, &scratch, &emitted),
        0,
        "{tag}: the source condition must decide the arm"
    );
}

/// A raw-deref arm source records the save it consumes; an unrelated take
/// does not.
///
/// `p439plainarm` stores `*p` in the then arm and calls a `take` in the else
/// arm. The then arm's source is the destination's own pointee, so it records
/// the save; the else take reads `*q`, which is a different object, so it does
/// not. The program's value loss comes from the pre-existing self-move of `*p`
/// into its own storage and is not pinned here.
#[test]
fn should_record_a_raw_arm_source_and_ignore_an_unrelated_take() {
    let tag = "p439plainarm";
    let (_scratch, emitted) = emit_host(
        tag,
        r#"module spec.cprimpl.p439plainarm;

import { String, stringOf } from std.text;

func takeS(take x: String): String { return x; }

func main(): i32 {
    let v: String = stringOf("vv");
    let w: String = stringOf("ww");
    let g: bool = true;
    unsafe {
        let p: raw String = &v;
        let q: raw String = &w;
        v = g ? (*p) : takeS(take (unsafe *q));
        if (v == null) { return 7; }
        return i32(len(v)) - 2;
    }
}
"#,
    );
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_p439plainarm_main(void)\n{",
    );
    assert_contains(tag, body, "v = *(p);");
    assert_contains(tag, body, "*(p) = NULL;");
    assert!(
        body.contains("((unsigned long)(p))") && body.contains("((unsigned long)(&(v)))"),
        "{tag}: the raw arm source must compare the pointee and destination addresses; body:\n{body}"
    );
    assert_eq!(
        body.matches("cl_spec_cprimpl_p439plainarm_takeS(*(q))").count(),
        1,
        "{tag}: the unrelated take must evaluate once; body:\n{body}"
    );
}

/// Unrelated and cast-alias takes compare through a `void *` address.
///
/// `p439crosstype` takes a struct through the pointer into a `String`
/// destination; the addresses cannot match as objects, so the flag stays clear
/// and the save releases normally. `p439castalias` reaches the same `String`
/// storage through a `raw S` cast, so the same comparison recognizes it. Both
/// strict-build and run 0.
#[test]
fn should_compare_take_addresses_through_a_void_pointer() {
    let tag = "p439crosstype";
    let (scratch, emitted) = emit_host(
        tag,
        r#"module spec.cprimpl.p439crosstype;

import { String, stringOf } from std.text;

struct S { s: String; other: String; }

func eatS(take v: S): i32 { return 1; }

func main(): i32 {
    let v: String = stringOf("vv");
    let x: String = stringOf("xx");
    let y: String = stringOf("yyyy");
    let a: S = S { s: stringOf("aa"), other: stringOf("oo") };
    let g: bool = true;
    unsafe {
        let p: raw S = &a;
        v = (g && (eatS(take (unsafe *p)) == 1)) ? x : y;
        return i32(len(v)) - 2;
    }
}
"#,
    );
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_p439crosstype_main(void)\n{",
    );
    assert!(
        body.contains("((unsigned long)(p))") && body.contains("((unsigned long)(&(v)))"),
        "{tag}: the take range must compare the source and destination addresses; body:\n{body}"
    );
    assert_eq!(run_host_c(tag, &scratch, &emitted), 0, "{tag}: the store must run");

    let tag = "p439castalias";
    let (scratch, emitted) = emit_host(
        tag,
        r#"module spec.cprimpl.p439castalias;

import { String, stringOf } from std.text;

struct S { s: String; }

func eatS(take x: S): i32 { return 1; }

func main(): i32 {
    let v: String = stringOf("vv");
    let x: String = stringOf("xx");
    let y: String = stringOf("yyyy");
    let g: bool = true;
    unsafe {
        let p: raw S = raw S(&v);
        v = (g && (eatS(take *p) == 1)) ? x : y;
        return i32(len(v)) - 2;
    }
}
"#,
    );
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_p439castalias_main(void)\n{",
    );
    assert!(
        body.contains("((unsigned long)(p))") && body.contains("((unsigned long)(&(v)))"),
        "{tag}: the take range must compare the source and destination addresses; body:\n{body}"
    );
    assert_eq!(run_host_c(tag, &scratch, &emitted), 0, "{tag}: the store must run");
}

// ---------------------------------------------------------------------------
// storelowering: the emitted shapes of the phase's store completions.
//
// The deref-rooted target renders the selected member or element through the
// pointer, the target index is pinned before the right-hand side, the
// fixed-array literal fills a temporary before the clear, the self-copy and
// raw self-alias emit no move, and the element/raw-target stores pin the
// owning deref value once.
// ---------------------------------------------------------------------------

/// The deref-rooted store renders the selected projection and pins its index.
///
/// `p421member` stores through `(*(p)).f` and `(*(p)).f.x`; `p421nested` pins
/// the element index into a temporary and stores through
/// `(*(p)).arr[__cl_tmp0].x`; `p421bump` reads the global index once before the
/// call and stores at the pinned element.
#[test]
fn should_emit_the_deref_store_target_projection() {
    let tag = "p421member";
    let (scratch, emitted) = emit_host(
        tag,
        r#"module spec.cprimpl.p421member;

struct Inner { x: i32; }
struct S { f: Inner; }

func main(): i32 {
    let s: S = S { f: Inner { x: 0 } };
    unsafe {
        let p: raw S = &s;
        (*p).f = Inner { x: 7 };
        (*p).f.x = 7;
    }
    return s.f.x - 7;
}
"#,
    );
    let body = function_body(tag, &emitted, "cl_spec_cprimpl_p421member_main(void)\n{");
    assert_contains(tag, body, "*(&((*(p)).f)) = (ClInner){ 7 };");
    assert_contains(tag, body, "*(&((*(p)).f.x)) = 7;");
    assert_eq!(run_host_c(tag, &scratch, &emitted), 0, "{tag}: the store must run");

    let tag = "p421nested";
    let (scratch, emitted) = emit_host(
        tag,
        r#"module spec.cprimpl.p421nested;

struct Inner { x: i32; }
struct S { arr: Inner[2]; }

func main(): i32 {
    let s: S = S { arr: [Inner { x: 1 }, Inner { x: 2 }] };
    let i: i32 = 1;
    unsafe {
        let p: raw S = &s;
        (*p).arr[i].x = 7;
    }
    return s.arr[1].x - 7;
}
"#,
    );
    let body = function_body(tag, &emitted, "cl_spec_cprimpl_p421nested_main(void)\n{");
    assert_contains(tag, body, "__cl_tmp0 = i;");
    assert_contains(tag, body, "*(&((*(p)).arr[__cl_tmp0].x)) = 7;");
    assert_eq!(run_host_c(tag, &scratch, &emitted), 0, "{tag}: the store must run");

    let tag = "p421bump";
    let (scratch, emitted) = emit_host(
        tag,
        r#"module spec.cprimpl.p421bump;

struct Elem { y: i32; }
struct S { arr: Elem[2]; }

let idx: i32 = 1;
let calls: i32 = 0;

func bump(): i32 {
    idx = 5;
    calls = calls + 1;
    return 9;
}

func main(): i32 {
    let s: S = S { arr: [Elem { y: 1 }, Elem { y: 2 }] };
    unsafe {
        let p: raw S = &s;
        (*p).arr[idx].y = bump();
    }
    return s.arr[1].y - 9;
}
"#,
    );
    let body = function_body(tag, &emitted, "cl_spec_cprimpl_p421bump_main(void)\n{");
    assert_contains(
        tag,
        body,
        "__cl_tmp0 = &((*(p)).arr[cl_spec_cprimpl_p421bump_idx]);",
    );
    assert_contains(
        tag,
        body,
        "*(&((*(__cl_tmp0)).y)) = cl_spec_cprimpl_p421bump_bump();",
    );
    assert_eq!(
        body.matches("cl_spec_cprimpl_p421bump_bump();").count(),
        1,
        "{tag}: the value call must run once; body:\n{body}"
    );
    assert_eq!(run_host_c(tag, &scratch, &emitted), 0, "{tag}: the store must run");
}

/// The whole-element store pins its index before the row call.
///
/// `p423row` reads the index into `__cl_tmp0`, guards it, runs `mkrow` once,
/// and copies the call result into the pinned element. The emitted order puts
/// the pin before the call and the store after it.
#[test]
fn should_pin_the_whole_element_store_before_the_value() {
    let tag = "p423row";
    let (scratch, emitted) = emit_host(
        tag,
        r#"module spec.cprimpl.p423row;

let idx: i32 = 1;

func mkrow(): i32[2] {
    idx = 5;
    return [7, 8];
}

func main(): i32 {
    let a: i32[2][2] = [[1, 2], [3, 4]];
    a[idx] = mkrow();
    return a[1][0] - 7;
}
"#,
    );
    let body = function_body(tag, &emitted, "cl_spec_cprimpl_p423row_main(void)\n{");
    assert_contains(tag, body, "__cl_tmp0 = cl_spec_cprimpl_p423row_idx;");
    let pin = body.find("__cl_tmp0 = cl_spec_cprimpl_p423row_idx;").unwrap();
    let call = body.find("cl_spec_cprimpl_p423row_mkrow(").unwrap();
    let store = body
        .find("memcpy(&((*(&(a[__cl_tmp0])))), &(__cl_call_1)")
        .unwrap();
    assert!(
        pin < call && call < store,
        "{tag}: the index, call, and store must run in that order; body:\n{body}"
    );
    assert_eq!(run_host_c(tag, &scratch, &emitted), 0, "{tag}: the store must run");
}

/// A raw self-alias and a projected self-copy emit no move.
///
/// `p424rawlocal` does not emit a store for `x = *(&x)` and still stores the
/// following distinct value; `p424generic` emits no address comparison or move
/// for the identical generic element copy. The fixed-array self-read fills
/// `__cl_rhs_1` before the save and clear.
#[test]
fn should_skip_the_raw_self_alias_and_the_projected_self_copy() {
    let tag = "p424rawlocal";
    let (scratch, emitted) = emit_host(
        tag,
        r#"module spec.cprimpl.p424rawlocal;

import { String, stringOf } from std.text;

func main(): i32 {
    let x: String = stringOf("aa");
    unsafe {
        x = *(&x);
    }
    if (i32(len(x)) != 2) { return 9; }
    x = stringOf("zz");
    return i32(len(x)) - 2;
}
"#,
    );
    let body = function_body(tag, &emitted, "cl_spec_cprimpl_p424rawlocal_main(void)\n{");
    assert_absent(tag, body, "*(&(x))");
    assert_contains(tag, body, "x = cl_std_text_stringOf(cl_str(\"zz\"));");
    assert_eq!(run_host_c(tag, &scratch, &emitted), 0, "{tag}: the value must stay");

    let tag = "p424generic";
    let (scratch, emitted) = emit_host(
        tag,
        r#"module spec.cprimpl.p424generic;

import { String, stringOf } from std.text;

struct W { items: String[2]; }

func same<T>(borrow w: T): void {
    w.items[1] = w.items[1];
}

func main(): i32 {
    let w: W = W { items: [stringOf("aa"), stringOf("bb")] };
    same<W>(borrow w);
    return i32(len(w.items[1])) - 2;
}
"#,
    );
    let body = function_body(tag, &emitted, "cl_same__W(ClW *w)\n{");
    assert_absent(tag, body, "!= &(");
    assert_absent(tag, body, "= (*w).items[1];");
    assert_eq!(run_host_c(tag, &scratch, &emitted), 0, "{tag}: the value must stay");

    let tag = "p424selfread";
    let (scratch, emitted) = emit_host(
        tag,
        r#"module spec.cprimpl.p424selfread;

import { String, stringOf } from std.text;

struct Elem { names: String[2]; }

func main(): i32 {
    let a: Elem[2] = [Elem { names: [stringOf("aa"), stringOf("ab")] }, Elem { names: [stringOf("bb"), stringOf("bc")] }];
    let i: i32 = 1;
    a[i].names = [stringOf(a[i].names[0]), stringOf("xx")];
    return i32(len(a[1].names[0])) - 2;
}
"#,
    );
    let body = function_body(tag, &emitted, "cl_spec_cprimpl_p424selfread_main(void)\n{");
    assert_contains(tag, body, "ClString __cl_rhs_1[2] = {0};");
    assert_contains(tag, body, "__cl_rhs_1[0] = cl_std_text_stringOf(cl_string_view(a[i].names[0]));");
    let save = body.find("ClString __cl_save_1[2] = {0};").unwrap();
    let fill = body.find("__cl_rhs_1[1] = cl_std_text_stringOf(cl_str(\"xx\"));").unwrap();
    assert!(
        fill < save,
        "{tag}: the literal must fill before the save and clear; body:\n{body}"
    );
    assert_eq!(run_host_c(tag, &scratch, &emitted), 0, "{tag}: the store must run");
}

/// The element and raw-target stores call each impure pointer once.
///
/// `p436element` pins the index and the `alt` call, then reads and empties the
/// same pointer temporary. `p436raw` has two `alt` expressions, one per side,
/// and both run once. The element program re-emits byte-identically.
#[test]
fn should_materialize_the_owning_deref_store_value_once() {
    let tag = "p436element";
    let source = r#"module spec.cprimpl.p436element;

import { String, stringOf } from std.text;

let calls: i32 = 0;

func alt<T>(s: raw T, t: raw T): raw T {
    let i: i32 = calls;
    calls = calls + 1;
    if (i == 0) { return s; }
    return t;
}

func main(): i32 {
    let a: String = stringOf("aa");
    let b: String = stringOf("bb");
    let xs: String[2] = [stringOf("AA"), stringOf("BB")];
    let i: i32 = 1;
    unsafe {
        let p: raw String = &a;
        let q: raw String = &b;
        xs[i] = *alt(p, q);
    }
    return i32(len(xs[1])) - 2;
}
"#;
    let (scratch, emitted) = emit_host(tag, source);
    let body = function_body(tag, &emitted, "cl_spec_cprimpl_p436element_main(void)\n{");
    assert_contains(tag, body, "__cl_tmp0 = i;");
    assert_contains(tag, body, "__cl_tmp1 = cl_alt__String(p, q);");
    assert_contains(tag, body, "*(&(xs[__cl_tmp0])) = *(__cl_tmp1);");
    assert_contains(tag, body, "*(__cl_tmp1) = NULL;");
    assert_eq!(
        body.matches("cl_alt__String(").count(),
        1,
        "{tag}: the pointer must run once; body:\n{body}"
    );
    assert_eq!(run_host_c(tag, &scratch, &emitted), 0, "{tag}: the store must run");

    let tag = "p436raw";
    let (scratch, emitted) = emit_host(
        tag,
        r#"module spec.cprimpl.p436raw;

import { String, stringOf } from std.text;

let calls: i32 = 0;

func alt<T>(s: raw T, t: raw T): raw T {
    let i: i32 = calls;
    calls = calls + 1;
    if (i == 0) { return s; }
    return t;
}

func main(): i32 {
    let a: String = stringOf("aa");
    let b: String = stringOf("bb");
    let v: String = stringOf("vv");
    unsafe {
        let p: raw String = &a;
        let q: raw String = &b;
        let rp: raw String = &v;
        let rq: raw String = &v;
        unsafe *alt(rp, rq) = *alt(p, q);
    }
    return i32(len(v)) - 2;
}
"#,
    );
    let body = function_body(tag, &emitted, "cl_spec_cprimpl_p436raw_main(void)\n{");
    assert_contains(tag, body, "__cl_tmp0 = cl_alt__String(rp, rq);");
    assert_contains(tag, body, "__cl_tmp1 = cl_alt__String(p, q);");
    assert_contains(tag, body, "*(&(*(__cl_tmp0))) = *(__cl_tmp1);");
    assert_eq!(
        body.matches("cl_alt__String(").count(),
        2,
        "{tag}: each pointer expression must run once; body:\n{body}"
    );
    assert_eq!(run_host_c(tag, &scratch, &emitted), 0, "{tag}: the store must run");

    let (first, second) = emit_twice("p436element", source);
    assert_eq!(
        first, second,
        "p436element: the two emissions differ; first:\n{first}\nsecond:\n{second}"
    );
}

/// Raw self-copies discard-read the target, aliases guard by address, and the
/// runtime self-copy still emits its bounds guard.
///
/// `p424rawself` and `p424fixedself` emit a discarded read of the target and no
/// destructive store. `p424rawalias` guards the store by the source and target
/// addresses, so an alias is a no-op while a distinct pointer moves. The
/// generic runtime self-copy keeps the index pin and the bounds trap.
#[test]
fn should_keep_the_raw_store_self_copies_and_guards_safe() {
    let tag = "p424rawselfimpl";
    let (scratch, emitted) = emit_host(
        tag,
        r#"module spec.cprimpl.p424rawselfimpl;

import { String, stringOf } from std.text;

struct S { name: String; }

func main(): i32 {
    let s: S = S { name: stringOf("aa") };
    unsafe {
        let p: raw S = &s;
        (*p).name = (*p).name;
    }
    return i32(len(s.name)) - 2;
}
"#,
    );
    let body = function_body(tag, &emitted, "cl_spec_cprimpl_p424rawselfimpl_main(void)\n{");
    assert_contains(tag, body, "(void)((*(p)).name);");
    assert_absent(tag, body, "(*(p)).name = NULL;");
    assert_eq!(run_host_c(tag, &scratch, &emitted), 0, "{tag}: the value must stay");

    let tag = "p424fixedselfimpl";
    let (scratch, emitted) = emit_host(
        tag,
        r#"module spec.cprimpl.p424fixedselfimpl;

import { String, stringOf } from std.text;

struct S { items: String[2]; }

func main(): i32 {
    let s: S = S { items: [stringOf("aa"), stringOf("bb")] };
    unsafe {
        let p: raw S = &s;
        (*p).items = (*p).items;
    }
    return i32(len(s.items[0])) - 2;
}
"#,
    );
    let body = function_body(tag, &emitted, "cl_spec_cprimpl_p424fixedselfimpl_main(void)\n{");
    assert_contains(tag, body, "(void)((*(p)).items);");
    assert_absent(tag, body, "memcpy(&((*(&((*(p)).items))))");
    assert_eq!(run_host_c(tag, &scratch, &emitted), 0, "{tag}: the value must stay");

    let tag = "p424rawaliasimpl";
    let (scratch, emitted) = emit_host(
        tag,
        r#"module spec.cprimpl.p424rawaliasimpl;

import { String, stringOf } from std.text;

struct S { name: String; }

func main(): i32 {
    let s: S = S { name: stringOf("aa") };
    unsafe {
        let p: raw S = &s;
        let q: raw S = &s;
        (*p).name = (*q).name;
    }
    return i32(len(s.name)) - 2;
}
"#,
    );
    let body = function_body(tag, &emitted, "cl_spec_cprimpl_p424rawaliasimpl_main(void)\n{");
    assert_contains(
        tag,
        body,
        "if (&(*(&((*(p)).name))) != &((*(q)).name)) {",
    );
    assert_eq!(run_host_c(tag, &scratch, &emitted), 0, "{tag}: the alias must be a no-op");

    let tag = "p424rawdistinctimpl";
    let (scratch, emitted) = emit_host(
        tag,
        r#"module spec.cprimpl.p424rawdistinctimpl;

import { String, stringOf } from std.text;

struct S { name: String; }

func main(): i32 {
    let a: S = S { name: stringOf("aa") };
    let b: S = S { name: stringOf("bb") };
    unsafe {
        let p: raw S = &a;
        let q: raw S = &b;
        (*p).name = (*q).name;
    }
    return i32(len(a.name)) - 2;
}
"#,
    );
    let body = function_body(tag, &emitted, "cl_spec_cprimpl_p424rawdistinctimpl_main(void)\n{");
    assert_contains(
        tag,
        body,
        "if (&(*(&((*(p)).name))) != &((*(q)).name)) {",
    );
    assert_eq!(run_host_c(tag, &scratch, &emitted), 0, "{tag}: the move must land");

    let tag = "p424genruntimeimpl";
    let (scratch, emitted) = emit_host(
        tag,
        r#"module spec.cprimpl.p424genruntimeimpl;

import { String, stringOf } from std.text;

struct W { items: String[2]; }

func same<T>(borrow w: T, i: i32): i32 {
    w.items[i] = w.items[i];
    return i32(len(w.items[i]));
}

func main(): i32 {
    let w: W = W { items: [stringOf("aa"), stringOf("bb")] };
    return same<W>(borrow w, 1) - 2;
}
"#,
    );
    let body = function_body(tag, &emitted, "cl_same__W(ClW *w, i32 i)\n{");
    assert_contains(tag, body, "__cl_tmp0 = i;");
    assert_contains(tag, body, "cl_trap(\"E11211\", \"index out of range\");");
    assert_absent(tag, body, "= (*w).items[");
    assert_eq!(run_host_c(tag, &scratch, &emitted), 0, "{tag}: the value must stay");

    let tag = "p421rawfieldimpl";
    let (scratch, emitted) = emit_host(
        tag,
        r#"module spec.cprimpl.p421rawfieldimpl;

struct Node { x: i32; next: raw Node; }

func main(): i32 {
    let b: Node = Node { x: 2, next: raw Node(0) };
    let a: Node = Node { x: 1, next: raw Node(0) };
    unsafe {
        a.next = &b;
        let p: raw Node = &a;
        (*p).next.x = 7;
    }
    return b.x - 7;
}
"#,
    );
    let body = function_body(tag, &emitted, "cl_spec_cprimpl_p421rawfieldimpl_main(void)\n{");
    assert_contains(tag, body, "*(&((*(p)).next->x)) = 7;");
    assert_eq!(run_host_c(tag, &scratch, &emitted), 0, "{tag}: the field store must run");
}

/// A fixed-array raw alias and a call-rooted raw alias compare addresses.
///
/// `p424fixedalias` guards the whole-array copy by the source and target
/// addresses, so two pointers to the same storage skip the copy and the clear.
/// `p424callalias` pins each call-rooted pointer once and guards the scalar
/// store by their addresses.
#[test]
fn should_guard_a_raw_fixed_array_and_a_call_rooted_alias() {
    let tag = "p424fixedaliasimpl";
    let (scratch, emitted) = emit_host(
        tag,
        r#"module spec.cprimpl.p424fixedaliasimpl;

import { String, stringOf } from std.text;

struct S { items: String[2]; other: i32; }

func main(): i32 {
    let a: S = S { items: [stringOf("aa"), stringOf("ab")], other: 1 };
    unsafe {
        let p: raw S = &a;
        let q: raw S = &a;
        (*p).items = (*q).items;
    }
    if (a.items[0] == null) { return 9; }
    return i32(len(a.items[0])) - 2;
}
"#,
    );
    let body = function_body(tag, &emitted, "cl_spec_cprimpl_p424fixedaliasimpl_main(void)\n{");
    assert_contains(
        tag,
        body,
        "if (((void *)&((*(&((*(p)).items))))) != ((void *)&((*(q)).items))) {",
    );
    assert_eq!(run_host_c(tag, &scratch, &emitted), 0, "{tag}: the alias must stay intact");

    let tag = "p424arrayaddrofimpl";
    let (scratch, emitted) = emit_host(
        tag,
        r#"module spec.cprimpl.p424arrayaddrofimpl;

import { String, stringOf } from std.text;

struct S { items: String[2]; }

func main(): i32 {
    let a: S = S { items: [stringOf("aa"), stringOf("bb")] };
    unsafe {
        let p: raw S = &a;
        (*p).items = *(&a.items);
    }
    return (i32(len(a.items[0])) - 2) + (i32(len(a.items[1])) - 2);
}
"#,
    );
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_p424arrayaddrofimpl_main(void)\n{",
    );
    assert_contains(tag, body, "__cl_tmp0 = &(a.items);");
    assert_contains(
        tag,
        body,
        "if (((void *)&((*(&((*(p)).items))))) != ((void *)&(*(__cl_tmp0)))) {",
    );
    assert_eq!(run_host_c(tag, &scratch, &emitted), 0, "{tag}: the alias must stay intact");

    let tag = "p424callaliasimpl";
    let (scratch, emitted) = emit_host(
        tag,
        r#"module spec.cprimpl.p424callaliasimpl;

import { String, stringOf } from std.text;

struct S { name: String; }

let calls: i32 = 0;

func alt<T>(s: raw T, t: raw T): raw T {
    let i: i32 = calls;
    calls = calls + 1;
    if (i % 2 == 0) { return s; }
    return t;
}

func main(): i32 {
    let s: S = S { name: stringOf("aa") };
    unsafe {
        let p: raw S = &s;
        (*alt(p, p)).name = (*alt(p, p)).name;
    }
    if (calls != 2) { return 8; }
    return i32(len(s.name)) - 2;
}
"#,
    );
    let body = function_body(tag, &emitted, "cl_spec_cprimpl_p424callaliasimpl_main(void)\n{");
    assert_contains(tag, body, "__cl_tmp0 = cl_alt__S(p, p);");
    assert_contains(tag, body, "__cl_tmp1 = cl_alt__S(p, p);");
    assert_contains(
        tag,
        body,
        "if (&(*(&((*(__cl_tmp0)).name))) != &((*(__cl_tmp1)).name)) {",
    );
    assert_eq!(
        body.matches("cl_alt__S(").count(),
        2,
        "{tag}: each call-rooted pointer must run once; body:\n{body}"
    );
    assert_eq!(run_host_c(tag, &scratch, &emitted), 0, "{tag}: the alias must stay intact");
}

/// The generic compound division emits the same three guards as its twin.
///
/// `p426genericimpl` and `p426concreteimpl` each contain three `E11210` trap
/// calls, and the generic body re-emits byte-identically.
#[test]
fn should_emit_the_generic_compound_division_guards() {
    let source = |module: &str, generic: bool| {
        format!(
            "module spec.cprimpl.{module};\n\n\
             import {{ String, stringOf }} from std.text;\n\n\
             struct H {{ n: i32; items: i32[2]; name: String; }}\n\n\
             func go{g}(borrow h: {param}, d: i32): void {{\n\
             \x20   h.n /= d;\n\
             \x20   h.items[0] %= d;\n\
             \x20   h.items[1] /= d;\n\
             }}\n\n\
             func main(): i32 {{\n\
             \x20   let h: H = H {{ n: 10, items: [11, 12], name: stringOf(\"h\") }};\n\
             \x20   go{call}(borrow h, 2);\n\
             \x20   return h.n + h.items[0] + h.items[1] - 12;\n\
             }}\n",
            module = module,
            g = if generic { "<T>" } else { "" },
            param = if generic { "T" } else { "H" },
            call = if generic { "<H>" } else { "" },
        )
    };
    let generic = source("p426genericimpl", true);
    let concrete = source("p426concreteimpl", false);
    let (scratch, emitted) = emit_host("p426genericimpl", &generic);
    let body = function_body(
        "p426genericimpl",
        &emitted,
        "cl_go__H(ClH *h, i32 d)\n{",
    );
    assert_eq!(
        body.matches("cl_trap(\"E11210\"").count(),
        3,
        "p426genericimpl: expected three division guards; body:\n{body}"
    );
    assert_contains("p426genericimpl", body, "d == 0");
    assert_contains("p426genericimpl", body, "&& d == -1");
    assert_eq!(
        run_host_c("p426genericimpl", &scratch, &emitted),
        0,
        "p426genericimpl: the value must match"
    );

    let (_scratch, emitted) = emit_host("p426concreteimpl", &concrete);
    let body = function_body(
        "p426concreteimpl",
        &emitted,
        "cl_spec_cprimpl_p426concreteimpl_go(ClH *h, i32 d)\n{",
    );
    assert_eq!(
        body.matches("cl_trap(\"E11210\"").count(),
        3,
        "p426concreteimpl: expected three division guards; body:\n{body}"
    );

    let (first, second) = emit_twice("p426genericimpl", &generic);
    assert_eq!(
        first, second,
        "p426genericimpl: the two emissions differ; first:\n{first}\nsecond:\n{second}"
    );
}

// ---------------------------------------------------------------------------
// literalconditional: the conditional literal read base, the ternary take
// literal, and the conditional array literal bind their hidden destinations.
// ---------------------------------------------------------------------------

/// The conditional literal read base stores each arm through its destination.
///
/// `p445both` passes a call-filled `str[2]` destination per arm, copies the row
/// into the arm, and selects the filled arm; `p445mixed` stores the place arm
/// plainly and the literal arm through its destination; `p445nested` stores
/// the field conditional per arm.
#[test]
fn should_bind_the_conditional_literal_destination() {
    let tag = "p445bothimpl";
    let both = r#"module spec.cprimpl.p445bothimpl;

struct Inner { row: str[2]; }

func n1(): str[2] { return ["AA", "BB"]; }
func n2(): str[2] { return ["CC", "DD"]; }

func use(ok: bool): i32 {
    return i32((ok ? Inner { row: n1() } : Inner { row: n2() }).row[0][0]);
}

func main(): i32 {
    return (use(true) - 65) + (use(false) - 67);
}
"#;
    let (scratch, emitted) = emit_host(tag, both);
    let body = function_body(tag, &emitted, "cl_spec_cprimpl_p445bothimpl_use(bool ok)\n{");
    assert_contains(tag, body, "ClStr __cl_call_2[2] = {0};");
    assert_contains(tag, body, "cl_spec_cprimpl_p445bothimpl_n1(__cl_call_2);");
    assert_contains(
        tag,
        body,
        "memcpy(&(__cl_arm_2.row), &(__cl_call_2), sizeof(__cl_arm_2.row));",
    );
    assert_contains(tag, body, "__cl_tmp0 = __cl_arm_2;");
    assert_contains(tag, body, "cl_spec_cprimpl_p445bothimpl_n2(__cl_call_3);");
    assert_eq!(run_host_c(tag, &scratch, &emitted), 0, "{tag}: the read base must run");

    let tag = "p445mixedimpl";
    let (scratch, emitted) = emit_host(
        tag,
        r#"module spec.cprimpl.p445mixedimpl;

struct Inner { row: str[2]; }

func n2(): str[2] { return ["CC", "DD"]; }

func use(ok: bool, inner: Inner): i32 {
    return i32((ok ? inner : Inner { row: n2() }).row[0][0]);
}

func main(): i32 {
    let inner: Inner = Inner { row: ["AA", "BB"] };
    return (use(true, inner) - 65) + (use(false, inner) - 67);
}
"#,
    );
    let body = function_body(tag, &emitted, "cl_spec_cprimpl_p445mixedimpl_use(bool ok, ClInner inner)\n{");
    assert_contains(tag, body, "__cl_tmp0 = inner;");
    assert_contains(tag, body, "cl_spec_cprimpl_p445mixedimpl_n2(__cl_call_1);");
    assert_contains(
        tag,
        body,
        "memcpy(&(__cl_arm_1.row), &(__cl_call_1), sizeof(__cl_arm_1.row));",
    );
    assert_eq!(run_host_c(tag, &scratch, &emitted), 0, "{tag}: the mixed base must run");

    let tag = "p445nestedimpl";
    let (scratch, emitted) = emit_host(
        tag,
        r#"module spec.cprimpl.p445nestedimpl;

struct Inner { row: str[2]; }
struct Outer { inner: Inner; }

func n1(): str[2] { return ["AA", "BB"]; }
func n2(): str[2] { return ["CC", "DD"]; }

func use(ok: bool): i32 {
    let o: Outer = Outer { inner: ok ? Inner { row: n1() } : Inner { row: n2() } };
    return i32(o.inner.row[0][0]);
}

func main(): i32 {
    return (use(true) - 65) + (use(false) - 67);
}
"#,
    );
    let body = function_body(tag, &emitted, "cl_spec_cprimpl_p445nestedimpl_use(bool ok)\n{");
    assert_contains(tag, body, "cl_spec_cprimpl_p445nestedimpl_n1(__cl_call_2);");
    assert_contains(tag, body, "o.inner = __cl_arm_2;");
    assert_eq!(run_host_c(tag, &scratch, &emitted), 0, "{tag}: the nested field must run");

    let (first, second) = emit_twice("p445bothimpl", both);
    assert_eq!(
        first, second,
        "p445bothimpl: the two emissions differ; first:\n{first}\nsecond:\n{second}"
    );
}

/// A take literal in an eager ternary arm stores per arm into the carrier.
///
/// `p446tern` writes the call-filled row directly into the carrier's field and
/// assigns the tag per arm; the call counter shows one call per selected arm.
#[test]
fn should_store_the_ternary_take_literal_per_arm() {
    let tag = "p446ternimpl";
    let (scratch, emitted) = emit_host(
        tag,
        r#"module spec.cprimpl.p446ternimpl;

import { String, stringOf } from std.text;

struct Inner { names: str[2]; }
struct Outer { inner: Inner; tag: String; }

let calls: i32 = 0;

func n1(): str[2] { calls = calls + 1; return ["AA", "BB"]; }
func n2(): str[2] { calls = calls + 1; return ["CC", "DD"]; }

func consumeO(take o: Outer): i32 { return i32(o.inner.names[0][0]); }

func use(g: bool, ok: bool): i32 {
    let x: i32 = g ? consumeO(ok ? Outer { inner: Inner { names: n1() }, tag: stringOf("x") }
                                : Outer { inner: Inner { names: n2() }, tag: stringOf("y") })
                   : 0;
    return x;
}

func main(): i32 {
    let a: i32 = use(true, true);
    let b: i32 = use(false, true);
    let c: i32 = use(true, false);
    return (a - 65) + (b - 0) + (c - 67) + calls - 2;
}
"#,
    );
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_p446ternimpl_use(bool g, bool ok)\n{",
    );
    assert_contains(
        tag,
        body,
        "cl_spec_cprimpl_p446ternimpl_n1(__cl_tmp0.inner.names)",
    );
    assert_contains(
        tag,
        body,
        "cl_spec_cprimpl_p446ternimpl_n2(__cl_tmp0.inner.names)",
    );
    assert_contains(tag, body, "x = (g ? ");
    assert_contains(
        tag,
        body,
        "cl_spec_cprimpl_p446ternimpl_consumeO(__cl_tmp0)",
    );
    assert_eq!(
        body.matches("cl_spec_cprimpl_p446ternimpl_n1(").count(),
        1,
        "{tag}: n1 must run once in the emitted tern; body:\n{body}"
    );
    assert_eq!(
        body.matches("cl_spec_cprimpl_p446ternimpl_n2(").count(),
        1,
        "{tag}: n2 must run once in the emitted tern; body:\n{body}"
    );
    assert_eq!(run_host_c(tag, &scratch, &emitted), 0, "{tag}: the tern must run");
}

/// A conditional array literal of view places stores into a destination array.
///
/// `p448cond` allocates a `ClStr[2]` temporary, stores the chosen arm's two
/// elements, and passes it to the take call.
#[test]
fn should_store_the_conditional_view_array_into_a_destination() {
    let tag = "p448condimpl";
    let source = r#"module spec.cprimpl.p448condimpl;

func consume(take xs: str[2]): i32 { return i32(xs[0][0]) - 65; }

func use(ok: bool): i32 {
    let a: str = "AA";
    let b: str = "BB";
    let c: str = "CC";
    let d: str = "DD";
    return consume(take (ok ? [a, b] : [c, d]));
}

func main(): i32 {
    return (use(true) - 0) + (use(false) - 2);
}
"#;
    let (scratch, emitted) = emit_host(tag, source);
    let body = function_body(tag, &emitted, "cl_spec_cprimpl_p448condimpl_use(bool ok)\n{");
    assert_contains(tag, body, "ClStr __cl_tmp0[2] = {0};");
    assert_contains(tag, body, "__cl_tmp0[0] = a;");
    assert_contains(tag, body, "__cl_tmp0[0] = c;");
    assert_contains(
        tag,
        body,
        "cl_spec_cprimpl_p448condimpl_consume(__cl_tmp0);",
    );
    assert_eq!(run_host_c(tag, &scratch, &emitted), 0, "{tag}: the conditional array must run");

    let (first, second) = emit_twice(tag, source);
    assert_eq!(
        first, second,
        "p448condimpl: the two emissions differ; first:\n{first}\nsecond:\n{second}"
    );
}

/// An owned conditional field saves and moves, and a guarded take fills late.
///
/// `p445ownedfield` saves the field, moves the place arm with `cl_zero` and
/// releases the save. `p446order` stores `inner` before `tag`. `p448guard`
/// fills the conditional array inside the evaluated ternary arm.
#[test]
fn should_keep_the_owned_conditional_field_and_guarded_take_shapes() {
    let tag = "p445ownedfieldimpl";
    let (scratch, emitted) = emit_host(
        tag,
        r#"module spec.cprimpl.p445ownedfieldimpl;

import { String, stringOf } from std.text;

struct Inner { row: str[2]; tag: String; }
struct Outer { inner: Inner; }

func use(ok: bool): i32 {
    let existing: Inner = Inner { row: ["CC", "DD"], tag: stringOf("yy") };
    let o: Outer = Outer { inner: ok ? Inner { row: ["AA", "BB"], tag: stringOf("xx") } : existing };
    return i32(len(o.inner.tag)) - 2;
}

func main(): i32 {
    return use(true) + use(false);
}
"#,
    );
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_p445ownedfieldimpl_use(bool ok)\n{",
    );
    assert_contains(tag, body, "ClInner __cl_save_1 = o.inner;");
    assert_contains(tag, body, "ClInner __cl_arm_1 = {0};");
    assert_contains(tag, body, "o.inner = __cl_arm_1;");
    assert_contains(tag, body, "o.inner = existing;");
    assert_contains(tag, body, "cl_zero(&existing);");
    assert_contains(tag, body, "if (__cl_save_1.tag) { FreeString(__cl_save_1.tag); }");
    assert_eq!(run_host_c(tag, &scratch, &emitted), 0, "{tag}: the field must run");

    let tag = "p446orderimpl";
    let (scratch, emitted) = emit_host(
        tag,
        r#"module spec.cprimpl.p446orderimpl;

import { String, stringOf } from std.text;

struct Inner { names: str[2]; }
struct Outer { tag: String; inner: Inner; }

let order: i32 = 0;
let calls: i32 = 0;

func n1(): str[2] { calls = calls + 1; order = order * 10 + 1; return ["AA", "BB"]; }
func tagOf(): String { order = order * 10 + 2; return stringOf("x"); }

func consumeO(take o: Outer): i32 { return i32(o.inner.names[0][0]); }

func use(g: bool, ok: bool): i32 {
    let x: i32 = g ? consumeO(Outer { inner: Inner { names: n1() }, tag: tagOf() }) : 0;
    return x;
}

func main(): i32 {
    return use(true, true) - 65 + (order - 12) * 10;
}
"#,
    );
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_p446orderimpl_use(bool g, bool ok)\n{",
    );
    let inner = body
        .find("cl_spec_cprimpl_p446orderimpl_n1(__cl_tmp0.inner.names)")
        .unwrap_or_else(|| panic!("{tag}: the inner call is missing; body:\n{body}"));
    let tag_store = body
        .find("__cl_tmp0.tag = cl_spec_cprimpl_p446orderimpl_tagOf(),")
        .unwrap_or_else(|| panic!("{tag}: the tag store is missing; body:\n{body}"));
    assert!(
        inner < tag_store,
        "{tag}: the source order must store inner before tag; body:\n{body}"
    );
    assert_eq!(run_host_c(tag, &scratch, &emitted), 0, "{tag}: the order must match");

    let tag = "p448guardimpl";
    let (scratch, emitted) = emit_host(
        tag,
        r#"module spec.cprimpl.p448guardimpl;

let calls: i32 = 0;

func n1(): str[2] { calls = calls + 1; return ["AA", "BB"]; }
func n2(): str[2] { calls = calls + 1; return ["CC", "DD"]; }

func consume(take xs: str[2]): i32 { return i32(xs[0][0]) - 65; }

func use(g: bool, ok: bool): i32 {
    let x: i32 = g ? consume(take (ok ? n1() : n2())) : 0;
    return x;
}

func main(): i32 {
    let a: i32 = use(true, true);
    let b: i32 = use(false, true);
    let c: i32 = use(true, false);
    return (a - 0) + b + (c - 2) + calls - 2;
}
"#,
    );
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_p448guardimpl_use(bool g, bool ok)\n{",
    );
    assert_contains(tag, body, "x = (g ? (");
    assert_contains(tag, body, "cl_spec_cprimpl_p448guardimpl_n1(__cl_tmp0)");
    assert_contains(tag, body, "cl_spec_cprimpl_p448guardimpl_n2(__cl_tmp0)");
    assert_contains(tag, body, "cl_spec_cprimpl_p448guardimpl_consume(__cl_tmp0)");
    assert_eq!(
        body.matches("cl_spec_cprimpl_p448guardimpl_n1(").count(),
        1,
        "{tag}: n1 must appear once in the evaluated path; body:\n{body}"
    );
    assert_eq!(
        body.matches("cl_spec_cprimpl_p448guardimpl_n2(").count(),
        1,
        "{tag}: n2 must appear once in the evaluated path; body:\n{body}"
    );
    assert_eq!(run_host_c(tag, &scratch, &emitted), 0, "{tag}: the guarded take must run");
}

/// An inline owning read-base literal binds at body scope and releases once.
///
/// `p447impl` reads `.s` from an inline `G` literal; the literal is stored in
/// `__cl_owned0` so the view outlives the read, and the scope release frees the
/// fields exactly once.
#[test]
fn should_bind_the_inline_read_base_literal_at_body_scope() {
    let tag = "p447impl";
    let (scratch, emitted) = emit_host(
        tag,
        r#"module spec.cprimpl.p447impl;

import { String, stringOf } from std.text;

struct G { s: String; tag: String; }

func use(): i32 {
    return i32(len(G { s: stringOf("aa"), tag: stringOf("x") }.s));
}

func main(): i32 {
    return use() - 2;
}
"#,
    );
    let body = function_body(tag, &emitted, "cl_spec_cprimpl_p447impl_use(void)\n{");
    // The declaration must sit at the function body's own scope, not in a
    // nested block: a view derived from the literal outlives the read.
    assert!(
        body.trim_start().starts_with("ClG __cl_owned0 = {0};"),
        "{tag}: the literal must bind at body scope; body:\n{body}"
    );
    assert_contains(
        tag,
        body,
        "__cl_owned0 = (ClG){ cl_std_text_stringOf(cl_str(\"aa\")), cl_std_text_stringOf(cl_str(\"x\")) };",
    );
    assert_contains(tag, body, "__cl_ret = (i32)((cl_string_view(__cl_owned0.s)).len);");
    assert_eq!(run_host_c(tag, &scratch, &emitted), 0, "{tag}: the read must run");
}

// ---------------------------------------------------------------------------
// pureptr: a pure indexed pointer is captured before the take call.
// ---------------------------------------------------------------------------

/// The take captures the indexed pointer in a temporary, then reuses it.
///
/// The callee mutates the global index; the call, the alias test, and the
/// move-out all read the captured `__cl_tmp0`, so the pointee emptied is the
/// one the callee received.
#[test]
fn should_capture_a_pure_index_pointer_before_the_take_call() {
    let tag = "pureptrimpl";
    let (scratch, emitted) = emit_host(
        tag,
        r#"module spec.cprimpl.pureptrimpl;

import { String, stringOf } from std.text;

let idx: i32 = 0;

func eat(take s: String): i32 { idx = 1; return 1; }

func main(): i32 {
    let arr: String[2] = [stringOf("aa"), stringOf("bb")];
    let x: String = stringOf("xx");
    let y: String = stringOf("yyyy");
    let v: String = stringOf("vv");
    let g: bool = true;
    let ps: raw String[2] = [unsafe &arr[0], unsafe &arr[1]];
    v = (g && (eat(take (unsafe *ps[idx])) == 1)) ? x : y;
    return i32(len(v)) - 2;
}
"#,
    );
    let body = function_body(
        tag,
        &emitted,
        "cl_spec_cprimpl_pureptrimpl_main(void)\n{",
    );
    assert_contains(tag, body, "ClString *__cl_tmp0 = NULL;");
    assert_contains(
        tag,
        &emitted,
        "__cl_tmp0 = ps[cl_spec_cprimpl_pureptrimpl_idx])",
    );
    assert_contains(tag, &emitted, "cl_spec_cprimpl_pureptrimpl_eat(*(__cl_tmp0))");
    assert_contains(tag, &emitted, "*(__cl_tmp0) = NULL");
    assert_eq!(run_host_c(tag, &scratch, &emitted), 0, "{tag}: the call must run");
}
