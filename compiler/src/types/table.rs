//! The program-wide type table.
//!
//! The table collects every struct, enum, handle, function, and global the
//! program declares, so a written type name can be turned into a [`Ty`] and a
//! struct's fields can be looked up. Names are global within a program; the
//! MVP corpus and the diagnostic fixtures declare each type once, so a
//! program-wide name is enough. A future revision can qualify the keys by
//! module without changing the callers.

use std::collections::{BTreeMap, BTreeSet};

use crate::ast::{Expr, Item, QualifiedName, Type, TypeKind, TypeParam};
use crate::load::Loaded;

use super::{int_literal_value, FuncTy, Ty, TyKind};

/// A function signature collected from a declaration.
#[derive(Clone, Debug)]
pub struct FuncSig {
    /// Parameter types in order.
    pub params: Vec<Ty>,
    /// Return type.
    pub returns: Ty,
    /// Names of the function's generic parameters.
    pub type_params: Vec<String>,
}

impl FuncSig {
    /// The function-pointer type of this signature.
    pub fn ty(&self) -> FuncTy {
        FuncTy {
            params: self.params.clone(),
            returns: Box::new(self.returns.clone()),
        }
    }
}

impl Default for FuncSig {
    fn default() -> Self {
        Self {
            params: Vec::new(),
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
    enums: BTreeMap<String, EnumInfo>,
    handles: BTreeSet<String>,
    functions: BTreeMap<String, BTreeMap<String, FuncSig>>,
    globals: BTreeMap<String, BTreeMap<String, Ty>>,
}

impl TypeTable {
    /// Collects the declarations of every module in two passes: first the name
    /// sets (so a reference can be classified), then the resolved types.
    pub fn build(loaded: &Loaded) -> Self {
        let mut table = TypeTable::default();
        table.handles.insert("String".to_owned());

        for module in loaded.modules.values() {
            for item in &module.items {
                match item {
                    Item::Struct(decl) => {
                        table.structs.entry(decl.name.clone()).or_default();
                    }
                    Item::Enum(decl) => {
                        table.enums.entry(decl.name.clone()).or_insert_with(|| EnumInfo {
                            backing: Ty::unknown(),
                            members: Vec::new(),
                        });
                    }
                    Item::Handle(decl) => {
                        table.handles.insert(decl.name.clone());
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
                                (field.name.clone(), resolve_ty(&field.ty, &params, &table))
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
                        let backing = resolve_ty(&decl.backing, &[], &table);
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
                                .map(|param| resolve_ty(&param.ty, &params, &table))
                                .collect(),
                            returns: resolve_ty(&decl.returns, &params, &table),
                            type_params: params,
                        };
                        if let Some(module_functions) = table.functions.get_mut(&module.name) {
                            module_functions.insert(decl.name.clone(), signature);
                        }
                    }
                    Item::GlobalLet(decl) => {
                        let ty = resolve_ty(&decl.ty, &[], &table);
                        if let Some(module_globals) = table.globals.get_mut(&module.name) {
                            module_globals.insert(decl.name.clone(), ty);
                        }
                    }
                    Item::GlobalConst(decl) => {
                        let ty = resolve_ty(&decl.ty, &[], &table);
                        if let Some(module_globals) = table.globals.get_mut(&module.name) {
                            module_globals.insert(decl.name.clone(), ty);
                        }
                    }
                    Item::Handle(_) | Item::Ffi(_) | Item::Intrinsic(_) => {}
                }
            }
        }
        table
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
            TyKind::Struct(name) => self.struct_fields(name).is_none_or(|fields| {
                fields
                    .iter()
                    .all(|(_, field)| self.duplicable_at(field, depth + 1))
            }),
            TyKind::Array { element, .. } => self.duplicable_at(element, depth + 1),
            _ => true,
        }
    }

    /// The storage size of a type in bytes, when it is known at this stage.
    ///
    /// The value is used by constant folding; a type whose size is not
    /// modelled yet yields `None` so the caller can defer.
    pub fn size_of(&self, ty: &Ty) -> Option<usize> {
        match &ty.kind {
            TyKind::Void => Some(0),
            TyKind::Scalar(scalar) => scalar_size(*scalar),
            TyKind::Raw(_) | TyKind::Func(_) => Some(4),
            TyKind::Str | TyKind::CString => Some(8),
            TyKind::Array { element, extents } => {
                let element = self.size_of(element)?;
                let mut total = element;
                for extent in extents {
                    total = total.checked_mul((*extent)?)?;
                }
                Some(total)
            }
            TyKind::Struct(name) => {
                let mut total = 0usize;
                for (_, field) in self.struct_fields(name)? {
                    total = total.checked_add(self.size_of(field)?)?;
                }
                Some(total)
            }
            _ => None,
        }
    }

