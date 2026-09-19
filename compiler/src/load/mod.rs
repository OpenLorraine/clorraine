//! Loading and module identity.
//!
//! The command-line file list is the whole program. Each parsed file belongs to
//! the module named by its `module` declaration, and files that declare the
//! same name merge into one module, so a name declared twice in the merged
//! module is an error. This stage builds the module table, binds imports
//! against user and standard-library exports, warns about unused imports, and
//! checks the executable entry point.
//!
//! Name resolution proper — lexical scopes and reference checking — is the next
//! stage; this one only decides which modules and module-level names exist.

use std::collections::{BTreeMap, BTreeSet};

use crate::ast::*;
use crate::diag::{Diagnostic, Diagnostics};
use crate::span::{FileId, Span};
use crate::stdlib;

mod scan;

use scan::collect_name_mentions;

/// What a module-level name declares.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeclKind {
    /// A function with a body.
    Func,
    /// A struct.
    Struct,
    /// An enum.
    Enum,
    /// A handle.
    Handle,
    /// A mutable module-level `let`.
    Global,
    /// An immutable module-level `const`.
    Const,
    /// An `intrinsic func` declaration.
    Intrinsic,
    /// An `ffi` block.
    Ffi,
}

/// One module-level declaration.
pub struct Decl {
    /// What the name declares.
    pub kind: DeclKind,
    /// Declared name.
    pub name: String,
    /// Span of the name.
    pub name_span: Span,
    /// True when the declaration carries `export`.
    pub exported: bool,
}

/// One import binding visible in module scope.
pub struct ImportBinding {
    /// Name bound in this module: the alias when one is written, otherwise the
    /// imported name.
    pub local: String,
    /// Name requested from the target module.
    pub original: String,
    /// Dotted name of the module the import reads from.
    pub module: String,
    /// Span of the binding.
    pub span: Span,
    /// True when the target module or name was missing. The binding stays in
    /// scope so later uses do not cascade into undeclared-name errors.
    pub poisoned: bool,
}

/// One module: every file that declared the same dotted name, merged.
pub struct Module {
    /// Dotted module name.
    pub name: String,
    /// Contributing files in command-line order.
    pub files: Vec<FileId>,
    /// Import bindings in source order.
    pub imports: Vec<ImportBinding>,
    /// Module-level declarations keyed by name.
    pub decls: BTreeMap<String, Decl>,
    /// Declarations in source order, merged across the contributing files.
    pub items: Vec<Item>,
}

impl Module {
    /// Looks up a module-level declaration by name.
    pub fn decl(&self, name: &str) -> Option<&Decl> {
        self.decls.get(name)
    }

    /// Looks up an import binding by its local name.
    pub fn import(&self, name: &str) -> Option<&ImportBinding> {
        self.imports.iter().find(|binding| binding.local == name)
    }
}

/// The loaded program: every module the program declares, keyed by dotted name.
pub struct Loaded {
    /// Modules in lexicographic order.
    pub modules: BTreeMap<String, Module>,
}

impl Loaded {
    /// Looks up a module by its dotted name.
    pub fn module(&self, name: &str) -> Option<&Module> {
        self.modules.get(name)
    }

    /// True when the program or the standard library provides the module.
    pub fn has_module(&self, name: &str) -> bool {
        self.modules.contains_key(name) || stdlib::lookup(name).is_some()
    }

    /// True when the module exports the name.
    pub fn module_exports(&self, module: &str, name: &str) -> bool {
        match self.modules.get(module) {
            Some(user) => user.decls.get(name).is_some_and(|decl| decl.exported),
            None => stdlib::exports(module, name),
        }
    }
}

/// The merged files of one module before its declarations are collected.
#[derive(Default)]
struct Group {
    /// Contributing files in command-line order.
    files: Vec<FileId>,
    /// Import statements from every contributing file.
    imports: Vec<ImportDecl>,
    /// Declarations from every contributing file.
    items: Vec<Item>,
}

