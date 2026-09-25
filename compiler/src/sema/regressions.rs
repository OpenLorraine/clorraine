//! Regression tests for the semantic checker.
//!
//! These pin behavior the correctness review called out: array extents that
//! fold constant names and arithmetic, nested array literals, dot-call
//! checking, the separation of integer literals from `bool`, overlay values,
//! the single-check `assert`, the compound-assignment result, and the
//! non-constant name in a constant context.

use super::tests::{assert_accepts, assert_codes};

#[test]
fn array_extents_fold_constant_names_and_arithmetic() {
    assert_accepts(
        r#"module test;
const WIDTH: usize = 8;
func demo(): usize {
    let row: u8[WIDTH] = [; 0];
    return len(row);
}
func main(): i32 { return 0; }
"#,
    );
    assert_accepts(
        r#"module test;
func demo(): usize {
    let pair: u8[1 + 1] = [; 0];
    return len(pair);
}
func main(): i32 { return 0; }
"#,
    );
}

#[test]
fn nested_array_literals_use_the_remaining_extents() {
    assert_accepts(
        r#"module test;
func demo(): u8 {
    let d: u8[3][2] = [[1, 2], [3, 4], [5, 6]];
    return d[0][0];
}
func main(): i32 { return 0; }
"#,
    );
}

#[test]
fn dot_calls_check_the_receiver_and_the_arguments() {
    // A value-headed dotted callee reaches the dot-call rules.
    assert_codes(
        r#"module test;
struct Point { x: i32; y: i32; }
func moveBy(borrow p: Point, dx: i32): void { p.x = p.x + dx; }
func demo(p: Point): void { p.moveBy("x"); }
func main(): i32 { return 0; }
"#,
        &["E10200"],
    );
    // A call-shaped receiver takes the same path.
    assert_codes(
        r#"module test;
struct Point { x: i32; y: i32; }
func moveBy(borrow p: Point, dx: i32): void { p.x = p.x + dx; }
func make(): Point { return Point { x: 1, y: 2 }; }
func demo(): void { make().moveBy("x"); }
func main(): i32 { return 0; }
"#,
        &["E10200"],
    );
    // A void dot-call used as a value is the assignment-result error.
    assert_codes(
        r#"module test;
struct Point { x: i32; y: i32; }
func bump(borrow p: Point): void { p.x = p.x + 1; }
func demo(p: Point): i32 { let r = p.bump(); return 0; }
func main(): i32 { return 0; }
"#,
        &["E10217"],
    );
    // `xs.len()` speaks for the `len` intrinsic, which yields `usize`.
    assert_codes(
        r#"module test;
func demo(xs: u8[]): u8 { let n: u8 = xs.len(); return n; }
func main(): i32 { return 0; }
"#,
        &["E10200"],
    );
    // A free function whose first parameter is the receiver still matches.
    assert_accepts(
        r#"module test;
struct Point { x: i32; y: i32; }
func moveBy(borrow p: Point, dx: i32): void { p.x = p.x + dx; }
func demo(p: Point): void { p.moveBy(3); }
func main(): i32 { return 0; }
"#,
    );
}

#[test]
fn integer_literals_do_not_adapt_to_a_bool_context() {
    assert_codes(
        r#"module test;
func demo(): bool { let b: bool = 1; return b; }
func main(): i32 { return 0; }
"#,
        &["E10200"],
    );
    assert_codes(
        r#"module test;
func sink(b: bool): void {}
func demo(): void { sink(0); }
func main(): i32 { return 0; }
"#,
        &["E10200"],
    );
}

#[test]
fn overlay_types_are_rejected_by_value_but_allowed_through_raw() {
    assert_codes(
        r#"module test;
struct CustomRegs { ctrl: volatile u16; }
func demo(): void { let r: CustomRegs; }
func main(): i32 { return 0; }
"#,
        &["E10207"],
    );
    assert_codes(
        r#"module test;
struct CustomRegs { ctrl: volatile u16; }
func demo(r: CustomRegs): void {}
func main(): i32 { return 0; }
"#,
        &["E10207"],
    );
    // The initializer is an opaque imported call, so the global's declared
    // overlay type is the only cause.
    assert_codes(
        r#"module test;
struct CustomRegs { ctrl: volatile u16; }
import { makeRegs } from std.mem;
let g: CustomRegs = makeRegs();
func main(): i32 { return 0; }
"#,
        &["E10207"],
    );
    assert_accepts(
        r#"module test;
struct CustomRegs { ctrl: volatile u16; }
func demo(): void { let regs: raw CustomRegs = raw CustomRegs(0); }
func main(): i32 { return 0; }
"#,
    );
}

#[test]
fn constant_operations_fold_at_their_own_type() {
    // The `i16` addition wraps to a negative value, so the extent is E10226
    // even though the constant's declared type is the wider `i32`.
    assert_codes(
        r#"module test;
const A: i32 = i16(30000) + i16(30000);
func demo(): void { let x: u8[A]; }
func main(): i32 { return 0; }
"#,
        &["E10226"],
    );
    // The `i16` operands make the division fault at `i16`.
    assert_codes(
        r#"module test;
const A: i32 = i16(-32768) / i16(-1);
func main(): i32 { return 0; }
"#,
        &["E10223"],
    );
    // The shift count masks to the 16-bit left operand, so `A` is `16`: the
    // extent `A - 16` is zero.
    assert_codes(
        r#"module test;
const A: u32 = u16(1) << 20;
func demo(): void { let x: u8[A - 16]; }
func main(): i32 { return 0; }
"#,
        &["E10226"],
    );
    // An enum member folds its explicit value at the operation's type.
    assert_codes(
        r#"module test;
enum E : i32 { A = i16(30000) + i16(30000) }
func demo(): void { let x: u8[E.A]; }
func main(): i32 { return 0; }
"#,
        &["E10226"],
    );
}

