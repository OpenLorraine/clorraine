//! The program-wide type table.
//!
//! The table collects every struct, enum, handle, function, and global the
//! program declares, so a written type name can be turned into a [`Ty`] and a
//! struct's fields can be looked up. Names are global within a program; the
//! MVP corpus and the diagnostic fixtures declare each type once, so a
//! program-wide name is enough. A future revision can qualify the keys by
//! module without changing the callers.

use std::collections::{BTreeMap, BTreeSet};

use crate::ast::{
    CallableBound, Expr, Item, Mode, QualifiedName, ScalarType, Type, TypeKind, TypeParam,
};
use crate::consts::ConstFolder;
use crate::load::Loaded;
use crate::span::Span;

use super::layout::AbiLayout;
use super::{int_literal_value, substitute_instance, FuncParam, FuncTy, Ty, TyKind};

/// A function signature collected from a declaration.
#[derive(Clone, Debug)]
pub struct FuncSig {
    /// Parameter types in order.
    pub params: Vec<Ty>,
    /// The declared mode of each parameter, in order.
    ///
    /// The checker keeps the modes so a call can tell a `take` argument, which
    /// moves its value, from a lend. A signature rebuilt from a function
    /// pointer type has no declaration, so its mode list is empty.
    pub modes: Vec<crate::ast::Mode>,
    /// Return type.
    pub returns: Ty,
    /// The function's generic parameters, each with its optional callable bound.
    pub type_params: Vec<TypeParamSig>,
}

/// A generic parameter with its optional callable bound.
///
/// The bound is resolved when the signature is collected, so the checker can
/// verify a concrete argument against it without re-reading the declaration.
#[derive(Clone, Debug)]
pub struct TypeParamSig {
    /// Parameter name.
    pub name: String,
    /// The `func(...)` bound, when the declaration wrote one.
    pub bound: Option<FuncTy>,
}

impl FuncSig {
    /// The function-pointer type of this signature.
    ///
    /// The modes are zipped back with the parameter types, so a declared
    /// `borrow` parameter keeps its mode in the function type. A signature
    /// whose mode list is shorter than its parameter list (the default shape)
    /// leaves the missing entries mode-less.
    pub fn ty(&self) -> FuncTy {
        FuncTy {
            params: self
                .params
                .iter()
                .enumerate()
                .map(|(index, ty)| FuncParam {
                    ty: ty.clone(),
                    mode: self.modes.get(index).copied().unwrap_or(Mode::None),
                })
                .collect(),
            returns: Box::new(self.returns.clone()),
        }
    }
}

impl Default for FuncSig {
    fn default() -> Self {
        Self {
            params: Vec::new(),
            modes: Vec::new(),
            returns: Ty::unknown(),
            type_params: Vec::new(),
        }
    }
}

/// The shape of a declared struct.
#[derive(Default)]
struct StructInfo {
    /// Fields in declaration order.
    fields: Vec<(String, Ty)>,
    /// Written names of the struct's generic parameters.
    type_params: Vec<String>,
    /// True when the struct has a `volatile` field, making it an overlay type.
    overlay: bool,
}

/// The shape of a declared enum.
struct EnumInfo {
    /// The required integer backing type.
    backing: Ty,
    /// Member names in declaration order.
    members: Vec<String>,
}

/// Every declared type of a loaded program, keyed by its written name.
#[derive(Default)]
pub struct TypeTable {
    structs: BTreeMap<String, StructInfo>,
    /// Every module that declares each simple struct name, in build order.
    ///
    /// The table keeps its simple-name keys; the origins let the semantic
    /// stage report a cross-module collision without changing the keys or the
    /// emitter's type identity.
    struct_origins: BTreeMap<String, Vec<(String, Span)>>,
    /// Every module that declares each simple handle name, in build order.
    ///
    /// A handle has exactly one deleter, so a cross-module duplicate is an
    /// error; the first origin is the declaration both stages use.
    handle_origins: BTreeMap<String, Vec<(String, Span)>>,
    enums: BTreeMap<String, EnumInfo>,
    handles: BTreeSet<String>,
    functions: BTreeMap<String, BTreeMap<String, FuncSig>>,
    globals: BTreeMap<String, BTreeMap<String, Ty>>,
    /// The target ABI's layout, for the sizes an extent position must match.
    layout: AbiLayout,
}

impl TypeTable {
    /// Reports each cross-module struct-name collision at its second origin.
    ///
    /// The table still keys structs by simple name; this query only reads the
    /// recorded declaration origins. A duplicate inside one module is the
    /// loader's declaration error, so only the first origin from a different
    /// module is returned for each name.
    pub fn cross_module_struct_collisions(&self) -> Vec<(String, String, Span)> {
        Self::cross_module_collisions(&self.struct_origins)
    }

    /// Reports each cross-module handle-name collision at its second origin.
    ///
    /// A handle has exactly one deleter, so two modules may not declare the
    /// same handle; the same-module duplicate is the loader's error.
    pub fn cross_module_handle_collisions(&self) -> Vec<(String, String, Span)> {
        Self::cross_module_collisions(&self.handle_origins)
    }

    /// Shared worker for the two origin maps.
    fn cross_module_collisions(
        origins: &BTreeMap<String, Vec<(String, Span)>>,
    ) -> Vec<(String, String, Span)> {
        let mut collisions = Vec::new();
        for (name, origins) in origins {
            let Some((first_module, _)) = origins.first() else {
                continue;
            };
            if let Some((module, span)) = origins
                .iter()
                .skip(1)
                .find(|(module, _)| module != first_module)
            {
                collisions.push((name.clone(), module.clone(), *span));
            }
        }
        collisions
    }

