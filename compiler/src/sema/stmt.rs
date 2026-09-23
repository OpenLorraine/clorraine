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
                        if returns.is_void() {
                            self.check_value(value, None);
                        } else {
                            self.check_into(value, &returns);
                        }
                    }
                    // A bare `return;` suits only a `void` function; anywhere
                    // else it returns nothing where a value is required.
                    None if !returns.is_void() => {
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
                self.check_into(value, ty);
                self.declare(&decl.name, ty.clone());
            }
            (Some(ty), None) => self.declare(&decl.name, ty.clone()),
            (None, Some(value)) => {
                let ty = self.check_value(value, None);
                self.declare(&decl.name, ty);
            }
            (None, None) => self.declare(&decl.name, Ty::unknown()),
        }
    }

    /// Checks a `const` binding, whose initializer is mandatory.
    fn check_const(&mut self, decl: &ConstStmt) {
        let declared = decl.ty.as_ref().map(|ty| self.resolve_type(ty));
        let ty = match declared {
            Some(ty) => {
                self.check_into(&decl.value, &ty);
                ty
            }
            None => self.check_value(&decl.value, None),
        };
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
        let element = match &iterable.kind {
            TyKind::Array { element, .. } => (**element).clone(),
            _ => Ty::unknown(),
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
