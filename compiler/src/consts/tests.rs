//! In-module tests for constant evaluation.
//!
//! The folder is driven directly over synthetic expression trees, plus a few
//! parsed and loaded modules where a `const` declaration, a fixed array, or a
//! struct layout is part of the answer.

use super::*;
use crate::ast::{QualifiedName, ScalarType, TypeKind};
use crate::load;
use crate::parse::Parser;
use crate::span::{SourceMap, Span};

/// A span for a synthetic expression; the folder carries it into errors.
pub(super) fn span() -> Span {
    let mut sources = SourceMap::new();
    let id = sources.add_file("test.clo".to_owned(), String::new());
    Span::new(id, 0, 0)
}

/// An integer literal expression.
pub(super) fn int(value: u64) -> Expr {
    Expr {
        kind: ExprKind::Int(value),
        span: span(),
    }
}

/// A boolean literal expression.
fn boolean(value: bool) -> Expr {
    Expr {
        kind: ExprKind::Bool(value),
        span: span(),
    }
}

/// A name reference.
pub(super) fn name(text: &str) -> Expr {
    Expr {
        kind: ExprKind::Name(text.to_owned()),
        span: span(),
    }
}

/// A dotted name reference.
pub(super) fn dotted(segments: &[&str]) -> Expr {
    Expr {
        kind: ExprKind::Qualified(QualifiedName {
            segments: segments.iter().map(|segment| (*segment).to_owned()).collect(),
            span: span(),
        }),
        span: span(),
    }
}

/// A unary operator application.
pub(super) fn unary(op: UnaryOp, operand: Expr) -> Expr {
    Expr {
        kind: ExprKind::Unary {
            op,
            operand: Box::new(operand),
        },
        span: span(),
    }
}

/// A binary operator application.
pub(super) fn op(operator: BinaryOp, left: Expr, right: Expr) -> Expr {
    Expr {
        kind: ExprKind::Binary {
            op: operator,
            left: Box::new(left),
            right: Box::new(right),
        },
        span: span(),
    }
}

/// A scalar cast `T(value)`.
pub(super) fn cast(scalar: ScalarType, value: Expr) -> Expr {
    Expr {
        kind: ExprKind::Cast {
            ty: scalar_type(scalar),
            value: Box::new(value),
        },
        span: span(),
    }
}

/// A named type with no arguments.
fn named_type(text: &str) -> Type {
    Type {
        kind: TypeKind::Named {
            name: QualifiedName {
                segments: vec![text.to_owned()],
                span: span(),
            },
            args: Vec::new(),
        },
        nullable: false,
        span: span(),
    }
}

/// A scalar type.
fn scalar_type(scalar: ScalarType) -> Type {
    Type {
        kind: TypeKind::Scalar(scalar),
        nullable: false,
        span: span(),
    }
}

/// An intrinsic call with one argument.
fn intrinsic(which: &str, arg: IntrinsicArg) -> Expr {
    Expr {
        kind: ExprKind::IntrinsicCall {
            name: which.to_owned(),
            args: vec![arg],
        },
        span: span(),
    }
}

/// Builds a folder over one parsed and loaded module named `test`.
///
/// The closure receives the folder so the table and module borrows stay
/// alive for the whole fold.
pub(super) fn with_folder<R>(source: &str, run: impl FnOnce(&mut ConstFolder) -> R) -> R {
    with_loaded(&[source], "test", run)
}

/// Builds a folder over several parsed sources, for the named module.
///
/// The closure receives the folder so the table and module borrows stay alive
/// for the whole fold.
pub(super) fn with_loaded<R>(sources: &[&str], module: &str, run: impl FnOnce(&mut ConstFolder) -> R) -> R {
    let mut source_map = SourceMap::new();
    let mut diagnostics = Diagnostics::new(false, None);
    let mut programs = Vec::new();
    for (index, text) in sources.iter().enumerate() {
        let id = source_map.add_file(format!("file{index}.clo"), (*text).to_owned());
        let program = Parser::new(&source_map, id, &mut diagnostics).parse_program();
        programs.push((id, program));
    }
    let loaded = load::load(programs, &mut diagnostics);
    let table = TypeTable::build(&loaded, crate::types::AbiLayout::host());
    let module = loaded.module(module).expect("the test module loaded");
    let mut folder = ConstFolder::new(&table, module, &loaded);
    run(&mut folder)
}

