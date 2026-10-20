//! Expression emission and the runtime guards.
//!
//! The emitter translates an [`ir::Value`] to a C expression and, separately,
//! emits the runtime guards a statement needs before it runs: the division
//! guard, the bounds check, and the float-to-integer range check. Guards are
//! emitted before the guarded statement and abort without running the cleanup
//! ladder, so a fault never reaches generated cleanup. Because the values come
//! from the typed IR, every type is read from the value rather than inferred
//! again from the source tree.

use crate::ast::{BinaryOp, ScalarType, UnaryOp};
use crate::ir::{
    Place, PlaceBase, Projection, Value, ValueKind, ValuePiece,
};
use crate::types::{is_signed_scalar, scalar_bits, CaptureMode, Ty, TyKind};

use super::render::{
    binary_token, element_type, escape_c_string, float_literal, index_text, unsigned_c,
    unsigned_scalar,
};
use super::types::{
    c_type, is_struct_view, scalar_of, type_key, zero_view_helper,
};
use super::Emitter;
use super::FuncInfo;

impl<'a> Emitter<'a> {
    // -- values --------------------------------------------------------------

    /// Renders a typed IR value as a C expression.
    pub(super) fn value(&self, value: &Value) -> String {
        match &value.kind {
            ValueKind::Place(place) => self.place(place),
            ValueKind::Int(number) => self.int_literal(*number as u64, &self.substitute(&value.ty)),
            ValueKind::Float(number) => float_literal(*number),
            ValueKind::Char(byte) => format!("{byte}"),
            ValueKind::Bool(flag) => if *flag { "1" } else { "0" }.to_owned(),
            ValueKind::Null => self.null_value(&value.ty),
            ValueKind::Str(text) => format!("cl_str({})", escape_c_string(text.as_bytes())),
            ValueKind::Path(segments) => self.path(segments),
            ValueKind::Member { base, name } => {
                // A fixed array's `.len` is its structural extent, exactly as
                // the `len` intrinsic folds it: there is no member to read and
                // the base evaluates nothing.
                if name == "len" {
                    if let TyKind::Array { extents, .. } = &self.substitute(&base.ty).kind {
                        if extents.iter().all(Option::is_some) {
                            let extent = extents.first().and_then(|size| *size).unwrap_or(0);
                            return format!("{extent}");
                        }
                    }
                }
                let base_c = self.deref_base(base, self.value(base));
                if matches!(self.substitute(&base.ty).kind, TyKind::Raw(_)) {
                    format!("{base_c}->{name}")
                } else {
                    format!("{base_c}.{name}")
                }
            }
            ValueKind::Index { base, index } => {
                let base_c = self.deref_base(base, self.value(base));
                let index_c = self.value(index);
                index_text(&self.substitute(&base.ty), &base_c, &index_c)
            }
            ValueKind::Unary { op, operand } => self.unary(*op, operand, value),
            // A short-circuit operator evaluates its right operand only when
            // the left decides to, so the right operand carries its guards
            // inline rather than as statements before the whole expression.
            ValueKind::Binary {
                op: BinaryOp::LogicalAnd,
                left,
                right,
            } => format!("({} && {})", self.value(left), self.guarded_value(right)),
            ValueKind::Binary {
                op: BinaryOp::LogicalOr,
                left,
                right,
            } => format!("({} || {})", self.value(left), self.guarded_value(right)),
            ValueKind::Binary { op, left, right } => self.binary(*op, left, right, value),
            ValueKind::Cast { value: inner, .. } => {
                let target = self.substitute(&value.ty);
                match &target.kind {
                    TyKind::Void => format!("(void)({})", self.value(inner)),
                    _ => format!("({})({})", c_type(&target, &self.sema.table), self.value(inner)),
                }
            }
            ValueKind::Conditional {
                condition,
                then_value,
                else_value,
            } => {
                let result = self.substitute(&value.ty);
                format!(
                    "({} ? {} : {})",
                    self.value(condition),
                    self.conditional_arm(then_value, &result),
                    self.conditional_arm(else_value, &result)
                )
            }
            ValueKind::StructLit { name, fields } => {
                self.struct_literal(&self.substitute(&value.ty), name, fields)
            }
            ValueKind::ArrayLit { elements, fill } => self.array_literal(&self.substitute(&value.ty), elements, fill),
            ValueKind::Template(pieces) => self.template(pieces),
            ValueKind::Intrinsic {
                name,
                args,
                type_arg,
            } => self.intrinsic(name, args, type_arg.as_ref()),
            ValueKind::Call {
                callee,
                type_args,
                args,
                receiver,
            } => self.call(callee, type_args, args, receiver.as_deref()),
            ValueKind::Closure { site, captures } => self.closure_value(*site, captures),
            ValueKind::ClosureCall { callable, args } => self.closure_call(callable, args),
            // A staged-out form never reaches a valid compile; the fallback
            // keeps the renderer total.
            ValueKind::Aggregate => "0".to_owned(),
            ValueKind::Temp(id) => format!("__cl_tmp{id}"),
            // The temporary is written where the value's guards are emitted;
            // reading it here reuses the single evaluation. A named
            // destination is read through its own place, whose scope owns the
            // value.
            ValueKind::Materialize {
                temp,
                destination,
                ..
            } => match (temp, destination) {
                (_, Some(place)) => self.place(place),
                (Some(temp), None) => format!("__cl_tmp{temp}"),
                (None, None) => {
                    unreachable!("a materialization names a temporary or a destination")
                }
            },
        }
    }

    /// Renders a base expression, parenthesizing a dereference.
    ///
    /// C binds `*` below a member or element selection, so `*(p).f` reads a
    /// member of the pointer and `*(p)[i]` indexes the pointer. The selection
    /// belongs to the pointee, so the dereference text is grouped first.
    fn deref_base(&self, base: &Value, base_c: String) -> String {
        if matches!(
            base.kind,
            ValueKind::Unary {
                op: UnaryOp::Deref,
                ..
            }
        ) {
            format!("({base_c})")
        } else {
            base_c
        }
    }

    /// Renders a place as a C lvalue or rvalue.
    pub(super) fn place(&self, place: &Place) -> String {
        let mut text = self.place_base(&place.base);
        let mut ty = self.substitute(&self.place_base_type(&place.base));
        for projection in &place.projections {
            match projection {
                Projection::Field(field) => {
                    // A fixed array has no members: `.len` is its structural
                    // extent, exactly as the `len` intrinsic and the member
                    // value fold it. The base evaluates nothing, so the
                    // spelling is the constant.
                    if field == "len" {
                        if let TyKind::Array { extents, .. } = &ty.kind {
                            if extents.iter().all(Option::is_some) {
                                let extent = extents.first().and_then(|size| *size).unwrap_or(0);
                                text = format!("{extent}");
                                ty = Ty::usize_ty();
                                continue;
                            }
                        }
                    }
                    let through_raw = matches!(ty.kind, TyKind::Raw(_));
                    text = if through_raw {
                        format!("{text}->{field}")
                    } else {
                        format!("{text}.{field}")
                    };
                    ty = self.field_type(&ty, field);
                }
                Projection::Element(Some(index)) => {
                    text = index_text(&ty, &text, &index.to_string());
                    ty = element_type(&ty);
                }
                Projection::ElementTemp(temp) => {
                    text = index_text(&ty, &text, &format!("__cl_tmp{temp}"));
                    ty = element_type(&ty);
                }
                Projection::Element(None) => {
                    // A run-time index does not survive on a projected place;
                    // the value forms carry it instead. A projected place only
                    // reaches here as an assignment target the checker bounded,
                    // so element zero keeps the C total.
                    text = index_text(&ty, &text, "0");
                    ty = element_type(&ty);
                }
            }
        }
        text
    }