    /// Collects the declarations of every module in two passes: first the name
    /// sets (so a reference can be classified), then the resolved types.
    pub fn build(loaded: &Loaded, layout: AbiLayout) -> Self {
        let mut table = TypeTable {
            layout,
            ..TypeTable::default()
        };
        table.handles.insert("String".to_owned());
        table.handle_origins.insert(
            "String".to_owned(),
            vec![("<predeclared>".to_owned(), Span::default())],
        );

        for module in loaded.modules.values() {
            for item in &module.items {
                match item {
                    Item::Struct(decl) => {
                        table.structs.entry(decl.name.clone()).or_default();
                        table
                            .struct_origins
                            .entry(decl.name.clone())
                            .or_default()
                            .push((module.name.clone(), decl.name_span));
                    }
                    Item::Enum(decl) => {
                        table.enums.entry(decl.name.clone()).or_insert_with(|| EnumInfo {
                            backing: Ty::unknown(),
                            members: Vec::new(),
                        });
                    }
                    Item::Handle(decl) => {
                        table.handles.insert(decl.name.clone());
                        table
                            .handle_origins
                            .entry(decl.name.clone())
                            .or_default()
                            .push((module.name.clone(), decl.name_span));
                    }
                    Item::Func(decl) | Item::Comptime(decl) => {
                        table
                            .functions
                            .entry(module.name.clone())
                            .or_default()
                            .insert(decl.name.clone(), FuncSig::default());
                    }
                    Item::GlobalLet(decl) => {
                        table
                            .globals
                            .entry(module.name.clone())
                            .or_default()
                            .insert(decl.name.clone(), Ty::unknown());
                    }
                    Item::GlobalConst(decl) => {
                        table
                            .globals
                            .entry(module.name.clone())
                            .or_default()
                            .insert(decl.name.clone(), Ty::unknown());
                    }
                    Item::Ffi(_) | Item::Intrinsic(_) => {}
                }
            }
        }

        for module in loaded.modules.values() {
            for item in &module.items {
                match item {
                    Item::Struct(decl) => {
                        let params = type_param_names(&decl.type_params);
                        let fields = decl
                            .fields
                            .iter()
                            .map(|field| {
                                (
                                    field.name.clone(),
                                    resolve_ty_folded(&field.ty, &params, &table, module, loaded),
                                )
                            })
                            .collect();
                        let overlay = decl.fields.iter().any(|field| field.volatile);
                        table.structs.insert(
                            decl.name.clone(),
                            StructInfo {
                                fields,
                                type_params: params,
                                overlay,
                            },
                        );
                    }
                    Item::Enum(decl) => {
                        let backing = resolve_ty_folded(&decl.backing, &[], &table, module, loaded);
                        let members = decl.members.iter().map(|member| member.name.clone()).collect();
                        table
                            .enums
                            .insert(decl.name.clone(), EnumInfo { backing, members });
                    }
                    Item::Func(decl) | Item::Comptime(decl) => {
                        let params = type_param_names(&decl.type_params);
                        let signature = FuncSig {
                            params: decl
                                .params
                                .iter()
                                .map(|param| {
                                    resolve_ty_folded(
                                        &param.ty,
                                        &params,
                                        &table,
                                        module,
                                        loaded,
                                    )
                                })
                                .collect(),
                            modes: decl.params.iter().map(|param| param.mode).collect(),
                            returns: resolve_ty_folded(
                                &decl.returns,
                                &params,
                                &table,
                                module,
                                loaded,
                            ),
                            type_params: type_param_sigs(&decl.type_params, &params, &table),
                        };
                        if let Some(module_functions) = table.functions.get_mut(&module.name) {
                            module_functions.insert(decl.name.clone(), signature);
                        }
                    }
                    Item::GlobalLet(decl) => {
                        let ty = resolve_ty_folded(&decl.ty, &[], &table, module, loaded);
                        if let Some(module_globals) = table.globals.get_mut(&module.name) {
                            module_globals.insert(decl.name.clone(), ty);
                        }
                    }
                    Item::GlobalConst(decl) => {
                        let ty = resolve_ty_folded(&decl.ty, &[], &table, module, loaded);
                        if let Some(module_globals) = table.globals.get_mut(&module.name) {
                            module_globals.insert(decl.name.clone(), ty);
                        }
                    }
                    Item::Handle(_) | Item::Ffi(_) | Item::Intrinsic(_) => {}
                }
            }
        }
        table.resolve_constant_extents(loaded);
        table
    }

