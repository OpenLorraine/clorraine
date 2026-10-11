//! Call-argument rendering and the compiler-known intrinsic helpers.
//!
//! The emitter renders a call's argument list here so a pointer-lowered
//! writable lend can take its place's address, and it renders the reserved
//! built-ins and the compiler-known view builders `rawSlice`, `rawStr`, and
//! `slice` here so the view struct and its bounds guard live in one place.

use crate::ast::Mode;
use crate::ir::{Value, ValueKind};
use crate::types::{contains_param, Ty, TyKind};

use super::render::escape_c_string;
use super::types::{c_type, is_pointer_param, is_struct_view, type_key};
use super::Emitter;

impl Emitter<'_> {
    /// Emits the bounds guard of the `slice` intrinsic.
    ///
    /// A slice is legal only when `lo <= hi` and `hi` lies inside the source
    /// view. The guard runs before the view is built, so an out-of-range
    /// request traps with E11211 instead of reading past the storage.
    pub(super) fn emit_slice_guard(&mut self, args: &[Value]) {
        if let Some(condition) = self.slice_guard_condition(args) {
            self.push(&format!(
                "    if ({condition}) cl_trap(\"E11211\", \"index out of range\");"
            ));
        }
    }

    /// The condition under which a `slice` request is out of range.
    ///
    /// A slice is illegal when `lo > hi` or `hi` lies past the source view.
    /// The same condition drives the statement guard and the inline guard a
    /// short-circuit operand embeds.
    pub(super) fn slice_guard_condition(&self, args: &[Value]) -> Option<String> {
        let view = args.first()?;
        let lo = args.get(1)?;
        let hi = args.get(2)?;
        let source = self.slice_source(view);
        let lo_c = self.value(lo);
        let hi_c = self.value(hi);
        let upper = format!("(usize)({hi_c}) > {}", source.length);
        // A constant zero low bound can never exceed an unsigned high bound,
        // so the lower clause is skipped; emitting `0 > x` would be dead code
        // and trips `-Wtype-limits` on an unsigned high bound.
        Some(match &lo.kind {
            ValueKind::Int(0) => upper,
            _ => format!("(usize)({lo_c}) > (usize)({hi_c}) || {upper}"),
        })
    }

    /// The pointer and element count of the storage a `slice` selects from.
    ///
    /// A growable view carries its own `ptr` and `len` fields; a fixed array
    /// decays to a pointer to its first element. The view's element is one
    /// iteration step, so a view of rows stores a row pointer and both the
    /// length and the guard bound count rows. This lets `slice` accept a fixed
    /// array without emitting `(arr).ptr`, which is not C.
    fn slice_source(&self, view: &Value) -> SliceSource {
        let ty = self.substitute(&view.ty);
        let value = self.value(view);
        match &ty.kind {
            TyKind::Array { extents, .. } if extents.iter().all(Option::is_some) => {
                let count = extents.first().and_then(|size| *size).unwrap_or(0);
                SliceSource {
                    pointer: value,
                    length: format!("(usize){count}"),
                }
            }
            _ => SliceSource {
                pointer: format!("({value}).ptr"),
                length: format!("(usize)(({value}).len)"),
            },
        }
    }

    /// Renders a call's argument list, taking the address of pointer-lowered
    /// writable lends and widening a `String` to a `str` view where required.
    ///
    /// A `borrow` parameter of a non-duplicable struct lowers to `T *`, so the
    /// caller passes the address of its place; a nullable `borrow` parameter is
    /// an ownership out-slot and lowers to a pointer the same way. A call
    /// through a `func` value takes its parameter types and modes from
    /// `func_ty`. A declared function's parameter types follow the call's
    /// explicit type arguments, so an instantiated nullable slot decides its own
    /// pointer spelling. An `ffi` callee instead follows its mode-ignoring ABI
    /// prototype, so its arguments stay by value. A `str` parameter widens a
    /// `String` argument to a view over the owned bytes. Every other argument
    /// is passed as its value.
    pub(super) fn call_arguments(
        &self,
        callee: &str,
        type_args: &[Ty],
        args: &[Value],
        func_ty: Option<&Ty>,
    ) -> String {
        let (modes, param_types) = match func_ty.and_then(|ty| match &self.substitute(ty).kind {
            TyKind::Func(signature) => Some((
                signature
                    .params
                    .iter()
                    .map(|param| param.mode)
                    .collect::<Vec<_>>(),
                signature
                    .params
                    .iter()
                    .map(|param| param.ty.clone())
                    .collect::<Vec<_>>(),
            )),
            _ => None,
        }) {
            Some((modes, types)) => (Some(modes), Some(types)),
            // A declared function keeps its own signature, instantiated with
            // the call's explicit type arguments when there are any.
            None => (
                self.callee_modes(callee),
                self.instance_param_types(callee, type_args),
            ),
        };
        let user_call = self.resolve_function(callee).is_some();
        let rendered: Vec<String> = args
            .iter()
            .enumerate()
            .map(|(index, arg)| {
                let expected = param_types.as_ref().and_then(|types| types.get(index));
                let mode = modes.as_ref().and_then(|modes| modes.get(index)).copied();
                // A null argument is typed by its parameter: a struct-typed
                // view wants the zero view, while an `ffi` parameter keeps the
                // plain null its mode-ignoring ABI prototype spells.
                let text = if matches!(arg.kind, ValueKind::Null)
                    && (func_ty.is_some() || user_call)
                {
                    match expected {
                        Some(expected) => self.null_value(expected),
                        None => self.argument_text(arg, expected),
                    }
                } else {
                    self.argument_text(arg, expected)
                };
                // An uncontracted `ffi` parameter takes its ABI type from the
                // FFI table, so a view argument crosses as the view's pointer
                // member rather than as the view struct, cast to the pointer
                // the prototype declares. An `APTR` parameter takes the
                // pointer as it is; a `cstring` parameter declares `char *`,
                // so the crossing spells that cast. A `cstring` literal never
                // reaches this path (it renders as the pointer already) and a
                // `String` handle crosses through the view widening only
                // where a view parameter asks for it. The prototype and the
                // tables are unchanged.
                let text = if func_ty.is_none() && !user_call {
                    match expected {
                        Some(expected) if self.argument_crosses_as_view(arg, expected) => {
                            match crate::abi::ffi_type_with(expected, None).as_str() {
                                "APTR" => format!("(APTR)({text}).ptr"),
                                // A `static str` literal already renders as
                                // the NUL-terminated C string a `cstring`
                                // parameter wants.
                                "char *" if !matches!(arg.kind, ValueKind::Str(_)) => {
                                    format!("(char *)({text}).ptr")
                                }
                                _ => text,
                            }
                        }
                        _ => text,
                    }
                } else {
                    text
                };
                // The declared parameter type decides whether the argument is
                // the address of a writable out-slot, not the written argument
                // type: a nullable borrow parameter always lowers to a pointer,
                // and the front end has already required its argument to be an
                // exact-typed place. A parameter type that still mentions a type
                // parameter is not yet the instance's type, so the substituted
                // argument type stands in for it. An `ffi` callee ignores modes
                // in its prototype, so its arguments stay by value.
                let pointer = if func_ty.is_none() && !user_call {
                    false
                } else {
                    match expected.filter(|expected| !contains_param(expected)) {
                        Some(expected) => {
                            is_pointer_param(expected, mode.unwrap_or(Mode::None), &self.sema.table)
                        }
                        None => {
                            user_call
                                && is_pointer_param(
                                    &self.substitute(&arg.ty),
                                    mode.unwrap_or(Mode::None),
                                    &self.sema.table,
                                )
                        }
                    }
                };
                if pointer {
                    // The address of a conditional is not an lvalue in C, so
                    // each arm takes its own address: the select then yields
                    // the chosen arm's pointer, and a nested conditional
                    // recurses per arm.
                    if matches!(arg.kind, ValueKind::Conditional { .. }) {
                        self.conditional_address_text(arg)
                    } else {
                        format!("&({text})")
                    }
                } else {
                    text
                }
            })
            .collect();
        rendered.join(", ")
    }

    /// Renders the address of a conditional argument, one address per arm.
    ///
    /// A conditional is not an lvalue in C, so the address of the whole select
    /// cannot be spelled; each arm takes its own address instead, and a nested
    /// conditional recurses. The condition is evaluated once, exactly as the
    /// select itself would evaluate it.
    fn conditional_address_text(&self, value: &Value) -> String {
        if let ValueKind::Conditional {
            condition,
            then_value,
            else_value,
        } = &value.kind
        {
            let condition = self.value(condition);
            let then_text = self.conditional_address_text(then_value);
            let else_text = self.conditional_address_text(else_value);
            return format!("({condition} ? {then_text} : {else_text})");
        }
        format!("&({})", self.value(value))
    }

    /// True when an uncontracted extern receives the argument as a view.
    ///
    /// A slice or `str` value crosses as its pointer member; an owned `String`
    /// widens to the `str` view first, so the widened value crosses the same
    /// way. A `cstring` argument is already a C pointer and never wraps.
    fn argument_crosses_as_view(&self, arg: &Value, expected: &Ty) -> bool {
        let arg_ty = self.substitute(&arg.ty);
        if is_struct_view(&arg_ty) {
            return true;
        }
        matches!(&arg_ty.kind, TyKind::Handle(name) if name == "String")
            && matches!(
                self.substitute(expected).kind,
                TyKind::Str | TyKind::StaticStr
            )
    }

    /// Renders one argument, widening an owned `String` to the `str` view a
    /// parameter expects, or a literal to the C string pointer a `cstring`
    /// expects.
    ///
    /// The view reads the owned bytes through the shim helpers, so it allocates
    /// nothing and stays valid for the call. The conversion is emitted as a
    /// single shim call so a complex argument such as a template builder is
    /// evaluated exactly once. A `static str` literal is already a
    /// NUL-terminated C string, so a `cstring` parameter takes it directly
    /// instead of a `ClStr` view.
    fn argument_text(&self, arg: &Value, expected: Option<&Ty>) -> String {
        let wants_cstring = expected.is_some_and(|expected| {
            matches!(self.substitute(expected).kind, TyKind::CString)
        });
        if wants_cstring {
            if let ValueKind::Str(literal) = &arg.kind {
                return escape_c_string(literal.as_bytes());
            }
        }
        let text = self.value(arg);
        let arg_ty = self.substitute(&arg.ty);
        let wants_str = expected.is_some_and(|expected| {
            matches!(
                self.substitute(expected).kind,
                TyKind::Str | TyKind::StaticStr
            )
        });
        if wants_str && matches!(&arg_ty.kind, TyKind::Handle(name) if name == "String") {
            return format!("cl_string_view({text})");
        }
        text
    }

    /// Renders a reserved built-in call or a compiler-known view builder.
    pub(super) fn intrinsic(&self, name: &str, args: &[Value], type_arg: Option<&Ty>) -> String {
        match name {
            "len" => {
                if let Some(inner) = args.first() {
                    let ty = self.resolved_value_ty(inner);
                    let inner_c = self.value(inner);
                    return match &ty.kind {
                        TyKind::Str | TyKind::StaticStr => format!("({inner_c}).len"),
                        // An owned `String` is a handle, not a view: its byte
                        // count comes from the view the implicit `String` to
                        // `str` widening builds.
                        TyKind::Handle(name) if name == "String" => {
                            format!("(cl_string_view({inner_c})).len")
                        }
                        // The growable array keeps its element count in a
                        // `count` field, so `len` reads that field.
                        TyKind::Struct(struct_name, _) if struct_name == "Array" => {
                            format!("({inner_c}).count")
                        }
                        TyKind::Array { extents, .. } if extents.iter().any(Option::is_none) => {
                            format!("({inner_c}).len")
                        }
                        // The elements of a fixed array are its
                        // first-dimension values; every later extent belongs
                        // to the element type, so `len` reports the outermost
                        // count.
                        TyKind::Array { extents, .. } => {
                            let extent = extents.first().and_then(|size| *size).unwrap_or(0);
                            format!("{extent}")
                        }
                        _ => "0".to_owned(),
                    };
                }
                "0".to_owned()
            }
            "rawSlice" => {
                let pointer = self.intrinsic_arg(args, 0);
                let count = self.intrinsic_arg(args, 1);
                let element = self.intrinsic_element(type_arg, args.first());
                format!(
                    "((ClSlice_{}){{ {}, (usize)({}) }})",
                    type_key(&element),
                    pointer,
                    count
                )
            }
            "rawStr" => {
                let pointer = self.intrinsic_arg(args, 0);
                let count = self.intrinsic_arg(args, 1);
                format!("((ClStr){{ {}, (usize)({}) }})", pointer, count)
            }
            "slice" => {
                let source = match args.first() {
                    Some(view) => self.slice_source(view),
                    None => SliceSource {
                        pointer: "0".to_owned(),
                        length: "0".to_owned(),
                    },
                };
                let low = self.intrinsic_arg(args, 1);
                let high = self.intrinsic_arg(args, 2);
                let element = self.intrinsic_element(type_arg, args.first());
                let offset = format!("({}) + ({low})", source.pointer);
                format!(
                    "((ClSlice_{}){{ {offset}, (usize)(({high}) - ({low})) }})",
                    type_key(&element)
                )
            }
            "sizeof" => match type_arg {
                Some(ty) => format!("sizeof({})", c_type(&self.substitute(ty), &self.sema.table)),
                None => "0".to_owned(),
            },
            // The hardware intrinsics become a `volatile`-qualified access at
            // the intrinsic's native width. The qualifier is what forbids
            // folding, caching, and reordering; no byte swap is emitted.
            "readVolatile8" | "readVolatile16" | "readVolatile32" => {
                let width = volatile_width(name);
                let addr = self.intrinsic_arg(args, 0);
                format!("(*(volatile {width} *)({addr}))")
            }
            "writeVolatile8" | "writeVolatile16" | "writeVolatile32" => {
                let width = volatile_width(name);
                let addr = self.intrinsic_arg(args, 0);
                let value = self.intrinsic_arg(args, 1);
                format!("(*(volatile {width} *)({addr}) = ({value}))")
            }
            _ => "0".to_owned(),
        }
    }

    /// The rendered C of an intrinsic argument, or `0` when it is absent.
    fn intrinsic_arg(&self, args: &[Value], index: usize) -> String {
        args.get(index)
            .map(|arg| self.value(arg))
            .unwrap_or_else(|| "0".to_owned())
    }

    /// The element type of a view-builder intrinsic.
    ///
    /// An explicit type argument wins; otherwise the element is one step of
    /// iteration over the source view, so a multi-extent source yields whole
    /// rows, or the pointee of a first `raw` argument.
    fn intrinsic_element(&self, type_arg: Option<&Ty>, first: Option<&Value>) -> Ty {
        if let Some(ty) = type_arg {
            return self.substitute(ty);
        }
        let Some(first) = first else {
            return Ty::unknown();
        };
        let ty = self.substitute(&first.ty);
        match &ty.kind {
            TyKind::Array { .. } => ty.iterated_element(),
            TyKind::Raw(inner) => (**inner).clone(),
            _ => Ty::unknown(),
        }
    }
}

/// The pointer and element count of a slice source.
struct SliceSource {
    /// The pointer the slice offsets advance in elements; for a view of rows
    /// it advances a whole row at a time.
    pointer: String,
    /// The number of elements the source holds.
    length: String,
}

/// The C scalar name a volatile intrinsic accesses, keyed by its name.
///
/// The suffix of `readVolatileN`/`writeVolatileN` is the access width, so the
/// qualifier is applied to the matching unsigned C type.
fn volatile_width(name: &str) -> &'static str {
    match name {
        "readVolatile8" | "writeVolatile8" => "u8",
        "readVolatile16" | "writeVolatile16" => "u16",
        "readVolatile32" | "writeVolatile32" => "u32",
        _ => "u8",
    }
}
