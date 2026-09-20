//! Function-body emission from the typed control-flow IR.
//!
//! This module owns statements, drops, defers, the cleanup ladder, and the
//! source mapping; the structured control-flow walk lives in the sibling
//! [`super::control`] module. Locals are hoisted to the top of the function
//! with their empty-state initializer, so a cleanup path may reference any of
//! them. A scope that owns a droppable value or registers a `defer` gets one
//! cleanup label; the labels form a single ladder at the end of the function,
//! innermost first. An early return jumps to the label of the innermost scope
//! it leaves and the ladder runs the remaining scopes on the way out. A scope
//! inside a loop body runs its cleanup inline, once per iteration, because the
//! ladder runs only once. Generated cleanup is attributed to the scope's
//! closing source line, so a debugger stepping it stays on the closing line.

use std::collections::{BTreeMap, BTreeSet};

use crate::borrow::is_view;
use crate::ir::{self, BlockId, Place, Projection, Statement, Terminator, ValueKind};
use crate::types::{Ty, TyKind};

use super::types::{declarator, empty_state};
use super::{sanitize_identifier, Emitter, FuncInfo};

/// One cleanup entry of a scope, in registration order.
#[derive(Clone)]
pub(crate) enum Action {
    /// Drop a place through its type's deleter.
    Drop(Place),
    /// Run a deferred block's statements.
    Defer(BlockId),
}

impl<'a> Emitter<'a> {
    // -- function emission ---------------------------------------------------

    /// Emits one function body from its lowered IR.
    ///
    /// `instance` carries the type substitution and symbol key of a generic
    /// instantiation; a non-generic body passes `None`.
    pub(super) fn emit_function(
        &mut self,
        info: &FuncInfo<'a>,
        instance: Option<(BTreeMap<String, Ty>, String)>,
    ) {
        self.subst = instance
            .as_ref()
            .map(|(subst, _)| subst.clone())
            .unwrap_or_default();
        self.used_names.clear();
        self.local_names.clear();
        self.scope_actions.clear();
        self.rungs.clear();
        self.labels.clear();
        self.result_var = None;
        self.reachable.clear();
        self.emitted.clear();
        self.pending_labels.clear();
        self.loop_entry.clear();
        self.suppress_output = false;

        let Some(function) = self
            .ownership
            .functions
            .iter()
            .find(|function| function.name == info.decl.name)
        else {
            // Every clean compile lowers a body for each declared function; a
            // miss means an earlier stage rejected the program, so there is
            // nothing to emit.
            return;
        };
        self.function = Some(function);
        self.return_ty = self.substitute(&function.ret);

        for param in &function.params {
            let c_name = sanitize_identifier(&param.name);
            self.used_names.insert(c_name.clone());
            self.local_names.insert(param.name.clone(), c_name);
        }
        let mut declared: BTreeSet<String> = BTreeSet::new();
        for local in &function.locals {
            if !declared.insert(local.name.clone()) {
                continue;
            }
            let c_name = self.unique_local(&local.name);
            self.local_names.insert(local.name.clone(), c_name);
        }

        self.collect_cleanup(function);
        let has_cleanup = self.scope_actions.values().any(|actions| !actions.is_empty());
        if has_cleanup && !self.return_ty.is_void() {
            self.result_var = Some("__cl_ret".to_owned());
        }
        self.check_stack_frame(function, info.decl.name_span);

        let symbol = match &instance {
            Some((_, key)) => format!("cl_{}__{}", info.decl.name, key),
            None => self.function_symbol(&info.module, &info.decl.name),
        };
        let signature = self.signature_with_symbol(&info.decl, &self.subst, &symbol);
        self.push(&signature);
        self.push("{");
        self.emit_local_declarations(function);
        self.emit_structured_body(function, has_cleanup);
        self.emit_closing_return();
        self.push("}");
        self.push("");

        self.function = None;
    }

    /// Writes the hoisted local declarations, the temporaries, and the result.
    fn emit_local_declarations(&mut self, function: &'a ir::Function) {
        let mut written: BTreeSet<String> = BTreeSet::new();
        for local in &function.locals {
            if !written.insert(local.name.clone()) {
                continue;
            }
            let c_name = self
                .local_names
                .get(&local.name)
                .cloned()
                .unwrap_or_else(|| sanitize_identifier(&local.name));
            let ty = self.substitute(&local.ty);
            self.push(&format!(
                "    {} = {};",
                declarator(&ty, &c_name),
                empty_state(&ty)
            ));
        }
        for (index, ty) in function.temps.iter().enumerate() {
            let ty = self.substitute(ty);
            self.push(&format!(
                "    {} = {};",
                declarator(&ty, &format!("__cl_tmp{index}")),
                empty_state(&ty)
            ));
        }
        if let Some(result) = self.result_var.clone() {
            let ret = self.return_ty.clone();
            self.push(&format!(
                "    {} = {};",
                declarator(&ret, &result),
                empty_state(&ret)
            ));
        }
    }