    /// Re-resolves struct fields and global types so a `const` name, or an
    /// arithmetic expression, supplies an array extent rather than leaving it
    /// omitted.
    ///
    /// The first two passes read integer literals only, because the table they
    /// would need does not exist yet. Once every declaration and its type is
    /// in place, a constant folder can resolve the remaining extents the same
    /// way the checker does. Each new type is collected before it is written,
    /// so the shared borrow the folder holds ends first. A type whose folder
    /// cannot resolve an extent keeps its previous, literal-only form.
    fn resolve_constant_extents(&mut self, loaded: &Loaded) {
        let mut struct_updates: Vec<(String, Vec<(String, Ty)>)> = Vec::new();
        let mut global_updates: Vec<(String, String, Ty)> = Vec::new();
        {
            for module in loaded.modules.values() {
                let mut folder = ConstFolder::for_extents(self, module, loaded);
                for item in &module.items {
                    match item {
                        Item::Struct(decl) => {
                            let params = type_param_names(&decl.type_params);
                            let mut alias = |name: &str| {
                                module.import(name).map(|binding| binding.original.clone())
                            };
                            let mut fields = Vec::with_capacity(decl.fields.len());
                            for field in &decl.fields {
                                let ty = resolve_ty_with_alias(
                                    &field.ty,
                                    &params,
                                    self,
                                    &mut |expr| folder.integer_at(expr, Some(ScalarType::Usize)),
                                    &mut alias,
                                    module.embedded && module.name == "std.mem",
                                );
                                fields.push((field.name.clone(), ty));
                            }
                            struct_updates.push((decl.name.clone(), fields));
                        }
                        Item::GlobalLet(decl) => {
                            let mut alias = |name: &str| {
                                module.import(name).map(|binding| binding.original.clone())
                            };
                            let ty = resolve_ty_with_alias(
                                &decl.ty,
                                &[],
                                self,
                                &mut |expr| folder.integer_at(expr, Some(ScalarType::Usize)),
                                &mut alias,
                                module.embedded && module.name == "std.mem",
                            );
                            global_updates.push((module.name.clone(), decl.name.clone(), ty));
                        }
                        Item::GlobalConst(decl) => {
                            let mut alias = |name: &str| {
                                module.import(name).map(|binding| binding.original.clone())
                            };
                            let ty = resolve_ty_with_alias(
                                &decl.ty,
                                &[],
                                self,
                                &mut |expr| folder.integer_at(expr, Some(ScalarType::Usize)),
                                &mut alias,
                                module.embedded && module.name == "std.mem",
                            );
                            global_updates.push((module.name.clone(), decl.name.clone(), ty));
                        }
                        _ => {}
                    }
                }
            }
        }
        for (name, fields) in struct_updates {
            if let Some(info) = self.structs.get_mut(&name) {
                info.fields = fields;
            }
        }
        for (module, name, ty) in global_updates {
            if let Some(globals) = self.globals.get_mut(&module) {
                globals.insert(name, ty);
            }
        }
    }

    /// True when the program declares a struct with this name.
    pub fn is_struct(&self, name: &str) -> bool {
        self.structs.contains_key(name)
    }

    /// True when the program declares an enum with this name.
    pub fn is_enum(&self, name: &str) -> bool {
        self.enums.contains_key(name)
    }

    /// True when the program declares a handle with this name, or it is the
    /// built-in `String`.
    pub fn is_handle(&self, name: &str) -> bool {
        self.handles.contains(name)
    }

    /// The declared fields of a struct, in declaration order.
    pub fn struct_fields(&self, name: &str) -> Option<&[(String, Ty)]> {
        self.structs.get(name).map(|info| info.fields.as_slice())
    }

    /// The written names of a struct's generic parameters.
    pub fn struct_type_params(&self, name: &str) -> Option<&[String]> {
        self.structs
            .get(name)
            .map(|info| info.type_params.as_slice())
    }

    /// True when a struct has a `volatile` field and is therefore an overlay
    /// type that cannot be used as an ordinary value.
    pub fn is_overlay(&self, name: &str) -> bool {
        self.structs.get(name).is_some_and(|info| info.overlay)
    }

    /// The type of one struct field.
    pub fn struct_field(&self, name: &str, field: &str) -> Option<Ty> {
        self.structs
            .get(name)
            .and_then(|info| info.fields.iter().find(|(f, _)| f == field))
            .map(|(_, ty)| ty.clone())
    }

    /// The declared type of a struct field with the instance's arguments
    /// substituted.
    ///
    /// `base` is the struct instance type (`Pair<i32>`), so a field written in
    /// terms of the declaration's parameters is resolved to the instance's
    /// argument. Returns `None` when `base` is not a struct, or the struct or
    /// field is unknown.
    pub fn instance_field(&self, base: &Ty, field: &str) -> Option<Ty> {
        let TyKind::Struct(name, args) = &base.kind else {
            return None;
        };
        let declared = self.struct_field(name, field)?;
        let params = self.struct_type_params(name).unwrap_or(&[]);
        Some(substitute_instance(&declared, params, args))
    }

    /// The backing integer type of an enum.
    pub fn enum_backing(&self, name: &str) -> Option<&Ty> {
        self.enums.get(name).map(|info| &info.backing)
    }

    /// True when the enum declares this member name.
    pub fn enum_has_member(&self, name: &str, member: &str) -> bool {
        self.enums
            .get(name)
            .is_some_and(|info| info.members.iter().any(|known| known == member))
    }

    /// The signature of a module-level function.
    pub fn function(&self, module: &str, name: &str) -> Option<&FuncSig> {
        self.functions.get(module).and_then(|fns| fns.get(name))
    }

    /// The declared type of a module-level `let` or `const`.
    pub fn global(&self, module: &str, name: &str) -> Option<&Ty> {
        self.globals.get(module).and_then(|globals| globals.get(name))
    }

    /// True when values of the type copy instead of moving.
    ///
    /// Duplicability is derived from the type's fields and elements: a handle
    /// owns a resource and is move-only, and an aggregate that contains one is
    /// move-only as well. The recursion is depth-limited so a malformed
    /// self-referential declaration cannot loop forever.
    pub fn duplicable(&self, ty: &Ty) -> bool {
        self.duplicable_at(ty, 0)
    }

