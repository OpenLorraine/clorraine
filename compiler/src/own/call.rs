//! Call lowering for the ownership stage.
//!
//! A call lowers to its callee name, its concrete type arguments, and its
//! arguments in signature order. A dot-call carries its receiver as the first
//! argument, matching the free-function signature the specification fixes. An
//! inferred generic call gets its concrete type arguments from the argument
//! types, so the emitter can name the monomorphized instance. The compiler-known
//! view builders are lowered to the intrinsic value the emitter renders
//! directly.

use super::*;

impl<'a, 'd> FunctionChecker<'a, 'd> {
    /// True when a call's callee is a dot-call with a value receiver.
    ///
    /// A `Member` callee always has one. A `Qualified` callee is a dot-call
    /// when its head names a value in scope, matching how the type stage
    /// decides between member access and a module path.
    pub(super) fn is_dot_call(&self, callee: &Expr) -> bool {
        match &callee.kind {
            ExprKind::Member { .. } => true,
            ExprKind::Qualified(name) => {
                name.segments.len() >= 2
                    && (self.lookup(&name.segments[0]).is_some()
                        || self.program.globals.contains_key(&name.segments[0]))
            }
            _ => false,
        }
    }

    /// Lowers the receiver of a dot-call to a place value, when it has one.
    fn lower_dot_receiver(&mut self, callee: &Expr) -> Option<Value> {
        match &callee.kind {
            ExprKind::Member { base, .. } => Some(self.lower_value(base)),
            ExprKind::Qualified(name) if name.segments.len() >= 2 => {
                let method = name.segments.len() - 1;
                let head = &name.segments[0];
                let projections: Vec<ir::Projection> = name.segments[1..method]
                    .iter()
                    .map(|segment| ir::Projection::Field(segment.clone()))
                    .collect();
                if let Some(id) = self.lookup(head) {
                    let mut ty = self.bindings[id].ty.clone();
                    for segment in &name.segments[1..method] {
                        ty = self.member_type(&ty, segment);
                    }
                    return Some(Value::place(Place {
                        base: self.binding_place(id).base,
                        projections,
                        ty,
                    }));
                }
                if self.program.globals.contains_key(head) {
                    let mut ty = self
                        .program
                        .globals
                        .get(head)
                        .map(|global| global.ty.clone())
                        .unwrap_or_else(Ty::unknown);
                    for segment in &name.segments[1..method] {
                        ty = self.member_type(&ty, segment);
                    }
                    return Some(Value::place(Place {
                        base: ir::PlaceBase::Global(head.clone()),
                        projections,
                        ty,
                    }));
                }
                None
            }
            _ => None,
        }
    }

    /// Lowers a call to its name, concrete type arguments, and arguments.
    ///
    /// A dot-call's receiver becomes the first argument, matching the
    /// free-function signature. An inferred generic call gets its concrete type
    /// arguments from the argument types so the emitter can name the instance
    /// symbol; an explicit list is used as written.
    pub(super) fn lower_call(
        &mut self,
        callee: &Expr,
        type_args: &[crate::ast::Type],
        args: &[Arg],
    ) -> (String, Vec<Ty>, Vec<Value>) {
        let name = self.callee_name(callee).unwrap_or_default();
        let mut values: Vec<Value> = Vec::new();
        let mut arg_types: Vec<Ty> = Vec::new();
        if let Some(receiver) = self.lower_dot_receiver(callee) {
            arg_types.push(receiver.ty.clone());
            values.push(receiver);
        }
        for arg in args {
            let value = self.lower_value(&arg.value);
            arg_types.push(value.ty.clone());
            values.push(value);
        }
        let explicit: Vec<Ty> = type_args.iter().map(|arg| self.resolve(arg)).collect();
        let resolved = if explicit.is_empty() {
            self.infer_call_types(&name, &arg_types).unwrap_or_default()
        } else {
            explicit
        };
        (name, resolved, values)
    }

    /// Infers the type arguments of a generic call from its argument types.
    ///
    /// Returns `None` when the callee is not generic or some parameter stays
    /// unbound; the emitter then treats the call as opaque rather than guessing.
    fn infer_call_types(&self, name: &str, arg_types: &[Ty]) -> Option<Vec<Ty>> {
        let Some(Callee::User(decl)) = self.program.callee(name) else {
            return None;
        };
        if decl.type_params.is_empty() {
            return None;
        }
        let declared: Vec<String> = decl.type_params.iter().map(|p| p.name.clone()).collect();
        let param_types: Vec<Ty> = decl
            .params
            .iter()
            .map(|param| resolve_ty(&param.ty, &declared, &self.sema.table))
            .collect();
        let mut inferred: BTreeMap<String, Ty> = BTreeMap::new();
        for (param, arg) in param_types.iter().zip(arg_types) {
            crate::sema::call::match_param(param, arg, &declared, &mut inferred);
        }
        let mut result = Vec::with_capacity(declared.len());
        for param in &declared {
            result.push(inferred.get(param).cloned()?);
        }
        Some(result)
    }

    /// Lowers a compiler-known intrinsic call, when the callee names one.
    ///
    /// The view builders `rawSlice`, `rawStr`, and `slice` become an intrinsic
    /// value so the emitter can spell the view struct and place the bounds
    /// guard. The dot-call `xs.len()` is the receiver-taking spelling of the
    /// `len` intrinsic, so its receiver becomes the intrinsic's argument.
    /// Returns `None` for every other call, which stays an ordinary call.
    pub(super) fn lower_intrinsic_call(
        &mut self,
        callee: &Expr,
        type_args: &[crate::ast::Type],
        args: &[Arg],
    ) -> Option<(Ty, ValueKind)> {
        let name = self.callee_name(callee)?;
        if name == "len" && type_args.is_empty() {
            // `xs.len()` is the dot-call spelling of `len(xs)`; the receiver
            // becomes the intrinsic's argument.
            let receiver = self.lower_dot_receiver(callee)?;
            return Some((
                Ty::usize_ty(),
                ValueKind::Intrinsic {
                    name: "len".to_owned(),
                    args: vec![receiver],
                    type_arg: None,
                },
            ));
        }
        if name != "rawSlice" && name != "rawStr" && name != "slice" {
            return None;
        }
        let explicit: Vec<Ty> = type_args.iter().map(|arg| self.resolve(arg)).collect();
        let lowered: Vec<Value> = args.iter().map(|arg| self.lower_value(&arg.value)).collect();
        let type_arg = explicit
            .first()
            .cloned()
            .or_else(|| element_of(lowered.first()?.ty.clone()))
            .or_else(|| match &lowered.first()?.ty.kind {
                TyKind::Raw(inner) => Some((**inner).clone()),
                _ => None,
            });
        let is_raw_str = name == "rawStr";
        let result = if is_raw_str {
            Ty {
                kind: TyKind::Str,
                nullable: false,
            }
        } else {
            Ty::array(type_arg.clone().unwrap_or_else(Ty::unknown), vec![None])
        };
        Some((
            result,
            ValueKind::Intrinsic {
                name,
                args: lowered,
                type_arg: if is_raw_str { None } else { type_arg },
            },
        ))
    }
}

/// The element type of an array, slice, or string view.
fn element_of(ty: Ty) -> Option<Ty> {
    match ty.kind {
        TyKind::Array { element, .. } => Some(*element),
        _ => None,
    }
}
