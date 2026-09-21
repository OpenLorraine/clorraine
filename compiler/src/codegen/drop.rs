//! Drop and deleter emission for owned values.
//!
//! A drop releases a handle through its declared deleter and empties the place,
//! so a drop can never run twice on the same storage. An aggregate recurses
//! into the fields that own a value. This module also answers whether a type
//! owns anything at all, which decides whether an assignment needs a scratch
//! local and whether a scope needs a cleanup rung.

use crate::borrow::is_view;
use crate::ir::Place;
use crate::types::{Ty, TyKind};

use super::Emitter;

impl<'a> Emitter<'a> {
    /// Emits a null-safe drop of a place, recursing into owned struct fields.
    ///
    /// The drop empties the place it releases, so a place that is dropped twice
    /// does not release its value twice. That invariant matters most for a
    /// local in a loop body, whose per-iteration cleanup runs on each pass.
    pub(super) fn emit_drop(&mut self, place: &Place) {
        let c_name = self.place(place);
        let ty = self.substitute(&place.ty);
        self.emit_drop_expr(&c_name, &ty);
    }

    /// Emits a null-safe drop of the C expression `c_name` of type `ty`.
    ///
    /// A handle is released through its deleter and then set to `NULL`; a
    /// struct recurses into each field that owns a value. A dropped place thus
    /// always ends in its empty state, so a later drop is a no-op.
    pub(super) fn emit_drop_expr(&mut self, c_name: &str, ty: &Ty) {
        match &ty.kind {
            TyKind::Handle(name) => {
                let deleter = self.deleter_symbol(name);
                let null = self.null_literal();
                self.push(&format!("    if ({c_name}) {{ {deleter}({c_name}); }}"));
                self.push(&format!("    {c_name} = {null};"));
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
                    self.emit_drop_expr(&format!("{c_name}.{field}"), &field_ty);
                }
            }
            _ => {}
        }
    }

    /// True when a value of this type drops owned storage in the emitted C.
    ///
    /// This refines [`Self::needs_drop`]: a type the emitter has no drop
    /// lowering for, such as an array, needs no old-value release and cannot
    /// use the assignment scratch local.
    pub(super) fn emits_drop(&self, ty: &Ty) -> bool {
        if !self.needs_drop(ty) {
            return false;
        }
        match &ty.kind {
            TyKind::Handle(_) => true,
            TyKind::Struct(name) => self
                .sema
                .table
                .struct_fields(name)
                .map(|fields| fields.iter().any(|(_, field_ty)| self.emits_drop(field_ty)))
                .unwrap_or(false),
            _ => false,
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
        if let Some(info) = self.resolve_function(&deleter) {
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
}