    /// Depth-limited worker for [`TypeTable::duplicable`].
    fn duplicable_at(&self, ty: &Ty, depth: usize) -> bool {
        if depth > 32 {
            return true;
        }
        match &ty.kind {
            TyKind::Handle(_) => false,
            TyKind::Box(_) => false,
            // A capturing closure is ephemeral: it owns its environment for one
            // synchronous call and cannot be copied, stored, or captured.
            TyKind::Closure(_) => false,
            TyKind::Struct(name, args) => self.struct_fields(name).is_none_or(|fields| {
                let params = self.struct_type_params(name).unwrap_or(&[]);
                fields.iter().all(|(_, field)| {
                    self.duplicable_at(&substitute_instance(field, params, args), depth + 1)
                })
            }),
            // A slice is a view: copying it copies a pointer and a length, so it
            // is always duplicable. A fixed array is owned storage, so it is
            // duplicable only when its element is.
            TyKind::Array { element, .. } => {
                if ty.is_slice() {
                    true
                } else {
                    self.duplicable_at(element, depth + 1)
                }
            }
            _ => true,
        }
    }

    /// True when a call passes the whole container by value.
    ///
    /// A duplicable struct is copied, so a view store fills only the callee's
    /// copy. A duplicable array or slice is not the same thing: a fixed array
    /// decays to a pointer and a slice copies only its pointer and length, so
    /// either spelling still reaches the caller's storage. A view-like root is
    /// shared storage by definition, not a copy of its backing.
    pub fn copies_whole_value(&self, ty: &Ty) -> bool {
        self.duplicable(ty)
            && !matches!(
                ty.kind,
                TyKind::Array { .. }
                    | TyKind::Str
                    | TyKind::StaticStr
                    | TyKind::CString
                    | TyKind::Raw(_)
            )
    }

    /// The storage size of a type in bytes, when it is known at this stage.
    ///
    /// The value is used by constant folding; a type whose size is not
    /// modelled yet yields `None` so the caller can defer.
    pub fn size_of(&self, ty: &Ty) -> Option<usize> {
        self.size_of_at(ty, 0)
    }

    /// The byte size of a type under the target ABI.
    ///
    /// The declared layout sums field widths without the target's alignment
    /// padding and assumes 32-bit pointers; an array extent must equal the C
    /// type the emitter spells, so the extent position measures through this
    /// ABI-faithful counterpart instead.
    pub fn abi_size_of(&self, ty: &Ty) -> Option<usize> {
        self.layout.size_of(self, ty)
    }

    /// Depth-limited worker for [`TypeTable::size_of`].
    ///
    /// A malformed self-referential declaration would otherwise recurse without
    /// bound; the limit makes the size unknown instead of overflowing.
    fn size_of_at(&self, ty: &Ty, depth: usize) -> Option<usize> {
        if depth > 32 {
            return None;
        }
        match &ty.kind {
            TyKind::Void => Some(0),
            TyKind::Scalar(scalar) => scalar_size(*scalar),
            TyKind::Raw(_) | TyKind::Func(_) | TyKind::Handle(_) | TyKind::Box(_) => Some(4),
            TyKind::Str | TyKind::StaticStr | TyKind::CString => Some(8),
            // An enum's storage is its declared backing scalar.
            TyKind::Enum(name) => self
                .enum_backing(name)
                .and_then(|backing| self.size_of_at(backing, depth + 1)),
            TyKind::Array { element, extents } => {
                // An omitted extent is a view: its storage is one pointer and
                // one length, regardless of the element or later extents.
                if extents.iter().any(Option::is_none) {
                    return Some(8);
                }
                let element = self.size_of_at(element, depth + 1)?;
                let mut total = element;
                for extent in extents {
                    total = total.checked_mul((*extent)?)?;
                }
                Some(total)
            }
            TyKind::Struct(name, args) => {
                let params = self.struct_type_params(name).unwrap_or(&[]);
                let mut total = 0usize;
                for (_, field) in self.struct_fields(name)? {
                    total = total.checked_add(
                        self.size_of_at(&substitute_instance(field, params, args), depth + 1)?,
                    )?;
                }
                Some(total)
            }
            _ => None,
        }
    }

    /// The byte offset of a struct field under the target ABI.
    ///
    /// `offset_of` sums the declared widths without alignment padding; an
    /// emitted `offsetof` and an extent position both use the C layout, so the
    /// two positions agree on a padded struct.
    pub fn abi_offset_of(&self, base: &Ty, field: &str) -> Option<usize> {
        self.layout.offset_of(self, base, field)
    }

    /// The byte offset of a struct field, when every earlier field's size is
    /// known.
    ///
    /// `base` is the struct instance type, so a generic instance's arguments are
    /// substituted into each preceding field's size before it is added.
    pub fn offset_of(&self, base: &Ty, field: &str) -> Option<usize> {
        let TyKind::Struct(name, args) = &base.kind else {
            return None;
        };
        let params = self.struct_type_params(name).unwrap_or(&[]);
        let mut offset = 0usize;
        for (field_name, field_ty) in self.struct_fields(name)? {
            if field_name == field {
                return Some(offset);
            }
            offset =
                offset.checked_add(self.size_of(&substitute_instance(field_ty, params, args))?)?;
        }
        None
    }
}

/// The byte width of a scalar.
pub(super) fn scalar_size(scalar: crate::ast::ScalarType) -> Option<usize> {
    use crate::ast::ScalarType;
    match scalar {
        ScalarType::I8 | ScalarType::U8 | ScalarType::Bool => Some(1),
        ScalarType::I16 | ScalarType::U16 => Some(2),
        ScalarType::I32 | ScalarType::U32 | ScalarType::F32 => Some(4),
        ScalarType::F64 => Some(8),
        ScalarType::Usize | ScalarType::Isize => Some(4),
    }
}

/// Collects the written names of a declaration's type parameters.
fn type_param_names(params: &[TypeParam]) -> Vec<String> {
    params.iter().map(|param| param.name.clone()).collect()
}