    /// Reports W10004 when the summed frame of the function is too large.
    ///
    /// Every local, parameter, and temporary contributes; a size the type
    /// table does not model yet suppresses the warning rather than guessing.
    fn check_stack_frame(&mut self, function: &'a ir::Function, span: crate::span::Span) {
        let limit = self.profile.stack_frame_limit();
        let mut total = 0usize;
        for ty in function
            .params
            .iter()
            .map(|param| &param.ty)
            .chain(function.locals.iter().map(|local| &local.ty))
            .chain(function.temps.iter())
        {
            match self.sema.table.size_of(&self.substitute(ty)) {
                Some(size) => total = total.saturating_add(size),
                None => return,
            }
        }
        if total > limit {
            self.warn(
                "W10004",
                "stack frame exceeds the profile threshold",
                span,
            );
        }
    }

    // -- cleanup collection ---------------------------------------------------

    /// Collects the cleanup actions and ladder rungs of one body.
    fn collect_cleanup(&mut self, function: &'a ir::Function) {
        for block in &function.blocks {
            for instr in &block.statements {
                match &instr.statement {
                    Statement::Drop { place, scope } => {
                        self.push_action(*scope, Action::Drop(place.clone()));
                    }
                    Statement::Defer { block, scope } => {
                        self.push_action(*scope, Action::Defer(*block));
                    }
                    _ => {}
                }
            }
        }

        let reachable = reachable_blocks(function);
        let mut normal_scopes: BTreeSet<usize> = BTreeSet::new();
        for block in &function.blocks {
            if !reachable.contains(&block.id) {
                continue;
            }
            for instr in &block.statements {
                match &instr.statement {
                    Statement::Drop { scope, .. }
                    | Statement::Defer { scope, .. }
                    | Statement::ScopeEnd { scope } => {
                        normal_scopes.insert(*scope);
                    }
                    _ => {}
                }
            }
        }

        let mut rungs: Vec<usize> = self
            .scope_actions
            .iter()
            .filter(|(scope, actions)| {
                !actions.is_empty()
                    && !function.scopes[**scope].in_loop
                    && normal_scopes.contains(scope)
            })
            .map(|(scope, _)| *scope)
            .collect();
        rungs.sort_by(|left, right| {
            scope_depth(function, *right)
                .cmp(&scope_depth(function, *left))
                .then(left.cmp(right))
        });
        for (index, scope) in rungs.iter().enumerate() {
            self.labels.insert(*scope, index + 1);
        }
        self.rungs = rungs;
    }

    /// Records a cleanup action for a scope, skipping an exact duplicate.
    fn push_action(&mut self, scope: usize, action: Action) {
        let actions = self.scope_actions.entry(scope).or_default();
        let duplicate = actions.iter().any(|known| match (known, &action) {
            (Action::Drop(a), Action::Drop(b)) => a == b,
            (Action::Defer(a), Action::Defer(b)) => a == b,
            _ => false,
        });
        if !duplicate {
            actions.push(action);
        }
    }

    /// True when a scope's cleanup runs inline rather than from the ladder.
    fn scope_is_inline(&self, function: &'a ir::Function, scope: usize) -> bool {
        function.scopes[scope].in_loop || !self.labels.contains_key(&scope)
    }

    // -- block emission -------------------------------------------------------

