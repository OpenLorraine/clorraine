//! Call-argument rendering and the compiler-known intrinsic helpers.
//!
//! The emitter renders a call's argument list here so a pointer-lowered
//! writable lend can take its place's address, and it renders the reserved
//! built-ins and the compiler-known view builders `rawSlice`, `rawStr`, and
//! `slice` here so the view struct and its bounds guard live in one place.

use crate::ast::Mode;
use crate::ir::{Value, ValueKind};
use crate::types::{Ty, TyKind};

use super::render::escape_c_string;
use super::types::{c_type, type_key};
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
        let (_ptr, len) = self.slice_source(view);
        let lo_c = self.value(lo);
        let hi_c = self.value(hi);
        Some(format!(
            "(usize)({lo_c}) > (usize)({hi_c}) || (usize)({hi_c}) > {len}"
        ))
    }

    /// The pointer and length expressions of the storage a `slice` selects from.
    ///
    /// A growable slice or string view carries its own `ptr` and `len` fields.
    /// A fixed array has no such fields, so its address decays to the element
    /// pointer and its extent is a compile-time constant. This lets `slice`
    /// accept a fixed array without emitting `(arr).ptr`, which is not C.
    fn slice_source(&self, view: &Value) -> (String, String) {
        let ty = self.substitute(&view.ty);
        let value = self.value(view);
        match &ty.kind {
            TyKind::Array { extents, .. } if extents.iter().all(Option::is_some) => {
                let extent = extents
                    .iter()
                    .filter_map(|size| *size)
                    .fold(1usize, usize::saturating_mul);
                let element = self.intrinsic_element(None, Some(view));
                (
                    format!("({} *){value}", c_type(&element)),
                    format!("(usize){extent}"),
                )
            }
            _ => (format!("({value}).ptr"), format!("(usize)(({value}).len)")),
        }
    }

    /// Renders a call's argument list, taking the address of pointer-lowered
    /// writable lends and widening a `String` to a `str` view where required.
    ///
    /// A `borrow` parameter of a non-duplicable struct lowers to `T *`, so the
    /// caller passes the address of its place. A `str` parameter accepts a
    /// `String` argument through the implicit widening, which lowers to a view
    /// over the owned bytes. Every other argument, and every `ffi` argument, is
    /// passed as its value.
    pub(super) fn call_arguments(&self, callee: &str, args: &[Value]) -> String {
        let modes = self.callee_modes(callee);
        let param_types = self.callee_param_types(callee);
        let user_call = self.resolve_function(callee).is_some();
        let rendered: Vec<String> = args
            .iter()
            .enumerate()
            .map(|(index, arg)| {
                let expected = param_types.as_ref().and_then(|types| types.get(index));
                let text = self.argument_text(arg, expected);
                let pointer = user_call
                    && modes
                        .as_ref()
                        .and_then(|modes| modes.get(index))
                        .is_some_and(|mode| {
                            *mode == Mode::Borrow
                                && matches!(arg.ty.kind, TyKind::Struct(..))
                                && !self.sema.table.duplicable(&arg.ty)
                        });
                if pointer {
                    format!("&({text})")
                } else {
                    text
                }
            })
            .collect();
        rendered.join(", ")
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
                    let ty = self.substitute(&inner.ty);
                    let inner_c = self.value(inner);
                    return match &ty.kind {
                        TyKind::Str | TyKind::StaticStr => format!("({inner_c}).len"),
                        // The growable array keeps its element count in a
                        // `count` field, so `len` reads that field.
                        TyKind::Struct(struct_name, _) if struct_name == "Array" => {
                            format!("({inner_c}).count")
                        }
                        TyKind::Array { extents, .. } if extents.iter().any(Option::is_none) => {
                            format!("({inner_c}).len")
                        }
                        TyKind::Array { extents, .. } => {
                            let extent = extents
                                .iter()
                                .filter_map(|size| *size)
                                .fold(1usize, usize::saturating_mul);
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
                let (pointer, _len) = match args.first() {
                    Some(view) => self.slice_source(view),
                    None => ("0".to_owned(), "0".to_owned()),
                };
                let low = self.intrinsic_arg(args, 1);
                let high = self.intrinsic_arg(args, 2);
                let element = self.intrinsic_element(type_arg, args.first());
                format!(
                    "((ClSlice_{}){{ ({pointer}) + ({low}), (usize)(({high}) - ({low})) }})",
                    type_key(&element)
                )
            }
            "sizeof" => match type_arg {
                Some(ty) => format!("sizeof({})", c_type(&self.substitute(ty))),
                None => "0".to_owned(),
            },
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
    /// An explicit type argument wins; otherwise the element type comes from
    /// the source view, or the pointee of a first `raw` argument.
    fn intrinsic_element(&self, type_arg: Option<&Ty>, first: Option<&Value>) -> Ty {
        if let Some(ty) = type_arg {
            return self.substitute(ty);
        }
        let Some(first) = first else {
            return Ty::unknown();
        };
        match self.substitute(&first.ty).kind {
            TyKind::Array { element, .. } => *element,
            TyKind::Raw(inner) => *inner,
            _ => Ty::unknown(),
        }
    }
}