/// Collects a declaration's type parameters, resolving any callable bound.
///
/// `names` is the same parameter list's names, reused so a bound that mentions a
/// sibling parameter (as in `F: func(T): T`) resolves it to an opaque
/// [`TyKind::Param`].
fn type_param_sigs(params: &[TypeParam], names: &[String], table: &TypeTable) -> Vec<TypeParamSig> {
    params
        .iter()
        .map(|param| TypeParamSig {
            name: param.name.clone(),
            bound: param
                .bound
                .as_ref()
                .map(|bound| resolve_callable_bound(bound, names, table)),
        })
        .collect()
}

/// Resolves a `func(...)` bound into a function-pointer signature.
fn resolve_callable_bound(
    bound: &CallableBound,
    type_params: &[String],
    table: &TypeTable,
) -> FuncTy {
    FuncTy {
        params: bound
            .params
            .iter()
            .map(|param| FuncParam {
                ty: resolve_ty(param, type_params, table),
                mode: Mode::None,
            })
            .collect(),
        returns: Box::new(resolve_ty(&bound.returns, type_params, table)),
    }
}

/// Resolves an AST type against the collected declarations.
///
/// A name that is not a known struct, enum, handle, type parameter, or the
/// built-in `String` becomes [`TyKind::Unknown`], so a not-yet-modelled type
/// does not cascade into false diagnostics.
///
/// Array extents are folded by [`int_literal_value`], which reads plain and
/// negated integer literals only. Callers that can evaluate a constant name or
/// arithmetic use [`resolve_ty_with`] to supply their own folder.
pub fn resolve_ty(ty: &Type, type_params: &[String], table: &TypeTable) -> Ty {
    resolve_ty_with(ty, type_params, table, &mut |expr| int_literal_value(expr))
}

/// Resolves an AST type, folding constant array extents through the module's
/// constant environment.
///
/// [`resolve_ty`] folds integer literals only, so a `const`-sized extent
/// becomes an omitted (slice) extent. A declaration type must resolve the same
/// way wherever it is read — the type stage, the ownership stage, and the
/// emitter — or the view a call builds disagrees with the view a signature
/// declares; those shared readers use this resolver.
pub fn resolve_ty_folded(
    ty: &Type,
    type_params: &[String],
    table: &TypeTable,
    module: &crate::load::Module,
    loaded: &Loaded,
) -> Ty {
    let mut folder = ConstFolder::for_extents(table, module, loaded);
    let mut alias = |name: &str| {
        module.import(name).map(|binding| binding.original.clone())
    };
    resolve_ty_with_alias(
        ty,
        type_params,
        table,
        &mut |expr| folder.integer_at(expr, Some(ScalarType::Usize)),
        &mut alias,
        module.embedded && module.name == "std.mem",
    )
}

/// Resolves an AST type, folding each array extent with `fold`.
///
/// `fold` returns the wide integer value of an extent expression, or `None`
/// when it is not constant. A negative or too-large value that does not fit a
/// `usize` leaves the extent unknown so the caller can report the fault.
pub fn resolve_ty_with<F>(
    ty: &Type,
    type_params: &[String],
    table: &TypeTable,
    fold: &mut F,
) -> Ty
where
    F: FnMut(&Expr) -> Option<i128>,
{
    resolve_ty_with_alias(ty, type_params, table, fold, &mut |_| None, false)
}

/// Resolves a written type with a module's import aliases.
///
/// `alias` maps a simple written name (for example `A`) to its original
/// declaration name (`Array`). It is consulted before the table lookup, so an
/// imported type alias reaches the same `Ty` as its original spelling.
pub fn resolve_ty_with_alias<F, A>(
    ty: &Type,
    type_params: &[String],
    table: &TypeTable,
    fold: &mut F,
    alias: &mut A,
    prefer_builtin_box: bool,
) -> Ty
where
    F: FnMut(&Expr) -> Option<i128>,
    A: FnMut(&str) -> Option<String>,
{
    let kind = match &ty.kind {
        TypeKind::Void => TyKind::Void,
        TypeKind::Scalar(scalar) => TyKind::Scalar(*scalar),
        TypeKind::Str => TyKind::Str,
        TypeKind::CString => TyKind::CString,
        TypeKind::StaticStr => TyKind::StaticStr,
        TypeKind::Raw(inner) => TyKind::Raw(Box::new(resolve_ty_with_alias(
            inner, type_params, table, fold, alias, prefer_builtin_box,
        ))),
        TypeKind::Array { element, extents } => TyKind::Array {
            element: Box::new(resolve_ty_with_alias(
                element, type_params, table, fold, alias, prefer_builtin_box,
            )),
            extents: extents
                .iter()
                .map(|extent| {
                    let value = extent.size.as_ref().and_then(|expr| fold(expr))?;
                    usize::try_from(value).ok()
                })
                .collect(),
        },
        TypeKind::Function { params, returns } => TyKind::Func(FuncTy {
            params: params
                .iter()
                .map(|param| FuncParam {
                    ty: resolve_ty_with_alias(
                        &param.ty,
                        type_params,
                        table,
                        fold,
                        alias,
                        prefer_builtin_box,
                    ),
                    mode: param.mode,
                })
                .collect(),
            returns: Box::new(resolve_ty_with_alias(
                returns, type_params, table, fold, alias, prefer_builtin_box,
            )),
        }),
        TypeKind::Named { name, args } => {
            let mut resolved_args = Vec::with_capacity(args.len());
            for arg in args {
                resolved_args.push(resolve_ty_with_alias(
                    arg,
                    type_params,
                    table,
                    fold,
                    alias,
                    prefer_builtin_box,
                ));
            }
            let aliased = if name.segments.len() == 1
                && !name
                    .segments
                    .first()
                    .is_some_and(|name| type_params.iter().any(|param| param == name))
            {
                name.segments.first().and_then(|name| alias(name)).map(|original| {
                    QualifiedName {
                        segments: vec![original],
                        span: name.span,
                    }
                })
            } else {
                None
            };
            resolve_named(
                aliased.as_ref().unwrap_or(name),
                resolved_args,
                type_params,
                table,
                prefer_builtin_box,
            )
        }
        TypeKind::Error => TyKind::Unknown,
    };
    Ty {
        kind,
        nullable: ty.nullable,
    }
}