    /// Emits one block's statements and returns the ladder entry for its exit.
    ///
    /// A `return` in the block jumps to the label of the innermost scope it
    /// leaves, so the returned value is that scope's ladder number when the
    /// block leaves through a non-inline scope. The caller emits the block's
    /// terminator, because the structured emitter turns most terminators into
    /// C control flow rather than a `goto`.
    pub(super) fn emit_block_body(
        &mut self,
        function: &'a ir::Function,
        id: BlockId,
    ) -> Option<usize> {
        let block = &function.blocks[id];
        let mut pending_scope: Option<usize> = None;
        let mut inlined_edge: BTreeSet<usize> = BTreeSet::new();
        let mut edge_scopes: Vec<usize> = Vec::new();
        for instr in &block.statements {
            match &instr.statement {
                Statement::ScopeEnd { scope } => {
                    pending_scope = Some(*scope);
                    if function.scopes[*scope].in_loop {
                        self.emit_scope_inline(*scope);
                    }
                }
                Statement::Drop { scope, .. } => {
                    if pending_scope == Some(*scope) {
                        // The fall-through cleanup of a scope end was already
                        // handled at the scope end (loop) or the ladder
                        // (function scope), so a matching drop is a no-op here.
                        continue;
                    }
                    if self.scope_is_inline(function, *scope) {
                        if inlined_edge.insert(*scope) {
                            self.emit_scope_inline(*scope);
                        }
                    } else if edge_scopes.first() != Some(scope) {
                        edge_scopes.push(*scope);
                    }
                }
                Statement::Defer { .. } => {
                    // Deferred bodies run from the ladder rung, never at the
                    // registration site.
                    pending_scope = None;
                }
                other => {
                    pending_scope = None;
                    self.emit_statement(instr.span, other);
                }
            }
        }
        edge_scopes
            .first()
            .and_then(|scope| self.labels.get(scope).copied())
    }

    /// Emits one deferred block and its closure at a ladder rung.
    fn emit_defer(&mut self, root: BlockId) {
        let Some(function) = self.function else {
            return;
        };
        self.emit_closure(function, root);
    }

    /// Emits a scope's cleanup actions inline, once.
    fn emit_scope_inline(&mut self, scope: usize) {
        self.emit_line_for_scope(scope);
        let actions = self.scope_actions.get(&scope).cloned().unwrap_or_default();
        self.emit_actions(&actions);
    }

    /// Writes a `#line` mapping for generated cleanup to the scope's close.
    ///
    /// Inserted cleanup belongs to the scope's closing source line, so a
    /// debugger stepping the generated drop stays on the closing line of the
    /// scope rather than entering a generated statement.
    fn emit_line_for_scope(&mut self, scope: usize) {
        let Some(function) = self.function else {
            return;
        };
        let Some(info) = function.scopes.get(scope) else {
            return;
        };
        let span = info.close_span;
        self.emit_close_line(span);
    }

    /// Writes a `#line` directive for the last line of a span.
    ///
    /// A scope's close span covers its closing brace, so the closing line is
    /// the line of the span's last covered byte.
    pub(super) fn emit_close_line(&mut self, span: crate::span::Span) {
        let last = span.end.saturating_sub(1);
        if span.is_empty() && last == 0 {
            return;
        }
        let line = self.sources.location(span.file, last as usize).line;
        let path = self.path_of(span).to_owned();
        self.push(&format!("#line {line} \"{path}\""));
    }

    /// Runs a list of cleanup actions in their collected order.
    fn emit_actions(&mut self, actions: &[Action]) {
        for action in actions {
            match action {
                Action::Drop(place) => self.emit_drop(place),
                Action::Defer(block) => self.emit_defer(*block),
            }
        }
    }

    /// Emits a null-safe drop of a place, recursing into owned struct fields.
    fn emit_drop(&mut self, place: &Place) {
        let c_name = self.place(place);
        let ty = self.substitute(&place.ty);
        match &ty.kind {
            TyKind::Handle(name) => {
                let deleter = self.deleter_symbol(name);
                self.push(&format!("    if ({c_name}) {{ {deleter}({c_name}); }}"));
            }
            TyKind::Struct(name) => {
                let fields = self
                    .sema
                    .table
                    .struct_fields(name)
                    .map(<[(String, Ty)]>::to_vec)
                    .unwrap_or_default();
                for (field, field_ty) in fields {
                    if !self.needs_drop(&field_ty) {
                        continue;
                    }
                    let mut projections = place.projections.clone();
                    projections.push(Projection::Field(field));
                    let place = Place {
                        base: place.base.clone(),
                        projections,
                        ty: field_ty,
                    };
                    self.emit_drop(&place);
                }
            }
            _ => {}
        }
    }

    /// True when a value of this type needs a drop.
    pub(super) fn needs_drop(&self, ty: &Ty) -> bool {
        !self.sema.table.duplicable(ty)
            && !is_view(ty)
            && !ty.is_unknown()
            && !ty.is_param()
            && !ty.is_void()
    }

