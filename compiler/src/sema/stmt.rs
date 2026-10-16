//! Statement checking.
//!
//! Statements drive the scoped walk: every block opens a scope, every binding
//! is declared after its own initializer has been checked, and control-flow
//! conditions are required to be `bool`.

use crate::ast::{
    Block, ConstStmt, DoWhileStmt, ElseBranch, ForCStmt, ForInStmt, ForInit, IfStmt, LetStmt, Stmt,
    SwitchLabel, SwitchStmt,
};
use crate::types::{integer_scalar, Ty, TyKind};

use super::Checker;

impl Checker<'_, '_> {
    /// Checks a block in a fresh scope.
    pub(super) fn check_block(&mut self, block: &Block) {
        self.push_scope();
        for statement in &block.statements {
            self.check_stmt(statement);
        }
        self.pop_scope();
    }

    /// Checks one statement.
    pub(super) fn check_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let(decl) => self.check_let(decl),
            Stmt::Const(decl) => self.check_const(decl),
            Stmt::Expr(expr) => {
                self.check_expr(expr, None);
            }
            Stmt::If(decl) => self.check_if(decl),
            Stmt::While(decl) => {
                let condition = self.check_value(&decl.condition, None);
                self.require_bool(&condition, decl.condition.span);
                self.check_block(&decl.body);
            }
            Stmt::DoWhile(decl) => self.check_do_while(decl),
            Stmt::ForC(decl) => self.check_for_c(decl),
            Stmt::ForIn(decl) => self.check_for_in(decl),
            Stmt::Return { value, span } => {
                let returns = self.returns.clone();
                match value {
                    Some(value) => {
                        let value_ty = if returns.is_void() {
                            self.check_value(value, None)
                        } else {
                            self.check_into(value, &returns)
                        };
                        self.reject_callable_only_value(&value_ty, value.span);
                        // A closure cannot be returned: its environment dies
                        // with the enclosing frame, so the caller would receive
                        // a dangling callable. A concrete `func(...)` return is
                        // the E10400 case, already reported above.
                        if value_ty.is_closure() && !matches!(returns.kind, TyKind::Func(_)) {
                            self.reject_ephemeral_storage(&value_ty, value.span);
                        }
                        self.reject_capture_move(value);
                        self.record_generic_capture_move(value);
                        self.record_lambda_return(&value_ty);
                    }
                    // A block-bodied lambda infers its return type from its
                    // `return` statements, so a bare `return;` there is not the
                    // void-return error. Anywhere else it returns nothing where
                    // the declared type needs a value.
                    None if !returns.is_void() && self.lambda_returns.is_empty() => {
                        self.error(
                            "E10200",
                            format!("cannot convert '()' to '{}' implicitly", returns.display()),
                            "return a value of the declared type",
                            *span,
                        );
                    }
                    None => {}
                }
            }
            Stmt::Break(_) | Stmt::Continue(_) => {}
            Stmt::Switch(decl) => self.check_switch(decl),
            Stmt::Defer(block) | Stmt::Unsafe(block) | Stmt::Block(block) => self.check_block(block),
        }
    }

    /// Checks a `let` binding; the name is declared after its initializer.
    ///
    /// An overlay type has no by-value form, so a declared overlay type is
    /// rejected before its initializer is checked.
    fn check_let(&mut self, decl: &LetStmt) {
        let declared = decl.ty.as_ref().map(|ty| self.resolve_type(ty));
        if let Some(ty) = &declared {
            self.check_overlay_value(ty, decl.name_span);
        }
        match (&declared, &decl.value) {
            (Some(ty), Some(value)) => {
                let value_ty = self.check_into(value, ty);
                self.reject_callable_only_value(&value_ty, value.span);
                // A closure is accepted only where a callable bound consumes it
                // inline; storing one in a named binding is E10404. A concrete
                // `func(...)` target is the E10400 case, reported above.
                if value_ty.is_closure() && !matches!(ty.kind, TyKind::Func(_)) {
                    self.reject_ephemeral_storage(&value_ty, value.span);
                }
                self.declare(&decl.name, ty.clone());
            }
            (Some(ty), None) => self.declare(&decl.name, ty.clone()),
            (None, Some(value)) => {
                let ty = self.check_value(value, None);
                self.reject_callable_only_value(&ty, value.span);
                self.reject_ephemeral_storage(&ty, value.span);
                self.declare(&decl.name, ty);
            }
            (None, None) => self.declare(&decl.name, Ty::unknown()),
        }
        // A `let` initializer is a move site of its own: the value's move may
        // not escape (a local binding receives it), so the capture rule is
        // recorded here for the per-instance judgment.
        if let Some(value) = &decl.value {
            self.record_generic_capture_move(value);
        }
    }

    /// Reports `E10304` when a callable-only type-parameter value is stored or
    /// returned instead of called or passed on.
    ///
    /// A value of a parameter declared `F: func(...)` has no storable form: its
    /// concrete function is chosen only at the call, so binding it to a name or
    /// returning it would outlive that choice.
    fn reject_callable_only_value(&mut self, ty: &Ty, span: crate::span::Span) {
        if let TyKind::Param(name) = &ty.kind {
            if self.callable_bound_params.iter().any(|bound| bound == name) {
                self.error(
                    "E10304",
                    format!("value of type parameter '{name}' cannot be stored or returned"),
                    "call it or pass it on",
                    span,
                );
            }
        }
    }

    /// Checks a `const` binding, whose initializer is mandatory.
    fn check_const(&mut self, decl: &ConstStmt) {
        let declared = decl.ty.as_ref().map(|ty| self.resolve_type(ty));
        let ty = match declared {
            Some(ty) => {
                let value_ty = self.check_into(&decl.value, &ty);
                if value_ty.is_closure() && !matches!(ty.kind, TyKind::Func(_)) {
                    self.reject_ephemeral_storage(&value_ty, decl.value.span);
                }
                ty
            }
            None => {
                let ty = self.check_value(&decl.value, None);
                self.reject_ephemeral_storage(&ty, decl.value.span);
                ty
            }
        };
        // A `const` initializer is a move site of its own, exactly as a `let`
        // initializer is: the value's move may not escape, so the capture rule
        // is recorded here for the per-instance judgment.
        self.record_generic_capture_move(&decl.value);
        self.declare(&decl.name, ty);
    }

    /// Checks an `if` statement, including a chained `else if`.
    fn check_if(&mut self, decl: &IfStmt) {
        let condition = self.check_value(&decl.condition, None);
        self.require_bool(&condition, decl.condition.span);
        self.check_block(&decl.then_block);
        match &decl.else_branch {
            Some(ElseBranch::If(nested)) => self.check_if(nested),
            Some(ElseBranch::Block(block)) => self.check_block(block),
            None => {}
        }
    }

    /// Checks a `do ... while` loop.
    fn check_do_while(&mut self, decl: &DoWhileStmt) {
        self.check_block(&decl.body);
        let condition = self.check_value(&decl.condition, None);
        self.require_bool(&condition, decl.condition.span);
    }

    /// Checks a three-clause `for` loop.
    fn check_for_c(&mut self, decl: &ForCStmt) {
        self.push_scope();
        match &decl.init {
            Some(ForInit::Let(binding)) => self.check_let(binding),
            Some(ForInit::Const(binding)) => self.check_const(binding),
            Some(ForInit::Expr(expressions)) => {
                for expression in expressions {
                    self.check_expr(expression, None);
                }
            }
            None => {}
        }
        if let Some(condition) = &decl.condition {
            let ty = self.check_value(condition, None);
            self.require_bool(&ty, condition.span);
        }
        for step in &decl.step {
            self.check_expr(step, None);
        }
        self.check_block(&decl.body);
        self.pop_scope();
    }

    /// Checks an element `for` loop, binding the element type when known.
    fn check_for_in(&mut self, decl: &ForInStmt) {
        let iterable = self.check_value(&decl.iterable, None);
        // A bare type parameter cannot name its one-step element, so the
        // binding carries the same placeholder the ownership stage writes; the
        // instance substitution fills it from the substituted argument's
        // element.
        let element = match &iterable.kind {
            TyKind::Param(name) => Ty {
                kind: TyKind::Param(crate::types::element_param(name)),
                nullable: false,
            },
            _ => iterable.iterated_element(),
        };
        self.push_scope();
        self.declare(&decl.name, element);
        self.check_block(&decl.body);
        self.pop_scope();
    }

    /// Checks a `switch` statement: the scrutinee, each label, and each body.
    fn check_switch(&mut self, decl: &SwitchStmt) {
        let scrutinee = self.check_value(&decl.scrutinee, None);
        for clause in &decl.clauses {
            self.push_scope();
            if let SwitchLabel::Case(value) = &clause.label {
                self.require_constant(value, integer_scalar(&scrutinee));
                self.check_value(value, Some(&scrutinee));
            }
            for statement in &clause.statements {
                self.check_stmt(statement);
            }
            self.pop_scope();
        }
    }
}
