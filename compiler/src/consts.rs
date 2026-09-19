//! Constant expression evaluation.
//!
//! A constant context is an array extent, an enum member value, a `case`
//! label, or a module-level `const` initializer. In those positions the value
//! must be computable at compile time, and the folding here follows the
//! runtime arithmetic rules: integer operations wrap in a wide accumulator and
//! shift counts are masked. A form the folder cannot compute is `E10224`; an
//! integer division or remainder by zero, including the minimum signed value
//! over `-1`, is `E10223`.
//!
//! The folder only sees module-level names. A name that is a module-level
//! `const` folds from its initializer, with a cycle guard for a malformed
//! self-reference; a module-level `let` or a function is not constant. A name
//! the folder cannot tie to a module declaration is treated as constant with
//! an opaque value, because the name stage has already validated it and the
//! folder has no access to local initializers.

use std::collections::{BTreeMap, BTreeSet};

use crate::ast::{BinaryOp, Expr, ExprKind, IntrinsicArg, Item, Type, UnaryOp};
use crate::diag::{Diagnostic, Diagnostics};
use crate::load::{DeclKind, Module};
use crate::span::Span;
use crate::types::{is_float_scalar, is_integer_scalar, resolve_ty, TyKind, TypeTable};

/// A value a constant expression folds to.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ConstValue {
    /// An integer, held in a wide signed accumulator.
    Int(i128),
    /// A boolean.
    Bool(bool),
    /// A floating-point number.
    Float(f64),
    /// A string literal, which is constant but never used in arithmetic.
    Str,
}

/// Why a constant expression could not be folded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConstError {
    /// The expression is not a constant form.
    NotConstant(Span),
    /// Integer division or remainder by zero, including the minimum signed
    /// value over `-1`.
    DivisionByZero(Span),
}

/// Folds the constant expressions of one module.
pub struct ConstFolder<'a> {
    /// The program's declared types, for `sizeof`, `offsetof`, and casts.
    table: &'a TypeTable,
    /// The module whose `const` declarations the folder may read.
    module: &'a Module,
    /// Constants currently being folded, so a cycle stops instead of looping.
    active: BTreeSet<String>,
    /// Folded values, cached so each constant is evaluated once.
    values: BTreeMap<String, ConstValue>,
}

impl<'a> ConstFolder<'a> {
    /// Creates a folder for one module.
    pub fn new(table: &'a TypeTable, module: &'a Module) -> Self {
        Self {
            table,
            module,
            active: BTreeSet::new(),
            values: BTreeMap::new(),
        }
    }

    /// Folds an expression to its constant value.
    pub fn fold(&mut self, expr: &Expr) -> Result<ConstValue, ConstError> {
        match &expr.kind {
            ExprKind::Int(value) => Ok(ConstValue::Int(*value as i128)),
            ExprKind::Bool(value) => Ok(ConstValue::Bool(*value)),
            ExprKind::Char(value) => Ok(ConstValue::Int(*value as i128)),
            ExprKind::Float(value) => Ok(ConstValue::Float(*value)),
            ExprKind::Str(_) => Ok(ConstValue::Str),
            ExprKind::Unary { op, operand } => self.fold_unary(*op, operand, expr.span),
            ExprKind::Binary { op, left, right } => {
                self.fold_binary(*op, left, right, expr.span)
            }
            ExprKind::Cast { ty, value } => self.fold_cast(ty, value, expr.span),
            ExprKind::Name(name) => self.fold_name(name, expr.span),
            // A qualified name is a module constant or an enum member, both of
            // which are constant; the folder does not compute enum numbering.
            ExprKind::Qualified(_) => Ok(ConstValue::Int(0)),
            ExprKind::IntrinsicCall { name, args } => self.fold_intrinsic(name, args, expr.span),
            _ => Err(ConstError::NotConstant(expr.span)),
        }
    }

    /// Folds an expression that must be constant, reporting `E10224` or
    /// `E10223` on failure.
    pub fn require(&mut self, expr: &Expr, diagnostics: &mut Diagnostics) {
        match self.fold(expr) {
            Ok(_) => {}
            Err(ConstError::NotConstant(span)) => diagnostics.push(
                Diagnostic::error("E10224", "expression is not constant", span).with_help(
                    "use literals, `const` bindings, enum members, and supported operators",
                ),
            ),
            Err(ConstError::DivisionByZero(span)) => diagnostics.push(
                Diagnostic::error("E10223", "constant expression divides by zero", span).with_help(
                    "remove the zero divisor or the MIN/-1 operation, or use a non-constant operand",
                ),
            ),
        }
    }

    /// The integer value of an expression, when it folds to one.
    pub fn integer(&mut self, expr: &Expr) -> Option<i128> {
        match self.fold(expr) {
            Ok(ConstValue::Int(value)) => Some(value),
            _ => None,
        }
    }