/// The baseline module every direct fold is evaluated against.
pub(super) const BASELINE: &str = "module test;\nfunc main(): i32 { return 0; }\n";

/// Folds one synthetic expression against an otherwise empty module.
pub(super) fn fold(expr: &Expr) -> Result<ConstValue, ConstError> {
    with_folder(BASELINE, |folder| folder.fold(expr))
}

#[test]
fn integer_arithmetic_wraps_in_the_wide_accumulator() {
    assert_eq!(
        fold_integer(BinaryOp::Add, i128::MAX, 1, span(), None),
        Ok(ConstValue::Int(i128::MIN))
    );
    assert_eq!(
        fold_integer(BinaryOp::Sub, i128::MIN, 1, span(), None),
        Ok(ConstValue::Int(i128::MAX))
    );
    assert_eq!(
        fold_integer(BinaryOp::Mul, i128::MAX, 2, span(), None),
        Ok(ConstValue::Int(-2))
    );
    assert_eq!(
        fold_integer(BinaryOp::Add, 2, 3, span(), None),
        Ok(ConstValue::Int(5))
    );
    assert_eq!(
        fold_integer(BinaryOp::Sub, 2, 5, span(), None),
        Ok(ConstValue::Int(-3))
    );
    assert_eq!(
        fold_integer(BinaryOp::Mul, -4, 3, span(), None),
        Ok(ConstValue::Int(-12))
    );
    assert_eq!(
        fold_integer(BinaryOp::BitAnd, 0b1100, 0b1010, span(), None),
        Ok(ConstValue::Int(0b1000))
    );
    assert_eq!(
        fold_integer(BinaryOp::BitOr, 0b1100, 0b1010, span(), None),
        Ok(ConstValue::Int(0b1110))
    );
    assert_eq!(
        fold_integer(BinaryOp::BitXor, 0b1100, 0b1010, span(), None),
        Ok(ConstValue::Int(0b0110))
    );
}

#[test]
fn shift_counts_wrap_and_signed_right_shift_is_arithmetic() {
    assert_eq!(
        fold_integer(BinaryOp::Shl, 1, 0, span(), None),
        Ok(ConstValue::Int(1))
    );
    assert_eq!(
        fold_integer(BinaryOp::Shl, 1, 8, span(), None),
        Ok(ConstValue::Int(256))
    );
    // The count is applied modulo the accumulator width.
    assert_eq!(
        fold_integer(BinaryOp::Shl, 1, 128, span(), None),
        Ok(ConstValue::Int(1))
    );
    assert_eq!(
        fold_integer(BinaryOp::Shl, 1, 127, span(), None),
        Ok(ConstValue::Int(i128::MIN))
    );
    assert_eq!(
        fold_integer(BinaryOp::Shl, 1, -1, span(), None),
        Ok(ConstValue::Int(i128::MIN))
    );
    assert_eq!(
        fold_integer(BinaryOp::Shr, -8, 2, span(), None),
        Ok(ConstValue::Int(-2))
    );
    assert_eq!(
        fold_integer(BinaryOp::Shr, 8, 2, span(), None),
        Ok(ConstValue::Int(2))
    );
}

#[test]
fn division_and_remainder_truncate_toward_zero() {
    assert_eq!(
        fold_integer(BinaryOp::Div, 7, 2, span(), None),
        Ok(ConstValue::Int(3))
    );
    assert_eq!(
        fold_integer(BinaryOp::Div, -7, 2, span(), None),
        Ok(ConstValue::Int(-3))
    );
    assert_eq!(
        fold_integer(BinaryOp::Rem, -7, 2, span(), None),
        Ok(ConstValue::Int(-1))
    );
    assert_eq!(
        fold_integer(BinaryOp::Rem, 7, -2, span(), None),
        Ok(ConstValue::Int(1))
    );
}

#[test]
fn division_and_remainder_by_zero_fault() {
    assert_eq!(
        fold_integer(BinaryOp::Div, 10, 0, span(), None),
        Err(ConstError::DivisionByZero(span()))
    );
    assert_eq!(
        fold_integer(BinaryOp::Rem, 10, 0, span(), None),
        Err(ConstError::DivisionByZero(span()))
    );
}