/// Builds the module table from the parsed files.
pub fn load(programs: Vec<(FileId, Program)>, diagnostics: &mut Diagnostics) -> Loaded {
    let mut groups: BTreeMap<String, Group> = BTreeMap::new();
    for (file, program) in programs {
        let Some(name) = file_module(file, &program, diagnostics) else {
            continue;
        };
        let group = groups.entry(name).or_default();
        group.files.push(file);
        group.imports.extend(program.imports);
        group.items.extend(program.items);
    }

    let mut decls_by_module: BTreeMap<String, BTreeMap<String, Decl>> = BTreeMap::new();
    let mut exports: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (name, group) in &groups {
        let decls = collect_declarations(name, &group.items, diagnostics);
        let exported = decls
            .values()
            .filter(|decl| decl.exported)
            .map(|decl| decl.name.clone())
            .collect();
        decls_by_module.insert(name.clone(), decls);
        exports.insert(name.clone(), exported);
    }

    let mut modules: BTreeMap<String, Module> = BTreeMap::new();
    for (name, group) in groups {
        let imports = resolve_imports(&group, &exports, diagnostics);
        let decls = decls_by_module.remove(&name).unwrap_or_default();
        modules.insert(
            name.clone(),
            Module {
                name,
                files: group.files,
                imports,
                decls,
                items: group.items,
            },
        );
    }

    let loaded = Loaded { modules };
    if diagnostics.error_count() == 0 {
        // A dropped declaration may have held the entry point, and the entry
        // point is dependent on the module structure being sound, so an
        // already-reported error suppresses this check.
        check_entry_point(&loaded, diagnostics);
    }
    loaded
}

/// Determines the module a file belongs to, reporting a missing or misplaced
/// declaration.
///
/// A missing declaration yields no module and the file's items are dropped: a
/// file without identity cannot contribute trustworthy names. A declaration
/// that is not first is reported but still names the module, so the rest of the
/// file stays usable.
fn file_module(file: FileId, program: &Program, diagnostics: &mut Diagnostics) -> Option<String> {
    let mut name = program.module.name.to_dotted();
    if let Some(misplaced) = &program.misplaced_module {
        diagnostics.push(
            Diagnostic::error(
                "E11105",
                "'module' must be the first declaration in the file",
                misplaced.span,
            )
            .with_help("move the module declaration to the top"),
        );
        if name.is_empty() {
            name = misplaced.name.to_dotted();
        }
    }
    if name.is_empty() {
        diagnostics.push(
            Diagnostic::error(
                "E11105",
                "'module' must be the first declaration in the file",
                Span::new(file, 0, 0),
            )
            .with_help("move the module declaration to the top"),
        );
        return None;
    }
    Some(name)
}

/// Describes the module-level declarations one top-level item introduces.
///
/// An `ffi` block introduces its own name and one declaration per signature
/// inside it: the C functions resolve globally within the module, so a
/// `handle T freedBy F` may name a deleter declared in any block, in any order.
fn declarations_of(item: &Item) -> Vec<Decl> {
    let (kind, name, name_span, exported) = match item {
        Item::Func(decl) => (DeclKind::Func, &decl.name, decl.name_span, decl.exported),
        Item::Comptime(decl) => (DeclKind::Func, &decl.name, decl.name_span, decl.exported),
        Item::Struct(decl) => (DeclKind::Struct, &decl.name, decl.name_span, decl.exported),
        Item::Enum(decl) => (DeclKind::Enum, &decl.name, decl.name_span, decl.exported),
        Item::Handle(decl) => (DeclKind::Handle, &decl.name, decl.name_span, decl.exported),
        Item::Ffi(block) => {
            let mut decls = vec![Decl {
                kind: DeclKind::Ffi,
                name: block.name.clone(),
                name_span: block.name_span,
                exported: false,
            }];
            for signature in &block.decls {
                decls.push(Decl {
                    kind: DeclKind::Ffi,
                    name: signature.name.clone(),
                    name_span: signature.name_span,
                    exported: false,
                });
            }
            return decls;
        }
        Item::Intrinsic(signature) => {
            (DeclKind::Intrinsic, &signature.name, signature.name_span, false)
        }
        Item::GlobalLet(decl) => (DeclKind::Global, &decl.name, decl.name_span, decl.exported),
        Item::GlobalConst(decl) => (DeclKind::Const, &decl.name, decl.name_span, decl.exported),
    };
    vec![Decl {
        kind,
        name: name.clone(),
        name_span,
        exported,
    }]
}

/// Collects a module's declarations and reports duplicate names.
///
/// Files that declare the same module share one namespace, so the second
/// declaration of a name is a duplicate regardless of which file holds it.
fn collect_declarations(
    module: &str,
    items: &[Item],
    diagnostics: &mut Diagnostics,
) -> BTreeMap<String, Decl> {
    let mut decls: BTreeMap<String, Decl> = BTreeMap::new();
    for decl in items.iter().flat_map(declarations_of) {
        if decls.contains_key(&decl.name) {
            diagnostics.push(
                Diagnostic::error(
                    "E11101",
                    format!("duplicate declaration of '{}' in module '{}'", decl.name, module),
                    decl.name_span,
                )
                .with_help("remove one declaration; declaration merging forbids duplicates"),
            );
            continue;
        }
        decls.insert(decl.name.clone(), decl);
    }
    decls
}

