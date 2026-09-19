//! The semantic type universe.
//!
//! A [`Ty`] is the checked counterpart of an [`crate::ast::Type`]: the same
//! surface forms (scalars, views, raw pointers, handles, structs, enums,
//! arrays, function pointers, generic parameters) with an explicit nullability
//! flag. The type table, which collects a whole program's declarations, lives
//! in the [`table`] submodule.
//!
//! The conversions here are the ones the specification calls the "T1a"
//! widenings: same-signedness integer growth and `f32` to `f64`. Every other
//! conversion needs an explicit cast, and a cast the language does not allow is
//! reported by the checking stage.

use crate::ast::{Expr, ExprKind, ScalarType};

mod table;

pub use table::{resolve_ty, resolve_ty_with, FuncSig, TypeTable};

/// A checked type: a shape plus the `?` empty-state flag.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ty {
    /// The type shape.
    pub kind: TyKind,
    /// True when the `?` suffix adds `null` as a value.
    pub nullable: bool,
}

/// The shapes a checked type can have.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TyKind {
    /// `void`, a return type only.
    Void,
    /// A fixed-width scalar, including `bool`, `usize`, and `isize`.
    Scalar(ScalarType),
    /// The `str` view.
    Str,
    /// The zero-terminated `cstring` view.
    CString,
    /// The `static str` value.
    StaticStr,
    /// A raw pointer `raw T`.
    Raw(Box<Ty>),
    /// An owned handle, user-declared or the built-in `String`.
    Handle(String),
    /// A named struct.
    Struct(String),
    /// A named enum.
    Enum(String),
    /// A fixed array or slice view; `None` extents are omitted ones.
    Array {
        /// The element type.
        element: Box<Ty>,
        /// One entry per bracketed extent.
        extents: Vec<Option<usize>>,
    },
    /// An exact-signature function pointer.
    Func(FuncTy),
    /// A generic type parameter, treated as opaque while checking.
    Param(String),
    /// An imported or otherwise unresolved type; it accepts every operation.
    Unknown,
}

/// A function-pointer signature.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FuncTy {
    /// Parameter types in order.
    pub params: Vec<Ty>,
    /// Return type.
    pub returns: Box<Ty>,
}

impl Ty {
    /// Builds a non-nullable scalar type.
    pub fn scalar(scalar: ScalarType) -> Self {
        Self {
            kind: TyKind::Scalar(scalar),
            nullable: false,
        }
    }

    /// The `bool` type.
    pub fn boolean() -> Self {
        Self::scalar(ScalarType::Bool)
    }

    /// The `i32` type, the default for an untyped integer literal.
    pub fn i32_ty() -> Self {
        Self::scalar(ScalarType::I32)
    }

    /// The `usize` type.
    pub fn usize_ty() -> Self {
        Self::scalar(ScalarType::Usize)
    }

    /// The `void` return type.
    pub fn void() -> Self {
        Self {
            kind: TyKind::Void,
            nullable: false,
        }
    }

    /// The opaque type for anything the checker cannot name.
    pub fn unknown() -> Self {
        Self {
            kind: TyKind::Unknown,
            nullable: false,
        }
    }

    /// A handle type with the given name.
    pub fn handle(name: impl Into<String>) -> Self {
        Self {
            kind: TyKind::Handle(name.into()),
            nullable: false,
        }
    }

    /// A struct type with the given name.
    pub fn struct_(name: impl Into<String>) -> Self {
        Self {
            kind: TyKind::Struct(name.into()),
            nullable: false,
        }
    }

    /// An enum type with the given name.
    pub fn enum_(name: impl Into<String>) -> Self {
        Self {
            kind: TyKind::Enum(name.into()),
            nullable: false,
        }
    }

    /// An array or slice type.
    pub fn array(element: Ty, extents: Vec<Option<usize>>) -> Self {
        Self {
            kind: TyKind::Array {
                element: Box::new(element),
                extents,
            },
            nullable: false,
        }
    }

