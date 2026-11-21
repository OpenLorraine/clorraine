//! Request-local member metadata for saved-source selection.
//!
//! Only written global types, struct fields and import identities are followed.
//! A complete declared field chain can avoid pulling a coincidental module into
//! the program. This is a selection policy, not compiler-wide name precedence:
//! unresolved expressions and all semantic diagnostics stay with the compiler.

use std::collections::BTreeMap;

use crate::ast::{Item, Program, QualifiedName, Type, TypeKind};
use crate::lsp::symbols::SymbolTable;
use crate::span::FileId;
use crate::types::{self, Ty, TyKind};

/// Borrowed declarations from one saved index, grouped by merged module.
pub(super) struct SourceMembers<'a> {
    /// Existing module/import identities; lexical visibility remains there too.
    table: &'a SymbolTable,
    /// The parsed declarations themselves, without cloning syntax or expressions.
    declarations: BTreeMap<String, BTreeMap<String, &'a Item>>,
}

impl<'a> SourceMembers<'a> {
    /// Indexes declarations once for the request; nothing is retained by the server.
    pub(super) fn new(programs: &'a [(FileId, Program)], table: &'a SymbolTable) -> Self {
        let mut declarations: BTreeMap<String, BTreeMap<String, &Item>> = BTreeMap::new();
        for (_, program) in programs {
            let entries = declarations.entry(program.module.name.to_dotted()).or_default();
            for item in &program.items {
                let name = match item {
                    Item::Func(d) | Item::Comptime(d) => &d.name,
                    Item::Struct(d) => &d.name,
                    Item::Enum(d) => &d.name,
                    Item::Handle(d) => &d.name,
                    Item::Ffi(d) => &d.name,
                    Item::Intrinsic(d) => &d.name,
                    Item::GlobalLet(d) => &d.name,
                    Item::GlobalConst(d) => &d.name,
                };
                // Duplicate declarations remain compiler diagnostics; selection
                // needs a stable representative, not another declaration checker.
                entries.entry(name.clone()).or_insert(item);
            }
        }
        Self { table, declarations }
    }

    /// True only when every member is established from a nonlocal declaration.
    ///
    /// A head's existence, its kind, or just its first field is insufficient.
    /// Generic substitution uses only explicit written arguments; no expression
    /// inference, overload resolution or constant evaluation is performed.
    pub(super) fn contains_chain(&self, module: &str, name: &QualifiedName) -> bool {
        let Some((head, tail)) = name.segments.split_first() else { return false };
        if tail.is_empty() {
            return false;
        }
        let Some((declaring_module, declaration)) = self.declaration(module, head) else {
            return false;
        };
        let written = match declaration {
            Item::GlobalLet(d) => &d.ty,
            Item::GlobalConst(d) => &d.ty,
            // Enum members and FFI signatures are declaration-scoped names,
            // not value fields; neither can start a further member chain.
            Item::Enum(d) => return tail.len() == 1 && d.members.iter().any(|m| m.name == tail[0]),
            Item::Ffi(d) => return tail.len() == 1 && d.decls.iter().any(|f| f.name == tail[0]),
            _ => return false,
        };
        let mut shape = self.written_type(declaring_module, written, &BTreeMap::new());
        for member in tail {
            let Some(next) = self.member_type(&shape, member) else { return false };
            shape = next;
        }
        true
    }

    /// Find a declaration directly, or through the module's existing import map.
    ///
    /// Imports belong to a merged module, so a sibling file's alias is visible.
    /// Importing is not re-exporting: the target must declare the original name.
    fn declaration(&self, module: &str, name: &str) -> Option<(&str, &'a Item)> {
        if let Some((key, items)) = self.declarations.get_key_value(module) {
            if let Some(item) = items.get(name) {
                return Some((key, *item));
            }
        }
        let import = self.table.imports.iter().find(|import| {
            import.local == name
                && self.table.module_for_file(import.file).is_some_and(|m| m.name == module)
        })?;
        self.direct_declaration(&import.module, &import.original)
    }

    /// Look up an exact declaring module/name without treating imports as exports.
    fn direct_declaration(&self, module: &str, name: &str) -> Option<(&str, &'a Item)> {
        let (key, items) = self.declarations.get_key_value(module)?;
        Some((key, *items.get(name)?))
    }

    /// Resolve a written type name in its declaration's module, including aliases.
    fn named_declaration(&self, module: &str, name: &QualifiedName) -> Option<(&str, &'a Item)> {
        let (last, head) = name.segments.split_last()?;
        if head.is_empty() {
            self.declaration(module, last)
        } else {
            self.direct_declaration(&head.join("."), last)
        }
    }

    /// Keep just the declared shapes needed by the compiler's shared member table.
    ///
    /// Struct identities are module-qualified here so unrelated modules cannot
    /// lend each other fields. Array sizes are deliberately not evaluated: a
    /// present extent stays fixed and an omitted extent stays a slice. Functions
    /// and other terminal types have no further fields, so unknown is sufficient.
    fn written_type(&self, module: &str, ty: &Type, bindings: &BTreeMap<String, Ty>) -> Ty {
        let mut shape = match &ty.kind {
            TypeKind::Scalar(scalar) => Ty::scalar(*scalar),
            TypeKind::Str => Ty { kind: TyKind::Str, nullable: false },
            TypeKind::StaticStr => Ty { kind: TyKind::StaticStr, nullable: false },
            TypeKind::Raw(inner) => Ty::raw(self.written_type(module, inner, bindings)),
            TypeKind::Array { element, extents } => Ty::array(
                self.written_type(module, element, bindings),
                extents.iter().map(|extent| extent.size.as_ref().map(|_| 0)).collect(),
            ),
            TypeKind::Named { name, args } => {
                if name.segments.len() == 1 && args.is_empty() {
                    if let Some(bound) = bindings.get(&name.segments[0]) {
                        return bound.clone();
                    }
                }
                let Some((declaring_module, Item::Struct(declaration))) =
                    self.named_declaration(module, name) else { return Ty::unknown() };
                if args.len() != declaration.type_params.len() {
                    return Ty::unknown();
                }
                Ty::struct_with(
                    format!("{declaring_module}.{}", declaration.name),
                    args.iter().map(|arg| self.written_type(module, arg, bindings)).collect(),
                )
            }
            TypeKind::Void | TypeKind::CString | TypeKind::Function { .. }
            | TypeKind::Error => Ty::unknown(),
        };
        shape.nullable = ty.nullable;
        shape
    }

    /// Follow one actual field, reusing the compiler's view-member shape rules.
    ///
    /// A raw receiver implies one dereference, not an arbitrary pointer chase.
    /// Struct recursion advances exactly one written path segment each time;
    /// recursive declarations therefore need no recursive type expansion.
    fn member_type(&self, base: &Ty, member: &str) -> Option<Ty> {
        let base = match &base.kind {
            TyKind::Raw(inner) => inner.as_ref(),
            _ => base,
        };
        if let TyKind::Struct(name, args) = &base.kind {
            let (module, name) = name.rsplit_once('.')?;
            let (module, Item::Struct(declaration)) = self.direct_declaration(module, name)? else {
                return None;
            };
            let field = declaration.fields.iter().find(|field| field.name == member)?;
            let bindings = declaration.type_params.iter().zip(args)
                .map(|(param, arg)| (param.name.clone(), arg.clone())).collect();
            return Some(self.written_type(module, &field.ty, &bindings));
        }
        types::view_member_ty(base, member)
    }
}