#[test]
fn the_signed_minimum_over_minus_one_faults_on_both_operators() {
    assert_eq!(
        fold_integer(BinaryOp::Div, i128::MIN, -1, span(), None),
        Err(ConstError::DivisionByZero(span()))
    );
    assert_eq!(
        fold_integer(BinaryOp::Rem, i128::MIN, -1, span(), None),
        Err(ConstError::DivisionByZero(span()))
    );
    assert_eq!(
        fold_integer(BinaryOp::Div, i128::MIN, 1, span(), None),
        Ok(ConstValue::Int(i128::MIN))
    );
}

#[test]
fn integer_comparisons_and_boolean_logic_fold() {
    assert_eq!(
        fold_integer(BinaryOp::Eq, 1, 1, span(), None),
        Ok(ConstValue::Bool(true))
    );
    assert_eq!(
        fold_integer(BinaryOp::Ne, 1, 1, span(), None),
        Ok(ConstValue::Bool(false))
    );
    assert_eq!(
        fold_integer(BinaryOp::Lt, 1, 2, span(), None),
        Ok(ConstValue::Bool(true))
    );
    assert_eq!(
        fold_integer(BinaryOp::Le, 2, 2, span(), None),
        Ok(ConstValue::Bool(true))
    );
    assert_eq!(
        fold_integer(BinaryOp::Gt, 3, 2, span(), None),
        Ok(ConstValue::Bool(true))
    );
    assert_eq!(
        fold_integer(BinaryOp::Ge, 2, 2, span(), None),
        Ok(ConstValue::Bool(true))
    );
    // `&&` and `||` need short-circuit semantics, so they stay unfoled.
    assert!(matches!(
        fold_integer(BinaryOp::LogicalAnd, 1, 1, span(), None),
        Err(ConstError::NotConstant(_))
    ));
    assert!(matches!(
        fold_integer(BinaryOp::LogicalOr, 1, 1, span(), None),
        Err(ConstError::NotConstant(_))
    ));
}

#[test]
fn float_folding_follows_ieee_and_never_faults_on_zero() {
    assert_eq!(
        fold_float(BinaryOp::Add, 1.5, 2.25, span()),
        Ok(ConstValue::Float(3.75))
    );
    assert_eq!(
        fold_float(BinaryOp::Div, 1.0, 0.0, span()),
        Ok(ConstValue::Float(f64::INFINITY))
    );
    assert!(matches!(
        fold_float(BinaryOp::Div, 0.0, 0.0, span()),
        Ok(ConstValue::Float(value)) if value.is_nan()
    ));
    assert_eq!(
        fold_float(BinaryOp::Eq, 1.0, 1.0, span()),
        Ok(ConstValue::Bool(true))
    );
    assert!(matches!(
        fold_float(BinaryOp::Rem, 1.0, 2.0, span()),
        Err(ConstError::NotConstant(_))
    ));
}

#[test]
fn literals_and_scalar_casts_fold() {
    assert_eq!(fold(&int(42)), Ok(ConstValue::Int(42)));
    assert_eq!(fold(&boolean(true)), Ok(ConstValue::Bool(true)));
    assert_eq!(
        fold(&Expr {
            kind: ExprKind::Float(1.5),
            span: span(),
        }),
        Ok(ConstValue::Float(1.5))
    );
    assert_eq!(
        fold(&Expr {
            kind: ExprKind::Char(b'A'),
            span: span(),
        }),
        Ok(ConstValue::Int(65))
    );
    assert_eq!(
        fold(&Expr {
            kind: ExprKind::Str(vec![b'h', b'i']),
            span: span(),
        }),
        Ok(ConstValue::Str)
    );

    let float_to_int = Expr {
        kind: ExprKind::Cast {
            ty: scalar_type(ScalarType::I32),
            value: Box::new(Expr {
                kind: ExprKind::Float(2.9),
                span: span(),
            }),
        },
        span: span(),
    };
    assert_eq!(fold(&float_to_int), Ok(ConstValue::Int(2)));
    let int_to_float = Expr {
        kind: ExprKind::Cast {
            ty: scalar_type(ScalarType::F64),
            value: Box::new(int(3)),
        },
        span: span(),
    };
    assert_eq!(fold(&int_to_float), Ok(ConstValue::Float(3.0)));
    let struct_cast = Expr {
        kind: ExprKind::Cast {
            ty: named_type("Point"),
            value: Box::new(int(1)),
        },
        span: span(),
    };
    assert!(matches!(
        fold(&struct_cast),
        Err(ConstError::NotConstant(_))
    ));
}