    /// The C spelling of a place's root.
    fn place_base(&self, base: &PlaceBase) -> String {
        match base {
            PlaceBase::Local(name) => self
                .local_names
                .get(name)
                .cloned()
                .unwrap_or_else(|| super::sanitize_identifier(name)),
            PlaceBase::Param(name) => {
                let c_name = self
                    .local_names
                    .get(name)
                    .cloned()
                    .unwrap_or_else(|| super::sanitize_identifier(name));
                if self.pointer_params.contains(name) {
                    // A writable lend of a non-duplicable struct is a C
                    // pointer, so a member access dereferences first.
                    format!("(*{c_name})")
                } else {
                    c_name
                }
            }
            PlaceBase::Global(id) => self.global_symbol(&id.module, &id.name),
            PlaceBase::Temp(id) => format!("__cl_tmp{id}"),
            // A capture is reached through the lifted function's environment
            // parameter, which is always named `__env`. A pointer field is
            // dereferenced so a read or write reaches the capturing frame.
            PlaceBase::EnvField {
                field, by_pointer, ..
            } => {
                if *by_pointer {
                    format!("(*__env->{field})")
                } else {
                    format!("(__env->{field})")
                }
            }
            // The temporary holds the address of the aliased element, so the
            // place is the dereferenced pointer.
            PlaceBase::DerefTemp { temp, .. } => format!("(*__cl_tmp{temp})"),
        }
    }

    /// The declared type of a place's root.
    ///
    /// A compiler temporary whose recorded type is still opaque resolves
    /// through the value that fills it, so a projected place rooted at such a
    /// temporary reads the instance's concrete type.
    pub(super) fn place_base_type(&self, base: &PlaceBase) -> Ty {
        match base {
            PlaceBase::Local(name) => self
                .function
                .and_then(|function| {
                    function.locals.iter().find(|local| &local.name == name)
                })
                .map(|local| local.ty.clone())
                .unwrap_or_else(Ty::unknown),
            PlaceBase::Param(name) => self
                .function
                .and_then(|function| {
                    function.params.iter().find(|param| &param.name == name)
                })
                .map(|param| param.ty.clone())
                .unwrap_or_else(Ty::unknown),
            PlaceBase::Global(id) => self
                .globals
                .get(id)
                .map(|global| global.ty.clone())
                .unwrap_or_else(Ty::unknown),
            PlaceBase::Temp(id) => {
                let recorded = self
                    .function
                    .and_then(|function| function.temps.get(*id as usize).cloned())
                    .unwrap_or_else(Ty::unknown);
                self.temp_type(*id, &recorded)
            }
            // The environment field carries its own captured type, so the
            // emitter spells a projected field without a site lookup.
            PlaceBase::EnvField { ty, .. } => ty.clone(),
            // The alias carries the type of the value the address points at.
            PlaceBase::DerefTemp { ty, .. } => ty.clone(),
        }
    }

    /// The type of a field selected from `base`, following a raw pointer.
    ///
    /// A generic struct declares its fields in terms of its type parameters, so
    /// the instance's type arguments are substituted into the declaration, and
    /// the enclosing generic context's substitution is applied on top. This
    /// keeps a projected field concrete (`Pair<i32>.first` is `i32`) so a later
    /// projection or index picks the right C type.
    pub(super) fn field_type(&self, base: &Ty, field: &str) -> Ty {
        match &base.kind {
            TyKind::Struct(..) => self
                .sema
                .table
                .instance_field(base, field)
                .map(|declared| self.substitute(&declared))
                .unwrap_or_else(Ty::unknown),
            TyKind::Raw(inner) => self.field_type(inner, field),
            // A view's ABI members resolve through the shared rule, so a
            // substituted member read types for the store's conversion.
            _ => crate::types::view_member_ty(base, field).unwrap_or_else(Ty::unknown),
        }
    }

    /// The type of a place after the active instance substitution.
    ///
    /// A place recorded in a generic body can still be opaque at the emission
    /// — a member of the element placeholder, for example — so the recorded
    /// type is resolved through the base and its projections exactly as the
    /// value resolver walks them. A store's drop decision must see the
    /// resolved type, or the owner it overwrites is never released.
    pub(super) fn resolved_place_ty(&self, place: &Place) -> Ty {
        let ty = self.substitute(&place.ty);
        if !ty.is_unknown() {
            return ty;
        }
        let mut current = self.substitute(&self.place_base_type(&place.base));
        for projection in &place.projections {
            if current.is_unknown() {
                return ty;
            }
            current = match projection {
                Projection::Field(field) => self.field_type(&current, field),
                Projection::Element(_) | Projection::ElementTemp(_) => element_type(&current),
            };
        }
        if current.is_unknown() {
            ty
        } else {
            current
        }
    }

    /// True when a text view's pointer member read reaches a raw destination.
    ///
    /// The member's C field is a `const u8 *` while its checked type is the
    /// plain `raw u8` such a destination holds, so C rejects the store as a
    /// discarded qualifier; the store spells the sanctioned cast instead. A
    /// slice's pointer member is already a mutable pointer, and a `cstring`
    /// destination keeps its own member conversion.
    pub(super) fn raw_text_view_pointer(&self, value: &Value, destination: &Ty) -> bool {
        if !matches!(self.substitute(destination).kind, TyKind::Raw(_)) {
            return false;
        }
        match &value.kind {
            ValueKind::Member { base, name } => {
                name == "ptr"
                    && matches!(
                        self.resolved_value_ty(base).kind,
                        TyKind::Str | TyKind::StaticStr
                    )
            }
            // The dotted spelling lowers to a place whose final projection is
            // the member read, so the type the projection selects from
            // decides the base, exactly as the place renderer walks it.
            ValueKind::Place(place) => {
                let Some((Projection::Field(name), rest)) = place.projections.split_last() else {
                    return false;
                };
                if name != "ptr" {
                    return false;
                }
                let mut current = self.substitute(&self.place_base_type(&place.base));
                for projection in rest {
                    if current.is_unknown() {
                        return false;
                    }
                    current = match projection {
                        Projection::Field(field) => self.field_type(&current, field),
                        Projection::Element(_) | Projection::ElementTemp(_) => {
                            element_type(&current)
                        }
                    };
                }
                matches!(current.kind, TyKind::Str | TyKind::StaticStr)
            }
            _ => false,
        }
    }