    /// A function-pointer type.
    pub fn func(signature: FuncTy) -> Self {
        Self {
            kind: TyKind::Func(signature),
            nullable: false,
        }
    }

    /// True when the type is `void`.
    pub fn is_void(&self) -> bool {
        matches!(self.kind, TyKind::Void)
    }

    /// True when the type is the opaque placeholder.
    pub fn is_unknown(&self) -> bool {
        matches!(self.kind, TyKind::Unknown)
    }

    /// True when the type is a generic parameter.
    pub fn is_param(&self) -> bool {
        matches!(self.kind, TyKind::Param(_))
    }

    /// True when the type is `bool`.
    pub fn is_bool(&self) -> bool {
        matches!(self.kind, TyKind::Scalar(ScalarType::Bool))
    }

    /// True when the type is an integer scalar.
    pub fn is_integer(&self) -> bool {
        matches!(self.kind, TyKind::Scalar(scalar) if is_integer_scalar(scalar))
    }

    /// True when the type is an integer or floating-point scalar.
    pub fn is_numeric(&self) -> bool {
        matches!(self.kind, TyKind::Scalar(scalar) if !matches!(scalar, ScalarType::Bool))
    }

    /// Renders the type the way a diagnostic shows it.
    pub fn display(&self) -> String {
        let base = match &self.kind {
            TyKind::Void => "void".to_owned(),
            TyKind::Scalar(scalar) => scalar_name(*scalar).to_owned(),
            TyKind::Str => "str".to_owned(),
            TyKind::CString => "cstring".to_owned(),
            TyKind::StaticStr => "static str".to_owned(),
            TyKind::Raw(inner) => format!("raw {}", inner.display()),
            TyKind::Handle(name) | TyKind::Struct(name) | TyKind::Enum(name) => name.clone(),
            TyKind::Param(name) => name.clone(),
            TyKind::Unknown => "?".to_owned(),
            TyKind::Array { element, extents } => {
                let mut text = element.display();
                for extent in extents {
                    match extent {
                        Some(size) => text.push_str(&format!("[{size}]")),
                        None => text.push_str("[]"),
                    }
                }
                text
            }
            TyKind::Func(signature) => {
                let params: Vec<String> = signature.params.iter().map(Ty::display).collect();
                format!("func({}): {}", params.join(", "), signature.returns.display())
            }
        };
        if self.nullable {
            format!("{base}?")
        } else {
            base
        }
    }
}

/// The name of a scalar type as it is written in source.
pub fn scalar_name(scalar: ScalarType) -> &'static str {
    match scalar {
        ScalarType::I8 => "i8",
        ScalarType::I16 => "i16",
        ScalarType::I32 => "i32",
        ScalarType::U8 => "u8",
        ScalarType::U16 => "u16",
        ScalarType::U32 => "u32",
        ScalarType::F32 => "f32",
        ScalarType::F64 => "f64",
        ScalarType::Bool => "bool",
        ScalarType::Usize => "usize",
        ScalarType::Isize => "isize",
    }
}

/// True for `i8`-`u32`, `usize`, and `isize`.
pub fn is_integer_scalar(scalar: ScalarType) -> bool {
    !matches!(scalar, ScalarType::F32 | ScalarType::F64 | ScalarType::Bool)
}

/// True for `f32` and `f64`.
pub fn is_float_scalar(scalar: ScalarType) -> bool {
    matches!(scalar, ScalarType::F32 | ScalarType::F64)
}

/// The bit width of a scalar type.
///
/// `usize` and `isize` are reported as 32-bit, matching the primary m68k
/// target. The width drives constant folding, where an operation wraps at the
/// width of its context type and a shift count is masked to it.
pub fn scalar_bits(scalar: ScalarType) -> u32 {
    match scalar {
        ScalarType::I8 | ScalarType::U8 | ScalarType::Bool => 8,
        ScalarType::I16 | ScalarType::U16 => 16,
        ScalarType::I32
        | ScalarType::U32
        | ScalarType::F32
        | ScalarType::Usize
        | ScalarType::Isize => 32,
        ScalarType::F64 => 64,
    }
}