    /// The C symbol of a handle's deleter.
    fn deleter_symbol(&self, handle: &str) -> String {
        if handle == "String" {
            return "FreeString".to_owned();
        }
        let deleter = self.handle_deleter(handle).unwrap_or_else(|| handle.to_owned());
        if let Some(info) = self.functions.get(&deleter) {
            self.function_symbol(&info.module, &deleter)
        } else {
            deleter
        }
    }

    /// Finds the `freedBy` deleter name of a handle declaration.
    fn handle_deleter(&self, handle: &str) -> Option<String> {
        for module in self.loaded.modules.values() {
            for item in &module.items {
                if let crate::ast::Item::Handle(decl) = item {
                    if decl.name == handle {
                        return Some(decl.freed_by.segments.join("_"));
                    }
                }
            }
        }
        None
    }

    // -- statements -----------------------------------------------------------

    /// Emits one IR statement that has a run-time effect.
    fn emit_statement(&mut self, span: crate::span::Span, statement: &Statement) {
        match statement {
            Statement::Assign { target, value } => {
                if let ValueKind::ArrayLit { elements, fill } = &value.kind {
                    let ty = self.substitute(&target.ty);
                    if matches!(&ty.kind, TyKind::Array { extents, .. } if extents.iter().all(Option::is_some))
                    {
                        self.emit_line_directive(span);
                        self.emit_array_assign(target, &ty, elements, fill);
                        return;
                    }
                }
                self.emit_line_directive(span);
                self.emit_guards(value);
                let target_c = self.place(target);
                let lowered = self.value(value);
                self.push(&format!("    {target_c} = {lowered};"));
            }
            Statement::Move { target, source } => {
                self.emit_line_directive(span);
                let target_c = self.place(target);
                let source_c = self.place(source);
                self.push(&format!("    {target_c} = {source_c};"));
                let source_ty = self.substitute(&source.ty);
                self.emit_move_zero(&source_c, &source_ty);
            }
            Statement::Call {
                callee,
                type_args,
                args,
                result,
            } => {
                self.emit_call(span, callee, type_args, args, result.as_ref());
            }
            Statement::Borrow { .. }
            | Statement::Drop { .. }
            | Statement::Defer { .. }
            | Statement::ScopeEnd { .. } => {}
            Statement::Nop => self.emit_line_directive(span),
        }
    }

    /// Emits a call, a discarded expression statement, or a trap intrinsic.
    fn emit_call(
        &mut self,
        span: crate::span::Span,
        callee: &str,
        type_args: &[Ty],
        args: &[ir::Value],
        result: Option<&Place>,
    ) {
        match callee {
            "assert" => {
                self.emit_line_directive(span);
                if let Some(condition) = args.first() {
                    self.emit_guards(condition);
                    let condition_c = self.value(condition);
                    self.push(&format!(
                        "    if (!({condition_c})) cl_trap(\"E11213\", \"assertion failed\");"
                    ));
                }
            }
            "panic" => {
                self.emit_line_directive(span);
                let message = args
                    .first()
                    .map(|arg| self.value(arg))
                    .unwrap_or_else(|| "panic".to_owned());
                self.push(&format!("    cl_panic({message});"));
            }
            _ => {
                self.emit_line_directive(span);
                for arg in args {
                    self.emit_guards(arg);
                }
                let call = self.call(callee, type_args, args);
                let prefix = match result {
                    Some(place) => format!("{} = ", self.place(place)),
                    None => String::new(),
                };
                self.push(&format!("    {prefix}{call};"));
            }
        }
    }

    /// Emits an array literal as element-by-element assignments.
    fn emit_array_assign(
        &mut self,
        target: &Place,
        ty: &Ty,
        elements: &[ir::Value],
        fill: &Option<Box<ir::Value>>,
    ) {
        let target_c = self.place(target);
        for (index, element) in elements.iter().enumerate() {
            self.emit_guards(element);
            let lowered = self.value(element);
            self.push(&format!("    {target_c}[{index}] = {lowered};"));
        }
        if elements.is_empty() {
            if let Some(fill) = fill {
                let extent = match &ty.kind {
                    TyKind::Array { extents, .. } => {
                        extents.first().and_then(|size| *size).unwrap_or(0)
                    }
                    _ => 0,
                };
                self.emit_guards(fill);
                let lowered = self.value(fill);
                for index in 0..extent {
                    self.push(&format!("    {target_c}[{index}] = {lowered};"));
                }
            }
        }
    }