    /// The type of a value after instance substitution, following projections.
    ///
    /// A projection recorded in a generic body carries the opaque type: the
    /// field cannot be resolved until the instance's type arguments are known.
    /// The substituted base makes it concrete, so the projection resolves
    /// through the same field lookup the place renderer walks and the `len`
    /// fold sees the real type.
    pub(super) fn resolved_value_ty(&self, value: &Value) -> Ty {
        let ty = self.substitute(&value.ty);
        if !ty.is_unknown() {
            return ty;
        }
        match &value.kind {
            ValueKind::Member { base, name } => {
                let base_ty = self.resolved_value_ty(base);
                let field = self.field_type(&base_ty, name);
                if field.is_unknown() {
                    ty
                } else {
                    field
                }
            }
            // A store target's pointer records the pointee type, which the
            // operand's projection chain carries. A generic declaration records
            // that type opaquely, so the address resolves through the element
            // or member the instance substitutes.
            ValueKind::Unary {
                op: UnaryOp::AddressOf,
                operand,
            } => self.resolved_value_ty(operand),
            ValueKind::Index { base, .. } => {
                let base_ty = self.resolved_value_ty(base);
                let element = element_type(&base_ty);
                if element.is_unknown() {
                    ty
                } else {
                    element
                }
            }
            // A conditional selects one arm at run time, so the projection
            // resolves only when both arms name the same substituted type; a
            // mixed pair stays opaque and keeps the existing behavior.
            ValueKind::Conditional {
                then_value,
                else_value,
                ..
            } => {
                // A `null` arm carries the other arm's nullable type. The type
                // stage leaves the conditional opaque when the owning arm is
                // still a generic projection, so resolving the non-null arm
                // here gives the temporary the instance's concrete shape.
                if matches!(then_value.kind, ValueKind::Null)
                    || matches!(else_value.kind, ValueKind::Null)
                {
                    let other = if matches!(then_value.kind, ValueKind::Null) {
                        else_value
                    } else {
                        then_value
                    };
                    let mut other_ty = self.resolved_value_ty(other);
                    if !other_ty.is_unknown() {
                        other_ty.nullable = true;
                        return other_ty;
                    }
                    return ty;
                }
                let then_ty = self.resolved_value_ty(then_value);
                let else_ty = self.resolved_value_ty(else_value);
                if then_ty.is_unknown() || then_ty != else_ty {
                    ty
                } else {
                    then_ty
                }
            }
            // A projected read carries the same opacity: the place's root
            // type is substituted and its projections walk like the place
            // renderer's, so `(*b).s` resolves through the instance.
            ValueKind::Place(place) => {
                let mut resolved = self.substitute(&self.place_base_type(&place.base));
                for projection in &place.projections {
                    resolved = match projection {
                        Projection::Field(field) => self.field_type(&resolved, field),
                        Projection::Element(_) | Projection::ElementTemp(_) => {
                            element_type(&resolved)
                        }
                    };
                }
                if resolved.is_unknown() {
                    ty
                } else {
                    resolved
                }
            }
            // A compiler temporary's recorded type can still be opaque when a
            // generic body created it for an unresolved value; the fill
            // resolves it, exactly as the place resolver resolves a projected
            // place.
            ValueKind::Temp(id) => self.temp_type(*id, &ty),
            ValueKind::Materialize {
                destination: Some(place),
                ..
            } => self.substitute(&place.ty),
            ValueKind::Materialize {
                temp: Some(temp),
                ..
            } => self.temp_type(*temp, &ty),
            _ => ty,
        }
    }

    /// True when a value carries the null literal's nullable-unknown shape.
    pub(super) fn is_null_shaped(&self, value: &Value) -> bool {
        crate::types::is_null_shaped(&self.substitute(&value.ty))
    }

    /// Renders an integer literal with a suffix when it does not fit `int`.
    fn int_literal(&self, value: u64, ty: &Ty) -> String {
        let unsigned = matches!(
            ty.kind,
            TyKind::Scalar(
                ScalarType::U8
                    | ScalarType::U16
                    | ScalarType::U32
                    | ScalarType::Usize
                    | ScalarType::Bool
            )
        );
        if unsigned || value > i32::MAX as u64 {
            format!("{value}u")
        } else {
            format!("{value}")
        }
    }

    /// Renders a dotted name: an enum member, a function value, or a field path.
    fn path(&self, segments: &[String]) -> String {
        let Some(first) = segments.first() else {
            return "0".to_owned();
        };
        let rest = segments[1..].join(".");
        if self.enums.contains_key(first) {
            return format!("{first}_{rest}");
        }
        // A single-segment name that resolves to a function is a function
        // value: a bare identifier names the function (there is no `&`), so it
        // lowers to the emitted symbol, which decays to the C function pointer
        // the declared `func(...)` signature expects. A generic template is not
        // a value on its own, so an uninstantiated name keeps its spelling.
        if rest.is_empty() {
            if let Some(info) = self.resolve_function(first) {
                if info.decl.type_params.is_empty() {
                    return self.function_symbol(&info.module, &info.decl.name);
                }
            }
        }
        // A name that is not a local names a module-level binding: the head
        // is the binding with its fields, or the leading segments are the
        // declaring module and the last segment is the binding. Either way
        // the read renders the mangled symbol, so a struct global's field
        // and a qualified binding resolve the same identity a bare imported
        // name does.
        if !self.local_names.contains_key(first) {
            if let Some((id, fields)) = self.global_binding_path(segments) {
                let symbol = self.global_symbol(&id.module, &id.name);
                return if fields.is_empty() {
                    symbol
                } else {
                    format!("{symbol}.{fields}")
                };
            }
        }
        let base = self
            .local_names
            .get(first)
            .cloned()
            .unwrap_or_else(|| super::sanitize_identifier(first));
        if rest.is_empty() {
            base
        } else {
            format!("{base}.{rest}")
        }
    }

    /// The module-level func-typed binding a call's callee name denotes.
    ///
    /// A callee spelled with its declaring module (`m2.pick`) names the
    /// binding exactly, so two same-named globals stay apart. A bare name is
    /// resolved the way a read is: the current module wins, and otherwise the
    /// name must be unique among the loaded modules, because taking one of
    /// several same-named bindings would silently reach the wrong one. A
    /// function the caller's module declares or one of its imports binds is
    /// the callee instead, and a module-level value that merely shares the
    /// name never shadows it.
    pub(super) fn func_global_binding(
        &self,
        callee: &str,
    ) -> Option<(crate::ir::GlobalId, Ty)> {
        // A function the caller's module declares or one of its imports binds
        // is the callee: the call reaches the function's own symbol, and a
        // module-level value that merely shares the name never shadows it. A
        // function of that name elsewhere is not in scope, so it must not
        // displace the binding the caller's module or its import names.
        if self.function_in_scope(callee) {
            return None;
        }
        if let Some((module, name)) = callee.rsplit_once('.') {
            return self
                .globals
                .iter()
                .find(|(id, info)| {
                    id.module == module
                        && id.name == name
                        && matches!(self.substitute(&info.ty).kind, TyKind::Func(_))
                })
                .map(|(id, info)| (id.clone(), self.substitute(&info.ty)));
        }
        let same_module = self.globals.iter().find(|(id, info)| {
            id.name == callee
                && id.module == self.current_module
                && matches!(self.substitute(&info.ty).kind, TyKind::Func(_))
        });
        if let Some((id, info)) = same_module {
            return Some((id.clone(), self.substitute(&info.ty)));
        }
        let mut candidates = self.globals.iter().filter(|(id, info)| {
            id.name == callee && matches!(self.substitute(&info.ty).kind, TyKind::Func(_))
        });
        let (id, info) = candidates.next()?;
        if candidates.next().is_some() {
            return None;
        }
        Some((id.clone(), self.substitute(&info.ty)))
    }