    /// Folds a unary operator over its constant operand.
    fn fold_unary(
        &mut self,
        op: UnaryOp,
        operand: &Expr,
        span: Span,
    ) -> Result<ConstValue, ConstError> {
        match (op, self.fold(operand)?) {
            (UnaryOp::Neg, ConstValue::Int(value)) => Ok(ConstValue::Int(value.wrapping_neg())),
            (UnaryOp::Neg, ConstValue::Float(value)) => Ok(ConstValue::Float(-value)),
            (UnaryOp::BitNot, ConstValue::Int(value)) => Ok(ConstValue::Int(!value)),
            (UnaryOp::Not, ConstValue::Bool(value)) => Ok(ConstValue::Bool(!value)),
            _ => Err(ConstError::NotConstant(span)),
        }
    }

    /// Folds a binary operator over its constant operands.
    fn fold_binary(
        &mut self,
        op: BinaryOp,
        left: &Expr,
        right: &Expr,
        span: Span,
    ) -> Result<ConstValue, ConstError> {
        match (self.fold(left)?, self.fold(right)?) {
            (ConstValue::Int(a), ConstValue::Int(b)) => fold_integer(op, a, b, span),
            (ConstValue::Float(a), ConstValue::Float(b)) => fold_float(op, a, b, span),
            (ConstValue::Bool(a), ConstValue::Bool(b)) => match op {
                BinaryOp::Eq => Ok(ConstValue::Bool(a == b)),
                BinaryOp::Ne => Ok(ConstValue::Bool(a != b)),
                BinaryOp::LogicalAnd => Ok(ConstValue::Bool(a && b)),
                BinaryOp::LogicalOr => Ok(ConstValue::Bool(a || b)),
                _ => Err(ConstError::NotConstant(span)),
            },
            _ => Err(ConstError::NotConstant(span)),
        }
    }

    /// Folds a scalar cast over its constant operand.
    fn fold_cast(
        &mut self,
        ty: &Type,
        value: &Expr,
        span: Span,
    ) -> Result<ConstValue, ConstError> {
        let target = resolve_ty(ty, &[], self.table);
        let value = self.fold(value)?;
        match (&target.kind, value) {
            (TyKind::Scalar(scalar), ConstValue::Int(v)) if is_integer_scalar(*scalar) => {
                Ok(ConstValue::Int(v))
            }
            (TyKind::Scalar(scalar), ConstValue::Float(v)) if is_integer_scalar(*scalar) => {
                Ok(ConstValue::Int(v as i128))
            }
            (TyKind::Scalar(scalar), ConstValue::Int(v)) if is_float_scalar(*scalar) => {
                Ok(ConstValue::Float(v as f64))
            }
            (TyKind::Scalar(scalar), ConstValue::Float(v)) if is_float_scalar(*scalar) => {
                Ok(ConstValue::Float(v))
            }
            _ => Err(ConstError::NotConstant(span)),
        }
    }

    /// Resolves a name in a constant context.
    fn fold_name(&mut self, name: &str, span: Span) -> Result<ConstValue, ConstError> {
        let Some(decl) = self.module.decl(name) else {
            // A local binding the folder cannot see. The name stage validated
            // it, so it is accepted as an opaque constant rather than guessed
            // to be a runtime value.
            return Ok(ConstValue::Int(0));
        };
        match decl.kind {
            DeclKind::Const => self
                .fold_module_const(name)
                .ok_or(ConstError::NotConstant(span)),
            _ => Err(ConstError::NotConstant(span)),
        }
    }

    /// Folds a module-level `const`, guarding against a self-reference.
    fn fold_module_const(&mut self, name: &str) -> Option<ConstValue> {
        if let Some(value) = self.values.get(name) {
            return Some(*value);
        }
        if !self.active.insert(name.to_owned()) {
            return None;
        }
        let module: &'a Module = self.module;
        let initializer = module.items.iter().find_map(|item| match item {
            Item::GlobalConst(decl) if decl.name == name => Some(&decl.value),
            _ => None,
        });
        let folded = initializer.and_then(|expr| self.fold(expr).ok());
        self.active.remove(name);
        if let Some(value) = folded {
            self.values.insert(name.to_owned(), value);
        }
        folded
    }