#[test]
fn unary_operators_fold_with_wrapping_negation() {
    assert_eq!(fold(&unary(UnaryOp::Neg, int(7))), Ok(ConstValue::Int(-7)));
    assert_eq!(
        fold(&unary(UnaryOp::BitNot, int(0))),
        Ok(ConstValue::Int(-1))
    );
    assert_eq!(
        fold(&unary(UnaryOp::Not, boolean(false))),
        Ok(ConstValue::Bool(true))
    );
    // The minimum signed value negates to itself under wrapping.
    let min = op(BinaryOp::Shl, int(1), int(127));
    assert_eq!(fold(&min), Ok(ConstValue::Int(i128::MIN)));
    assert_eq!(
        fold(&unary(UnaryOp::Neg, op(BinaryOp::Shl, int(1), int(127)))),
        Ok(ConstValue::Int(i128::MIN))
    );
    // An operator on the wrong operand shape is not constant.
    let bad = unary(UnaryOp::Not, int(1));
    assert!(matches!(fold(&bad), Err(ConstError::NotConstant(_))));
}

#[test]
fn unknown_expression_forms_are_not_constant() {
    let index = Expr {
        kind: ExprKind::Index {
            base: Box::new(name("items")),
            index: Box::new(int(0)),
        },
        span: span(),
    };
    assert!(matches!(fold(&index), Err(ConstError::NotConstant(_))));
    assert!(matches!(
        fold(&Expr {
            kind: ExprKind::Error,
            span: span(),
        }),
        Err(ConstError::NotConstant(_))
    ));
}

#[test]
fn module_constants_fold_and_a_cycle_is_not_constant() {
    let source = "module test;
const A: i32 = 2;
const B: i32 = A + 1;
const C: i32 = C;
func main(): i32 { return 0; }
";
    with_folder(source, |folder| {
        assert_eq!(folder.fold(&name("A")), Ok(ConstValue::Int(2)));
        assert_eq!(folder.fold(&name("B")), Ok(ConstValue::Int(3)));
        assert!(matches!(
            folder.fold(&name("C")),
            Err(ConstError::NotConstant(_))
        ));
    });
}

#[test]
fn runtime_globals_and_functions_are_not_constant() {
    let source = "module test;
let limit: i32 = 5;
func helper(): i32 { return 1; }
func main(): i32 { return 0; }
";
    with_folder(source, |folder| {
        assert!(matches!(
            folder.fold(&name("limit")),
            Err(ConstError::NotConstant(_))
        ));
        assert!(matches!(
            folder.fold(&name("helper")),
            Err(ConstError::NotConstant(_))
        ));
        // A name the folder cannot tie to a constant declaration is not a
        // constant, so a runtime local is reported rather than guessed at.
        assert!(matches!(
            folder.fold(&name("mystery")),
            Err(ConstError::NotConstant(_))
        ));
    });
}

#[test]
fn integer_helper_reads_only_integer_folds() {
    with_folder(BASELINE, |folder| {
        assert_eq!(folder.integer(&int(9)), Some(9));
        assert_eq!(folder.integer(&boolean(true)), None);
        assert_eq!(folder.integer(&op(BinaryOp::Add, int(1), int(2))), Some(3));
    });
}

#[test]
fn require_reports_the_two_constant_fault_codes() {
    let codes = with_folder(BASELINE, |folder| {
        let mut diagnostics = Diagnostics::new(false, None);
        folder.require(&op(BinaryOp::Div, int(1), int(0)), &mut diagnostics);
        folder.require(&name("mystery"), &mut diagnostics);
        let call = Expr {
            kind: ExprKind::Call {
                callee: Box::new(name("f")),
                type_args: Vec::new(),
                args: Vec::new(),
            },
            span: span(),
        };
        folder.require(&call, &mut diagnostics);
        diagnostics
            .items()
            .iter()
            .map(|item| item.code)
            .collect::<Vec<_>>()
    });
    assert_eq!(codes, ["E10223", "E10224", "E10224"]);
}