    /// The module-level binding a dotted name names, with its field path.
    ///
    /// A name whose head is a module-level binding names that binding with
    /// its fields (`h.call`); otherwise the leading segments spell the
    /// declaring module and the binding follows them (`spec.cri.globalcallee.g`,
    /// or a name the lowering already qualified with its module:
    /// `spec.cri.globalcallee.h.call`). A simple head prefers the current
    /// module, exactly as the name resolver does, and it is taken from another
    /// module only when exactly one module declares it; several same-named
    /// bindings with no module spelled leave the name unresolved rather than
    /// silently reading one of them.
    fn global_binding_path(
        &self,
        segments: &[String],
    ) -> Option<(crate::ir::GlobalId, String)> {
        let (first, rest) = segments.split_first()?;
        // The leading segments may spell the declaring module with the binding
        // after them, so a qualified name resolves the exact binding and the
        // segments after it are its fields. The longest module prefix wins,
        // and it wins over a value head of the same spelling (Ch 10 MOD-13).
        for split in (1..segments.len()).rev() {
            let module = segments[..split].join(".");
            let name = &segments[split];
            if let Some((id, _)) = self
                .globals
                .iter()
                .find(|(id, _)| id.module == module && &id.name == name)
            {
                return Some((id.clone(), segments[split + 1..].join(".")));
            }
        }
        let head = |id: &crate::ir::GlobalId| &id.name == first;
        if let Some((id, _)) = self
            .globals
            .iter()
            .find(|(id, _)| head(id) && id.module == self.current_module)
        {
            return Some((id.clone(), rest.join(".")));
        }
        let mut candidates = self.globals.iter().filter(|(id, _)| head(id));
        let head_match = match candidates.next() {
            Some(found) => candidates.next().is_none().then_some(found),
            None => None,
        };
        if let Some((id, _)) = head_match {
            return Some((id.clone(), rest.join(".")));
        }
        None
    }

    /// Renders a unary operator, wrapping signed negation through unsigned.
    fn unary(&self, op: UnaryOp, operand: &Value, whole: &Value) -> String {
        let operand_c = self.value(operand);
        match op {
            UnaryOp::Not => format!("!({operand_c})"),
            UnaryOp::BitNot => format!("~({operand_c})"),
            UnaryOp::Neg => {
                let ty = self.substitute(&whole.ty);
                match scalar_of(&ty) {
                    Some(scalar) if is_signed_scalar(scalar) => {
                        let unsigned = unsigned_scalar(scalar);
                        format!("({})(-({}){operand_c})", c_type(&ty, &self.sema.table), unsigned_c(unsigned))
                    }
                    _ => format!("-({operand_c})"),
                }
            }
            UnaryOp::AddressOf => format!("&({operand_c})"),
            UnaryOp::Deref => format!("*({operand_c})"),
            UnaryOp::PreIncrement => format!("++({operand_c})"),
            UnaryOp::PreDecrement => format!("--({operand_c})"),
            UnaryOp::PostIncrement => format!("({operand_c})++"),
            UnaryOp::PostDecrement => format!("({operand_c})--"),
        }
    }

    /// True when a value carries the null literal's shape in its own storage.
    ///
    /// The typed-null lowering applies to a place or a compiler temporary —
    /// the values a null-shaped local or literal produces. A value that merely
    /// *types* as the shape (a lambda body returning a null-shaped capture,
    /// for example) still computes something, so it renders and runs instead
    /// of being replaced by the destination's empty value.
    pub(super) fn carries_null_shape(&self, value: &Value) -> bool {
        self.is_null_shaped(value)
            && matches!(value.kind, ValueKind::Place(_) | ValueKind::Temp(_))
    }

    /// True when a type carries the null shape's empty value.
    ///
    /// The frozen nullable set is a handle, a `raw` pointer, a `cstring`, and
    /// a view or slice: those lower the null shape to their own empty value.
    /// Every other type — a scalar, an enum, a `bool`, a struct, a `func` —
    /// has no empty state a null literal can name, so the position's own
    /// accepting rule keeps the value's rendering there.
    pub(super) fn carries_null_value(&self, ty: &Ty) -> bool {
        matches!(
            self.substitute(ty).kind,
            TyKind::Handle(_)
                | TyKind::Raw(_)
                | TyKind::CString
                | TyKind::Str
                | TyKind::StaticStr
                | TyKind::Array { .. }
        )
    }

    /// Renders the null literal for a value of `ty`.
    ///
    /// A pointer-like type spells null as `NULL` (or the profile's zero), but a
    /// view is an emitted pointer-and-length struct, so its empty state is the
    /// zero struct; `NULL` would not type-check against it. On the profiles
    /// whose null literal is the integer constant, a pointer destination takes
    /// its own pointer cast: a conditional whose arms are both the bare
    /// constant types as `int`, and the store to the pointer fails the
    /// compiler's `-Werror=int-conversion`.
    pub(super) fn null_value(&self, ty: &Ty) -> String {
        let ty = self.substitute(ty);
        if let Some(helper) = zero_view_helper(&ty) {
            // A view's empty state is its zero value; the helper is emitted
            // with the view typedefs so no compound literal is needed.
            format!("{helper}()")
        } else if self.null_literal() == "0"
            && matches!(ty.kind, TyKind::CString | TyKind::Raw(_))
        {
            format!("({})0", c_type(&ty, &self.sema.table))
        } else {
            self.null_literal().to_owned()
        }
    }

