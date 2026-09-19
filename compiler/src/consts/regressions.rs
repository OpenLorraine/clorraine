//! Regression tests for constant folding at a context width.
//!
//! These pin the review fixes: `+ - * ~` wrap at the context width, shift
//! counts mask to it, the signed minimum over `-1` faults there, a module
//! `const` folds at its declared width, an imported `const` and an enum member
//! fold, a field access does not, and a constant extent folds inside `sizeof`.

use super::tests::{cast, dotted, fold, int, name, op, span, unary, with_folder, with_loaded, BASELINE};
use super::*;
use crate::ast::{BinaryOp, ScalarType, UnaryOp};

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