    /// Folds the `sizeof`, `offsetof`, and `len` intrinsics whose operand is
    /// known at compile time.
    fn fold_intrinsic(
        &mut self,
        name: &str,
        args: &[IntrinsicArg],
        span: Span,
    ) -> Result<ConstValue, ConstError> {
        match name {
            "sizeof" => {
                let Some(IntrinsicArg::Type(ty)) = args.first() else {
                    return Err(ConstError::NotConstant(span));
                };
                let resolved = resolve_ty(ty, &[], self.table);
                self.table
                    .size_of(&resolved)
                    .map(|size| ConstValue::Int(size as i128))
                    .ok_or(ConstError::NotConstant(span))
            }
            "offsetof" => {
                let Some(IntrinsicArg::Type(ty)) = args.first() else {
                    return Err(ConstError::NotConstant(span));
                };
                let resolved = resolve_ty(ty, &[], self.table);
                let TyKind::Struct(struct_name) = resolved.kind else {
                    return Err(ConstError::NotConstant(span));
                };
                let Some(IntrinsicArg::Expr(field)) = args.get(1) else {
                    return Err(ConstError::NotConstant(span));
                };
                let ExprKind::Name(field_name) = &field.kind else {
                    return Err(ConstError::NotConstant(span));
                };
                self.table
                    .offset_of(&struct_name, field_name)
                    .map(|offset| ConstValue::Int(offset as i128))
                    .ok_or(ConstError::NotConstant(span))
            }
            "len" => {
                let Some(IntrinsicArg::Expr(inner)) = args.first() else {
                    return Err(ConstError::NotConstant(span));
                };
                self.fold_len(inner, span)
            }
            _ => Err(ConstError::NotConstant(span)),
        }
    }

    /// Folds `len` of a fixed literal, string literal, or module array whose
    /// extent is known.
    fn fold_len(&mut self, expr: &Expr, span: Span) -> Result<ConstValue, ConstError> {
        match &expr.kind {
            ExprKind::ArrayLiteral { elements, .. } => Ok(ConstValue::Int(elements.len() as i128)),
            ExprKind::Str(bytes) => Ok(ConstValue::Int(bytes.len() as i128)),
            ExprKind::Name(name) => {
                let global = self.table.global(&self.module.name, name).cloned();
                match global.map(|ty| ty.kind) {
                    Some(TyKind::Array { extents, .. }) => match extents.first() {
                        Some(Some(extent)) => Ok(ConstValue::Int(*extent as i128)),
                        _ => Err(ConstError::NotConstant(span)),
                    },
                    _ => Err(ConstError::NotConstant(span)),
                }
            }
            _ => Err(ConstError::NotConstant(span)),
        }
    }
}

/// Folds a binary operator over two integers with runtime semantics.
fn fold_integer(op: BinaryOp, a: i128, b: i128, span: Span) -> Result<ConstValue, ConstError> {
    match op {
        BinaryOp::Add => Ok(ConstValue::Int(a.wrapping_add(b))),
        BinaryOp::Sub => Ok(ConstValue::Int(a.wrapping_sub(b))),
        BinaryOp::Mul => Ok(ConstValue::Int(a.wrapping_mul(b))),
        BinaryOp::Div => a
            .checked_div(b)
            .map(ConstValue::Int)
            .ok_or(ConstError::DivisionByZero(span)),
        BinaryOp::Rem => a
            .checked_rem(b)
            .map(ConstValue::Int)
            .ok_or(ConstError::DivisionByZero(span)),
        BinaryOp::BitAnd => Ok(ConstValue::Int(a & b)),
        BinaryOp::BitOr => Ok(ConstValue::Int(a | b)),
        BinaryOp::BitXor => Ok(ConstValue::Int(a ^ b)),
        BinaryOp::Shl => Ok(ConstValue::Int(a.wrapping_shl(b as u32))),
        BinaryOp::Shr => Ok(ConstValue::Int(a.wrapping_shr(b as u32))),
        BinaryOp::Eq => Ok(ConstValue::Bool(a == b)),
        BinaryOp::Ne => Ok(ConstValue::Bool(a != b)),
        BinaryOp::Lt => Ok(ConstValue::Bool(a < b)),
        BinaryOp::Le => Ok(ConstValue::Bool(a <= b)),
        BinaryOp::Gt => Ok(ConstValue::Bool(a > b)),
        BinaryOp::Ge => Ok(ConstValue::Bool(a >= b)),
        BinaryOp::LogicalAnd | BinaryOp::LogicalOr => Err(ConstError::NotConstant(span)),
    }
}

/// Folds a binary operator over two floats. A float division by zero yields an
/// infinity rather than an error, matching the defined float behavior.
fn fold_float(op: BinaryOp, a: f64, b: f64, span: Span) -> Result<ConstValue, ConstError> {
    match op {
        BinaryOp::Add => Ok(ConstValue::Float(a + b)),
        BinaryOp::Sub => Ok(ConstValue::Float(a - b)),
        BinaryOp::Mul => Ok(ConstValue::Float(a * b)),
        BinaryOp::Div => Ok(ConstValue::Float(a / b)),
        BinaryOp::Eq => Ok(ConstValue::Bool(a == b)),
        BinaryOp::Ne => Ok(ConstValue::Bool(a != b)),
        BinaryOp::Lt => Ok(ConstValue::Bool(a < b)),
        BinaryOp::Le => Ok(ConstValue::Bool(a <= b)),
        BinaryOp::Gt => Ok(ConstValue::Bool(a > b)),
        BinaryOp::Ge => Ok(ConstValue::Bool(a >= b)),
        _ => Err(ConstError::NotConstant(span)),
    }
}