#[test]
fn struct_field_extents_fold_constant_names() {
    // The field extent is the constant name `W`, so the struct has a known
    // size and `sizeof(S)` folds instead of reporting E10224.
    assert_accepts(
        r#"module test;
const W: usize = 4;
struct S { a: u8[W]; }
const N: usize = sizeof(S);
func main(): i32 { return 0; }
"#,
    );
    // The extent also reaches a global declared with the constant.
    assert_accepts(
        r#"module test;
const W: usize = 4;
let buffer: u8[W] = [; 0];
func main(): i32 { return 0; }
"#,
    );
}

#[test]
fn overlay_values_are_rejected_through_arrays_and_fields() {
    assert_codes(
        r#"module test;
struct CustomRegs { ctrl: volatile u16; }
func demo(): void { let r: CustomRegs[4]; }
func main(): i32 { return 0; }
"#,
        &["E10207"],
    );
    assert_codes(
        r#"module test;
struct Inner { v: volatile u16; }
struct Outer { i: Inner; }
func demo(o: Outer): void {}
func main(): i32 { return 0; }
"#,
        &["E10207"],
    );
    assert_accepts(
        r#"module test;
struct CustomRegs { ctrl: volatile u16; }
func demo(): void { let regs: raw CustomRegs = raw CustomRegs(0); }
func main(): i32 { return 0; }
"#,
    );
}

#[test]
fn assert_checks_its_condition_once() {
    assert_codes(
        r#"module test;
func demo(): void { assert(1 && true); }
func main(): i32 { return 0; }
"#,
        &["E10218"],
    );
}

#[test]
fn compound_assignment_converts_the_result_back_to_the_target() {
    assert_codes(
        r#"module test;
func demo(): void { let x: u8 = 1; let y: u16 = 2; x += y; }
func main(): i32 { return 0; }
"#,
        &["E10200"],
    );
    assert_accepts(
        r#"module test;
func demo(): u16 { let x: u16 = 1; x += 2; x <<= 1; return x; }
func main(): i32 { return 0; }
"#,
    );
}

#[test]
fn unclassified_names_are_not_constant_in_extents_and_cases() {
    assert_codes(
        r#"module test;
func demo(x: i32): void {
    let y: i32 = 1;
    switch (x) { case y: break; default: break; }
}
func main(): i32 { return 0; }
"#,
        &["E10224"],
    );
    // A runtime extent reports the constant fault alone, not a zero-extent or
    // fill-extent error.
    assert_codes(
        r#"module test;
func demo(): usize { let n: usize = 3; let a: u8[n] = [; 0]; return len(a); }
func main(): i32 { return 0; }
"#,
        &["E10224"],
    );
}

#[test]
fn a_bare_return_is_allowed_only_in_a_void_function() {
    // `return;` in a value-returning function returns nothing where the return
    // type requires a value, which is the implicit-conversion error.
    assert_codes(
        r#"module test;
func demo(): i32 { return; }
func main(): i32 { return 0; }
"#,
        &["E10200"],
    );
    // The same bare return is the normal exit of a `void` function.
    assert_accepts(
        r#"module test;
func demo(): void { return; }
func main(): i32 { demo(); return 0; }
"#,
    );
}

#[test]
fn an_increment_operand_must_be_a_place() {
    // `x++` is `x = x + 1`, so a call result cannot be incremented.
    assert_codes(
        r#"module test;
func make(): i32 { return 1; }
func demo(): void { make()++; }
func main(): i32 { return 0; }
"#,
        &["E10221"],
    );
    assert_accepts(
        r#"module test;
func demo(): i32 { let x: i32 = 0; x++; return x; }
func main(): i32 { return 0; }
"#,
    );
}

#[test]
fn a_generic_struct_field_yields_its_substituted_type() {
    // Reading a field of an instantiated generic struct uses that instance's
    // type argument, so a callable-bound check sees a concrete `i32` and the
    // concrete function satisfies `F: func(T): T`.
    assert_accepts(
        r#"module test;
struct Pair<T> { first: T; second: T; }
func addOne(x: i32): i32 { return x + 1; }
func apply<T, F: func(T): T>(x: T, f: F): T { return f(x); }
func main(): i32 {
    let p = Pair<i32> { first: 5, second: 2 };
    return apply(p.first, addOne);
}
"#,
    );
    // The field keeps the instantiated type: `Pair<u8>.first` is `u8`, not the
    // bare parameter `T`, so passing it where an `i32` is required is rejected
    // instead of silently accepted.
    assert_codes(
        r#"module test;
struct Pair<T> { first: T; second: T; }
func takesI32(x: i32): i32 { return x; }
func main(): i32 {
    let p = Pair<u8> { first: 5, second: 2 };
    return takesI32(p.first);
}
"#,
        &["E10200"],
    );
    // A generic struct literal checks each field against the substituted type
    // as well, so a wrong-typed field is reported.
    assert_codes(
        r#"module test;
struct Pair<T> { first: T; second: T; }
func main(): i32 {
    let p = Pair<i32> { first: "x", second: 2 };
    return p.second;
}
"#,
        &["E10200"],
    );
}
