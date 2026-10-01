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
use super::types::{c_type, is_pointer_param, type_key};
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

    /// The C pointer-to-array type and base-element type of a view of rows.
    ///
    /// A fixed or slice array with more than one extent has rows as its
    /// elements, so its offsets and lengths count rows; the pointer addresses
    /// a whole row (`T (*)[M]`). `None` for every shape that iterates scalars.
    fn row_slice_types(&self, ty: &Ty) -> Option<(String, String)> {
        let TyKind::Array { element, extents } = &ty.kind else {
            return None;
        };
        if extents.len() < 2 {
            return None;
        }
        let row = Ty::array((**element).clone(), extents[1..].to_vec());
        let row_type = super::types::pointer_to_array_type(&row, &self.sema.table)?;
        Some((row_type, c_type(element, &self.sema.table)))
    }

    /// The pointer, count, and stored-pointer cast of the storage a `slice`
    /// selects from.
    ///
    /// A growable slice or string view carries its own `ptr` and `len` fields;
    /// a view of rows measures both in rows, so its pointer is cast to a
    /// pointer-to-array and offset row by row, then cast back to the
    /// base-element pointer the view stores. A fixed array has no fields, so
    /// its address decays to the element pointer and its extent is a
    /// compile-time constant. This lets `slice` accept a fixed array without
    /// emitting `(arr).ptr`, which is not C.
    fn slice_source(&self, view: &Value) -> SliceSource {
        let ty = self.substitute(&view.ty);
        let value = self.value(view);
        let rows = self.row_slice_types(&ty);
        match &ty.kind {
            TyKind::Array { extents, .. } if extents.iter().all(Option::is_some) => {
                if let Some((row_type, base)) = rows {
                    let count = extents.first().and_then(|size| *size).unwrap_or(0);
                    return SliceSource {
                        pointer: format!("({row_type}){value}"),
                        length: format!("(usize){count}"),
                        store_cast: Some(format!("({base} *)")),
                    };
                }
                let extent = extents
                    .iter()
                    .filter_map(|size| *size)
                    .fold(1usize, usize::saturating_mul);
                let element = self.intrinsic_element(None, Some(view));
                SliceSource {
                    pointer: format!("({} *){value}", c_type(&element, &self.sema.table)),
                    length: format!("(usize){extent}"),
                    store_cast: None,
                }
            }
            _ => {
                if let Some((row_type, base)) = rows {
                    return SliceSource {
                        pointer: format!("({row_type})({value}).ptr"),
                        length: format!("(usize)(({value}).len)"),
                        store_cast: Some(format!("({base} *)")),
                    };
                }
                SliceSource {
                    pointer: format!("({value}).ptr"),
                    length: format!("(usize)(({value}).len)"),
                    store_cast: None,
                }
            }
        }
    }

    /// Renders a call's argument list, taking the address of pointer-lowered
    /// writable lends and widening a `String` to a `str` view where required.
    ///
    /// A `borrow` parameter of a non-duplicable struct lowers to `T *`, so the
    /// caller passes the address of its place. A call through a `func` value
    /// takes its parameter types and modes from `func_ty`, so a `borrow`
    /// parameter passes the address too, and a `str` parameter widens a
    /// `String` argument exactly as a declared signature does. A `str`
    /// parameter accepts a `String` argument through the implicit widening,
    /// which lowers to a view over the owned bytes. Every other argument, and
    /// every `ffi` argument, is passed as its value.
    pub(super) fn call_arguments(
        &self,
        callee: &str,
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
            // A declared function or an FFI callee keeps its own signature.
            None => (self.callee_modes(callee), self.callee_param_types(callee)),
        };
        let user_call = self.resolve_function(callee).is_some();
        let rendered: Vec<String> = args
            .iter()
            .enumerate()
            .map(|(index, arg)| {
                let expected = param_types.as_ref().and_then(|types| types.get(index));
                let text = self.argument_text(arg, expected);
                let mode = modes.as_ref().and_then(|modes| modes.get(index)).copied();
                let pointer = if func_ty.is_some() {
                    // A function type applies the same named-parameter rule as
                    // the function it holds: only a non-duplicable struct lend
                    // is the pointer to the value.
                    expected.is_some_and(|expected| {
                        is_pointer_param(expected, mode.unwrap_or(Mode::None), &self.sema.table)
                    })
                } else {
                    user_call
                        && is_pointer_param(&arg.ty, mode.unwrap_or(Mode::None), &self.sema.table)
                };
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
                        store_cast: None,
                    },
                };
                let low = self.intrinsic_arg(args, 1);
                let high = self.intrinsic_arg(args, 2);
                let element = self.intrinsic_element(type_arg, args.first());
                let offset = format!("({}) + ({low})", source.pointer);
                let stored = match &source.store_cast {
                    Some(cast) => format!("{cast}({offset})"),
                    None => offset,
                };
                format!(
                    "((ClSlice_{}){{ {stored}, (usize)(({high}) - ({low})) }})",
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

/// The pointer, element count, and stored-pointer cast of a slice source.
struct SliceSource {
    /// The pointer the slice offsets advance in elements; for a view of rows
    /// it advances a whole row at a time.
    pointer: String,
    /// The number of elements the source holds.
    length: String,
    /// The cast that stores the offset pointer back in the view's
    /// base-element field, when the pointer addresses rows.
    store_cast: Option<String>,
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