    /// Renders one conditional arm, giving a null arm the result's empty value.
    ///
    /// A null arm is typed by the conditional's result, so a struct-typed view
    /// result needs its zero view rather than the untyped null literal. A
    /// conditional is not always typed by the context on its own: an all-null
    /// conditional carries the null literal's unknown type, and an owned-handle
    /// conditional beside a null arm must widen to the destination's view, so
    /// the outer result type carries into those arms. An owned `String` arm
    /// widens to the view result through the shim view, exactly as a store
    /// spells the widening, because one C `?:` must carry one type in both
    /// arms.
    pub(super) fn conditional_arm(&self, value: &Value, result: &Ty) -> String {
        // A null-shaped arm carries no computed value: the arm takes the
        // result's own empty value exactly as the null literal does.
        if matches!(value.kind, ValueKind::Null) || self.carries_null_shape(value) {
            return self.null_value(result);
        }
        if let ValueKind::Conditional {
            condition,
            then_value,
            else_value,
        } = &value.kind
        {
            let own = self.substitute(&value.ty);
            let nested = if value.ty.is_unknown()
                || (matches!(&own.kind, TyKind::Handle(name) if name == "String")
                    && is_struct_view(result))
            {
                result.clone()
            } else {
                own
            };
            return format!(
                "({} ? {} : {})",
                self.value(condition),
                self.conditional_arm(then_value, &nested),
                self.conditional_arm(else_value, &nested)
            );
        }
        // A nullable owned arm keeps its nullness through the widening: the
        // guard reads the arm's place twice and selects the result's empty
        // view for a null handle. A fresh arm binds to a temporary before the
        // conditional's store instead, so only a place-rooted arm spells it.
        // The arm has no statement position for its operand guards, so they
        // ride inline, once, ahead of the test. The arm's type resolves
        // through the instance substitution, so a generic member read reaches
        // the nullness decision and the widening with its concrete type.
        let value_ty = self.resolved_value_ty(value);
        if let Some(guarded) = self.guarded_view_widening_inline(value, &value_ty, result) {
            return guarded;
        }
        let text = self.guarded_value(value);
        let text = self.widen_text_view(text, &value_ty, result);
        // A text view's pointer member arm at a raw result spells the
        // sanctioned cast, exactly as a store does.
        if self.raw_text_view_pointer(value, result) {
            return format!("(u8 *)({text})");
        }
        text
    }

    /// Renders a null test on a struct-typed view, when the pair is one.
    ///
    /// A view is an emitted pointer-and-length struct, so C cannot compare it
    /// with a null pointer directly; the test reads its `ptr` member instead.
    /// Returns `None` when the comparison is not a view-to-null equality test.
    fn view_null_test(&self, op: BinaryOp, left: &Value, right: &Value) -> Option<String> {
        if !matches!(op, BinaryOp::Eq | BinaryOp::Ne) {
            return None;
        }
        let token = if matches!(op, BinaryOp::Eq) { "==" } else { "!=" };
        let view = match (&left.kind, &right.kind) {
            // Two null literals carry no view value whose member could be
            // read, so they keep the plain pointer comparison.
            (ValueKind::Null, ValueKind::Null) => {
                return Some(format!(
                    "({} {token} {})",
                    self.null_literal(),
                    self.null_literal()
                ));
            }
            (_, ValueKind::Null) => left,
            (ValueKind::Null, _) => right,
            // A local holding the null literal's shape (a nullable unknown) is
            // a null operand too: its value is the null pointer, so the pair
            // reads it against the view's pointer, exactly as the checker's
            // null-pair rule reads the view's emptiness.
            _ => {
                let (null_side, view) =
                    match (self.is_null_shaped(left), self.is_null_shaped(right)) {
                        (true, false) => (left, right),
                        (false, true) => (right, left),
                        _ => return None,
                    };
                let view_ty = self.substitute(&view.ty);
                if !is_struct_view(&view_ty) {
                    return None;
                }
                let null_text = self.value(null_side);
                let view_text = self.value(view);
                return Some(format!("({null_text} {token} ({view_text}).ptr)"));
            }
        };
        let ty = self.substitute(&view.ty);
        if !is_struct_view(&ty) {
            return None;
        }
        let text = self.value(view);
        Some(format!("(({text}).ptr {token} {})", self.null_literal()))
    }

    /// Renders a binary operator with the wrapping and masking strategy.
    fn binary(&self, op: BinaryOp, left: &Value, right: &Value, whole: &Value) -> String {
        if let Some(test) = self.view_null_test(op, left, right) {
            return test;
        }
        let left_c = self.value(left);
        let right_c = self.value(right);
        let ty = self.substitute(&whole.ty);
        // A compound store recorded in a generic body carries the opaque
        // destination type. The left operand names the destination place, whose
        // per-instance type resolves the arithmetic shape, so the signed wrap
        // and shift masks match the concrete spelling.
        let ty = if ty.is_unknown() {
            let resolved = self.resolved_value_ty(left);
            if resolved.is_unknown() {
                self.resolved_value_ty(right)
            } else {
                resolved
            }
        } else {
            ty
        };
        if matches!(op, BinaryOp::Shl | BinaryOp::Shr) {
            if let Some(scalar) = scalar_of(&ty) {
                let mask = scalar_bits(scalar).saturating_sub(1);
                let count = format!("({right_c} & {mask})");
                // A signed left shift into the sign bit is undefined, so `<<`
                // runs on an unsigned operand and is cast back. A signed `>>`
                // stays signed: the specification defines it as arithmetic, and
                // masking the count already keeps the shift well-defined.
                if matches!(op, BinaryOp::Shl) && is_signed_scalar(scalar) {
                    let unsigned = unsigned_c(unsigned_scalar(scalar));
                    return format!("({})(({}){left_c} << {count})", c_type(&ty, &self.sema.table), unsigned);
                }
                let operator = if matches!(op, BinaryOp::Shl) { "<<" } else { ">>" };
                return format!("({left_c} {operator} {count})");
            }
        }
        if matches!(op, BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul) {
            if let Some(scalar) = scalar_of(&ty) {
                if is_signed_scalar(scalar) {
                    let unsigned = unsigned_c(unsigned_scalar(scalar));
                    let operator = match op {
                        BinaryOp::Add => "+",
                        BinaryOp::Sub => "-",
                        BinaryOp::Mul => "*",
                        _ => unreachable!(),
                    };
                    return format!(
                        "({})(({}){left_c} {operator} ({}){right_c})",
                        c_type(&ty, &self.sema.table),
                        unsigned,
                        unsigned
                    );
                }
            }
        }
        format!("({left_c} {} {right_c})", binary_token(op))
    }

    /// The move-out of a place as a comma-expression operand.
    ///
    /// A deferred `take` move-out must run inside the expression that
    /// evaluates a guarded operand, so the place is emptied with the same
    /// spelling `Statement::MoveOut` uses: a handle becomes `(place = NULL)`,
    /// every other type `(cl_zero(&place), 0)`. A fixed-array parameter walks
    /// its deepest elements, because `cl_zero` would take `sizeof` of the
    /// adjusted pointer.
    pub(super) fn move_zero_expr(&self, place: &Place) -> String {
        let c_name = self.place(place);
        // A place recorded in a generic body can carry the opaque type — a
        // deferred take's carrier, for example — so the emptying resolves it
        // through the place's own base and projections.
        let ty = self.resolved_place_ty(place);
        if self.is_array_param_place(place) {
            if let TyKind::Array { element, extents } = &ty.kind {
                let dims: Vec<usize> = extents.iter().filter_map(|extent| *extent).collect();
                let parts = array_param_zero_expr(&c_name, element, &dims);
                return format!("({})", parts.join(", "));
            }
        }
        self.move_zero_text_expr(&c_name, &ty)
    }

