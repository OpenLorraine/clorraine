//! Regression tests for constant folding at each expression's own type.
//!
//! These pin the review fixes: `+ - * ~` wrap at the operation's result type,
//! shift counts mask to the left operand's width, the signed minimum over `-1`
//! faults there, a module `const` folds at its declared width, an imported
//! `const` and an enum member fold, a field access does not, and a constant
//! extent folds inside `sizeof` and in a struct field.

use super::tests::{cast, dotted, fold, int, name, op, span, unary, with_folder, with_loaded, BASELINE};
use super::*;
use crate::ast::{BinaryOp, ScalarType, UnaryOp};

#[test]
fn an_operation_wraps_at_its_own_type_not_the_context() {
    // The two `i16` operands make the addition an `i16` operation, so it wraps
    // even though the constant's declared type is the wider `i32`.
    let source = "module test;
const A: i32 = i16(30000) + i16(30000);
const B: u32 = u16(1) << 20;
const C: i32 = i16(32767) + 1;
const D: u8 = u8(255) + u8(1);
func main(): i32 { return 0; }
";
    with_folder(source, |folder| {
        assert_eq!(folder.integer(&name("A")), Some(-5536));
        // The count masks to the 16-bit left operand, so `1 << (20 & 15)`.
        assert_eq!(folder.integer(&name("B")), Some(16));
        // The literal `1` adopts the `i16` type of its sibling operand.
        assert_eq!(folder.integer(&name("C")), Some(-32768));
        // A same-width operation still wraps as before.
        assert_eq!(folder.integer(&name("D")), Some(0));
    });
}

#[test]
fn a_const_reference_carries_the_declared_type() {
    // `A` is written `i32`, so a later operation uses the declared `i32`
    // width rather than the narrower `i16` of its initializer.
    let source = "module test;
const A: i32 = i16(1);
const B: i32 = A << 20;
func main(): i32 { return 0; }
";
    with_folder(source, |folder| {
        assert_eq!(folder.integer(&name("B")), Some(1_048_576));
    });
}

#[test]
fn the_minimum_over_minus_one_faults_at_the_operation_type() {
    // The operands are `i16`, so the division overflows `i16` even though the
    // constant's declared context is the wider `i32`.
    let quotient = op(
        BinaryOp::Div,
        cast(ScalarType::I16, unary(UnaryOp::Neg, int(32768))),
        cast(ScalarType::I16, unary(UnaryOp::Neg, int(1))),
    );
    let codes = with_folder(BASELINE, |folder| {
        let mut diagnostics = Diagnostics::new(false, None);
        folder.require_at(&quotient, Some(ScalarType::I32), &mut diagnostics);
        diagnostics
            .items()
            .iter()
            .map(|item| item.code)
            .collect::<Vec<_>>()
    });
    assert_eq!(codes, ["E10223"]);
}

#[test]
fn an_enum_member_keeps_its_operation_type() {
    let source = "module test;
enum E : i32 { A = i16(30000) + i16(30000) }
func main(): i32 { return 0; }
";
    with_folder(source, |folder| {
        assert_eq!(folder.integer(&dotted(&["E", "A"])), Some(-5536));
    });
}

#[test]
fn a_struct_field_extent_folds_a_constant_name() {
    let source = "module test;
const W: usize = 4;
struct S { a: u8[W]; b: u16; }
const SIZE: usize = sizeof(S);
const OFFSET: usize = offsetof(S, b);
func main(): i32 { return 0; }
";
    with_folder(source, |folder| {
        assert_eq!(folder.integer(&name("SIZE")), Some(6));
        assert_eq!(folder.integer(&name("OFFSET")), Some(4));
    });
}