    /// Zeroes a moved-out source: `NULL` for a handle, `cl_zero` otherwise.
    fn emit_move_zero(&mut self, source: &str, ty: &Ty) {
        match &ty.kind {
            TyKind::Handle(_) | TyKind::Raw(_) | TyKind::CString => {
                self.push(&format!("    {source} = NULL;"));
            }
            _ => self.push(&format!("    cl_zero(&{source});")),
        }
    }

    // -- terminators ----------------------------------------------------------

    /// Emits a `return`, using `jump` as the ladder entry for a cleanup exit.
    ///
    /// A function with a cleanup ladder stores the value in the result variable
    /// and jumps to the innermost scope label it leaves, so the ladder can drop
    /// the scopes on the way out. A function without one returns directly.
    pub(super) fn emit_return(
        &mut self,
        span: crate::span::Span,
        value: Option<&ir::Value>,
        jump: Option<usize>,
    ) {
        self.emit_line_directive(span);
        if let Some(value) = value {
            self.emit_guards(value);
            let lowered = self.value(value);
            if let Some(result) = self.result_var.clone() {
                self.push(&format!("    {result} = {lowered};"));
            } else {
                self.push(&format!("    return {lowered};"));
                return;
            }
        }
        if let Some(label) = jump {
            self.push(&format!("    goto __cl_c{label};"));
        } else if let Some(result) = self.result_var.clone() {
            self.push(&format!("    return {result};"));
        } else {
            self.push("    return;");
        }
    }

    /// Writes the function-end cleanup ladder.
    pub(super) fn emit_ladder(&mut self) {
        let rungs = self.rungs.clone();
        for (index, scope) in rungs.iter().enumerate() {
            let label = self.labels[scope];
            self.emit_line_for_scope(*scope);
            self.push(&format!("__cl_c{label}:"));
            let actions = self.scope_actions.get(scope).cloned().unwrap_or_default();
            self.emit_actions(&actions);
            if index + 1 < rungs.len() {
                let next = self.labels[&rungs[index + 1]];
                self.push(&format!("    goto __cl_c{next};"));
            } else if let Some(result) = self.result_var.clone() {
                self.push(&format!("    return {result};"));
            } else {
                self.push("    return;");
            }
        }
    }

    /// Writes a final return so a non-void function never falls off its end.
    ///
    /// Every reachable path already returns; this line only satisfies the C
    /// compiler's return-type analysis when the last emitted block ends with a
    /// `goto` into the ladder.
    fn emit_closing_return(&mut self) {
        if self.return_ty.is_void() {
            return;
        }
        if let Some(result) = self.result_var.clone() {
            self.push(&format!("    return {result};"));
            return;
        }
        let ret = self.return_ty.clone();
        let value = match &ret.kind {
            TyKind::Handle(_) | TyKind::Raw(_) | TyKind::CString | TyKind::Func(_) => {
                "NULL".to_owned()
            }
            TyKind::Str | TyKind::Struct(_) => format!("({}){{0}}", super::types::c_type(&ret)),
            _ => "0".to_owned(),
        };
        self.push(&format!("    return {value};"));
    }
}

/// The block ids reachable from a function's entry through its terminators.
pub(super) fn reachable_blocks(function: &ir::Function) -> BTreeSet<BlockId> {
    closure_from(function, function.entry)
}

/// The block ids reachable from `root` through terminators.
pub(super) fn closure_from(function: &ir::Function, root: BlockId) -> BTreeSet<BlockId> {
    let mut seen: BTreeSet<BlockId> = BTreeSet::new();
    let mut stack = vec![root];
    while let Some(id) = stack.pop() {
        if !seen.insert(id) {
            continue;
        }
        for successor in successors(&function.blocks[id].terminator) {
            stack.push(successor);
        }
    }
    seen
}

/// The blocks a terminator can transfer control to.
fn successors(terminator: &Terminator) -> Vec<BlockId> {
    match terminator {
        Terminator::Goto(target) => vec![*target],
        Terminator::Branch {
            on_true, on_false, ..
        } => vec![*on_true, *on_false],
        Terminator::Switch { arms, default, .. } => {
            let mut targets: Vec<BlockId> = arms.iter().map(|(_, block)| *block).collect();
            targets.push(*default);
            targets
        }
        Terminator::Return(_) | Terminator::Unreachable => Vec::new(),
    }
}

/// The lexical depth of a scope, counted from the function body scope.
fn scope_depth(function: &ir::Function, scope: usize) -> usize {
    let mut depth = 0;
    let mut current = scope;
    while let Some(parent) = function.scopes[current].parent {
        depth += 1;
        current = parent;
    }
    depth
}
