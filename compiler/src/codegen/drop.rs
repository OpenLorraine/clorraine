//! Drop and deleter emission for owned values.
//!
//! A drop releases a handle through its declared deleter and empties the place,
//! so a drop can never run twice on the same storage. An aggregate recurses
//! into the fields that own a value. This module also answers whether a type
//! owns anything at all, which decides whether an assignment needs a scratch
//! local and whether a scope needs a cleanup rung.

use crate::borrow::is_view;
use crate::ir::Place;
use crate::types::{substitute_instance, Ty, TyKind};

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
            TyKind::Array { element, extents } if extents.iter().all(Option::is_some) => {
                if self.needs_drop(element) {
                    let dims: Vec<usize> =
                        extents.iter().filter_map(|size| *size).collect();
                    self.emit_fixed_array_drop(c_name, element, &dims);
                }
            }
            TyKind::Struct(name, args) if name == "Array" && args.len() == 1 => {
                // The growable container owns its buffer and its elements, so it
                // releases the elements through the buffer before the generic
                // field walk frees the buffer. A duplicable element needs no
                // element pass, so it takes the ordinary field walk.
                let element = self.substitute(&args[0]);
                if self.needs_drop(&element) {
                    self.emit_growable_array_drop(c_name, &element);
                } else {
                    self.emit_struct_drop(c_name, name, args);
                }
            }
            TyKind::Struct(name, args) => self.emit_struct_drop(c_name, name, args),
            _ => {}
        }
    }

    /// Emits the drop of a struct by recursing into each owned field.
    ///
    /// The container special cases that handle their own storage do not call
    /// this, so a field is never released twice.
    fn emit_struct_drop(&mut self, c_name: &str, name: &str, args: &[Ty]) {
        let fields = self
            .sema
            .table
            .struct_fields(name)
            .map(<[(String, Ty)]>::to_vec)
            .unwrap_or_default();
        let params = self
            .sema
            .table
            .struct_type_params(name)
            .unwrap_or(&[])
            .to_vec();
        for (field, declared) in fields {
            let field_ty = self.substitute(&substitute_instance(&declared, &params, args));
            if !self.needs_drop(&field_ty) {
                continue;
            }
            self.emit_drop_expr(&format!("{c_name}.{field}"), &field_ty);
        }
    }

    /// Emits the drop of a fixed array one extent per level.
    ///
    /// The C declaration keeps every extent (`ClString grid[2][2]`), so each
    /// level indexes one dimension; a flattened index would name a row, not an
    /// element. The base element is dropped at the deepest level.
    fn emit_fixed_array_drop(&mut self, c_name: &str, element: &Ty, extents: &[usize]) {
        match extents.split_first() {
            None => self.emit_drop_expr(c_name, element),
            Some((head, tail)) => {
                for index in 0..*head {
                    self.emit_fixed_array_drop(&format!("{c_name}[{index}]"), element, tail);
                }
            }
        }
    }

    /// Emits the drop of a growable `Array<T>`: its elements through the buffer,
    /// then the buffer itself.
    ///
    /// The buffer pointer is read once and the `data` guard makes an already
    /// dropped array a no-op, so a second drop never reads a freed buffer. The
    /// loop locals are unique, so a user local of the same name is not shadowed.
    fn emit_growable_array_drop(&mut self, c_name: &str, element: &Ty) {
        let data = format!("{c_name}.data");
        let element_c = super::types::c_type(element);
        let elems = self.unique_local("__cl_elems");
        let index = self.unique_local("__cl_i");
        self.push(&format!("    if ({data}) {{"));
        self.push(&format!(
            "        {element_c} *{elems} = ({element_c} *)cl_buffer_ptr({data});"
        ));
        self.push(&format!("        usize {index} = 0;"));
        self.push(&format!("        while ({index} < {c_name}.count) {{"));
        self.emit_drop_expr(&format!("{elems}[{index}]"), element);
        self.push(&format!("            {index} = {index} + 1;"));
        self.push("        }");
        self.emit_drop_expr(&data, &Ty::handle("Buffer"));
        self.push(&format!("        {c_name}.count = 0;"));
        self.push("    }");
    }

    /// True when a value of this type can use the assignment scratch local to
    /// release its old value.
    ///
    /// This refines [`Self::needs_drop`]. A fixed array is owned and drops its
    /// elements, but it has no arm here: the scratch local would be a C array,
    /// which cannot be assigned, so a fixed-array assignment is routed to its
    /// own byte-copy path before this decision is reached. That path keeps the
    /// old value with a byte copy of its own, so the no-scratch answer here
    /// stays correct.
    pub(super) fn emits_drop(&self, ty: &Ty) -> bool {
        if !self.needs_drop(ty) {
            return false;
        }
        match &ty.kind {
            TyKind::Handle(_) => true,
            TyKind::Struct(name, args) => {
                let fields = self
                    .sema
                    .table
                    .struct_fields(name)
                    .map(<[(String, Ty)]>::to_vec)
                    .unwrap_or_default();
                let params = self
                    .sema
                    .table
                    .struct_type_params(name)
                    .unwrap_or(&[])
                    .to_vec();
                fields.iter().any(|(_, declared)| {
                    self.emits_drop(&self.substitute(&substitute_instance(declared, &params, args)))
                })
            }
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
