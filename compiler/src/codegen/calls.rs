//! Call-argument rendering and the compiler-known intrinsic helpers.
//!
//! The emitter renders a call's argument list here so a pointer-lowered
//! writable lend can take its place's address, and it renders the reserved
//! built-ins and the compiler-known view builders `rawSlice`, `rawStr`, and
//! `slice` here so the view struct and its bounds guard live in one place.

use crate::ast::Mode;
use crate::ir::Value;
use crate::types::{Ty, TyKind};

use super::types::{c_type, type_key};
use super::Emitter;

impl Emitter<'_> {
    /// Emits the bounds guard of the `slice` intrinsic.
    ///
    /// A slice is legal only when `lo <= hi` and `hi` lies inside the source
    /// view. The guard runs before the view is built, so an out-of-range
    /// request traps with E11211 instead of reading past the storage.
    pub(super) fn emit_slice_guard(&mut self, args: &[Value]) {
        let Some(view) = args.first() else {
            return;
        };
        let Some(lo) = args.get(1) else {
            return;
        };
        let Some(hi) = args.get(2) else {
            return;
        };
        let view_c = self.value(view);
        let lo_c = self.value(lo);
        let hi_c = self.value(hi);
        self.push(&format!(
            "    if ((usize)({lo_c}) > (usize)({hi_c}) || (usize)({hi_c}) > (usize)(({view_c}).len)) cl_trap(\"E11211\", \"index out of range\");"
        ));
    }

    /// Renders a call's argument list, taking the address of pointer-lowered
    /// writable lends.
    ///
    /// A `borrow` parameter of a non-duplicable struct lowers to `T *`, so the
    /// caller passes the address of its place. Every other argument, and every
    /// `ffi` argument, is passed as its value.
    pub(super) fn call_arguments(&self, callee: &str, args: &[Value]) -> String {
        let modes = self.callee_modes(callee);
        let user_call = self.resolve_function(callee).is_some();
        let rendered: Vec<String> = args
            .iter()
            .enumerate()
            .map(|(index, arg)| {
                let text = self.value(arg);
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
                let view = self.intrinsic_arg(args, 0);
                let low = self.intrinsic_arg(args, 1);
                let high = self.intrinsic_arg(args, 2);
                let element = self.intrinsic_element(type_arg, args.first());
                format!(
                    "((ClSlice_{}){{ ({view}).ptr + ({low}), (usize)(({high}) - ({low})) }})",
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