/// Binds a module's imports and reports unresolved, duplicate, and unused ones.
fn resolve_imports(
    group: &Group,
    exports: &BTreeMap<String, BTreeSet<String>>,
    diagnostics: &mut Diagnostics,
) -> Vec<ImportBinding> {
    let used = collect_name_mentions(&group.items);
    let mut bindings: Vec<ImportBinding> = Vec::new();
    for import in &group.imports {
        let target = import.from.to_dotted();
        for item in &import.items {
            let local = item.alias.clone().unwrap_or_else(|| item.name.clone());
            let resolved = exports
                .get(&target)
                .is_some_and(|names| names.contains(&item.name))
                || stdlib::exports(&target, &item.name);
            if !resolved {
                diagnostics.push(
                    Diagnostic::error(
                        "E11102",
                        format!("'{}' is not exported from module '{}'", item.name, target),
                        item.name_span,
                    )
                    .with_help("check the name or add `export`"),
                );
            }
            let duplicated = bindings.iter().any(|binding| binding.local == local);
            if duplicated {
                diagnostics.push(
                    Diagnostic::error(
                        "E11103",
                        format!("duplicate import of '{local}'"),
                        item.name_span,
                    )
                    .with_help("use `as` to bind a new name"),
                );
            }
            if resolved && !duplicated && !used.contains(&local) {
                diagnostics.push(Diagnostic::warning(
                    "W10006",
                    format!("unused import '{local}'"),
                    item.span,
                ));
            }
            bindings.push(ImportBinding {
                local,
                original: item.name.clone(),
                module: target.clone(),
                span: item.span,
                poisoned: !resolved,
            });
        }
    }
    bindings
}

