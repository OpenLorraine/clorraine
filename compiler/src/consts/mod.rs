//! Constant expression evaluation.
//!
//! A constant context is an array extent, an enum member value, a `case`
//! label, or a module-level `const` initializer. In those positions the value
//! must be computable at compile time, and the folding here follows the
//! runtime arithmetic rules: integer operations wrap at the width of the
//! context type, and shift counts are masked to that width. A form the folder
//! cannot compute is `E10224`; an integer division or remainder by zero,
//! including the minimum signed value over `-1`, is `E10223`.
//!
//! The folder only sees module-level names. A name that is a module-level
//! `const`, or a `const` imported from another module, folds from its
//! initializer, with a cycle guard for a malformed self-reference; a
//! module-level `let` or a function is not constant. A name the folder cannot
//! tie to a constant declaration is not constant, so a runtime local used as
//! an extent is reported rather than guessed at.

use std::collections::{BTreeMap, BTreeSet};

use crate::ast::{BinaryOp, Expr, ExprKind, IntrinsicArg, Item, QualifiedName, ScalarType, Type, UnaryOp};
use crate::diag::{Diagnostic, Diagnostics};
use crate::load::{DeclKind, Loaded, Module};
use crate::span::Span;
use crate::types::{
    integer_scalar, is_float_scalar, is_integer_scalar, is_signed_scalar, resolve_ty_with,
    scalar_bits, Ty, TyKind, TypeTable,
};

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
    /// The module whose `const` declarations the folder reads first.
    module: &'a Module,
    /// Every loaded module, so an imported `const` or an enum member declared
    /// in another module can still fold.
    loaded: &'a Loaded,
    /// Constants currently being folded, so a cycle stops instead of looping.
    active: BTreeSet<String>,
    /// Folded values, cached under a module- or enum-qualified key.
    values: BTreeMap<String, ConstValue>,
}

impl<'a> ConstFolder<'a> {
    /// Creates a folder for one module.
    pub fn new(table: &'a TypeTable, module: &'a Module, loaded: &'a Loaded) -> Self {
        Self {
            table,
            module,
            loaded,
            active: BTreeSet::new(),
            values: BTreeMap::new(),
        }
    }

    /// Folds an expression to its constant value without a context type.
    pub fn fold(&mut self, expr: &Expr) -> Result<ConstValue, ConstError> {
        self.fold_with(expr, None)
    }

    /// Folds an expression that must be constant, reporting `E10224` or
    /// `E10223` on failure.
    pub fn require(&mut self, expr: &Expr, diagnostics: &mut Diagnostics) {
        let result = self.fold_with(expr, None);
        report(result.err(), diagnostics);
    }

    /// Folds a constant-context expression at its declared width, reporting
    /// `E10224` or `E10223` on failure.
    pub fn require_at(
        &mut self,
        expr: &Expr,
        context: Option<ScalarType>,
        diagnostics: &mut Diagnostics,
    ) {
        let result = self.fold_with(expr, context);
        report(result.err(), diagnostics);
    }

    /// The integer value of an expression, when it folds to one.
    pub fn integer(&mut self, expr: &Expr) -> Option<i128> {
        match self.fold_with(expr, None) {
            Ok(ConstValue::Int(value)) => Some(value),
            _ => None,
        }
    }

    /// The integer value of a constant-context expression, when it folds.
    pub fn integer_at(&mut self, expr: &Expr, context: Option<ScalarType>) -> Option<i128> {
        match self.fold_with(expr, context) {
            Ok(ConstValue::Int(value)) => Some(value),
            _ => None,
        }
    }

    /// Folds an expression at an integer width derived from a declared type.
    pub fn integer_in(&mut self, expr: &Expr, expected: &Ty) -> Option<i128> {
        let context = integer_scalar(expected);
        self.integer_at(expr, context)
    }

