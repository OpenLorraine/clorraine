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
use crate::types::{is_signed_scalar, scalar_bits, Ty, TyKind};

use super::render::{
    binary_token, bounds_condition, element_type, escape_c_string, float_literal, index_text,
    signed_min, unsigned_c, unsigned_scalar,
};
use super::types::{c_type, scalar_of, type_key};
use super::Emitter;

impl<'a> Emitter<'a> {
    // -- guards --------------------------------------------------------------

    /// Emits every runtime guard a value's evaluation needs.
    pub(super) fn emit_guards(&mut self, value: &Value) {
        match &value.kind {
            ValueKind::Binary { op, left, right } => {
                self.emit_guards(left);
                self.emit_guards(right);
                if matches!(op, BinaryOp::Div | BinaryOp::Rem) {
                    self.emit_division_guard(left, right);
                }
            }
            ValueKind::Index { base, index } => {
                self.emit_guards(base);
                self.emit_guards(index);
                self.emit_bounds_guard(base, index);
            }
            ValueKind::Cast { value: inner, .. } => {
                self.emit_guards(inner);
                let target = self.substitute(&value.ty);
                let source = self.substitute(&inner.ty);
                if target.is_integer() && source.is_numeric() && !source.is_integer() {
                    self.emit_float_guard(inner, &target);
                }
            }
            ValueKind::Call { args, .. } => {
                for arg in args {
                    self.emit_guards(arg);
                }
            }
            ValueKind::Unary { operand, .. } => self.emit_guards(operand),
            ValueKind::Conditional {
                condition,
                then_value,
                else_value,
            } => {
                self.emit_guards(condition);
                self.emit_guards(then_value);
                self.emit_guards(else_value);
            }
            ValueKind::Member { base, .. } => self.emit_guards(base),
            ValueKind::StructLit { fields, .. } => {
                for (_, field) in fields {
                    self.emit_guards(field);
                }
            }
            ValueKind::ArrayLit { elements, fill } => {
                for element in elements {
                    self.emit_guards(element);
                }
                if let Some(fill) = fill {
                    self.emit_guards(fill);
                }
            }
            ValueKind::Template(pieces) => {
                for piece in pieces {
                    if let ValuePiece::Placeholder(inner) = piece {
                        self.emit_guards(inner);
                    }
                }
            }
            ValueKind::Intrinsic { name, args, .. } => {
                for arg in args {
                    self.emit_guards(arg);
                }
                if name == "slice" {
                    self.emit_slice_guard(args);
                }
            }
            _ => {}
        }
    }

    /// Emits the integer division and modulo guard.
    ///
    /// Float division is IEEE-defined and never faults, so only an integer
    /// operation gets a guard.
    fn emit_division_guard(&mut self, left: &Value, right: &Value) {
        let ty = self.substitute(&left.ty);
        if !ty.is_integer() {
            return;
        }
        let dividend = self.value(left);
        let divisor = self.value(right);
        match scalar_of(&ty) {
            Some(scalar) if is_signed_scalar(scalar) => {
                let min = signed_min(scalar);
                self.push(&format!(
                    "    if ({divisor} == 0 || ({dividend} == {min} && {divisor} == -1)) cl_trap(\"E11210\", \"integer division by zero\");"
                ));
            }
            _ => {
                self.push(&format!(
                    "    if ({divisor} == 0) cl_trap(\"E11210\", \"integer division by zero\");"
                ));
            }
        }
    }

