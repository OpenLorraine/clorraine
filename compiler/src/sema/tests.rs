//! In-module tests for the semantic checker.
//!
//! The scoped checker keeps its `Checker` type private, so these tests drive
//! the public `check` entry point over parsed and loaded source. They pin the
//! edge cases the command-line golden and stage suites do not own: the full
//! operator matrix, place and `void` rules, aggregate and index behavior, and
//! declaration scoping.

use super::*;
use crate::diag::Diagnostics;
use crate::load;
use crate::parse::Parser;
use crate::span::SourceMap;

/// Parses and loads the sources, then type-checks them, returning `(code,
/// message)` for every diagnostic the semantic stage reports.
///
/// The parse and load diagnostics are deliberately discarded: these cases pin
/// the semantic stage, and the driver never runs it after an earlier error.
pub(super) fn check_sources(files: &[&str]) -> Vec<(&'static str, String)> {
    let mut sources = SourceMap::new();
    let mut parse_diagnostics = Diagnostics::new(false, None);
    let mut programs = Vec::new();
    for (index, text) in files.iter().enumerate() {
        let id = sources.add_file(format!("file{index}.clo"), (*text).to_owned());
        let program = Parser::new(&sources, id, &mut parse_diagnostics).parse_program();
        programs.push((id, program));
    }
    let loaded = load::load(programs, &mut parse_diagnostics);
    let mut diagnostics = Diagnostics::new(false, None);
    check(&loaded, Profile::Host, &mut diagnostics);
    diagnostics
        .items()
        .iter()
        .map(|diagnostic| (diagnostic.code, diagnostic.message.clone()))
        .collect()
}

