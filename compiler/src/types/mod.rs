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

use std::collections::BTreeMap;

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
    /// A named struct with its concrete type arguments.
    ///
    /// A non-generic struct carries an empty argument list; the growable
    /// `Array<T>` keeps its element type here so callers can infer `T` from a
    /// receiver.
    Struct(String, Vec<Ty>),
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

    /// A struct type with the given name and no type arguments.
    pub fn struct_(name: impl Into<String>) -> Self {
        Self::struct_with(name, Vec::new())
    }

    /// A struct type with the given name and type arguments.
    pub fn struct_with(name: impl Into<String>, args: Vec<Ty>) -> Self {
        Self {
            kind: TyKind::Struct(name.into(), args),
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

    /// A raw pointer type with the given pointee.
    pub fn raw(inner: Ty) -> Self {
        Self {
            kind: TyKind::Raw(Box::new(inner)),
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
            TyKind::Handle(name) | TyKind::Enum(name) => name.clone(),
            TyKind::Struct(name, args) => {
                if args.is_empty() {
                    name.clone()
                } else {
                    let rendered: Vec<String> = args.iter().map(Ty::display).collect();
                    format!("{name}<{}>", rendered.join(", "))
                }
            }
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

/// Applies a generic substitution to a type.
///
/// A parameter atom is replaced by its binding; an array, raw pointer, or
/// struct recurses into its element, pointee, or type arguments. Every other
/// shape is unchanged. An empty substitution returns the type unchanged.
pub fn substitute(ty: &Ty, bindings: &BTreeMap<String, Ty>) -> Ty {
    if bindings.is_empty() {
        return ty.clone();
    }
    match &ty.kind {
        TyKind::Param(name) => bindings.get(name).cloned().unwrap_or_else(|| ty.clone()),
        TyKind::Array { element, extents } => Ty {
            kind: TyKind::Array {
                element: Box::new(substitute(element, bindings)),
                extents: extents.clone(),
            },
            nullable: ty.nullable,
        },
        TyKind::Raw(inner) => Ty {
            kind: TyKind::Raw(Box::new(substitute(inner, bindings))),
            nullable: ty.nullable,
        },
        TyKind::Struct(name, args) => Ty {
            kind: TyKind::Struct(
                name.clone(),
                args.iter().map(|arg| substitute(arg, bindings)).collect(),
            ),
            nullable: ty.nullable,
        },
        _ => ty.clone(),
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
    if from.is_unknown()
        || to.is_unknown()
        || from.is_param()
        || to.is_param()
        || contains_param(from)
        || contains_param(to)
    {
        return true;
    }
    match (&from.kind, &to.kind) {
        (TyKind::StaticStr, TyKind::Str) => true,
        (TyKind::Handle(name), TyKind::Str) if name == "String" => true,
        (TyKind::Scalar(a), TyKind::Scalar(b)) => scalar_widens(*a, *b),
        _ => false,
    }
}

/// True when a type mentions a generic parameter anywhere inside it.
///
/// The checker treats such a type as compatible with any concrete type, the
/// same way it treats a bare parameter, so an unsubstituted generic signature
/// does not cascade into conversion errors.
fn contains_param(ty: &Ty) -> bool {
    match &ty.kind {
        TyKind::Param(_) => true,
        TyKind::Array { element, .. } => contains_param(element),
        TyKind::Raw(inner) => contains_param(inner),
        TyKind::Struct(_, args) => args.iter().any(contains_param),
        TyKind::Func(signature) => {
            signature.params.iter().any(contains_param) || contains_param(&signature.returns)
        }
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
mod tests;