    /// Folds an expression, wrapping each integer operation at `context`.
    ///
    /// `context` is the integer type the constant context requires; `None`
    /// evaluates in the wide accumulator without a wrap width.
    fn fold_with(
        &mut self,
        expr: &Expr,
        context: Option<ScalarType>,
    ) -> Result<ConstValue, ConstError> {
        match &expr.kind {
            ExprKind::Int(value) => Ok(ConstValue::Int(*value as i128)),
            ExprKind::Bool(value) => Ok(ConstValue::Bool(*value)),
            ExprKind::Char(value) => Ok(ConstValue::Int(*value as i128)),
            ExprKind::Float(value) => Ok(ConstValue::Float(*value)),
            ExprKind::Str(_) => Ok(ConstValue::Str),
            ExprKind::Unary { op, operand } => self.fold_unary(*op, operand, expr.span, context),
            ExprKind::Binary { op, left, right } => {
                self.fold_binary(*op, left, right, expr.span, context)
            }
            ExprKind::Cast { ty, value } => self.fold_cast(ty, value, expr.span),
            ExprKind::Name(name) => self.fold_name(name, expr.span),
            ExprKind::Qualified(name) => self.fold_qualified(name, expr.span),
            ExprKind::IntrinsicCall { name, args } => self.fold_intrinsic(name, args, expr.span),
            _ => Err(ConstError::NotConstant(expr.span)),
        }
    }

    /// Folds a unary operator over its constant operand.
    fn fold_unary(
        &mut self,
        op: UnaryOp,
        operand: &Expr,
        span: Span,
        context: Option<ScalarType>,
    ) -> Result<ConstValue, ConstError> {
        match (op, self.fold_with(operand, context)?) {
            (UnaryOp::Neg, ConstValue::Int(value)) => {
                Ok(ConstValue::Int(wrap_int(value.wrapping_neg(), context)))
            }
            (UnaryOp::Neg, ConstValue::Float(value)) => Ok(ConstValue::Float(-value)),
            (UnaryOp::BitNot, ConstValue::Int(value)) => {
                Ok(ConstValue::Int(wrap_int(!value, context)))
            }
            (UnaryOp::Not, ConstValue::Bool(value)) => Ok(ConstValue::Bool(!value)),
            _ => Err(ConstError::NotConstant(span)),
        }
    }

