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
    binary_token, element_type, escape_c_string, float_literal, index_text, unsigned_c,
    unsigned_scalar,
};
use super::types::{c_type, scalar_of, type_key};
use super::Emitter;

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
                self.guarded_value(then_value),
                self.guarded_value(else_value)
            ),
            ValueKind::StructLit { name, fields } => {
                self.struct_literal(&self.substitute(&value.ty), name, fields)
            }
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
                receiver,
            } => self.call(callee, type_args, args, receiver.as_deref()),
            ValueKind::Closure { site, captures } => self.closure_value(*site, captures),
            ValueKind::ClosureCall { callable, args } => self.closure_call(callable, args),
            // A staged-out form never reaches a valid compile; the fallback
            // keeps the renderer total.
            ValueKind::Aggregate => "0".to_owned(),
            ValueKind::Temp(id) => format!("__cl_tmp{id}"),
            // The temporary is written where the value's guards are emitted;
            // reading it here reuses the single evaluation.
            ValueKind::Materialize { temp, .. } => format!("__cl_tmp{temp}"),
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
            PlaceBase::Temp(id) => self
                .function
                .and_then(|function| function.temps.get(*id as usize).cloned())
                .unwrap_or_else(Ty::unknown),
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
    fn field_type(&self, base: &Ty, field: &str) -> Ty {
        match &base.kind {
            TyKind::Struct(..) => self
                .sema
                .table
                .instance_field(base, field)
                .map(|declared| self.substitute(&declared))
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
        let mut rendered: Vec<String> = Vec::new();
        if !args.is_empty() {
            rendered.push(self.call_arguments(callee, args));
        }
        if let Some(destination) = destination {
            rendered.push(destination.to_owned());
        }
        let argument_list = rendered.join(", ");
        if let Some(receiver) = receiver {
            let target = self.value(receiver);
            return format!("({target})({argument_list})");
        }
        // A func-typed binding shadows a module function of the same written
        // name, so the call goes through the binding's own C local before any
        // function or helper name is resolved.
        if let Some(spelled) = self.func_binding_spelling(callee) {
            return format!("{spelled}({argument_list})");
        }
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
        // Inside a monomorphized generic body, a callable parameter bound to a
        // closure is called through its two-word value. A parameter bound to a
        // plain `func` type stays a direct function-pointer call. A capture-less
        // lambda in a non-generic caller is not a parameter, so it falls through
        // to the named-function spelling below.
        if let Some(closure) = self.closure_param_type(callee) {
            let callable = self.place(&Place::param(callee).with_type(closure));
            return format!("(({callable}).fn(({callable}).env, {argument_list}))");
        }
        match callee {
            "text" => self.text_call(args.first()),
            _ => {
                // A func-typed local is called through its own C spelling, so
                // the compiler's local names map resolves it; every other
                // unresolved name is a compiler-known helper spelled as-is.
                let spelled = self
                    .local_names
                    .get(callee)
                    .cloned()
                    .unwrap_or_else(|| callee.to_owned());
                format!("{spelled}({argument_list})")
            }
        }
    }

    /// The closure type of a callable parameter of the open function.
    ///
    /// A generic body's `f(x)` names no declared function; when the active
    /// instance substitutes `f`'s parameter with a closure type, the call is a
    /// two-word closure call. A `func`-typed parameter, and every non-parameter
    /// callee, yields `None`.
    fn closure_param_type(&self, callee: &str) -> Option<Ty> {
        let function = self.function?;
        let param = function.params.iter().find(|param| param.name == callee)?;
        let ty = self.substitute(&param.ty);
        ty.is_closure().then_some(ty)
    }

    /// The C spelling of a func-typed local or parameter the callee names.
    ///
    /// A func-typed binding shadows a module function of the same written
    /// name. The binding is found by its IR spelling, which the lowering puts
    /// in the call for a local callee; a shadowing local takes a suffixed IR
    /// name, so a module call written before the local's declaration still
    /// resolves by function name.
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
        None
    }

    /// Renders a lambda value.
    ///
    /// A capture-less lambda is a plain C function pointer, so it is the lifted
    /// function's name. A capturing lambda is a two-word struct holding the
    /// function pointer and the address of a stack environment built inline; the
    /// environment is valid for the synchronous call that receives it.
    fn closure_value(&self, site: usize, captures: &[Value]) -> String {
        if captures.is_empty() {
            return format!("cl_lambda_{site}");
        }
        let environment = self.environment_literal(site, captures);
        format!("((ClClosure_{site}){{ cl_lambda_{site}, &({environment}) }})")
    }

    /// Renders a call of a value that is an ephemeral closure.
    ///
    /// A literal closure is called through its lifted function directly, so the
    /// environment expression is evaluated once. Any other callable renders its
    /// two-word value once into a C temporary expression and calls through the
    /// pointer with the environment as the first argument.
    fn closure_call(&self, callable: &Value, args: &[Value]) -> String {
        let argument_list = args
            .iter()
            .map(|arg| self.value(arg))
            .collect::<Vec<String>>()
            .join(", ");
        if let ValueKind::Closure { site, captures } = &callable.kind {
            if captures.is_empty() {
                return format!("cl_lambda_{site}({argument_list})");
            }
            let environment = self.environment_literal(*site, captures);
            return if argument_list.is_empty() {
                format!("cl_lambda_{site}(&({environment}))")
            } else {
                format!("cl_lambda_{site}(&({environment}), {argument_list})")
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
    fn environment_literal(&self, site: usize, captures: &[Value]) -> String {
        let fields: Vec<String> = captures.iter().map(|capture| self.value(capture)).collect();
        format!("((struct ClEnv_{site}){{ {} }})", fields.join(", "))
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
            _ => format!("cl_text_{}({value})", type_key(&ty)),
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
                Some((_, value)) => values.push(self.value(value)),
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
            TyKind::Struct(..) => c_type(ty),
            _ => format!("Cl{name}"),
        };
        format!("({cast}){{ {} }}", values.join(", "))
    }
}