    /// Emits the index bounds check for a fixed array, slice, or string view.
    ///
    /// A signed index also has to be non-negative: the upper-bound comparison
    /// alone lets `xs[-1]` through. The comparison casts both sides to the
    /// unsigned `usize` so a signed index does not trigger
    /// `-Werror=sign-compare`; the negative check runs first, so the cast
    /// never sees a value that wrapped.
    fn emit_bounds_guard(&mut self, base: &Value, index: &Value) {
        let ty = self.substitute(&base.ty);
        let index_c = self.value(index);
        let signed = scalar_of(&self.substitute(&index.ty)).is_some_and(is_signed_scalar);
        match &ty.kind {
            TyKind::Array { extents, .. } => {
                if extents.iter().all(Option::is_some) {
                    let extent = extents
                        .iter()
                        .filter_map(|size| *size)
                        .fold(1usize, usize::saturating_mul);
                    let condition = bounds_condition(&index_c, signed, &extent.to_string());
                    self.push(&format!(
                        "    if ({condition}) cl_trap(\"E11211\", \"index out of range\");"
                    ));
                } else {
                    let base_c = self.value(base);
                    let bound = format!("({base_c}).len");
                    let condition = bounds_condition(&index_c, signed, &bound);
                    self.push(&format!(
                        "    if ({condition}) cl_trap(\"E11211\", \"index out of range\");"
                    ));
                }
            }
            TyKind::Str | TyKind::StaticStr => {
                let base_c = self.value(base);
                let bound = format!("({base_c}).len");
                let condition = bounds_condition(&index_c, signed, &bound);
                self.push(&format!(
                    "    if ({condition}) cl_trap(\"E11211\", \"index out of range\");"
                ));
            }
            _ => {}
        }
    }

    /// Emits the float-to-integer range guard.
    fn emit_float_guard(&mut self, value: &Value, target: &Ty) {
        let value_c = self.value(value);
        let (low, high) = match scalar_of(target) {
            Some(ScalarType::I8) => ("-128.0", "127.0"),
            Some(ScalarType::I16) => ("-32768.0", "32767.0"),
            Some(ScalarType::I32) | Some(ScalarType::Isize) => ("-2147483648.0", "2147483647.0"),
            Some(ScalarType::U8) => ("0.0", "255.0"),
            Some(ScalarType::U16) => ("0.0", "65535.0"),
            Some(ScalarType::U32) | Some(ScalarType::Usize) => ("0.0", "4294967295.0"),
            _ => return,
        };
        self.push(&format!(
            "    if (cl_isnan({value_c}) || !({value_c} >= {low} && {value_c} <= {high})) cl_trap(\"E11212\", \"float-to-integer conversion out of range\");"
        ));
    }

    // -- values --------------------------------------------------------------