/// Checks that the program has exactly one well-typed entry point.
fn check_entry_point(loaded: &Loaded, diagnostics: &mut Diagnostics) {
    let mains: Vec<&FuncDecl> = loaded
        .modules
        .values()
        .flat_map(|module| module.items.iter())
        .filter_map(|item| match item {
            Item::Func(decl) if decl.name == "main" => Some(decl),
            _ => None,
        })
        .collect();
    let valid = mains.len() == 1
        && mains[0].params.is_empty()
        && matches!(&mains[0].returns.kind, TypeKind::Scalar(ScalarType::I32))
        && !mains[0].returns.nullable;
    if valid {
        return;
    }
    let span = mains.first().map(|decl| decl.name_span).or_else(|| {
        loaded
            .modules
            .values()
            .next()
            .and_then(|module| module.files.first().map(|file| Span::new(*file, 0, 0)))
    });
    let Some(span) = span else {
        return;
    };
    diagnostics.push(
        Diagnostic::error(
            "E11104",
            "executable builds need exactly one 'func main(): i32'",
            span,
        )
        .with_help("declare or fix `main`"),
    );
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::diag::Severity;
    use crate::parse::Parser;
    use crate::span::SourceMap;

    /// Parses and loads the given file contents, returning the loaded program
    /// and the diagnostic codes in report order.
    fn compile(files: &[&str]) -> (Loaded, Vec<(&'static str, Severity)>) {
        let mut sources = SourceMap::new();
        let mut diagnostics = Diagnostics::new(false, None);
        let mut programs = Vec::new();
        for (index, text) in files.iter().enumerate() {
            let id = sources.add_file(format!("file{index}.clo"), (*text).to_owned());
            let program = Parser::new(&sources, id, &mut diagnostics).parse_program();
            programs.push((id, program));
        }
        let loaded = load(programs, &mut diagnostics);
        let codes = diagnostics
            .items()
            .iter()
            .map(|diagnostic| (diagnostic.code, diagnostic.severity))
            .collect();
        (loaded, codes)
    }

    /// Codes of the errors in a diagnostic list.
    fn errors(codes: &[(&'static str, Severity)]) -> Vec<&'static str> {
        codes
            .iter()
            .filter(|(_, severity)| *severity == Severity::Error)
            .map(|(code, _)| *code)
            .collect()
    }

    #[test]
    fn merged_files_share_one_module_namespace() {
        let (loaded, codes) = compile(&[
            "module app.core;\nfunc f(): i32 { return 1; }\n",
            "module app.core;\nfunc f(): i32 { return 2; }\nfunc main(): i32 { return f(); }\n",
        ]);
        assert_eq!(errors(&codes), ["E11101"]);
        let module = loaded.module("app.core").expect("module exists");
        assert_eq!(module.files.len(), 2);
        assert_eq!(module.decl("f").map(|decl| decl.kind), Some(DeclKind::Func));
    }

    #[test]
    fn missing_and_misplaced_module_declarations_report_e11105() {
        let (_, missing) = compile(&["func main(): i32 { return 0; }\n"]);
        assert_eq!(errors(&missing), ["E11105"]);

        let (loaded, misplaced) = compile(&[
            "func helper(): i32 { return 1; }\nmodule app.late;\nfunc main(): i32 { return 0; }\n",
        ]);
        assert!(errors(&misplaced).contains(&"E11105"));
        assert!(loaded.module("app.late").is_some());
    }

    #[test]
    fn imports_bind_user_and_standard_library_exports() {
        let (loaded, codes) = compile(&[
            "module dep;\nexport func helper(): i32 { return 1; }\n",
            "module app;\nimport { helper } from dep;\nfunc main(): i32 { return helper(); }\n",
        ]);
        assert!(codes.is_empty(), "unexpected diagnostics: {codes:?}");
        let binding = loaded
            .module("app")
            .and_then(|module| module.import("helper"))
            .expect("binding exists");
        assert!(!binding.poisoned);
        assert_eq!(binding.original, "helper");

        let (loaded, codes) = compile(&[
            "module app;\nimport { Buffer } from std.mem;\nfunc accept(b: Buffer): void {}\nfunc main(): i32 { return 0; }\n",
        ]);
        assert!(codes.is_empty(), "unexpected diagnostics: {codes:?}");
        assert!(loaded.module_exports("std.mem", "Buffer"));
    }

    #[test]
    fn unresolved_and_duplicate_imports_report_their_codes() {
        let (_, missing) = compile(&[
            "module app;\nimport { nope } from dep;\nfunc main(): i32 { return 0; }\n",
        ]);
        assert_eq!(errors(&missing), ["E11102"]);

        let (_, duplicate) = compile(&[
            "module dep;\nexport func helper(): i32 { return 1; }\n",
            "module app;\nimport { helper } from dep;\nimport { helper } from dep;\nfunc main(): i32 { return helper(); }\n",
        ]);
        assert!(errors(&duplicate).contains(&"E11103"));
    }

    #[test]
    fn unused_and_used_imports_are_distinguished() {
        let (_, unused) = compile(&[
            "module app;\nimport { Buffer } from std.mem;\nfunc main(): i32 { return 0; }\n",
        ]);
        assert_eq!(unused, [("W10006", Severity::Warning)]);

        let (_, used) = compile(&[
            "module app;\nimport { Buffer } from std.mem;\nfunc accept(b: Buffer): void {}\nfunc main(): i32 { return 0; }\n",
        ]);
        assert!(used.is_empty(), "unexpected diagnostics: {used:?}");
    }

    #[test]
    fn qualified_access_does_not_count_as_an_import_use() {
        let (_, codes) = compile(&[
            "module app;\nimport { Buffer } from std.mem;\nfunc accept(b: std.mem.Buffer): void {}\nfunc main(): i32 { return 0; }\n",
        ]);
        assert_eq!(codes, [("W10006", Severity::Warning)]);
    }

    #[test]
    fn entry_point_requires_exactly_one_typed_main() {
        let (_, good) = compile(&["module app;\nfunc main(): i32 { return 0; }\n"]);
        assert!(good.is_empty(), "unexpected diagnostics: {good:?}");

        let (_, mistyped) = compile(&["module app;\nfunc main(): void {}\n"]);
        assert_eq!(errors(&mistyped), ["E11104"]);

        let (_, parameterized) =
            compile(&["module app;\nfunc main(argc: i32): i32 { return argc; }\n"]);
        assert_eq!(errors(&parameterized), ["E11104"]);

        let (_, two_mains) = compile(&[
            "module a;\nfunc main(): i32 { return 0; }\n",
            "module b;\nfunc main(): i32 { return 0; }\n",
        ]);
        assert_eq!(errors(&two_mains), ["E11104"]);
    }

    #[test]
    fn ffi_signatures_are_module_declarations() {
        let (_, codes) = compile(&[r#"
module app;

ffi lib {
    func CloseFile(value: i32): void;
}

func CloseFile(value: i32): void {}

func main(): i32 {
    return 0;
}
"#]);
        assert_eq!(errors(&codes), ["E11101"]);
    }

    #[test]
    fn entry_point_check_is_suppressed_by_earlier_errors() {
        let (_, codes) = compile(&[
            "func main(): i32 { return 0; }\n",
            "module dep;\nfunc other(): i32 { return 1; }\n",
        ]);
        assert_eq!(errors(&codes), ["E11105"]);
    }
}
