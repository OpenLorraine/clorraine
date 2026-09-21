//! Module initialization order.
//!
//! The generated entry stub runs each module's `cl_init_*` function before the
//! user's `main`. A module must be initialized after every module it imports or
//! names in a global initializer, transitively; independent modules break ties
//! on the dotted module name compared byte-wise, so the order is deterministic.
//! A dependency cycle is rejected before codegen, but the walk stays total and
//! falls back to name order rather than looping.

use std::collections::{BTreeMap, BTreeSet};

use crate::ast::{Expr, ExprKind, Item};
use crate::load::Module;

use super::Emitter;

impl<'a> Emitter<'a> {
    /// The user modules in initialization order.
    ///
    /// A module is initialized after every module it imports or names in a
    /// global initializer, transitively; independent modules break ties on the
    /// dotted name compared byte-wise, so the order is deterministic.
    pub(super) fn init_order(&self) -> Vec<String> {
        let modules: Vec<String> = self.loaded.modules.keys().cloned().collect();
        let known: BTreeSet<String> = modules.iter().cloned().collect();
        let mut prereqs: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for name in &modules {
            let mut deps: BTreeSet<String> = BTreeSet::new();
            if let Some(module) = self.loaded.module(name) {
                for import in &module.imports {
                    if known.contains(&import.module) && &import.module != name {
                        deps.insert(import.module.clone());
                    }
                }
                for item in &module.items {
                    let init = match item {
                        Item::GlobalLet(decl) => Some(&decl.value),
                        Item::GlobalConst(decl) => Some(&decl.value),
                        _ => None,
                    };
                    if let Some(init) = init {
                        self.collect_name_modules(module, init, &known, &mut deps);
                    }
                }
            }
            prereqs.insert(name.clone(), deps);
        }

        let mut order: Vec<String> = Vec::new();
        let mut done: BTreeSet<String> = BTreeSet::new();
        while order.len() < modules.len() {
            let ready = modules
                .iter()
                .filter(|name| !done.contains(*name))
                .filter(|name| {
                    prereqs
                        .get(*name)
                        .is_none_or(|deps| deps.iter().all(|dep| done.contains(dep)))
                })
                .min()
                .cloned();
            match ready {
                Some(name) => {
                    done.insert(name.clone());
                    order.push(name);
                }
                None => {
                    // A dependency cycle is E11100 before codegen; keep the
                    // emitter total by appending the rest in name order.
                    for name in &modules {
                        if !done.contains(name) {
                            done.insert(name.clone());
                            order.push(name.clone());
                        }
                    }
                }
            }
        }
        order
    }

    /// Adds the user modules an initializer expression names through imports.
    pub(super) fn collect_name_modules(
        &self,
        module: &Module,
        expr: &Expr,
        known: &BTreeSet<String>,
        deps: &mut BTreeSet<String>,
    ) {
        match &expr.kind {
            ExprKind::Name(name) => {
                if let Some(binding) = module.import(name) {
                    if known.contains(&binding.module) && binding.module != module.name {
                        deps.insert(binding.module.clone());
                    }
                }
            }
            ExprKind::Qualified(name) => {
                if let Some(first) = name.segments.first() {
                    if let Some(binding) = module.import(first) {
                        if known.contains(&binding.module) && binding.module != module.name {
                            deps.insert(binding.module.clone());
                        }
                    }
                }
            }
            ExprKind::Call { callee, args, .. } => {
                self.collect_name_modules(module, callee, known, deps);
                for arg in args {
                    self.collect_name_modules(module, &arg.value, known, deps);
                }
            }
            ExprKind::Index { base, index } => {
                self.collect_name_modules(module, base, known, deps);
                self.collect_name_modules(module, index, known, deps);
            }
            ExprKind::Member { base, .. } => self.collect_name_modules(module, base, known, deps),
            ExprKind::Unary { operand, .. } => {
                self.collect_name_modules(module, operand, known, deps)
            }
            ExprKind::Binary { left, right, .. } => {
                self.collect_name_modules(module, left, known, deps);
                self.collect_name_modules(module, right, known, deps);
            }
            ExprKind::Assign { target, value, .. } => {
                self.collect_name_modules(module, target, known, deps);
                self.collect_name_modules(module, value, known, deps);
            }
            ExprKind::Conditional {
                condition,
                then_value,
                else_value,
            } => {
                self.collect_name_modules(module, condition, known, deps);
                self.collect_name_modules(module, then_value, known, deps);
                self.collect_name_modules(module, else_value, known, deps);
            }
            ExprKind::Cast { value, .. } | ExprKind::RawCast { value, .. } => {
                self.collect_name_modules(module, value, known, deps)
            }
            ExprKind::StructLiteral { fields, .. } => {
                for field in fields {
                    self.collect_name_modules(module, &field.value, known, deps);
                }
            }
            ExprKind::ArrayLiteral { elements, fill } => {
                for element in elements {
                    self.collect_name_modules(module, element, known, deps);
                }
                if let Some(fill) = fill {
                    self.collect_name_modules(module, fill, known, deps);
                }
            }
            ExprKind::Unsafe(inner) => self.collect_name_modules(module, inner, known, deps),
            ExprKind::IntrinsicCall { args, .. } => {
                for arg in args {
                    if let crate::ast::IntrinsicArg::Expr(inner) = arg {
                        self.collect_name_modules(module, inner, known, deps);
                    }
                }
            }
            ExprKind::Template(pieces) => {
                for piece in pieces {
                    if let crate::ast::TemplatePiece::Placeholder(inner) = piece {
                        self.collect_name_modules(module, inner, known, deps);
                    }
                }
            }
            _ => {}
        }
    }
}