/// Resolves a named type to its kind.
///
/// A struct keeps its concrete type arguments so a generic consumer can infer
/// its parameters; every other named type ignores them. A qualified path
/// resolves through its last segment: a type table is keyed by the simple
/// declaration name, so `std.mem.Array<i32>` names the same instance as the
/// imported `Array<i32>`. A type parameter is only reachable by its bare name,
/// because a qualified spelling belongs to a module path.
fn resolve_named(
    name: &QualifiedName,
    args: Vec<Ty>,
    type_params: &[String],
    table: &TypeTable,
    prefer_builtin_box: bool,
) -> TyKind {
    let Some(last) = name.segments.last() else {
        return TyKind::Unknown;
    };
    if name.segments.len() == 1 && type_params.iter().any(|param| param == last) {
        return TyKind::Param(last.clone());
    }
    // Inside the embedded `std.mem`, the built-in box wins over a same-named
    // user struct, so the library's own `Box<T>` signatures stay the built-in.
    // A user module keeps the ordinary shadowing order.
    if prefer_builtin_box && last == "Box" && args.len() == 1 {
        return TyKind::Box(Box::new(args.into_iter().next().expect("one argument")));
    }
    if table.is_struct(last) {
        return TyKind::Struct(last.clone(), args);
    }
    if last == "String" {
        return TyKind::Handle("String".to_owned());
    }
    if table.is_handle(last) {
        return TyKind::Handle(last.clone());
    }
    if table.is_enum(last) {
        return TyKind::Enum(last.clone());
    }
    // The built-in owning pointer is a type-only name. A wrong argument count
    // stays unresolved so the declaration check reports E10300 once, rather
    // than inventing a Box of the wrong shape.
    if last == "Box" && args.len() == 1 {
        return TyKind::Box(Box::new(args.into_iter().next().expect("one argument")));
    }
    TyKind::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::ScalarType;

    /// Inserts a struct with the given fields for a layout test.
    fn insert_struct(table: &mut TypeTable, name: &str, fields: Vec<(&str, Ty)>) {
        insert_generic_struct(table, name, &[], fields);
    }

    /// Inserts a struct whose fields may name the given generic parameters.
    ///
    /// The parameter names are recorded so a field written in terms of `T` can
    /// be resolved once an instance supplies that argument.
    fn insert_generic_struct(
        table: &mut TypeTable,
        name: &str,
        params: &[&str],
        fields: Vec<(&str, Ty)>,
    ) {
        table.structs.insert(
            name.to_owned(),
            StructInfo {
                fields: fields
                    .into_iter()
                    .map(|(field, ty)| (field.to_owned(), ty))
                    .collect(),
                type_params: params.iter().map(|param| (*param).to_owned()).collect(),
                overlay: false,
            },
        );
    }

    /// A generic-parameter atom, as a struct field is declared before an
    /// instance supplies the argument.
    fn param(name: &str) -> Ty {
        Ty {
            kind: TyKind::Param(name.to_owned()),
            nullable: false,
        }
    }

    /// A non-nullable scalar type.
    fn scalar(scalar: ScalarType) -> Ty {
        Ty::scalar(scalar)
    }

    #[test]
    fn a_handle_field_makes_a_struct_move_only() {
        let mut table = TypeTable::default();
        insert_struct(&mut table, "Label", vec![("name", Ty::handle("String"))]);
        insert_struct(
            &mut table,
            "Point",
            vec![("x", Ty::i32_ty()), ("y", Ty::i32_ty())],
        );
        assert!(!table.duplicable(&Ty::struct_("Label")));
        assert!(table.duplicable(&Ty::struct_("Point")));
        assert!(!table.duplicable(&Ty::handle("String")));
        let fixed = Ty::array(Ty::i32_ty(), vec![Some(2)]);
        assert!(table.duplicable(&fixed));
        assert!(!table.copies_whole_value(&fixed));
        let slice = Ty::array(Ty::i32_ty(), vec![None]);
        assert!(table.duplicable(&slice));
        assert!(!table.copies_whole_value(&slice));
        assert!(table.copies_whole_value(&Ty::struct_("Point")));
        assert!(!table.copies_whole_value(&Ty::struct_("Label")));
        assert!(table.duplicable(&scalar(ScalarType::F64)));
    }

    #[test]
    fn duplicability_recurses_through_fields_and_elements() {
        let mut table = TypeTable::default();
        insert_struct(&mut table, "Owner", vec![("name", Ty::handle("String"))]);
        insert_struct(&mut table, "Nested", vec![("owner", Ty::struct_("Owner"))]);
        insert_struct(&mut table, "Plain", vec![("n", Ty::i32_ty())]);
        insert_struct(
            &mut table,
            "Batch",
            vec![("items", Ty::array(Ty::handle("String"), vec![Some(2)]))],
        );
        assert!(!table.duplicable(&Ty::struct_("Owner")));
        assert!(!table.duplicable(&Ty::struct_("Nested")));
        assert!(!table.duplicable(&Ty::struct_("Batch")));
        assert!(table.duplicable(&Ty::struct_("Plain")));
        assert!(table.duplicable(&Ty::array(Ty::i32_ty(), vec![Some(4)])));
        // A raw pointer is a copy even when it names an owner.
        assert!(table.duplicable(&Ty {
            kind: TyKind::Raw(Box::new(Ty::struct_("Owner"))),
            nullable: false,
        }));
        assert!(table.duplicable(&Ty::unknown()));
    }

    #[test]
    fn a_generic_struct_holding_a_handle_is_move_only() {
        // `Box<String>` owns the `String`, so it copies only if the instance's
        // argument reaches the field; reading the declared `T` would call it
        // duplicable and let a handle be copied without a diagnostic.
        let mut table = TypeTable::default();
        insert_generic_struct(&mut table, "Box", &["T"], vec![("value", param("T"))]);
        assert!(table.duplicable(&Ty::struct_with("Box", vec![Ty::i32_ty()])));
        assert!(!table.duplicable(&Ty::struct_with("Box", vec![Ty::handle("String")])));
    }

    #[test]
    fn a_generic_struct_size_substitutes_its_type_argument() {
        // `sizeof(Box<i32>)` folds to the concrete field width rather than
        // deferring because the field is the bare parameter `T`.
        let mut table = TypeTable::default();
        insert_generic_struct(&mut table, "Box", &["T"], vec![("value", param("T"))]);
        assert_eq!(
            table.size_of(&Ty::struct_with("Box", vec![Ty::i32_ty()])),
            Some(4)
        );
    }

    #[test]
    fn a_generic_struct_field_offset_substitutes_its_type_argument() {
        // `offsetof(Pair<i32>, second)` folds to the width of the substituted
        // first field rather than deferring on the bare parameter.
        let mut table = TypeTable::default();
        insert_generic_struct(
            &mut table,
            "Pair",
            &["T"],
            vec![("first", param("T")), ("second", param("T"))],
        );
        assert_eq!(
            table.offset_of(&Ty::struct_with("Pair", vec![Ty::i32_ty()]), "second"),
            Some(4)
        );
    }

    #[test]
    fn substitute_instance_keeps_an_unbound_parameter() {
        // No parameters or no arguments is a no-op, so a non-generic type is
        // returned unchanged and a partial list never corrupts one.
        let declared = param("T");
        assert_eq!(substitute_instance(&declared, &[], &[]), declared);
        assert_eq!(
            substitute_instance(&declared, &["T".to_owned()], &[]),
            declared
        );
        // A short argument list binds what it can and leaves the rest in place.
        let params = ["T".to_owned(), "U".to_owned()];
        assert_eq!(
            substitute_instance(&param("U"), &params, &[Ty::i32_ty()]),
            param("U")
        );
        assert_eq!(
            substitute_instance(&param("T"), &params, &[Ty::i32_ty()]),
            Ty::i32_ty()
        );
    }

    #[test]
    fn instance_field_substitutes_and_is_a_no_op_without_arguments() {
        let mut table = TypeTable::default();
        insert_generic_struct(&mut table, "Box", &["T"], vec![("value", param("T"))]);
        insert_struct(&mut table, "Point", vec![("x", Ty::i32_ty())]);
        // A generic instance resolves the field to its argument.
        assert_eq!(
            table.instance_field(&Ty::struct_with("Box", vec![Ty::i32_ty()]), "value"),
            Some(Ty::i32_ty())
        );
        // A non-generic struct's field is returned unchanged.
        assert_eq!(
            table.instance_field(&Ty::struct_("Point"), "x"),
            Some(Ty::i32_ty())
        );
        assert_eq!(table.instance_field(&Ty::struct_("Point"), "missing"), None);
        // `offset_of` on a non-generic struct is unchanged.
        assert_eq!(table.offset_of(&Ty::struct_("Point"), "x"), Some(0));
    }

    #[test]
    fn sizes_and_offsets_fold_for_plain_layouts() {
        let mut table = TypeTable::default();
        insert_struct(
            &mut table,
            "Point",
            vec![("x", Ty::i32_ty()), ("y", Ty::i32_ty())],
        );
        assert_eq!(table.size_of(&Ty::struct_("Point")), Some(8));
        assert_eq!(table.offset_of(&Ty::struct_("Point"), "y"), Some(4));
        assert_eq!(table.offset_of(&Ty::struct_("Point"), "missing"), None);
    }

    #[test]
    fn mixed_field_widths_drive_size_and_field_offsets() {
        let mut table = TypeTable::default();
        insert_struct(
            &mut table,
            "Mixed",
            vec![
                ("tag", scalar(ScalarType::U8)),
                ("count", scalar(ScalarType::U16)),
                ("value", scalar(ScalarType::U32)),
                ("scale", scalar(ScalarType::F64)),
            ],
        );
        assert_eq!(table.size_of(&Ty::struct_("Mixed")), Some(15));
        assert_eq!(table.offset_of(&Ty::struct_("Mixed"), "tag"), Some(0));
        assert_eq!(table.offset_of(&Ty::struct_("Mixed"), "count"), Some(1));
        assert_eq!(table.offset_of(&Ty::struct_("Mixed"), "value"), Some(3));
        assert_eq!(table.offset_of(&Ty::struct_("Mixed"), "scale"), Some(7));
        assert_eq!(table.offset_of(&Ty::struct_("Mixed"), "missing"), None);
    }

    #[test]
    fn the_target_layouts_measure_the_emitted_c_types() {
        let mut table = TypeTable::default();
        insert_struct(
            &mut table,
            "S",
            vec![("a", Ty::i32_ty()), ("b", scalar(ScalarType::U8))],
        );
        insert_struct(
            &mut table,
            "Mixed",
            vec![
                ("tag", scalar(ScalarType::U8)),
                ("count", scalar(ScalarType::U16)),
                ("value", scalar(ScalarType::U32)),
            ],
        );
        let host = AbiLayout::host();
        let m68k = AbiLayout::m68k();
        let view = Ty {
            kind: TyKind::Str,
            nullable: false,
        };
        // The host C ABI pads `S` to its four-byte alignment and the view to
        // its eight-byte pointer alignment; the m68k ABI aligns to two bytes.
        // Both numbers are the ones the host `cc` and `m68k-amigaos-gcc`
        // report for the emitted C types.
        assert_eq!(host.size_of(&table, &Ty::struct_("S")), Some(8));
        assert_eq!(m68k.size_of(&table, &Ty::struct_("S")), Some(6));
        assert_eq!(host.size_of(&table, &view), Some(16));
        assert_eq!(m68k.size_of(&table, &view), Some(8));
        assert_eq!(host.size_of(&table, &Ty::struct_("Mixed")), Some(8));
        assert_eq!(m68k.size_of(&table, &Ty::struct_("Mixed")), Some(8));
        // The declared layout keeps its unpadded numbers for the value folds.
        assert_eq!(table.size_of(&Ty::struct_("S")), Some(5));
    }

    #[test]
    fn aggregate_sizes_follow_scalars_extents_and_elements() {
        let table = TypeTable::default();
        assert_eq!(table.size_of(&Ty::boolean()), Some(1));
        assert_eq!(table.size_of(&scalar(ScalarType::I16)), Some(2));
        assert_eq!(table.size_of(&scalar(ScalarType::F32)), Some(4));
        assert_eq!(table.size_of(&scalar(ScalarType::F64)), Some(8));
        assert_eq!(table.size_of(&scalar(ScalarType::Usize)), Some(4));
        assert_eq!(table.size_of(&Ty::void()), Some(0));
        assert_eq!(
            table.size_of(&Ty {
                kind: TyKind::Raw(Box::new(Ty::i32_ty())),
                nullable: false,
            }),
            Some(4)
        );
        assert_eq!(
            table.size_of(&Ty {
                kind: TyKind::Str,
                nullable: false,
            }),
            Some(8)
        );
        assert_eq!(
            table.size_of(&Ty::array(scalar(ScalarType::U16), vec![Some(3), Some(2)])),
            Some(12)
        );
        // A slice carries one pointer and one length, whatever its element.
        assert_eq!(
            table.size_of(&Ty::array(scalar(ScalarType::U16), vec![None])),
            Some(8)
        );
        assert_eq!(table.size_of(&Ty::unknown()), None);
    }

    #[test]
    fn a_slice_field_contributes_its_view_width_to_the_layout() {
        let mut table = TypeTable::default();
        insert_struct(
            &mut table,
            "Flexible",
            vec![
                ("head", scalar(ScalarType::U8)),
                ("body", Ty::array(scalar(ScalarType::U8), vec![None])),
                ("tail", Ty::i32_ty()),
            ],
        );
        // The slice view is a pointer and a length, so the declared layout is
        // 1 + 8 + 4 bytes; the tail starts after the view.
        assert_eq!(table.size_of(&Ty::struct_("Flexible")), Some(13));
        assert_eq!(table.offset_of(&Ty::struct_("Flexible"), "head"), Some(0));
        assert_eq!(table.offset_of(&Ty::struct_("Flexible"), "tail"), Some(9));
    }

    #[test]
    fn array_duplicability_follows_nested_elements_and_the_pointer_boundary() {
        let table = TypeTable::default();
        let owner = || Ty::handle("String");
        assert!(table.duplicable(&Ty::array(Ty::i32_ty(), vec![Some(2)])));
        // A slice is a view: copying it copies a pointer and a length, so it is
        // duplicable even when its element owns a resource.
        assert!(table.duplicable(&Ty::array(owner(), vec![None])));
        // A fixed array is owned storage, so it follows its element.
        assert!(!table.duplicable(&Ty::array(owner(), vec![Some(2)])));
        // A nested array recurses to its innermost element. The shape is built
        // directly because the array constructor now folds a fixed element
        // under a fixed outer extent into one flat extent list.
        let nested = |element: Ty| Ty {
            kind: TyKind::Array {
                element: Box::new(element),
                extents: vec![Some(2)],
            },
            nullable: false,
        };
        assert!(table.duplicable(&Ty {
            kind: TyKind::Array {
                element: Box::new(nested(Ty::i32_ty())),
                extents: vec![Some(3)],
            },
            nullable: false,
        }));
        assert!(!table.duplicable(&Ty {
            kind: TyKind::Array {
                element: Box::new(nested(owner())),
                extents: vec![Some(3)],
            },
            nullable: false,
        }));
        // A raw pointer copies, so an array of pointers is duplicable even
        // when the pointee owns a resource.
        assert!(table.duplicable(&Ty::array(Ty::raw(owner()), vec![Some(1)])));
    }

    #[test]
    fn duplicability_stops_recursing_at_the_depth_cap() {
        let table = TypeTable::default();
        // The shape is built directly: the array constructor folds a fixed
        // element under a fixed outer extent, so repeated construction would
        // collapse into one flat extent list and never reach the cap.
        let mut nested = Ty::handle("String");
        for _ in 0..40 {
            nested = Ty {
                kind: TyKind::Array {
                    element: Box::new(nested),
                    extents: vec![Some(1)],
                },
                nullable: false,
            };
        }
        // Past the cap the check assumes a copy instead of recursing without
        // bound, so a pathologically deep type still terminates.
        assert!(table.duplicable(&nested));
    }
}