    /// The move-out of an already-rendered C lvalue as a comma operand.
    ///
    /// A deferred conditional store empties its chosen arm inside the store
    /// expression, where the source is already rendered and cannot be turned
    /// back into a place. The spelling matches [`Self::move_zero_expr`].
    pub(super) fn move_zero_text_expr(&self, c_name: &str, ty: &Ty) -> String {
        match &ty.kind {
            TyKind::Handle(_) | TyKind::Raw(_) | TyKind::CString => {
                format!("({c_name} = {})", self.null_literal())
            }
            // A call operand has a side effect, so a plain comma operand
            // carries it without an unused-value warning for either side.
            _ => format!("(cl_zero(&{c_name}))"),
        }
    }

    /// Renders a call to a user function, an FFI function, or a helper.
    ///
    /// A call whose callee is a `func`-typed field renders the field as the
    /// callable expression and passes no receiver argument; every other call
    /// resolves the callee name to a C symbol. An unresolved callee name is
    /// spelled as-is so a compiler-known helper such as `text` still renders.
    pub(super) fn call(
        &self,
        callee: &str,
        type_args: &[Ty],
        args: &[Value],
        receiver: Option<&Value>,
    ) -> String {
        self.render_call(callee, type_args, args, receiver, None)
    }

    /// Renders a call whose by-value fixed-array result goes to a destination.
    ///
    /// C cannot return an array by value, so a fixed-array-returning function
    /// takes the caller's destination as one extra trailing argument. The
    /// destination is appended after the rendered arguments.
    pub(super) fn call_with_destination(
        &self,
        callee: &str,
        type_args: &[Ty],
        args: &[Value],
        receiver: Option<&Value>,
        destination: &str,
    ) -> String {
        self.render_call(callee, type_args, args, receiver, Some(destination))
    }

    /// Renders a call, optionally appending one trailing destination argument.
    fn render_call(
        &self,
        callee: &str,
        type_args: &[Ty],
        args: &[Value],
        receiver: Option<&Value>,
        destination: Option<&str>,
    ) -> String {
        // A call through a func value takes its parameter types and modes from
        // the value's own signature, so a `borrow` parameter and a `str`
        // widening reach the argument renderer.
        let func_ty = self.callee_func_type(callee, receiver);
        let mut rendered: Vec<String> = Vec::new();
        if !args.is_empty() {
            rendered.push(self.call_arguments(callee, type_args, args, func_ty.as_ref()));
        }
        if let Some(destination) = destination {
            rendered.push(destination.to_owned());
        }
        let argument_list = rendered.join(", ");
        if let Some(receiver) = receiver {
            let target = self.value(receiver);
            return format!("({target})({argument_list})");
        }
        // A closure-typed parameter shadows a module function of the same
        // written name, exactly as a local does. Inside a monomorphized
        // generic body such a call goes through the two-word value; testing it
        // before the named-function lookup keeps the callee and the result
        // type consistent.
        if let Some(closure) = self.closure_param_type(callee) {
            let callable = self.place(&Place::param(callee).with_type(closure));
            return if argument_list.is_empty() {
                format!("(({callable}).fn(({callable}).env))")
            } else {
                format!("(({callable}).fn(({callable}).env, {argument_list}))")
            };
        }
        // A func-typed binding shadows a module function of the same written
        // name, so the call goes through the binding's own C local before any
        // function or helper name is resolved.
        if let Some(spelled) = self.func_binding_spelling(callee) {
            return format!("{spelled}({argument_list})");
        }
        if let Some(info) = self.resolve_function(callee) {
            // The resolved declaration's own name spells the symbol: a name
            // bound by an aliased import resolves to the imported function,
            // whose C symbol keeps its declared name.
            let name = info.decl.name.as_str();
            let symbol = if type_args.is_empty() {
                self.function_symbol(&info.module, name)
            } else {
                let keys: Vec<String> = self
                    .resolved_call_type_args(info, type_args, args)
                    .iter()
                    .map(type_key)
                    .collect();
                format!("cl_{name}__{}", keys.join("_"))
            };
            return format!("{symbol}({argument_list})");
        }
        match callee {
            "text" => self.text_call(args.first()),
            _ => {
                // A func-typed local is called through its own C spelling, so
                // the compiler's local names map resolves it. An `ffi` call,
                // qualified or not, names the declaration's own C symbol. Every
                // other unresolved name is a compiler-known helper spelled
                // as-is.
                let spelled = self
                    .local_names
                    .get(callee)
                    .cloned()
                    .or_else(|| {
                        self.ffi_callee_signature(callee)
                            .map(|signature| signature.name.clone())
                    })
                    .unwrap_or_else(|| callee.to_owned());
                format!("{spelled}({argument_list})")
            }
        }
    }

    /// The concrete type arguments of a call in the instance being emitted.
    ///
    /// A recorded argument can still be opaque in a generic body — a member
    /// read on the element placeholder, for example — because the declaration
    /// has no substitution to resolve it with. The substituted argument values
    /// resolve it through the same matcher the call's own inference uses, so
    /// the symbol names the concrete instance the monomorphization declares.
    pub(super) fn resolved_call_type_args(
        &self,
        info: &FuncInfo,
        type_args: &[Ty],
        args: &[Value],
    ) -> Vec<Ty> {
        let resolved: Vec<Ty> = type_args.iter().map(|ty| self.substitute(ty)).collect();
        if !resolved.iter().any(|ty| ty.is_unknown() || ty.is_param()) {
            return resolved;
        }
        let declared: Vec<String> = info
            .decl
            .type_params
            .iter()
            .map(|param| param.name.clone())
            .collect();
        let param_types: Vec<Ty> = info
            .decl
            .params
            .iter()
            .map(|param| self.resolve_in_module(&info.module, &param.ty, &declared))
            .collect();
        let arg_types: Vec<Ty> = args.iter().map(|arg| self.resolved_value_ty(arg)).collect();
        crate::sema::call::infer_type_args(&declared, &param_types, &arg_types).unwrap_or(resolved)
    }

    /// The closure type of a callable parameter of the open function.
    ///
    /// A generic body's `f(x)` names no declared function; when the active
    /// instance substitutes `f`'s parameter with a closure type, the call is a
    /// two-word closure call. A `func`-typed parameter, and every non-parameter
    /// callee, yields `None`.
    pub(super) fn closure_param_type(&self, callee: &str) -> Option<Ty> {
        let function = self.function?;
        let param = function.params.iter().find(|param| param.name == callee)?;
        let ty = self.substitute(&param.ty);
        ty.is_closure().then_some(ty)
    }