#[test]
fn sizeof_and_offsetof_fold_from_the_type_table() {
    let source = "module test;
struct Mixed { tag: u8; count: u16; value: u32; }
func main(): i32 { return 0; }
";
    with_folder(source, |folder| {
        let size = intrinsic("sizeof", IntrinsicArg::Type(named_type("Mixed")));
        assert_eq!(folder.fold(&size), Ok(ConstValue::Int(7)));
        let offset = Expr {
            kind: ExprKind::IntrinsicCall {
                name: "offsetof".to_owned(),
                args: vec![
                    IntrinsicArg::Type(named_type("Mixed")),
                    IntrinsicArg::Expr(name("value")),
                ],
            },
            span: span(),
        };
        // The offset uses the target ABI's padding: `count` ends at 4 and
        // `value` starts there.
        assert_eq!(folder.fold(&offset), Ok(ConstValue::Int(4)));
        let missing = Expr {
            kind: ExprKind::IntrinsicCall {
                name: "offsetof".to_owned(),
                args: vec![
                    IntrinsicArg::Type(named_type("Mixed")),
                    IntrinsicArg::Expr(name("missing")),
                ],
            },
            span: span(),
        };
        assert!(matches!(
            folder.fold(&missing),
            Err(ConstError::NotConstant(_))
        ));
        // A type the table does not model has no known size.
        let unknown = intrinsic("sizeof", IntrinsicArg::Type(named_type("Unmodelled")));
        assert!(matches!(
            folder.fold(&unknown),
            Err(ConstError::NotConstant(_))
        ));
        // `sizeof` of a non-type argument is malformed.
        let malformed = intrinsic("sizeof", IntrinsicArg::Expr(int(1)));
        assert!(matches!(
            folder.fold(&malformed),
            Err(ConstError::NotConstant(_))
        ));
    });
}

#[test]
fn len_folds_literals_strings_and_fixed_globals() {
    let literal = Expr {
        kind: ExprKind::ArrayLiteral {
            elements: vec![int(1), int(2), int(3)],
            fill: None,
        },
        span: span(),
    };
    assert_eq!(
        fold(&intrinsic("len", IntrinsicArg::Expr(literal))),
        Ok(ConstValue::Int(3))
    );
    let text = Expr {
        kind: ExprKind::Str(vec![b'a', b'b', b'c', b'd']),
        span: span(),
    };
    assert_eq!(
        fold(&intrinsic("len", IntrinsicArg::Expr(text))),
        Ok(ConstValue::Int(4))
    );

    let fixed = "module test;
let primes: u32[3] = [2, 3, 5];
func main(): i32 { return 0; }
";
    with_folder(fixed, |folder| {
        assert_eq!(
            folder.fold(&intrinsic("len", IntrinsicArg::Expr(name("primes")))),
            Ok(ConstValue::Int(3))
        );
    });

    let slice = "module test;
let view: u32[] = [];
func main(): i32 { return 0; }
";
    with_folder(slice, |folder| {
        assert!(matches!(
            folder.fold(&intrinsic("len", IntrinsicArg::Expr(name("view")))),
            Err(ConstError::NotConstant(_))
        ));
    });
}

#[test]
fn an_unknown_intrinsic_is_not_constant() {
    assert!(matches!(
        fold(&intrinsic("embed", IntrinsicArg::Expr(int(1)))),
        Err(ConstError::NotConstant(_))
    ));
}

/// A raw cast `raw T(value)`.
fn raw_cast(target: &str, value: Expr) -> Expr {
    Expr {
        kind: ExprKind::RawCast {
            ty: named_type(target),
            value: Box::new(value),
        },
        span: span(),
    }
}

#[test]
fn a_raw_cast_of_an_integer_address_is_constant() {
    // A module-level register base is `raw T(0xADDRESS)`; the address is the
    // constant and the raw target only names the pointer type.
    assert_eq!(
        fold(&raw_cast("Regs", int(0x00DFF000))),
        Ok(ConstValue::Int(0x00DFF000))
    );
}

#[test]
fn a_raw_cast_of_a_non_integer_is_not_constant() {
    assert!(matches!(
        fold(&raw_cast("Regs", boolean(true))),
        Err(ConstError::NotConstant(_))
    ));
}