    /// The byte offset of a struct field, when every earlier field's size is
    /// known.
    pub fn offset_of(&self, struct_name: &str, field: &str) -> Option<usize> {
        let mut offset = 0usize;
        for (name, ty) in self.struct_fields(struct_name)? {
            if name == field {
                return Some(offset);
            }
            offset = offset.checked_add(self.size_of(ty)?)?;
        }
        None
    }
}

/// The byte width of a scalar.
fn scalar_size(scalar: crate::ast::ScalarType) -> Option<usize> {
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
    let kind = match &ty.kind {
        TypeKind::Void => TyKind::Void,
        TypeKind::Scalar(scalar) => TyKind::Scalar(*scalar),
        TypeKind::Str => TyKind::Str,
        TypeKind::CString => TyKind::CString,
        TypeKind::StaticStr => TyKind::StaticStr,
        TypeKind::Raw(inner) => {
            TyKind::Raw(Box::new(resolve_ty_with(inner, type_params, table, fold)))
        }
        TypeKind::Array { element, extents } => TyKind::Array {
            element: Box::new(resolve_ty_with(element, type_params, table, fold)),
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
                .map(|param| resolve_ty_with(&param.ty, type_params, table, fold))
                .collect(),
            returns: Box::new(resolve_ty_with(returns, type_params, table, fold)),
        }),
        TypeKind::Named { name, .. } => resolve_named(name, type_params, table),
        TypeKind::Error => TyKind::Unknown,
    };
    Ty {
        kind,
        nullable: ty.nullable,
    }
}

/// Resolves a named type to its kind.
fn resolve_named(name: &QualifiedName, type_params: &[String], table: &TypeTable) -> TyKind {
    let Some(last) = name.segments.last() else {
        return TyKind::Unknown;
    };
    if name.segments.len() == 1 {
        if type_params.iter().any(|param| param == last) {
            return TyKind::Param(last.clone());
        }
        if last == "String" {
            return TyKind::Handle("String".to_owned());
        }
        if table.is_handle(last) {
            return TyKind::Handle(last.clone());
        }
        if table.is_struct(last) {
            return TyKind::Struct(last.clone());
        }
        if table.is_enum(last) {
            return TyKind::Enum(last.clone());
        }
    }
    TyKind::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::ScalarType;

    /// Inserts a struct with the given fields for a layout test.
    fn insert_struct(table: &mut TypeTable, name: &str, fields: Vec<(&str, Ty)>) {
        table.structs.insert(
            name.to_owned(),
            StructInfo {
                fields: fields
                    .into_iter()
                    .map(|(field, ty)| (field.to_owned(), ty))
                    .collect(),
                type_params: Vec::new(),
                overlay: false,
            },
        );
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
    fn sizes_and_offsets_fold_for_plain_layouts() {
        let mut table = TypeTable::default();
        insert_struct(
            &mut table,
            "Point",
            vec![("x", Ty::i32_ty()), ("y", Ty::i32_ty())],
        );
        assert_eq!(table.size_of(&Ty::struct_("Point")), Some(8));
        assert_eq!(table.offset_of("Point", "y"), Some(4));
        assert_eq!(table.offset_of("Point", "missing"), None);
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
        assert_eq!(table.offset_of("Mixed", "tag"), Some(0));
        assert_eq!(table.offset_of("Mixed", "count"), Some(1));
        assert_eq!(table.offset_of("Mixed", "value"), Some(3));
        assert_eq!(table.offset_of("Mixed", "scale"), Some(7));
        assert_eq!(table.offset_of("Mixed", "missing"), None);
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
        // A slice has an omitted extent, so its size is not known yet.
        assert_eq!(
            table.size_of(&Ty::array(scalar(ScalarType::U16), vec![None])),
            None
        );
        assert_eq!(table.size_of(&Ty::unknown()), None);
    }

    #[test]
    fn an_unmodelled_field_size_defers_the_whole_layout() {
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
        assert_eq!(table.size_of(&Ty::struct_("Flexible")), None);
        assert_eq!(table.offset_of("Flexible", "head"), Some(0));
        // The first field has a known offset; the one past the slice does not.
        assert_eq!(table.offset_of("Flexible", "tail"), None);
    }
}