    /// The C spelling of a func-typed name the callee refers to.
    ///
    /// A func-typed binding shadows a module function of the same written
    /// name. The binding is found by its IR spelling, which the lowering puts
    /// in the call for a local callee; a shadowing local takes a suffixed IR
    /// name, so a module call written before the local's declaration still
    /// resolves by function name. Inside a lifted lambda, a captured
    /// func-typed binding is reached through the environment, and a
    /// module-level func-typed global resolves to its own symbol, so neither
    /// call falls through to a same-named function or a bare name.
    fn func_binding_spelling(&self, callee: &str) -> Option<String> {
        let function = self.function?;
        let declared = function
            .locals
            .iter()
            .map(|local| (&local.name, &local.ty))
            .chain(
                function
                    .params
                    .iter()
                    .map(|param| (&param.name, &param.ty)),
            );
        for (name, ty) in declared {
            if name == callee && matches!(self.substitute(ty).kind, TyKind::Func(_)) {
                return Some(
                    self.local_names
                        .get(name)
                        .cloned()
                        .unwrap_or_else(|| name.clone()),
                );
            }
        }
        if let Some(site) = function.lambda_site {
            if let Some(closure) = self.sema.closures.get(&site) {
                if let Some(capture) = closure.captures.iter().find(|cap| cap.name == callee) {
                    if matches!(self.substitute(&capture.ty).kind, TyKind::Func(_)) {
                        // A copied capture is the field itself; a borrow
                        // capture holds the enclosing frame's address, so the
                        // call dereferences the field first.
                        return Some(match capture.mode {
                            CaptureMode::Copy => format!("(__env->{callee})"),
                            CaptureMode::ReadOnlyBorrow | CaptureMode::WriteBorrow => {
                                format!("(*__env->{callee})")
                            }
                        });
                    }
                }
            }
        }
        // A module-level func-typed global resolves through its own identity:
        // a qualified or imported spelling names the exact binding, and a
        // bare name is taken only when the current module declares it or a
        // single module does.
        self.func_global_binding(callee)
            .map(|(id, _)| self.global_symbol(&id.module, &id.name))
    }

    /// Renders a lambda value.
    ///
    /// A capture-less lambda is a plain C function pointer, so it is the lifted
    /// function's name. A capturing lambda is a two-word struct holding the
    /// function pointer and the address of a stack environment built inline; the
    /// environment is valid for the synchronous call that receives it. A lambda
    /// in a generic body names the active instance's specialization.
    fn closure_value(&self, site: usize, captures: &[Value]) -> String {
        let suffix = self.closure_instance_suffix(site);
        if captures.is_empty() {
            return format!("cl_lambda_{site}{suffix}");
        }
        let environment = self.environment_literal(site, captures);
        format!("((ClClosure_{site}{suffix}){{ cl_lambda_{site}{suffix}, &({environment}) }})")
    }

    /// The instance suffix of a lifted lambda's C names.
    ///
    /// A lambda in a generic body that the active enclosing instance
    /// specializes carries the instance key; a shared lifted function, or a
    /// closure rendered outside any instance body, keeps the site-only names.
    fn closure_instance_suffix(&self, site: usize) -> String {
        match &self.instance_key {
            Some(key) if self.has_lifted_specialization(site, key) => format!("__{key}"),
            _ => String::new(),
        }
    }

    /// Renders a call of a value that is an ephemeral closure.
    ///
    /// A literal closure is called through its lifted function directly, so the
    /// environment expression is evaluated once. Any other callable renders its
    /// two-word value once into a C temporary expression and calls through the
    /// pointer with the environment as the first argument.
    fn closure_call(&self, callable: &Value, args: &[Value]) -> String {
        self.closure_call_with_destination(callable, args, None)
    }

    /// Renders a closure call, optionally passing a fixed-array destination.
    ///
    /// A lifted function with a by-value fixed-array return takes the caller's
    /// destination as one extra trailing argument, after the environment and
    /// the lambda's own arguments, exactly like a named call's hidden
    /// out-parameter.
    pub(super) fn closure_call_with_destination(
        &self,
        callable: &Value,
        args: &[Value],
        destination: Option<&str>,
    ) -> String {
        let mut rendered: Vec<String> = args.iter().map(|arg| self.value(arg)).collect();
        if let Some(destination) = destination {
            rendered.push(destination.to_owned());
        }
        let argument_list = rendered.join(", ");
        if let ValueKind::Closure { site, captures } = &callable.kind {
            let suffix = self.closure_instance_suffix(*site);
            if captures.is_empty() {
                return format!("cl_lambda_{site}{suffix}({argument_list})");
            }
            let environment = self.environment_literal(*site, captures);
            return if argument_list.is_empty() {
                format!("cl_lambda_{site}{suffix}(&({environment}))")
            } else {
                format!("cl_lambda_{site}{suffix}(&({environment}), {argument_list})")
            };
        }
        let callable_c = self.value(callable);
        if argument_list.is_empty() {
            format!("(({callable_c}).fn(({callable_c}).env))")
        } else {
            format!("(({callable_c}).fn(({callable_c}).env, {argument_list}))")
        }
    }

    /// Renders an environment struct literal holding one capture per field.
    ///
    /// A fixed-array capture cannot initialize its array field from a
    /// whole-array expression — C has no array assignment — so the field is
    /// initialized element by element from the captured place. Every other
    /// capture renders as one expression.
    fn environment_literal(&self, site: usize, captures: &[Value]) -> String {
        let fields: Vec<String> = captures
            .iter()
            .map(|capture| self.capture_initializer(capture))
            .collect();
        let suffix = self.closure_instance_suffix(site);
        format!("((struct ClEnv_{site}{suffix}){{ {} }})", fields.join(", "))
    }

    /// Renders one capture value for the environment literal.
    fn capture_initializer(&self, capture: &Value) -> String {
        let ty = self.substitute(&capture.ty);
        if ty.is_fixed_array() {
            if let ValueKind::Place(place) = &capture.kind {
                return self.array_elements_literal(&self.place(place), &ty);
            }
        }
        self.value(capture)
    }

    /// Renders an array place as a brace-enclosed element list.
    ///
    /// A multi-extent array nests one brace per extent, so the emitted
    /// initializer matches the field's own C shape.
    fn array_elements_literal(&self, c_name: &str, ty: &Ty) -> String {
        let TyKind::Array { element, extents } = &ty.kind else {
            return c_name.to_owned();
        };
        let extent = extents.first().and_then(|size| *size).unwrap_or(0);
        if extents.len() > 1 {
            let nested = Ty::array((**element).clone(), extents[1..].to_vec());
            let parts: Vec<String> = (0..extent)
                .map(|index| {
                    self.array_elements_literal(&format!("{c_name}[{index}]"), &nested)
                })
                .collect();
            return format!("{{ {} }}", parts.join(", "));
        }
        let parts: Vec<String> = (0..extent)
            .map(|index| format!("{c_name}[{index}]"))
            .collect();
        format!("{{ {} }}", parts.join(", "))
    }

