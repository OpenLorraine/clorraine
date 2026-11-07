//! Constant expression evaluation.
//!
//! A constant context is an array extent, an enum member value, a `case`
//! label, or a module-level `const` initializer. In those positions the value
//! must be computable at compile time, and the folding here follows the
//! runtime arithmetic rules: each integer operation wraps at its own result
//! type, a shift count is masked to the left operand's width, and the declared
//! context type is the fallback a bare literal takes. A form the folder cannot
//! compute is `E10224`; an integer division or remainder by zero, including the
//! minimum signed value over `-1`, is `E10223`.
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
    int_literal_value, integer_scalar, is_float_scalar, is_integer_scalar, is_signed_scalar,
    resolve_ty_with, scalar_bits, Ty, TyKind, TypeTable,
};

mod arithmetic;
use arithmetic::{
    binary_operand_scalar, float_result_scalar, fold_float, fold_integer, result_scalar,
    round_float, usize_value, wrap_int, FoldValue,
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
    /// A constant float-to-integer conversion that is NaN or outside the
    /// target type's range. The target scalar supplies the E10202 message.
    FloatOutOfRange(Span, ScalarType),
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
    values: BTreeMap<String, FoldValue>,
    /// Enum member sequences, cached by enum name.
    enum_values: BTreeMap<String, Vec<(String, i128)>>,
    /// True when array extents are being folded.
    ///
    /// An extent must equal the C type the emitter spells, so `sizeof` here
    /// measures through the target ABI instead of the declared layout; every
    /// other fold keeps the declared sizes.
    extents: bool,
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
            enum_values: BTreeMap::new(),
            extents: false,
        }
    }

    /// Creates a folder for array extents, whose `sizeof` uses the target ABI.
    ///
    /// The value-position folds keep the declared widths; only an extent
    /// measures the size the emitted C type has on the target.
    pub fn for_extents(table: &'a TypeTable, module: &'a Module, loaded: &'a Loaded) -> Self {
        Self {
            extents: true,
            ..Self::new(table, module, loaded)
        }
    }

    /// Folds an expression to its constant value without a context type.
    pub fn fold(&mut self, expr: &Expr) -> Result<ConstValue, ConstError> {
        self.fold_value(expr, None).map(|folded| folded.value)
    }

    /// Folds an expression that must be constant, reporting `E10224` or
    /// `E10223` on failure.
    pub fn require(&mut self, expr: &Expr, diagnostics: &mut Diagnostics) {
        let result = self.fold_value(expr, None);
        report(result.err(), diagnostics);
    }

    /// Folds a constant-context expression, reporting `E10224` or `E10223` on
    /// failure.
    pub fn require_at(
        &mut self,
        expr: &Expr,
        context: Option<ScalarType>,
        diagnostics: &mut Diagnostics,
    ) {
        let result = self.fold_value(expr, context);
        report(result.err(), diagnostics);
    }

    /// The integer value of an expression, when it folds to one.
    pub fn integer(&mut self, expr: &Expr) -> Option<i128> {
        match self.fold_value(expr, None) {
            Ok(FoldValue {
                value: ConstValue::Int(value),
                ..
            }) => Some(value),
            _ => None,
        }
    }

    /// The integer value of a constant-context expression, when it folds.
    pub fn integer_at(&mut self, expr: &Expr, context: Option<ScalarType>) -> Option<i128> {
        match self.fold_value(expr, context) {
            Ok(FoldValue {
                value: ConstValue::Int(value),
                ..
            }) => Some(value),
            _ => None,
        }
    }

    /// Folds an expression at an integer width derived from a declared type.
    pub fn integer_in(&mut self, expr: &Expr, expected: &Ty) -> Option<i128> {
        let context = integer_scalar(expected);
        self.integer_at(expr, context)
    }

    /// Folds an expression, carrying the scalar type each result evaluates at.
    ///
    /// `context` is the integer type the constant context requires; a bare
    /// literal takes it. `None` evaluates in the wide accumulator, which has
    /// no wrap width. An operation replaces the context with its own result
    /// type, so a narrower operation wraps and faults at that narrower width.
    fn fold_value(
        &mut self,
        expr: &Expr,
        context: Option<ScalarType>,
    ) -> Result<FoldValue, ConstError> {
        match &expr.kind {
            ExprKind::Int(value) => Ok(FoldValue {
                scalar: context,
                value: ConstValue::Int(*value as i128),
            }),
            ExprKind::Bool(value) => Ok(FoldValue {
                scalar: None,
                value: ConstValue::Bool(*value),
            }),
            ExprKind::Char(value) => Ok(FoldValue {
                scalar: Some(ScalarType::U8),
                value: ConstValue::Int(*value as i128),
            }),
            ExprKind::Float(value) => Ok(FoldValue {
                scalar: Some(ScalarType::F64),
                value: ConstValue::Float(*value),
            }),
            ExprKind::Str(_) => Ok(FoldValue {
                scalar: None,
                value: ConstValue::Str,
            }),
            ExprKind::Unary { op, operand } => self.fold_unary(*op, operand, expr.span, context),
            ExprKind::Binary { op, left, right } => {
                self.fold_binary(*op, left, right, expr.span, context)
            }
            ExprKind::Cast { ty, value } => self.fold_cast(ty, value, expr.span),
            ExprKind::RawCast { value, .. } => self.fold_raw_cast(value, expr.span),
            ExprKind::Name(name) => self.fold_name(name, expr.span),
            ExprKind::Qualified(name) => self.fold_qualified(name, expr.span),
            ExprKind::IntrinsicCall { name, args } => self.fold_intrinsic(name, args, expr.span),
            _ => Err(ConstError::NotConstant(expr.span)),
        }
    }

    /// Folds a unary operator over its constant operand.
    ///
    /// The result keeps the operand's own type, falling back to the context
    /// for a bare literal, so the negation and bitwise complement wrap there.
    fn fold_unary(
        &mut self,
        op: UnaryOp,
        operand: &Expr,
        span: Span,
        context: Option<ScalarType>,
    ) -> Result<FoldValue, ConstError> {
        let operand = self.fold_value(operand, context)?;
        let scalar = operand.scalar.or(context);
        match (op, operand.value) {
            (UnaryOp::Neg, ConstValue::Int(value)) => Ok(FoldValue {
                scalar,
                value: ConstValue::Int(wrap_int(value.wrapping_neg(), scalar)),
            }),
            (UnaryOp::Neg, ConstValue::Float(value)) => Ok(FoldValue {
                scalar,
                value: ConstValue::Float(-value),
            }),
            (UnaryOp::BitNot, ConstValue::Int(value)) => Ok(FoldValue {
                scalar,
                value: ConstValue::Int(wrap_int(!value, scalar)),
            }),
            (UnaryOp::Not, ConstValue::Bool(value)) => Ok(FoldValue {
                scalar: None,
                value: ConstValue::Bool(!value),
            }),
            _ => Err(ConstError::NotConstant(span)),
        }
    }

    /// Folds a binary operator over its constant operands.
    ///
    /// Each operand keeps its own type; a bare integer literal adopts the other
    /// operand's type when it has one, matching the runtime operand adaptation.
    /// The result of an integer operation is wrapped, and a division or shift
    /// faulted, at the operation's own result type rather than the context.
    fn fold_binary(
        &mut self,
        op: BinaryOp,
        left_expr: &Expr,
        right_expr: &Expr,
        span: Span,
        context: Option<ScalarType>,
    ) -> Result<FoldValue, ConstError> {
        let mut left = self.fold_value(left_expr, context)?;
        let mut right = self.fold_value(right_expr, context)?;
        if int_literal_value(left_expr).is_some() {
            if let Some(scalar) = right.scalar {
                left.scalar = Some(scalar);
            }
        }
        if int_literal_value(right_expr).is_some() {
            if let Some(scalar) = left.scalar {
                right.scalar = Some(scalar);
            }
        }
        match (left.value, right.value) {
            (ConstValue::Int(a), ConstValue::Int(b)) => {
                let operand = binary_operand_scalar(op, left.scalar, right.scalar);
                let value = fold_integer(op, a, b, span, operand)?;
                Ok(FoldValue {
                    scalar: result_scalar(op, operand),
                    value,
                })
            }
            (ConstValue::Float(a), ConstValue::Float(b)) => {
                let value = fold_float(op, a, b, span)?;
                let scalar = match value {
                    ConstValue::Bool(_) => None,
                    _ => float_result_scalar(left.scalar, right.scalar),
                };
                let value = match value {
                    ConstValue::Float(v) => ConstValue::Float(round_float(v, scalar)),
                    other => other,
                };
                Ok(FoldValue { scalar, value })
            }
            (ConstValue::Bool(a), ConstValue::Bool(b)) => {
                let value = match op {
                    BinaryOp::Eq => ConstValue::Bool(a == b),
                    BinaryOp::Ne => ConstValue::Bool(a != b),
                    BinaryOp::LogicalAnd => ConstValue::Bool(a && b),
                    BinaryOp::LogicalOr => ConstValue::Bool(a || b),
                    _ => return Err(ConstError::NotConstant(span)),
                };
                Ok(FoldValue { scalar: None, value })
            }
            _ => Err(ConstError::NotConstant(span)),
        }
    }

    /// Folds a scalar cast over its constant operand.
    ///
    /// The operand folds at its own type, then the result wraps at the cast's
    /// target, so a cast truncates exactly the way the runtime cast does.
    fn fold_cast(
        &mut self,
        ty: &Type,
        value: &Expr,
        span: Span,
    ) -> Result<FoldValue, ConstError> {
        let target = self.resolve_const_type(ty);
        let inner = self.fold_value(value, None)?;
        match (&target.kind, inner.value) {
            (TyKind::Scalar(scalar), ConstValue::Int(v)) if is_integer_scalar(*scalar) => {
                Ok(FoldValue {
                    scalar: Some(*scalar),
                    value: ConstValue::Int(wrap_int(v, Some(*scalar))),
                })
            }
            (TyKind::Scalar(scalar), ConstValue::Float(v)) if is_integer_scalar(*scalar) => {
                let value = Self::float_to_int(v, *scalar, span)?;
                Ok(FoldValue {
                    scalar: Some(*scalar),
                    value: ConstValue::Int(value),
                })
            }
            (TyKind::Scalar(scalar), ConstValue::Int(v)) if is_float_scalar(*scalar) => {
                let rounded = round_float(v as f64, Some(*scalar));
                Ok(FoldValue {
                    scalar: Some(*scalar),
                    value: ConstValue::Float(rounded),
                })
            }
            (TyKind::Scalar(scalar), ConstValue::Float(v)) if is_float_scalar(*scalar) => {
                Ok(FoldValue {
                    scalar: Some(*scalar),
                    value: ConstValue::Float(round_float(v, Some(*scalar))),
                })
            }
            _ => Err(ConstError::NotConstant(span)),
        }
    }

    /// Converts a folded float to an integer at the cast's target type.
    ///
    /// The runtime guard rejects a value outside the target range before the
    /// C truncation, so the folder applies the same rule: `127.5` to `i8` is
    /// out of range even though `trunc` would be 127. A NaN or an infinity,
    /// and an out-of-range value, is E10202: the frozen rule treats a constant
    /// float-to-integer overflow as a constant-context fit error, not a
    /// runtime trap.
    fn float_to_int(value: f64, scalar: ScalarType, span: Span) -> Result<i128, ConstError> {
        if !value.is_finite() {
            return Err(ConstError::FloatOutOfRange(span, scalar));
        }
        let bits = scalar_bits(scalar);
        let (low, high) = if is_signed_scalar(scalar) {
            (-(1i128 << (bits - 1)), (1i128 << (bits - 1)) - 1)
        } else {
            (0, (1i128 << bits) - 1)
        };
        if value < low as f64 || value > high as f64 {
            return Err(ConstError::FloatOutOfRange(span, scalar));
        }
        Ok(value.trunc() as i128)
    }

    /// Folds a constant `raw` cast of an integer address.
    ///
    /// A module-level register base is written `raw T(0xADDRESS)`. The address
    /// itself is the constant; the raw target only names how the pointer is
    /// typed, so the fold keeps the inner integer value. A non-integer operand
    /// is not a constant address.
    fn fold_raw_cast(&mut self, value: &Expr, span: Span) -> Result<FoldValue, ConstError> {
        let inner = self.fold_value(value, None)?;
        match inner.value {
            ConstValue::Int(address) => Ok(FoldValue {
                scalar: None,
                value: ConstValue::Int(address),
            }),
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
    fn fold_name(&mut self, name: &str, span: Span) -> Result<FoldValue, ConstError> {
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
    fn fold_qualified(&mut self, name: &QualifiedName, span: Span) -> Result<FoldValue, ConstError> {
        if name.segments.len() >= 2 {
            let first = &name.segments[0];
            if self.table.is_enum(first) {
                let member = name.segments.last().expect("a qualified name");
                return self
                    .fold_enum_member(first, member)
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
    /// The declared type supplies the width the initializer folds at, and a
    /// reference to the constant carries that declared type: the initializer's
    /// own narrower type has already done its wrapping. The active set and the
    /// value cache both use a module-qualified key, so two modules may declare
    /// the same constant name and a cycle is caught.
    fn fold_module_const(&mut self, module_name: &str, name: &str) -> Option<FoldValue> {
        // The cache key carries the `sizeof` layout policy: a value-position
        // fold and an enum-member or extent fold can see different sizes for
        // the same constant, and the cached result must not cross between them.
        let layout = if self.extents { "abi" } else { "decl" };
        let key = format!("@{layout}@{module_name}::{name}");
        if let Some(value) = self.values.get(&key) {
            return Some(*value);
        }
        if !self.active.insert(key.clone()) {
            return None;
        }
        let loaded: &'a Loaded = self.loaded;
        let module: &'a Module = if module_name == self.module.name {
            self.module
        } else if let Some(module) = loaded.module(module_name) {
            module
        } else {
            // Remove the cycle key before the early return, or a failed lookup
            // would poison every later reference to the same constant.
            self.active.remove(&key);
            return None;
        };
        // The initializer folds in its declaring module, so a chain such as
        // `NEXT = BASE + 1` resolves `BASE` where it is declared even when the
        // reference comes from another module.
        let saved_module = self.module;
        self.module = module;
        let initializer = module.items.iter().find_map(|item| match item {
            Item::GlobalConst(decl) if decl.name == name => Some(&decl.value),
            _ => None,
        });
        let declared = self.table.global(module_name, name).cloned();
        let context = declared.as_ref().and_then(integer_scalar);
        let folded = initializer.and_then(|expr| {
            self.fold_value(expr, context).ok().map(|mut folded| {
                if let Some(TyKind::Scalar(scalar)) = declared.as_ref().map(|ty| &ty.kind) {
                    if is_float_scalar(*scalar) {
                        folded.value = match folded.value {
                            ConstValue::Float(value) => {
                                ConstValue::Float(round_float(value, Some(*scalar)))
                            }
                            ConstValue::Int(value) => {
                                ConstValue::Float(round_float(value as f64, Some(*scalar)))
                            }
                            other => other,
                        };
                        folded.scalar = Some(*scalar);
                    } else if context.is_some() {
                        folded.scalar = context;
                    }
                } else if context.is_some() {
                    folded.scalar = context;
                }
                folded
            })
        });
        self.module = saved_module;
        self.active.remove(&key);
        if let Some(value) = folded {
            self.values.insert(key, value);
        }
        folded
    }

    /// Returns every member value of an enum in declaration order.
    ///
    /// The values are held in the wide accumulator. An explicit value folds at
    /// the backing type; a member with no explicit value continues from the
    /// previous value plus one. No wrap is applied here: a value that does not
    /// fit the backing type is returned so the declaration check can report
    /// E10202, and an accepted program never reaches emission with one. A
    /// non-constant explicit value returns `None`; the checker has already
    /// reported it.
    pub fn enum_member_values(&mut self, enum_name: &str) -> Option<Vec<(String, i128)>> {
        if let Some(values) = self.enum_values.get(enum_name) {
            return Some(values.clone());
        }
        let key = format!("#{enum_name}::@values");
        if !self.active.insert(key.clone()) {
            return None;
        }
        let context = self.table.enum_backing(enum_name).and_then(integer_scalar);
        let mut result = None;
        if let Some((module_name, declaration)) = Self::find_enum(self.loaded, enum_name) {
            if let Some(declaring) = self.loaded.module(module_name) {
                // A member expression is evaluated in the enum's declaring
                // module and with the target ABI layout, because the emitted
                // macro must agree with the runtime `sizeof` the C compiler
                // spells. The emitter uses the same rule.
                let saved_module = self.module;
                let saved_extents = self.extents;
                self.module = declaring;
                self.extents = true;
                let mut current: i128 = 0;
                let mut values = Vec::with_capacity(declaration.members.len());
                let mut complete = true;
                for declared in &declaration.members {
                    let value = match &declared.value {
                        Some(expr) => match self.fold_value(expr, context) {
                            Ok(FoldValue {
                                value: ConstValue::Int(value),
                                ..
                            }) => value,
                            _ => {
                                complete = false;
                                break;
                            }
                        },
                        None => current,
                    };
                    values.push((declared.name.clone(), value));
                    // Cache the computed prefix so a later member can read an
                    // earlier member (`C = A | B`) without re-entering the
                    // walk. A forward or self reference has no entry and still
                    // hits the active guard.
                    self.values.insert(
                        format!("#{enum_name}::{}", declared.name),
                        FoldValue {
                            scalar: context,
                            value: ConstValue::Int(value),
                        },
                    );
                    current = value.wrapping_add(1);
                }
                self.extents = saved_extents;
                self.module = saved_module;
                if complete {
                    result = Some(values);
                }
            }
        }
        self.active.remove(&key);
        if let Some(values) = &result {
            self.enum_values.insert(enum_name.to_owned(), values.clone());
        }
        result
    }

    /// Folds one enum member through the shared member walk.
    ///
    /// A computed member is read from the walk's cache, so a backward
    /// reference does not start a nested walk. The value is returned even when
    /// it is outside the backing type: the declaration's own fit check is the
    /// only place that reports E10202, and returning the raw value lets a
    /// later member read an earlier out-of-range member without a false
    /// "not constant" cascade.
    fn fold_enum_member(&mut self, enum_name: &str, member: &str) -> Option<FoldValue> {
        if let Some(cached) = self.values.get(&format!("#{enum_name}::{member}")).copied() {
            if let ConstValue::Int(value) = cached.value {
                let context = self.table.enum_backing(enum_name).and_then(integer_scalar);
                return Some(FoldValue {
                    scalar: context,
                    value: ConstValue::Int(value),
                });
            }
        }
        let values = self.enum_member_values(enum_name)?;
        let value = values.iter().find(|(name, _)| name == member)?.1;
        let context = self.table.enum_backing(enum_name).and_then(integer_scalar);
        Some(FoldValue {
            scalar: context,
            value: ConstValue::Int(value),
        })
    }

    /// Finds an enum declaration and its declaring module by name.
    fn find_enum(
        loaded: &'a Loaded,
        enum_name: &str,
    ) -> Option<(&'a str, &'a crate::ast::EnumDecl)> {
        for module in loaded.modules.values() {
            for item in &module.items {
                if let Item::Enum(decl) = item {
                    if decl.name == enum_name {
                        return Some((module.name.as_str(), decl));
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
    ) -> Result<FoldValue, ConstError> {
        match name {
            "sizeof" => {
                let Some(IntrinsicArg::Type(ty)) = args.first() else {
                    return Err(ConstError::NotConstant(span));
                };
                let resolved = self.resolve_const_type(ty);
                let size = if self.extents {
                    self.table.abi_size_of(&resolved)
                } else {
                    self.table.size_of(&resolved)
                }
                .ok_or(ConstError::NotConstant(span))?;
                Ok(usize_value(size))
            }
            "offsetof" => {
                let Some(IntrinsicArg::Type(ty)) = args.first() else {
                    return Err(ConstError::NotConstant(span));
                };
                let resolved = self.resolve_const_type(ty);
                if !matches!(resolved.kind, TyKind::Struct(..)) {
                    return Err(ConstError::NotConstant(span));
                }
                let Some(IntrinsicArg::Expr(field)) = args.get(1) else {
                    return Err(ConstError::NotConstant(span));
                };
                let ExprKind::Name(field_name) = &field.kind else {
                    return Err(ConstError::NotConstant(span));
                };
                let offset = self
                    .table
                    .abi_offset_of(&resolved, field_name)
                    .ok_or(ConstError::NotConstant(span))?;
                Ok(usize_value(offset))
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
    fn fold_len(&mut self, expr: &Expr, span: Span) -> Result<FoldValue, ConstError> {
        match &expr.kind {
            ExprKind::ArrayLiteral { elements, .. } => Ok(usize_value(elements.len())),
            ExprKind::Str(bytes) => Ok(usize_value(bytes.len())),
            ExprKind::Name(name) => {
                let global = self.table.global(&self.module.name, name).cloned();
                match global.map(|ty| ty.kind) {
                    Some(TyKind::Array { extents, .. }) => match extents.first() {
                        Some(Some(extent)) => Ok(usize_value(*extent)),
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
        Some(ConstError::FloatOutOfRange(span, scalar)) => {
            let target = Ty::scalar(scalar);
            diagnostics.push(
                Diagnostic::error(
                    "E10202",
                    format!("literal does not fit '{}'", target.display()),
                    span,
                )
                .with_help("use a wider type or a hex/binary form"),
            );
        }
    }
}

#[cfg(test)]
mod regressions;
#[cfg(test)]
mod tests;