/// Codes of a diagnostic list.
pub(super) fn codes(diagnostics: &[(&'static str, String)]) -> Vec<&'static str> {
    diagnostics.iter().map(|(code, _)| *code).collect()
}

/// Asserts that the semantic stage accepts a snippet with no diagnostics.
pub(super) fn assert_accepts(source: &str) {
    let diagnostics = check_sources(&[source]);
    assert!(diagnostics.is_empty(), "unexpected diagnostics: {diagnostics:?}");
}

/// Asserts that the semantic stage reports exactly the given codes.
pub(super) fn assert_codes(source: &str, expected: &[&str]) {
    assert_eq!(codes(&check_sources(&[source])), expected);
}

#[test]
fn equality_is_defined_for_scalars_enums_raw_handles_and_cstrings() {
    assert_accepts(
        r#"module test;
enum E : u8 { A, B }
handle File freedBy Close;
func demo(e: E, a: raw u8, c: cstring, f: File, b: bool): bool {
    return e == E.B && a == a && c == c && f == f && b == b;
}
func main(): i32 { return 0; }
"#,
    );
}

/// Two `str` or `static str` views have no defined equality.
///
/// The frozen operator table covers scalars, enums, `raw`, handles, and
/// `cstring`; a view is outside it, so the pair reports `E10222`. The null
/// pairing is the view's emptiness test and stays accepted.
#[test]
fn views_have_no_equality_but_pair_with_null() {
    assert_codes(
        r#"module test;
func demo(a: str, b: static str): bool {
    return a == b;
}
func main(): i32 { return 0; }
"#,
        &["E10222"],
    );
    assert_accepts(
        r#"module test;
func demo(a: str): bool {
    return a == null && null != a;
}
func main(): i32 { return 0; }
"#,
    );
}

#[test]
fn structs_and_arrays_have_no_equality() {
    assert_codes(
        r#"module test;
struct Point { x: i32; y: i32; }
func demo(p: Point, values: u8[2]): bool {
    let same_point = p == p;
    let same_values = values == values;
    return same_point;
}
func main(): i32 { return 0; }
"#,
        &["E10222", "E10222"],
    );
}

#[test]
fn equality_between_different_numeric_types_is_allowed() {
    assert_accepts(
        r#"module test;
func demo(a: i32, b: u32): bool {
    return a == b;
}
func main(): i32 { return 0; }
"#,
    );
}

#[test]
fn boolean_values_have_no_ordering() {
    assert_codes(
        r#"module test;
func demo(a: bool, b: bool): bool {
    return a < b;
}
func main(): i32 { return 0; }
"#,
        &["E10222"],
    );
}

#[test]
fn ordered_comparison_unifies_its_numeric_operands() {
    assert_accepts(
        r#"module test;
func demo(a: u8, b: u16): bool {
    return a < b;
}
func main(): i32 { return 0; }
"#,
    );
    assert_codes(
        r#"module test;
func demo(a: i32, b: u32): bool {
    return a < b;
}
func main(): i32 { return 0; }
"#,
        &["E10200"],
    );
}

#[test]
fn integer_only_operators_reject_float_operands() {
    assert_codes(
        r#"module test;
func demo(a: f64, b: i32): i32 {
    let remainder = a % a;
    let bits = a & a;
    let shifted = a << b;
    let inverted = ~a;
    return 0;
}
func main(): i32 { return 0; }
"#,
        &["E10222", "E10222", "E10222", "E10222"],
    );
}

#[test]
fn unary_operators_require_the_right_operand_family() {
    assert_codes(
        r#"module test;
func demo(a: i32, b: bool): bool {
    let not_int = !a;
    let negated_bool = -b;
    return not_int;
}
func main(): i32 { return 0; }
"#,
        &["E10218", "E10222"],
    );
}

#[test]
fn address_of_and_deref_round_trip_through_a_raw_pointer() {
    assert_accepts(
        r#"module test;
func demo(x: i32): i32 {
    let p: raw i32 = &x;
    let y: i32 = *p;
    return y;
}
func main(): i32 { return 0; }
"#,
    );
    assert_codes(
        r#"module test;
func demo(x: i32): i32 {
    let bad: i32 = &x;
    return bad;
}
func main(): i32 { return 0; }
"#,
        &["E10200"],
    );
}

#[test]
fn assignment_requires_a_place_and_yields_void() {
    assert_codes(
        r#"module test;
func demo(): i32 {
    let x: i32 = 0;
    1 = x;
    return x;
}
func main(): i32 { return 0; }
"#,
        &["E10221"],
    );
    assert_codes(
        r#"module test;
func demo(): i32 {
    let x: i32 = 0;
    let y = (x = 1);
    return y;
}
func main(): i32 { return 0; }
"#,
        &["E10217"],
    );
}

#[test]
fn assignments_accept_places_and_reject_temporaries() {
    assert_accepts(
        r#"module test;
struct Point { x: i32; y: i32; }
func demo(points: Point[2], values: u8[2], ptr: raw i32): void {
    points[0].x = 1;
    values[0] = 2;
    *ptr = 3;
}
func main(): i32 { return 0; }
"#,
    );
}

#[test]
fn compound_assignment_applies_the_binary_operator_rules() {
    assert_accepts(
        r#"module test;
func demo(): u16 {
    let x: u16 = 1;
    x += 2;
    x *= 3;
    x %= 4;
    x &= 5;
    x <<= 1;
    return x;
}
func main(): i32 { return 0; }
"#,
    );
    assert_codes(
        r#"module test;
func demo(): void {
    let f: f64 = 1.0;
    f %= 2.0;
}
func main(): i32 { return 0; }
"#,
        &["E10222"],
    );
}

#[test]
fn conditional_unifies_its_branches() {
    assert_accepts(
        r#"module test;
func pick(flag: bool, a: u8, b: u16): u16 {
    return flag ? a : b;
}
func main(): i32 { return 0; }
"#,
    );
    assert_codes(
        r#"module test;
func pick(flag: bool, a: u8, b: u16): u8 {
    return flag ? a : b;
}
func main(): i32 { return 0; }
"#,
        &["E10200"],
    );
    assert_codes(
        r#"module test;
struct Point { x: i32; y: i32; }
func pick(flag: bool, p: Point): i32 {
    return flag ? 1 : p;
}
func main(): i32 { return 0; }
"#,
        &["E10200"],
    );
}

#[test]
fn struct_literals_check_fields_overlays_and_zero_values() {
    assert_accepts(
        r#"module test;
struct Point { x: i32; y: i32; }
func demo(): i32 {
    let zero = Point {};
    let full = Point { x: 1, y: 2 };
    return full.x;
}
func main(): i32 { return 0; }
"#,
    );
    assert_codes(
        r#"module test;
struct Point { x: i32; y: i32; }
struct Reg { ctrl: volatile u16; }
func demo(): i32 {
    let short = Point { x: 1 };
    let regs = Reg { ctrl: 0 };
    return short.x;
}
func main(): i32 { return 0; }
"#,
        &["E10203", "E10207"],
    );
}

#[test]
fn an_unknown_struct_member_reports() {
    assert_codes(
        r#"module test;
struct Point { x: i32; y: i32; }
func make(): Point { return Point { x: 1, y: 2 }; }
func demo(): i32 {
    return make().z;
}
func main(): i32 { return 0; }
"#,
        &["E10212"],
    );
}

#[test]
fn a_value_member_chain_is_a_field_access() {
    assert_accepts(
        r#"module test;
struct Point { x: i32; y: i32; }
func demo(point: Point): i32 {
    point.x = 2;
    return point.x + point.y;
}
func main(): i32 { return 0; }
"#,
    );
    assert_codes(
        r#"module test;
struct Point { x: i32; y: i32; }
func demo(point: Point): i32 {
    point.x = 2;
    return point.z;
}
func main(): i32 { return 0; }
"#,
        &["E10212"],
    );
}

#[test]
fn a_field_takes_precedence_over_a_free_function_of_the_same_name() {
    assert_accepts(
        r#"module test;
struct Point { x: i32; tally: i32; }
func tally(n: i32): i32 { return n; }
func make(): Point { return Point { x: 1, tally: 2 }; }
func demo(): i32 {
    return make().tally;
}
func main(): i32 { return 0; }
"#,
    );
    assert_accepts(
        r#"module test;
struct Bag { count: i32; }
func tally(n: i32): i32 { return n; }
func make(): Bag { return Bag { count: 1 }; }
func demo(): i32 {
    return make().tally;
}
func main(): i32 { return 0; }
"#,
    );
}

#[test]
fn generic_struct_uses_require_their_type_arguments() {
    assert_codes(
        r#"module test;
struct Box<T> { item: T; }
func demo(value: Box<i32, u8>): void {}
func main(): i32 { return 0; }
"#,
        &["E10300"],
    );
    assert_codes(
        r#"module test;
struct Box<T> { item: T; }
func demo(): void {
    let boxed = Box {};
}
func main(): i32 { return 0; }
"#,
        &["E10301"],
    );
}

#[test]
fn indexing_returns_the_element_type_and_checks_constant_bounds() {
    assert_accepts(
        r#"module test;
func demo(text: str, bytes: raw u8, grid: u8[2][3]): u8 {
    let first: u8 = text[0];
    let element: u8 = bytes[0];
    let next: raw u8 = bytes + 1;
    let back: raw u8 = bytes - 1;
    let row: u8[3] = grid[0];
    return first;
}
func main(): i32 { return 0; }
"#,
    );
    assert_codes(
        r#"module test;
func demo(): u8 {
    let values: u8[3] = [1, 2, 3];
    return values[-1];
}
func main(): i32 { return 0; }
"#,
        &["E10216"],
    );
}

#[test]
fn intrinsics_check_their_arguments_and_return_usize() {
    assert_accepts(
        r#"module test;
struct Point { x: i32; y: i32; }
func demo(): usize {
    let size: usize = sizeof(i32);
    let offset: usize = offsetof(Point, y);
    let count: usize = len([1, 2, 3]);
    assert(count > 0);
    return size + offset + count;
}
func main(): i32 { return 0; }
"#,
    );
    assert_codes(
        r#"module test;
func demo(): void {
    assert(1);
}
func main(): i32 { return 0; }
"#,
        &["E10218"],
    );
}

#[test]
fn templates_accept_formattable_placeholders_and_yield_strings() {
    assert_accepts(
        r#"module test;
func demo(count: u16, ratio: f64, label: str, flag: bool): void {
    let a: String = $"count={count}";
    let b: String = $"ratio={ratio} flag={flag}";
    let c: String = $"label={label}";
    let d: str = $"plain";
}
func main(): i32 { return 0; }
"#,
    );
}

#[test]
fn a_local_binding_shadows_a_module_function() {
    assert_accepts(
        r#"module test;
func helper(): i32 { return 1; }
func demo(): i32 {
    let helper: i32 = 5;
    return helper;
}
func main(): i32 { return 0; }
"#,
    );
}

#[test]
fn a_block_local_does_not_escape_its_block() {
    assert_accepts(
        r#"module test;
func helper(): i32 { return 1; }
func demo(): i32 {
    {
        let helper: i32 = 5;
    }
    return helper();
}
func main(): i32 { return 0; }
"#,
    );
}

#[test]
fn a_void_call_cannot_be_used_as_a_value() {
    assert_codes(
        r#"module test;
func produce(): void {}
func demo(): i32 {
    let x = produce();
    return 0;
}
func main(): i32 { return 0; }
"#,
        &["E10217"],
    );
}

#[test]
fn function_values_require_an_exact_signature() {
    assert_accepts(
        r#"module test;
func addOne(x: i32): i32 { return x + 1; }
func demo(): i32 {
    let f: func(i32): i32 = addOne;
    return f(1);
}
func main(): i32 { return 0; }
"#,
    );
    assert_codes(
        r#"module test;
func addOne(x: i32): i32 { return x + 1; }
func demo(): i32 {
    let f: func(i32): u8 = addOne;
    return 0;
}
func main(): i32 { return 0; }
"#,
        &["E10210"],
    );
}

/// A growable `Array<T>` accepts a move-only element.
///
/// Version 1 generates the element drop glue, so the former staged `E90000`
/// no longer fires for `Array<Buffer>` or a struct that owns a `String`. The
/// case keeps the duplicable, nested, and unresolved spellings accepted too.
#[test]
fn a_move_only_array_element_is_accepted() {
    assert_accepts(
        r#"module test;
handle Buffer freedBy FreeBuffer;
ffi lib { func FreeBuffer(take b: Buffer): void; }
struct Owned { name: String; }
func takesBuffer(xs: Array<Buffer>): void {}
func takesOwned(xs: Array<Owned>): void {}
func takesInt(xs: Array<i32>): void {}
func takesNested(xs: Array<Array<i32>>): void {}
func takesUnknown(xs: Array<Undeclared>): void {}
func main(): i32 { return 0; }
"#,
    );
}