    /// Renders a call to the compiler-known `text` conversion.
    ///
    /// The compiler chooses the shim conversion from the argument type. A `str`
    /// or `static str` passes its pointer and length, because a view is a
    /// pointer-and-length struct that the shim cannot receive by value; a
    /// `String` is copied so the caller owns a fresh handle; a scalar passes
    /// its value directly. This is the same conversion a template placeholder
    /// lowers to, so both spellings agree.
    fn text_call(&self, arg: Option<&Value>) -> String {
        let Some(arg) = arg else {
            return "cl_text_i32()".to_owned();
        };
        let ty = self.substitute(&arg.ty);
        let value = self.value(arg);
        match &ty.kind {
            TyKind::Str | TyKind::StaticStr => {
                format!("cl_text_str(({value}).ptr, ({value}).len)")
            }
            TyKind::Handle(name) if name == "String" => format!("cl_string_copy({value})"),
            // The shim defines one converter per distinct C integer width it
            // can format. A narrower or wider scalar is normalized to the
            // converter that preserves its value: a 32-bit signed or unsigned
            // conversion covers every machine-word integer, and a `float` is
            // promoted to `double`, matching how C already passes it to a
            // variadic formatter.
            TyKind::Scalar(scalar) => self.scalar_text(*scalar, &value),
            _ => {
                // The shim defines one converter per value type; a nullable
                // value formats through the same converter, so the helper
                // spelling must not carry the nullability marker.
                let value_ty = Ty {
                    kind: ty.kind.clone(),
                    nullable: false,
                };
                format!("cl_text_{}({value})", type_key(&value_ty))
            }
        }
    }

    /// The shim call that formats a scalar, normalizing to the converters the
    /// shim defines.
    ///
    /// A 16-bit or 8-bit integer and the pointer-sized signed integer fit the
    /// `i32` converter; `usize` on the 32-bit target fits the `u32` converter;
    /// an `f32` promotes to the `f64` converter. The decimal value is unchanged
    /// by the widening cast, so the printed text is the same.
    fn scalar_text(&self, scalar: ScalarType, value: &str) -> String {
        match scalar {
            ScalarType::I8
            | ScalarType::I16
            | ScalarType::I32
            | ScalarType::Isize => format!("cl_text_i32((i32)({value}))"),
            ScalarType::U8 | ScalarType::U16 | ScalarType::U32 | ScalarType::Usize => {
                format!("cl_text_u32((u32)({value}))")
            }
            ScalarType::Bool => format!("cl_text_bool((int)({value}))"),
            ScalarType::F32 | ScalarType::F64 => format!("cl_text_f64((double)({value}))"),
        }
    }

    /// Renders a template as a string builder call, or a plain literal.
    ///
    /// A template with no placeholder is a `str` view of its literal bytes and
    /// allocates nothing. A template with a placeholder is an owned `String`,
    /// so its literal segments and converted placeholders become pieces for the
    /// shim builder, which concatenates them and takes over the owned pieces.
    /// Placeholders convert through [`Self::text_call`], the same conversion an
    /// explicit `text` call uses.
    fn template(&self, pieces: &[ValuePiece]) -> String {
        let has_placeholder = pieces
            .iter()
            .any(|piece| matches!(piece, ValuePiece::Placeholder(_)));
        if !has_placeholder {
            let mut literal = Vec::new();
            for piece in pieces {
                if let ValuePiece::Literal(text) = piece {
                    literal.extend_from_slice(text.as_bytes());
                }
            }
            return format!("cl_str({})", escape_c_string(&literal));
        }
        let mut parts: Vec<String> = Vec::new();
        for piece in pieces {
            match piece {
                ValuePiece::Literal(text) if text.is_empty() => {}
                ValuePiece::Literal(text) => parts.push(format!(
                    "{{ (const u8 *){}, {}u, NULL }}",
                    escape_c_string(text.as_bytes()),
                    text.len()
                )),
                ValuePiece::Placeholder(inner) => {
                    parts.push(format!("cl_str_part_owned({})", self.text_call(Some(inner))));
                }
            }
        }
        format!(
            "cl_string_concat((ClStrPart[]){{ {} }}, {}u)",
            parts.join(", "),
            parts.len()
        )
    }

    /// Renders an array literal as a C brace list.
    ///
    /// Each element renders through its store spelling, so a view element
    /// widens an owned source exactly as a store does and its owner keeps the
    /// single release; a literal element keeps its own brace list.
    pub(super) fn array_literal(&self, ty: &Ty, elements: &[Value], fill: &Option<Box<Value>>) -> String {
        let element_ty = ty.iterated_element();
        if elements.is_empty() {
            if let Some(fill) = fill {
                return format!("{{ {} }}", self.store_text(fill, &element_ty));
            }
        }
        let items: Vec<String> = elements
            .iter()
            .map(|element| self.store_text(element, &element_ty))
            .collect();
        format!("{{ {} }}", items.join(", "))
    }

    /// Renders a struct literal with its fields in declaration order.
    ///
    /// The cast names the literal's concrete type so a generic instantiation
    /// takes its instantiated C struct. A field the literal omits gets the
    /// field type's empty state: the instance's type arguments are substituted
    /// (so a nested generic field becomes a zero state, not a scalar `0`), and
    /// the active generic context's substitution is applied on top.
    fn struct_literal(&self, ty: &Ty, name: &str, fields: &[(String, Value)]) -> String {
        let declared = self
            .sema
            .table
            .struct_fields(name)
            .map(<[(String, Ty)]>::to_vec)
            .unwrap_or_default();
        let mut values: Vec<String> = Vec::new();
        for (field, _) in &declared {
            match fields.iter().find(|(written, _)| written == field) {
                Some((_, value)) => {
                    let text = self.value(value);
                    // A text view's pointer member at a raw field destination
                    // spells the sanctioned cast, exactly as a store does.
                    let field_ty = self.field_type(ty, field);
                    values.push(if self.raw_text_view_pointer(value, &field_ty) {
                        format!("(u8 *)({text})")
                    } else {
                        text
                    });
                }
                None => {
                    let empty = self.empty_state(&self.field_type(ty, field));
                    values.push(empty);
                }
            }
        }
        if declared.is_empty() {
            for (_, value) in fields {
                values.push(self.value(value));
            }
        }
        let cast = match &ty.kind {
            TyKind::Struct(..) => c_type(ty, &self.sema.table),
            _ => format!("Cl{name}"),
        };
        format!("({cast}){{ {} }}", values.join(", "))
    }
}

/// Renders the move-out zero of a fixed-array parameter as comma operands.
///
/// The parameter is C-adjusted to a pointer, so `cl_zero` cannot take its
/// `sizeof`; each deepest element is zeroed instead, which empties every
/// handle the deferred `take` move-out transferred.
fn array_param_zero_expr(c_name: &str, element: &Ty, extents: &[usize]) -> Vec<String> {
    match extents.split_first() {
        None => vec![format!("(cl_zero(&{c_name}))")],
        Some((head, tail)) => {
            let mut parts = Vec::new();
            for index in 0..*head {
                parts.extend(array_param_zero_expr(
                    &format!("{c_name}[{index}]"),
                    element,
                    tail,
                ));
            }
            parts
        }
    }
}