/// True for the signed integer scalars.
pub fn is_signed_scalar(scalar: ScalarType) -> bool {
    matches!(
        scalar,
        ScalarType::I8 | ScalarType::I16 | ScalarType::I32 | ScalarType::Isize
    )
}

/// The integer scalar a type denotes, when it is an integer scalar.
///
/// A constant folder uses this to learn the width an expression wraps at; a
/// non-integer context, such as `bool` or a float, yields `None` so the folder
/// keeps its context-free behavior.
pub fn integer_scalar(ty: &Ty) -> Option<ScalarType> {
    match ty.kind {
        TyKind::Scalar(scalar) if is_integer_scalar(scalar) => Some(scalar),
        _ => None,
    }
}

/// True when `from` widens to `to` implicitly (the T1a set).
fn scalar_widens(from: ScalarType, to: ScalarType) -> bool {
    matches!(
        (from, to),
        (ScalarType::U8, ScalarType::U16)
            | (ScalarType::U8, ScalarType::U32)
            | (ScalarType::U16, ScalarType::U32)
            | (ScalarType::I8, ScalarType::I16)
            | (ScalarType::I8, ScalarType::I32)
            | (ScalarType::I16, ScalarType::I32)
            | (ScalarType::F32, ScalarType::F64)
    )
}

/// True when a value of `from` may be used where `to` is expected without a
/// cast. Nullability is ignored here; the possibly-null rules are separate.
pub fn implicit_conversion(from: &Ty, to: &Ty) -> bool {
    if from.kind == to.kind {
        return true;
    }
    if from.is_unknown() || to.is_unknown() || from.is_param() || to.is_param() {
        return true;
    }
    match (&from.kind, &to.kind) {
        (TyKind::StaticStr, TyKind::Str) => true,
        (TyKind::Handle(name), TyKind::Str) if name == "String" => true,
        (TyKind::Scalar(a), TyKind::Scalar(b)) => scalar_widens(*a, *b),
        _ => false,
    }
}

/// Unifies two numeric operands to the type a binary operator results in.
///
/// Returns `None` for a mixed-sign integer pair or a pair that is not a T1a
/// widening in either direction; the caller turns that into `E10200`.
pub fn unify_numeric(left: &Ty, right: &Ty) -> Option<Ty> {
    if left.is_unknown() || left.is_param() {
        return Some(right.clone());
    }
    if right.is_unknown() || right.is_param() {
        return Some(left.clone());
    }
    if left.kind == right.kind {
        return Some(left.clone());
    }
    if let (TyKind::Scalar(a), TyKind::Scalar(b)) = (&left.kind, &right.kind) {
        if scalar_widens(*a, *b) {
            return Some(right.clone());
        }
        if scalar_widens(*b, *a) {
            return Some(left.clone());
        }
    }
    None
}

/// True when an explicit scalar cast from `from` to `to` is allowed.
///
/// Explicit casts cover every scalar-to-scalar pair (including `bool`), an
/// enum to its integer form, and a raw pointer to an integer address.
pub fn scalar_cast_allowed(from: &Ty, to: &Ty) -> bool {
    if from.is_unknown() || to.is_unknown() || from.is_param() || to.is_param() {
        return true;
    }
    matches!(
        (&from.kind, &to.kind),
        (TyKind::Scalar(_), TyKind::Scalar(_))
            | (TyKind::Enum(_), TyKind::Scalar(_))
            | (TyKind::Raw(_), TyKind::Scalar(_))
    )
}