    /// Folds a binary operator over its constant operands.
    ///
    /// Both operands are folded at the context width, and an integer result is
    /// wrapped at that width, matching the runtime operator rules.
    fn fold_binary(
        &mut self,
        op: BinaryOp,
        left: &Expr,
        right: &Expr,
        span: Span,
        context: Option<ScalarType>,
    ) -> Result<ConstValue, ConstError> {
        match (
            self.fold_with(left, context)?,
            self.fold_with(right, context)?,
        ) {
            (ConstValue::Int(a), ConstValue::Int(b)) => fold_integer(op, a, b, span, context),
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
    ///
    /// The operand is folded at the cast's target width, so a nested operation
    /// wraps the way the cast's type does.
    fn fold_cast(
        &mut self,
        ty: &Type,
        value: &Expr,
        span: Span,
    ) -> Result<ConstValue, ConstError> {
        let target = self.resolve_const_type(ty);
        let context = integer_scalar(&target);
        let value = self.fold_with(value, context)?;
        match (&target.kind, value) {
            (TyKind::Scalar(scalar), ConstValue::Int(v)) if is_integer_scalar(*scalar) => {
                Ok(ConstValue::Int(wrap_int(v, Some(*scalar))))
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

    /// Resolves an AST type, folding constant array extents with this folder.
    fn resolve_const_type(&mut self, ty: &Type) -> Ty {
        let table = self.table;
        resolve_ty_with(ty, &[], table, &mut |expr| {
            self.integer_at(expr, Some(ScalarType::Usize))
        })
    }

    /// Resolves a name in a constant context.
    ///
    /// A module-level `const` folds from its initializer, an imported `const`
    /// folds from the declaration in its own module, and any other name is not
    /// constant. A local binding is invisible here, so it is deliberately not
    /// treated as an opaque constant.
    fn fold_name(&mut self, name: &str, span: Span) -> Result<ConstValue, ConstError> {
        let own_module = self.module.name.clone();
        let kind = self.module.decl(name).map(|decl| decl.kind);
        match kind {
            Some(DeclKind::Const) => self
                .fold_module_const(&own_module, name)
                .ok_or(ConstError::NotConstant(span)),
            Some(_) => Err(ConstError::NotConstant(span)),
            None => {
                let imported = self
                    .module
                    .import(name)
                    .map(|binding| (binding.poisoned, binding.module.clone(), binding.original.clone()));
                match imported {
                    Some((false, module, original)) => self
                        .fold_module_const(&module, &original)
                        .ok_or(ConstError::NotConstant(span)),
                    _ => Err(ConstError::NotConstant(span)),
                }
            }
        }
    }

    /// Folds a qualified name: an enum member or a module-qualified constant.
    ///
    /// Any other dotted name, such as a value's field access, is not constant.
    fn fold_qualified(&mut self, name: &QualifiedName, span: Span) -> Result<ConstValue, ConstError> {
        if name.segments.len() >= 2 {
            let first = &name.segments[0];
            if self.table.is_enum(first) {
                let member = name.segments.last().expect("a qualified name");
                return self
                    .fold_enum_member(first, member)
                    .map(ConstValue::Int)
                    .ok_or(ConstError::NotConstant(span));
            }
            if let Some((last, head)) = name.segments.split_last() {
                if !head.is_empty() {
                    let module = head.join(".");
                    if let Some(value) = self.fold_module_const(&module, last) {
                        return Ok(value);
                    }
                }
            }
        }
        Err(ConstError::NotConstant(span))
    }

    /// Folds a module-level `const` declared in the named module.
    ///
    /// The declared type supplies the width the initializer folds at. The
    /// active set and the value cache both use a module-qualified key, so two
    /// modules may declare the same constant name and a cycle is caught.
    fn fold_module_const(&mut self, module_name: &str, name: &str) -> Option<ConstValue> {
        let key = format!("@{module_name}::{name}");
        if let Some(value) = self.values.get(&key) {
            return Some(*value);
        }
        if !self.active.insert(key.clone()) {
            return None;
        }
        let loaded: &'a Loaded = self.loaded;
        let module: &'a Module = if module_name == self.module.name {
            self.module
        } else {
            loaded.module(module_name)?
        };
        let initializer = module.items.iter().find_map(|item| match item {
            Item::GlobalConst(decl) if decl.name == name => Some(&decl.value),
            _ => None,
        });
        let context = self
            .table
            .global(module_name, name)
            .and_then(integer_scalar);
        let folded = initializer.and_then(|expr| self.fold_with(expr, context).ok());
        self.active.remove(&key);
        if let Some(value) = folded {
            self.values.insert(key, value);
        }
        folded
    }

    /// Folds an enum member by walking the members in declaration order.
    ///
    /// A member with no explicit value continues from the previous value plus
    /// one, wrapped at the backing type; an explicit value folds in the backing
    /// type's width.
    fn fold_enum_member(&mut self, enum_name: &str, member: &str) -> Option<i128> {
        let key = format!("#{enum_name}::{member}");
        if let Some(value) = self.values.get(&key) {
            return match value {
                ConstValue::Int(value) => Some(*value),
                _ => None,
            };
        }
        if !self.active.insert(key.clone()) {
            return None;
        }
        let context = self.table.enum_backing(enum_name).and_then(integer_scalar);
        let mut result = None;
        if let Some(declaration) = self.find_enum(enum_name) {
            let mut current: i128 = 0;
            for declared in &declaration.members {
                let value = match &declared.value {
                    Some(expr) => match self.fold_with(expr, context) {
                        Ok(ConstValue::Int(value)) => value,
                        _ => break,
                    },
                    None => current,
                };
                self.values.insert(
                    format!("#{enum_name}::{}", declared.name),
                    ConstValue::Int(value),
                );
                current = wrap_int(value.wrapping_add(1), context);
                if declared.name == member {
                    result = Some(value);
                    break;
                }
            }
        }
        self.active.remove(&key);
        result
    }

    /// Finds an enum declaration by name in any loaded module.
    fn find_enum(&self, enum_name: &str) -> Option<&'a crate::ast::EnumDecl> {
        for module in self.loaded.modules.values() {
            for item in &module.items {
                if let Item::Enum(decl) = item {
                    if decl.name == enum_name {
                        return Some(decl);
                    }
                }
            }
        }
        None
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
                let resolved = self.resolve_const_type(ty);
                self.table
                    .size_of(&resolved)
                    .map(|size| ConstValue::Int(size as i128))
                    .ok_or(ConstError::NotConstant(span))
            }
            "offsetof" => {
                let Some(IntrinsicArg::Type(ty)) = args.first() else {
                    return Err(ConstError::NotConstant(span));
                };
                let resolved = self.resolve_const_type(ty);
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

/// Reports a failed constant fold, when the failure has a diagnostic.
fn report(error: Option<ConstError>, diagnostics: &mut Diagnostics) {
    match error {
        None => {}
        Some(ConstError::NotConstant(span)) => diagnostics.push(
            Diagnostic::error("E10224", "expression is not constant", span).with_help(
                "use literals, `const` bindings, enum members, and supported operators",
            ),
        ),
        Some(ConstError::DivisionByZero(span)) => diagnostics.push(
            Diagnostic::error("E10223", "constant expression divides by zero", span).with_help(
                "remove the zero divisor or the MIN/-1 operation, or use a non-constant operand",
            ),
        ),
    }
}

/// Wraps a wide value to the width and signedness of an integer context.
///
/// A `None` context leaves the value untouched in the wide accumulator.
fn wrap_int(value: i128, context: Option<ScalarType>) -> i128 {
    let Some(scalar) = context.filter(|scalar| is_integer_scalar(*scalar)) else {
        return value;
    };
    let bits = scalar_bits(scalar);
    if bits >= 128 {
        return value;
    }
    let modulus = 1i128 << bits;
    let mask = modulus - 1;
    let truncated = value & mask;
    if is_signed_scalar(scalar) && (truncated & (modulus >> 1)) != 0 {
        truncated - modulus
    } else {
        truncated
    }
}

/// The most negative value an integer context can hold, when it is signed.
fn signed_min(scalar: ScalarType) -> Option<i128> {
    match scalar {
        ScalarType::I8 => Some(i8::MIN as i128),
        ScalarType::I16 => Some(i16::MIN as i128),
        ScalarType::I32 | ScalarType::Isize => Some(i32::MIN as i128),
        _ => None,
    }
}

/// Folds a binary operator over two integers with runtime semantics.
///
/// `context` is the integer type the operation is evaluated at: it fixes the
/// wrap width, the shift-count mask, and the signed minimum over `-1`. Without
/// a context the wide accumulator's own rules apply.
fn fold_integer(
    op: BinaryOp,
    a: i128,
    b: i128,
    span: Span,
    context: Option<ScalarType>,
) -> Result<ConstValue, ConstError> {
    let wrap = |value: i128| ConstValue::Int(wrap_int(value, context));
    match op {
        BinaryOp::Add => Ok(wrap(a.wrapping_add(b))),
        BinaryOp::Sub => Ok(wrap(a.wrapping_sub(b))),
        BinaryOp::Mul => Ok(wrap(a.wrapping_mul(b))),
        BinaryOp::Div => {
            if let Some(min) = context.and_then(signed_min) {
                if a == min && b == -1 {
                    return Err(ConstError::DivisionByZero(span));
                }
            }
            a.checked_div(b)
                .map(wrap)
                .ok_or(ConstError::DivisionByZero(span))
        }
        BinaryOp::Rem => {
            if let Some(min) = context.and_then(signed_min) {
                if a == min && b == -1 {
                    return Err(ConstError::DivisionByZero(span));
                }
            }
            a.checked_rem(b)
                .map(wrap)
                .ok_or(ConstError::DivisionByZero(span))
        }
        BinaryOp::BitAnd => Ok(wrap(a & b)),
        BinaryOp::BitOr => Ok(wrap(a | b)),
        BinaryOp::BitXor => Ok(wrap(a ^ b)),
        BinaryOp::Shl => {
            let count = shift_count(b, context);
            Ok(wrap(a.wrapping_shl(count)))
        }
        BinaryOp::Shr => {
            let count = shift_count(b, context);
            Ok(wrap(a.wrapping_shr(count)))
        }
        BinaryOp::Eq => Ok(ConstValue::Bool(a == b)),
        BinaryOp::Ne => Ok(ConstValue::Bool(a != b)),
        BinaryOp::Lt => Ok(ConstValue::Bool(a < b)),
        BinaryOp::Le => Ok(ConstValue::Bool(a <= b)),
        BinaryOp::Gt => Ok(ConstValue::Bool(a > b)),
        BinaryOp::Ge => Ok(ConstValue::Bool(a >= b)),
        BinaryOp::LogicalAnd | BinaryOp::LogicalOr => Err(ConstError::NotConstant(span)),
    }
}

/// Masks a shift count to the width of the operand's type.
fn shift_count(count: i128, context: Option<ScalarType>) -> u32 {
    let width = context
        .filter(|scalar| is_integer_scalar(*scalar))
        .map(scalar_bits)
        .unwrap_or(128);
    (count as u32) & (width - 1)
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

#[cfg(test)]
mod regressions;
#[cfg(test)]
mod tests;