    /// Renders a typed IR value as a C expression.
    pub(super) fn value(&self, value: &Value) -> String {
        match &value.kind {
            ValueKind::Place(place) => self.place(place),
            ValueKind::Int(number) => self.int_literal(*number as u64, &self.substitute(&value.ty)),
            ValueKind::Float(number) => float_literal(*number),
            ValueKind::Char(byte) => format!("{byte}"),
            ValueKind::Bool(flag) => if *flag { "1" } else { "0" }.to_owned(),
            ValueKind::Null => self.null_literal().to_owned(),
            ValueKind::Str(text) => format!("cl_str({})", escape_c_string(text.as_bytes())),
            ValueKind::Path(segments) => self.path(segments),
            ValueKind::Member { base, name } => {
                let base_c = self.value(base);
                if matches!(self.substitute(&base.ty).kind, TyKind::Raw(_)) {
                    format!("{base_c}->{name}")
                } else {
                    format!("{base_c}.{name}")
                }
            }
            ValueKind::Index { base, index } => {
                let base_c = self.value(base);
                let index_c = self.value(index);
                index_text(&self.substitute(&base.ty), &base_c, &index_c)
            }
            ValueKind::Unary { op, operand } => self.unary(*op, operand, value),
            ValueKind::Binary { op, left, right } => self.binary(*op, left, right, value),
            ValueKind::Cast { value: inner, .. } => {
                let target = self.substitute(&value.ty);
                match &target.kind {
                    TyKind::Void => format!("(void)({})", self.value(inner)),
                    _ => format!("({})({})", c_type(&target), self.value(inner)),
                }
            }
            ValueKind::Conditional {
                condition,
                then_value,
                else_value,
            } => format!(
                "({} ? {} : {})",
                self.value(condition),
                self.value(then_value),
                self.value(else_value)
            ),
            ValueKind::StructLit { name, fields } => self.struct_literal(name, fields),
            ValueKind::ArrayLit { elements, fill } => self.array_literal(elements, fill),
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
            } => self.call(callee, type_args, args),
            // A staged-out form never reaches a valid compile; the fallback
            // keeps the renderer total.
            ValueKind::Aggregate => "0".to_owned(),
            ValueKind::Temp(id) => format!("__cl_tmp{id}"),
        }
    }

    /// Renders a place as a C lvalue or rvalue.
    pub(super) fn place(&self, place: &Place) -> String {
        let mut text = self.place_base(&place.base);
        let mut ty = self.substitute(&self.place_base_type(&place.base));
        for projection in &place.projections {
            match projection {
                Projection::Field(field) => {
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
            PlaceBase::Global(name) => self
                .globals
                .get(name)
                .map(|global| self.global_symbol(&global.module, &global.name))
                .unwrap_or_else(|| name.clone()),
            PlaceBase::Temp(id) => format!("__cl_tmp{id}"),
        }
    }

    /// The declared type of a place's root.
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
            PlaceBase::Global(name) => self
                .globals
                .get(name)
                .map(|global| global.ty.clone())
                .unwrap_or_else(Ty::unknown),
            PlaceBase::Temp(id) => self
                .function
                .and_then(|function| function.temps.get(*id as usize).cloned())
                .unwrap_or_else(Ty::unknown),
        }
    }

    /// The type of a field selected from `base`, following a raw pointer.
    fn field_type(&self, base: &Ty, field: &str) -> Ty {
        match &base.kind {
            TyKind::Struct(name, _) => self
                .sema
                .table
                .struct_fields(name)
                .and_then(|fields| {
                    fields
                        .iter()
                        .find(|(declared, _)| declared == field)
                        .map(|(_, ty)| ty.clone())
                })
                .unwrap_or_else(Ty::unknown),
            TyKind::Raw(inner) => self.field_type(inner, field),
            _ => Ty::unknown(),
        }
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
                        format!("({})(-({}){operand_c})", c_type(&ty), unsigned_c(unsigned))
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

    /// Renders a binary operator with the wrapping and masking strategy.
    fn binary(&self, op: BinaryOp, left: &Value, right: &Value, whole: &Value) -> String {
        let left_c = self.value(left);
        let right_c = self.value(right);
        let ty = self.substitute(&whole.ty);
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
                    return format!("({})(({}){left_c} << {count})", c_type(&ty), unsigned);
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
                        c_type(&ty),
                        unsigned,
                        unsigned
                    );
                }
            }
        }
        format!("({left_c} {} {right_c})", binary_token(op))
    }

    /// Renders a call to a user function, an FFI function, or a helper.
    pub(super) fn call(&self, callee: &str, type_args: &[Ty], args: &[Value]) -> String {
        let argument_list = self.call_arguments(callee, args);
        if let Some(info) = self.resolve_function(callee) {
            let symbol = if type_args.is_empty() {
                self.function_symbol(&info.module, callee)
            } else {
                let keys: Vec<String> = type_args
                    .iter()
                    .map(|ty| type_key(&self.substitute(ty)))
                    .collect();
                format!("cl_{callee}__{}", keys.join("_"))
            };
            return format!("{symbol}({argument_list})");
        }
        match callee {
            "text" => self.text_call(args.first()),
            _ => format!("{callee}({argument_list})"),
        }
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
            _ => format!("cl_text_{}({value})", type_key(&ty)),
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

    /// Renders an array literal as a compound initializer.
    pub(super) fn array_literal(&self, elements: &[Value], fill: &Option<Box<Value>>) -> String {
        if elements.is_empty() {
            if let Some(fill) = fill {
                return format!("{{ {} }}", self.value(fill));
            }
        }
        let items: Vec<String> = elements.iter().map(|element| self.value(element)).collect();
        format!("{{ {} }}", items.join(", "))
    }

    /// Renders a struct literal with its fields in declaration order.
    fn struct_literal(&self, name: &str, fields: &[(String, Value)]) -> String {
        let declared = self
            .sema
            .table
            .struct_fields(name)
            .map(<[(String, Ty)]>::to_vec)
            .unwrap_or_default();
        let mut values: Vec<String> = Vec::new();
        for (field, field_ty) in &declared {
            match fields.iter().find(|(written, _)| written == field) {
                Some((_, value)) => values.push(self.value(value)),
                None => {
                    let empty = self.empty_state(field_ty);
                    values.push(empty);
                }
            }
        }
        if declared.is_empty() {
            for (_, value) in fields {
                values.push(self.value(value));
            }
        }
        format!("(Cl{name}){{ {} }}", values.join(", "))
    }
}