#[test]
fn integer_operations_wrap_at_the_context_width() {
    // `255 + 1` in `u8` wraps to zero instead of overflowing the accumulator.
    assert_eq!(
        fold_integer(BinaryOp::Add, 255, 1, span(), Some(ScalarType::U8)),
        Ok(ConstValue::Int(0))
    );
    // `127 + 1` in `i8` wraps to the most negative value.
    assert_eq!(
        fold_integer(BinaryOp::Add, 127, 1, span(), Some(ScalarType::I8)),
        Ok(ConstValue::Int(-128))
    );
    // A multiply that overflows the context width wraps there too.
    assert_eq!(
        fold_integer(BinaryOp::Mul, 200, 2, span(), Some(ScalarType::U8)),
        Ok(ConstValue::Int(144))
    );
}

#[test]
fn shift_counts_mask_to_the_context_width() {
    assert_eq!(
        fold_integer(BinaryOp::Shl, 1, 32, span(), Some(ScalarType::U32)),
        Ok(ConstValue::Int(1))
    );
    assert_eq!(
        fold_integer(BinaryOp::Shl, 1, 8, span(), Some(ScalarType::U8)),
        Ok(ConstValue::Int(1))
    );
    assert_eq!(
        fold_integer(BinaryOp::Shl, 1, 7, span(), Some(ScalarType::U8)),
        Ok(ConstValue::Int(128))
    );
    // A `u8` value shifted right fills with zero.
    assert_eq!(
        fold_integer(BinaryOp::Shr, 128, 1, span(), Some(ScalarType::U8)),
        Ok(ConstValue::Int(64))
    );
}

#[test]
fn the_minimum_signed_value_over_minus_one_faults_at_the_context_width() {
    // `i16(-32768) / i16(-1)` overflows `i16`, which is a constant fault even
    // though the wide accumulator could represent the quotient.
    let quotient = op(
        BinaryOp::Div,
        cast(ScalarType::I16, unary(UnaryOp::Neg, int(32768))),
        cast(ScalarType::I16, unary(UnaryOp::Neg, int(1))),
    );
    let codes = with_folder(BASELINE, |folder| {
        let mut diagnostics = Diagnostics::new(false, None);
        folder.require_at(&quotient, Some(ScalarType::I16), &mut diagnostics);
        diagnostics
            .items()
            .iter()
            .map(|item| item.code)
            .collect::<Vec<_>>()
    });
    assert_eq!(codes, ["E10223"]);
}

#[test]
fn a_module_const_folds_at_its_declared_width() {
    let source = "module test;
const A: u8 = 255 + 1;
const B: usize = sizeof(u8[4]);
func main(): i32 { return 0; }
";
    with_folder(source, |folder| {
        assert_eq!(folder.fold(&name("A")), Ok(ConstValue::Int(0)));
        assert_eq!(folder.fold(&name("B")), Ok(ConstValue::Int(4)));
    });
}

#[test]
fn an_imported_const_and_an_enum_member_fold() {
    let cfg = "module cfg;
export const WIDTH: usize = 8;
export enum Level : u8 { LOW, MID = 5, HIGH }
";
    let app = "module app;
import { WIDTH, Level } from cfg;
func main(): i32 { return 0; }
";
    with_loaded(&[cfg, app], "app", |folder| {
        assert_eq!(folder.integer(&name("WIDTH")), Some(8));
        assert_eq!(folder.integer(&dotted(&["Level", "LOW"])), Some(0));
        assert_eq!(folder.integer(&dotted(&["Level", "MID"])), Some(5));
        assert_eq!(folder.integer(&dotted(&["Level", "HIGH"])), Some(6));
    });
}

#[test]
fn a_qualified_field_access_is_not_constant() {
    assert!(matches!(
        fold(&dotted(&["point", "x"])),
        Err(ConstError::NotConstant(_))
    ));
}

#[test]
fn sizeof_folds_a_constant_extent_in_its_operand_type() {
    let source = "module test;
const WIDTH: usize = 8;
const BYTES: usize = sizeof(u8[WIDTH]);
func main(): i32 { return 0; }
";
    with_folder(source, |folder| {
        assert_eq!(folder.integer(&name("BYTES")), Some(8));
    });
}