/// True when an integer value fits the scalar type's range.
///
/// `usize` and `isize` are treated as 32-bit, matching the primary m68k
/// target; the exact width only matters for a literal that nearly fills them.
pub fn int_fits(value: i128, scalar: ScalarType) -> bool {
    match scalar {
        ScalarType::I8 => (-128..=127).contains(&value),
        ScalarType::I16 => (-32768..=32767).contains(&value),
        ScalarType::I32 | ScalarType::Isize => (-2147483648..=2147483647).contains(&value),
        ScalarType::U8 => (0..=255).contains(&value),
        ScalarType::U16 => (0..=65535).contains(&value),
        ScalarType::U32 | ScalarType::Usize => (0..=4294967295).contains(&value),
        ScalarType::F32 | ScalarType::F64 | ScalarType::Bool => true,
    }
}

/// Reads the integer value of a decimal literal, including a negated one.
pub fn int_literal_value(expr: &Expr) -> Option<i128> {
    match &expr.kind {
        ExprKind::Int(value) => Some(*value as i128),
        ExprKind::Unary {
            op: crate::ast::UnaryOp::Neg,
            operand,
        } => match &operand.kind {
            ExprKind::Int(value) => Some(-(*value as i128)),
            _ => None,
        },
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A span for a synthetic expression; only `int_literal_value` reads it.
    fn span() -> crate::span::Span {
        let mut sources = crate::span::SourceMap::new();
        let id = sources.add_file("test.clo".to_owned(), String::new());
        crate::span::Span::new(id, 0, 0)
    }

    /// An integer literal expression.
    fn int_literal(value: u64) -> Expr {
        Expr {
            kind: ExprKind::Int(value),
            span: span(),
        }
    }

    /// A `-operand` expression.
    fn negated(operand: Expr) -> Expr {
        Expr {
            kind: ExprKind::Unary {
                op: crate::ast::UnaryOp::Neg,
                operand: Box::new(operand),
            },
            span: span(),
        }
    }

    /// A non-nullable type with the given kind.
    fn ty(kind: TyKind) -> Ty {
        Ty {
            kind,
            nullable: false,
        }
    }

    #[test]
    fn implicit_conversion_matrix_is_exactly_the_t1a_widenings() {
        use ScalarType::*;
        let scalars = [I8, I16, I32, U8, U16, U32, F32, F64, Bool, Usize, Isize];
        let allowed = [
            (U8, U16),
            (U8, U32),
            (U16, U32),
            (I8, I16),
            (I8, I32),
            (I16, I32),
            (F32, F64),
        ];
        for from in scalars {
            for to in scalars {
                let expected = from == to || allowed.contains(&(from, to));
                assert_eq!(
                    implicit_conversion(&Ty::scalar(from), &Ty::scalar(to)),
                    expected,
                    "{} -> {}",
                    scalar_name(from),
                    scalar_name(to)
                );
            }
        }
    }

    #[test]
    fn views_handles_and_aggregates_convert_only_where_allowed() {
        let str_ty = ty(TyKind::Str);
        let static_str = ty(TyKind::StaticStr);
        let cstring = ty(TyKind::CString);
        assert!(implicit_conversion(&static_str, &str_ty));
        assert!(!implicit_conversion(&str_ty, &static_str));
        assert!(implicit_conversion(&Ty::handle("String"), &str_ty));
        assert!(!implicit_conversion(&Ty::handle("File"), &str_ty));
        assert!(!implicit_conversion(&str_ty, &cstring));
        assert!(implicit_conversion(
            &Ty::struct_("Point"),
            &Ty::struct_("Point")
        ));
        assert!(!implicit_conversion(
            &Ty::struct_("Point"),
            &Ty::struct_("Other")
        ));
        assert!(implicit_conversion(&Ty::enum_("E"), &Ty::enum_("E")));
        assert!(!implicit_conversion(&Ty::enum_("E"), &Ty::i32_ty()));
        assert!(!implicit_conversion(
            &Ty::array(Ty::i32_ty(), vec![Some(3)]),
            &str_ty
        ));
    }

    #[test]
    fn unknown_and_parameter_types_convert_freely() {
        let param = ty(TyKind::Param("T".to_owned()));
        assert!(implicit_conversion(&Ty::unknown(), &Ty::i32_ty()));
        assert!(implicit_conversion(&Ty::i32_ty(), &Ty::unknown()));
        assert!(implicit_conversion(&param, &Ty::void()));
        assert!(implicit_conversion(&Ty::void(), &param));
        assert!(param.is_param());
        assert!(!param.is_unknown());
    }

    #[test]
    fn nullability_does_not_change_implicit_conversion() {
        let nullable = Ty {
            kind: TyKind::Scalar(ScalarType::I32),
            nullable: true,
        };
        assert!(implicit_conversion(&nullable, &Ty::i32_ty()));
        assert!(implicit_conversion(&Ty::i32_ty(), &nullable));
    }

    #[test]
    fn numeric_unification_picks_the_wider_operand() {
        use ScalarType::*;
        let u8_ty = Ty::scalar(U8);
        let u16_ty = Ty::scalar(U16);
        assert_eq!(unify_numeric(&u8_ty, &u16_ty), Some(u16_ty.clone()));
        assert_eq!(unify_numeric(&u16_ty, &u8_ty), Some(u16_ty.clone()));
        assert_eq!(
            unify_numeric(&Ty::scalar(F32), &Ty::scalar(F64)),
            Some(Ty::scalar(F64))
        );
        assert_eq!(
            unify_numeric(&Ty::scalar(I8), &Ty::scalar(I32)),
            Some(Ty::i32_ty())
        );
        assert_eq!(
            unify_numeric(&Ty::i32_ty(), &Ty::i32_ty()),
            Some(Ty::i32_ty())
        );
    }

    #[test]
    fn numeric_unification_rejects_mixed_sign_and_non_widenings() {
        use ScalarType::*;
        assert_eq!(unify_numeric(&Ty::i32_ty(), &Ty::scalar(U32)), None);
        assert_eq!(unify_numeric(&Ty::scalar(U8), &Ty::scalar(I8)), None);
        assert_eq!(unify_numeric(&Ty::scalar(U16), &Ty::scalar(I32)), None);
        assert_eq!(unify_numeric(&Ty::scalar(F64), &Ty::scalar(I32)), None);
        assert_eq!(unify_numeric(&Ty::boolean(), &Ty::scalar(I32)), None);
        let param = ty(TyKind::Param("T".to_owned()));
        assert_eq!(
            unify_numeric(&Ty::unknown(), &Ty::scalar(U16)),
            Some(Ty::scalar(U16))
        );
        assert_eq!(unify_numeric(&Ty::scalar(U16), &param), Some(Ty::scalar(U16)));
    }

    #[test]
    fn explicit_scalar_casts_cover_narrowing_and_cross_family_pairs() {
        use ScalarType::*;
        assert!(scalar_cast_allowed(&Ty::scalar(U16), &Ty::scalar(U8)));
        assert!(scalar_cast_allowed(&Ty::scalar(F64), &Ty::i32_ty()));
        assert!(scalar_cast_allowed(&Ty::boolean(), &Ty::scalar(U8)));
        assert!(scalar_cast_allowed(&Ty::scalar(U8), &Ty::boolean()));
        assert!(scalar_cast_allowed(&Ty::enum_("E"), &Ty::scalar(U16)));
        assert!(scalar_cast_allowed(
            &ty(TyKind::Raw(Box::new(Ty::i32_ty()))),
            &Ty::usize_ty()
        ));
    }

    #[test]
    fn explicit_casts_reject_non_scalar_conversions() {
        assert!(!scalar_cast_allowed(&Ty::struct_("Point"), &Ty::i32_ty()));
        assert!(!scalar_cast_allowed(&Ty::i32_ty(), &Ty::enum_("E")));
        assert!(!scalar_cast_allowed(&Ty::enum_("E"), &Ty::struct_("S")));
        assert!(!scalar_cast_allowed(&ty(TyKind::Str), &Ty::i32_ty()));
        assert!(!scalar_cast_allowed(&Ty::i32_ty(), &ty(TyKind::Str)));
        assert!(scalar_cast_allowed(&Ty::unknown(), &Ty::struct_("S")));
        assert!(scalar_cast_allowed(
            &ty(TyKind::Param("T".to_owned())),
            &Ty::void()
        ));
    }

    #[test]
    fn integer_literal_fit_boundaries_are_inclusive() {
        use ScalarType::*;
        assert!(int_fits(-128, I8));
        assert!(int_fits(127, I8));
        assert!(!int_fits(-129, I8));
        assert!(!int_fits(128, I8));
        assert!(int_fits(0, U8));
        assert!(int_fits(255, U8));
        assert!(!int_fits(-1, U8));
        assert!(!int_fits(256, U8));
        assert!(int_fits(-32768, I16));
        assert!(int_fits(32767, I16));
        assert!(!int_fits(32768, I16));
        assert!(int_fits(65535, U16));
        assert!(!int_fits(65536, U16));
        assert!(int_fits(-2147483648, I32));
        assert!(int_fits(2147483647, I32));
        assert!(!int_fits(2147483648, I32));
        assert!(int_fits(4294967295, U32));
        assert!(!int_fits(4294967296, U32));
        assert!(int_fits(-2147483648, Isize));
        assert!(!int_fits(2147483648, Isize));
        assert!(int_fits(4294967295, Usize));
        assert!(!int_fits(-1, Usize));
        assert!(int_fits(-999, F32));
        assert!(int_fits(999, Bool));
    }

    #[test]
    fn integer_literal_value_reads_plain_and_negated_literals() {
        assert_eq!(int_literal_value(&int_literal(7)), Some(7));
        assert_eq!(int_literal_value(&negated(int_literal(7))), Some(-7));
        assert_eq!(int_literal_value(&negated(int_literal(0))), Some(0));
        let boolean = Expr {
            kind: ExprKind::Bool(true),
            span: span(),
        };
        assert_eq!(int_literal_value(&boolean), None);
        assert_eq!(int_literal_value(&negated(boolean)), None);
        let named = Expr {
            kind: ExprKind::Name("x".to_owned()),
            span: span(),
        };
        assert_eq!(int_literal_value(&named), None);
    }

    #[test]
    fn type_predicates_and_display_match_the_surface_spelling() {
        use ScalarType::*;
        assert!(Ty::scalar(I32).is_integer());
        assert!(!Ty::scalar(F32).is_integer());
        assert!(!Ty::boolean().is_integer());
        assert!(Ty::scalar(F32).is_numeric());
        assert!(!Ty::boolean().is_numeric());
        assert!(Ty::boolean().is_bool());
        assert!(Ty::void().is_void());
        assert!(Ty::unknown().is_unknown());
        assert_eq!(Ty::scalar(I32).display(), "i32");
        assert_eq!(Ty::scalar(U16).display(), "u16");
        assert_eq!(Ty::void().display(), "void");
        assert_eq!(
            Ty {
                kind: TyKind::Scalar(U8),
                nullable: true
            }
            .display(),
            "u8?"
        );
        assert_eq!(ty(TyKind::StaticStr).display(), "static str");
        assert_eq!(ty(TyKind::CString).display(), "cstring");
        assert_eq!(ty(TyKind::Raw(Box::new(Ty::i32_ty()))).display(), "raw i32");
        assert_eq!(Ty::array(Ty::scalar(U8), vec![Some(4)]).display(), "u8[4]");
        assert_eq!(Ty::array(Ty::scalar(U8), vec![None]).display(), "u8[]");
        assert_eq!(
            Ty::array(Ty::scalar(U8), vec![Some(2), Some(3)]).display(),
            "u8[2][3]"
        );
        assert_eq!(Ty::handle("String").display(), "String");
        assert_eq!(Ty::unknown().display(), "?");
        let signature = FuncTy {
            params: vec![Ty::i32_ty()],
            returns: Box::new(Ty::boolean()),
        };
        assert_eq!(Ty::func(signature).display(), "func(i32): bool");
    }
}
