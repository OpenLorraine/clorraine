//! Drop and deleter emission for owned values.
//!
//! A drop releases a handle through its declared deleter and empties the place,
//! so a drop can never run twice on the same storage. An aggregate recurses
//! into the fields that own a value. This module also answers whether a type
//! owns anything at all, which decides whether an assignment needs a scratch
//! local and whether a scope needs a cleanup rung.

use crate::borrow::is_view;
use crate::ir::Place;
use crate::types::{substitute_instance, type_key, Ty, TyKind};

use super::Emitter;

impl<'a> Emitter<'a> {
    /// Emits a null-safe drop of a place, recursing into owned struct fields.
    ///
    /// The drop empties the place it releases, so a place that is dropped twice
    /// does not release its value twice. That invariant matters most for a
    /// local in a loop body, whose per-iteration cleanup runs on each pass.
    pub(super) fn emit_drop(&mut self, place: &Place) {
        let c_name = self.place(place);
        let ty = self.resolved_place_ty(place);
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
            // A box releases its pointee recursively through the generated
            // helper, then frees the block and empties the place.
            TyKind::Box(inner) => {
                let helper = format!("cl_box_drop__{}", type_key(inner));
                let null = self.null_literal();
                self.push(&format!("    if ({c_name}) {{ {helper}({c_name}); }}"));
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
            let base = suffix_base(c_name);
            self.emit_drop_expr(&format!("{base}.{field}"), &field_ty);
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
                    let base = suffix_base(c_name);
                    self.emit_fixed_array_drop(&format!("{base}[{index}]"), element, tail);
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
        let element_c = super::types::c_type(element, &self.sema.table);
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

    /// Renders a null-safe drop of a C lvalue as an expression.
    ///
    /// A drop that must embed inside an expression — a guarded operand
    /// re-materialized on a conditional path — cannot be a statement. A handle
    /// becomes `(x ? (deleter(x), x = NULL) : 0)`; a fixed array joins its
    /// element drops with commas; a struct joins its owned fields. Returns
    /// `None` when the shape has no expression form (a growable array's
    /// element loop), so the caller leaves the value as it is.
    pub(super) fn drop_expr_text(&self, c_name: &str, ty: &Ty) -> Option<String> {
        match &ty.kind {
            TyKind::Handle(name) => {
                let deleter = self.deleter_symbol(name);
                let null = self.null_literal();
                Some(format!(
                    "({c_name} ? ({deleter}({c_name}), {c_name} = {null}) : 0)"
                ))
            }
            // A box releases its pointee through the generated helper, then
            // frees the block and empties the place.
            TyKind::Box(inner) => {
                let helper = format!("cl_box_drop__{}", type_key(inner));
                let null = self.null_literal();
                Some(format!(
                    "({c_name} ? ({helper}({c_name}), {c_name} = {null}) : 0)"
                ))
            }
            TyKind::Array { element, extents } if extents.iter().all(Option::is_some) => {
                if !self.needs_drop(element) {
                    return Some("0".to_owned());
                }
                let dims: Vec<usize> = extents.iter().filter_map(|size| *size).collect();
                self.fixed_array_drop_expr(c_name, element, &dims)
            }
            // A growable container whose element needs no drop releases its
            // buffer and resets the count as one expression, so a guarded
            // operand can drop the previous value before re-materializing it.
            // The element loop has no C expression form, so a drop-needing
            // element keeps the null form and the caller skips the drop.
            TyKind::Struct(name, args) if name == "Array" && args.len() == 1 => {
                let element = self.substitute(&args[0]);
                if self.needs_drop(&element) {
                    return None;
                }
                let data = format!("{c_name}.data");
                let deleter = self.deleter_symbol("Buffer");
                let null = self.null_literal();
                Some(format!(
                    "({data} ? ({deleter}({data}), {data} = {null}, {c_name}.count = 0) : 0)"
                ))
            }
            TyKind::Struct(name, args) => self.struct_drop_expr(c_name, name, args),
            _ => None,
        }
    }

    /// Renders the element drops of a fixed array as one comma expression.
    fn fixed_array_drop_expr(&self, c_name: &str, element: &Ty, extents: &[usize]) -> Option<String> {
        match extents.split_first() {
            None => self.drop_expr_text(c_name, element),
            Some((head, tail)) => {
                let mut parts = Vec::new();
                for index in 0..*head {
                    let base = suffix_base(c_name);
                    parts.push(self.fixed_array_drop_expr(
                        &format!("{base}[{index}]"),
                        element,
                        tail,
                    )?);
                }
                Some(parts.join(", "))
            }
        }
    }

    /// Renders the owned fields of a struct as one comma expression.
    fn struct_drop_expr(&self, c_name: &str, name: &str, args: &[Ty]) -> Option<String> {
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
        let mut parts = Vec::new();
        for (field, declared) in fields {
            let field_ty = self.substitute(&substitute_instance(&declared, &params, args));
            if !self.needs_drop(&field_ty) {
                continue;
            }
            let base = suffix_base(c_name);
            parts.push(self.drop_expr_text(&format!("{base}.{field}"), &field_ty)?);
        }
        if parts.is_empty() {
            Some("0".to_owned())
        } else {
            Some(parts.join(", "))
        }
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
            TyKind::Box(_) => true,
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
    ///
    /// The identity was resolved once in the handle's declaring module, so an
    /// ordinary function mangles its own module and an `ffi` function keeps
    /// its C name. A dotted written name is never passed to either spelling.
    fn deleter_symbol(&self, handle: &str) -> String {
        if handle == "String" {
            return "FreeString".to_owned();
        }
        let identity = self
            .ownership
            .deleter_identities
            .get(handle)
            .unwrap_or_else(|| panic!("handle '{handle}' has no resolved deleter"));
        if identity.ffi {
            identity.name.clone()
        } else {
            self.function_symbol(&identity.module, &identity.name)
        }
    }
}

/// The base a field or element suffix appends to.
///
/// A suffix binds tighter than a unary operator, so a base that is not a
/// primary or postfix expression — the dereferenced element-store target
/// `*(&(out[i]))` — must be parenthesized before `.field` or `[index]` is
/// appended; the target otherwise parses as `*((&(out[i])).a)`, which the
/// strict C build rejects. Identifier, member, index, and already
/// parenthesized bases keep their existing spelling, so every other emitted
/// drop stays byte-identical.
fn suffix_base(c_name: &str) -> String {
    if c_name.starts_with('*') || c_name.starts_with('&') {
        format!("({c_name})")
    } else {
        c_name.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::suffix_base;

    #[test]
    fn suffix_bases_are_grouped_only_when_they_are_not_primary() {
        // A dereferenced store target needs grouping before a suffix append.
        assert_eq!(suffix_base("*(&(out[i]))"), "(*(&(out[i])))");
        assert_eq!(suffix_base("&(p.a)"), "(&(p.a))");
        // Identifier, member, index, and parenthesized bases are unchanged.
        assert_eq!(suffix_base("s"), "s");
        assert_eq!(suffix_base("p.a"), "p.a");
        assert_eq!(suffix_base("rows[0]"), "rows[0]");
        assert_eq!(suffix_base("(*__cl_tmp0)"), "(*__cl_tmp0)");
    }
}
