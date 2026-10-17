//! Module-global initialization dependencies.
//!
//! The compiler needs one dependency relation in two places: the semantic stage
//! rejects an initializer cycle, and code generation orders the module
//! initializers and the globals inside one module. Both read the same facts, so
//! they are built here in one place:
//!
//! * the module-level bindings an initializer expression reads directly, and
//! * the modules an initializer expression names.
//!
//! A qualified reference resolves the same way the name resolver does. It may
//! name a same-module binding with field projections (`config.limit`), an
//! imported binding with field projections, or a binding through a full module
//! path (`test.Beta.ready`). Because the resolution lives here, the cycle check
//! and the emitter can never disagree about what an initializer depends on.

use std::collections::{BTreeMap, BTreeSet};

use crate::ast::{Expr, ExprKind, IntrinsicArg, Item, QualifiedName, TemplatePiece};
use crate::load::{DeclKind, Loaded, Module};
use crate::span::Span;

/// A module-level binding, keyed by its dotted module name and simple name.
pub type GlobalKey = (String, String);

/// A resolved reference to a module-level binding.
pub struct GlobalRef {
    /// Dotted module the binding is declared in.
    pub module: String,
    /// Simple name of the binding.
    pub name: String,
    /// Field path applied after the binding, when the head was a value binding.
    ///
    /// A same-module or imported head keeps its trailing segments as fields
    /// (`config.limit`); a full module path resolves its last segment as the
    /// binding and leaves this empty (`test.Beta.ready`).
    pub fields: Vec<String>,
}

/// Resolves a qualified name to the module-level binding it reads, if any.
///
/// The lookup follows the name resolver: a full module path wins first, then a
/// same-module binding or an imported binding whose trailing segments are
/// fields. `None` means the reference names no module-level binding, so it is
/// not an initialization dependency.
pub fn resolve_global(loaded: &Loaded, module: &Module, name: &QualifiedName) -> Option<GlobalRef> {
    resolve_segments(loaded, module, &name.segments)
}

/// Resolves a bare name to the module-level binding it reads, if any.
///
/// A bare name has no field path, so the returned reference is either a
/// same-module declaration or the target of an import binding.
pub fn resolve_name_global(loaded: &Loaded, module: &Module, name: &str) -> Option<GlobalRef> {
    let segments = [name.to_owned()];
    resolve_segments(loaded, module, &segments)
}

/// The direct module-level bindings an initializer expression reads.
///
/// Only the reads written in the expression count: a call to a function that
/// happens to read a global is not an edge, because the function body runs
/// later. The returned keys include bindings in other modules; a caller that
/// needs one module's internal order filters on the module name.
pub fn global_reads(loaded: &Loaded, module: &Module, expr: &Expr) -> Vec<GlobalKey> {
    let mut collector = Collector::new(loaded, module);
    collector.walk(expr);
    collector.reads
}

/// The modules an initializer expression names.
///
/// An imported name names its module, and a dotted name whose head is a module
/// path names that module. A same-module declaration names no module.
pub fn named_modules(loaded: &Loaded, module: &Module, expr: &Expr) -> BTreeSet<String> {
    let mut collector = Collector::new(loaded, module);
    collector.walk(expr);
    collector.modules
}

/// The initialization dependency relation over the loaded modules.
///
/// A module depends on every module it imports and every module its `let` or
/// `const` initializer expressions name, which is the order the specification
/// assigns to module initialization. Only loaded modules appear; a dependency
/// on a declaration-only surface cannot be initialized and is filtered out.
pub fn module_dependencies(loaded: &Loaded) -> BTreeMap<String, BTreeSet<String>> {
    let mut deps = BTreeMap::new();
    for (name, module) in &loaded.modules {
        let mut prereqs: BTreeSet<String> = BTreeSet::new();
        for import in &module.imports {
            if &import.module != name && loaded.modules.contains_key(&import.module) {
                prereqs.insert(import.module.clone());
            }
        }
        for item in &module.items {
            let init = match item {
                Item::GlobalLet(decl) => Some(&decl.value),
                Item::GlobalConst(decl) => Some(&decl.value),
                _ => None,
            };
            if let Some(init) = init {
                for target in named_modules(loaded, module, init) {
                    if &target != name && loaded.modules.contains_key(&target) {
                        prereqs.insert(target);
                    }
                }
            }
        }
        deps.insert(name.clone(), prereqs);
    }
    deps
}

/// True when a module has at least one runtime-initialized global (`let`).
///
/// A module dependency cycle is only an error when it involves runtime work; a
/// cycle of `const`-only modules is allowed.
pub fn has_runtime_global(loaded: &Loaded, module: &str) -> bool {
    loaded
        .module(module)
        .is_some_and(|node| node.items.iter().any(|item| matches!(item, Item::GlobalLet(_))))
}

/// The span of a module's first runtime global initializer, when it has one.
///
/// Used as the location of a module-level cycle diagnostic; a cycle is only
/// reported when at least one of its modules has a runtime global.
pub fn runtime_span(loaded: &Loaded, module: &str) -> Option<Span> {
    let node = loaded.module(module)?;
    node.items.iter().find_map(|item| match item {
        Item::GlobalLet(decl) => Some(decl.value.span),
        _ => None,
    })
}

/// Resolves a dotted segment list the way the name resolver does.
fn resolve_segments(loaded: &Loaded, module: &Module, segments: &[String]) -> Option<GlobalRef> {
    // A module path can reach a binding with fields after it: the leading
    // segments spell a loaded module (or the reading module itself) and the
    // next segment names a binding the module declares and exports, so the
    // segments after the binding are field projections. The longest module
    // prefix wins, matching the renderer's binding-path policy.
    for split in (1..segments.len()).rev() {
        let path = segments[..split].join(".");
        if path.is_empty() {
            continue;
        }
        let binding = &segments[split];
        if path != module.name
            && !(loaded.has_module(&path) && loaded.module_exports(&path, binding))
        {
            continue;
        }
        if let Some((found, name)) = binding_in_module(loaded, &path, binding) {
            return Some(GlobalRef {
                module: found,
                name,
                fields: segments[split + 1..].to_vec(),
            });
        }
    }
    // A member read of a same-module binding or an imported binding. The head
    // is the binding and every later segment is a field projection.
    let first = segments.first()?;
    let fields: Vec<String> = segments[1..].to_vec();
    if let Some((module, name)) = binding_in_module(loaded, &module.name, first) {
        return Some(GlobalRef {
            module,
            name,
            fields,
        });
    }
    let binding = module.import(first)?;
    let (module, name) = binding_in_module(loaded, &binding.module, &binding.original)?;
    Some(GlobalRef {
        module,
        name,
        fields,
    })
}

/// The key of `name` when the module declares it as a `let` or `const`.
fn binding_in_module(loaded: &Loaded, module: &str, name: &str) -> Option<GlobalKey> {
    let node = loaded.module(module)?;
    match node.decl(name) {
        Some(decl) if matches!(decl.kind, DeclKind::Global | DeclKind::Const) => {
            Some((module.to_owned(), name.to_owned()))
        }
        _ => None,
    }
}

/// Walks an expression, collecting the bindings it reads and the modules it
/// names.
struct Collector<'a> {
    /// The loaded program, for module-aware resolution.
    loaded: &'a Loaded,
    /// The module the expression belongs to.
    module: &'a Module,
    /// The bindings read directly by the expression.
    reads: Vec<GlobalKey>,
    /// The other modules the expression names.
    modules: BTreeSet<String>,
}

impl<'a> Collector<'a> {
    /// Creates a collector for one module's expression.
    fn new(loaded: &'a Loaded, module: &'a Module) -> Self {
        Self {
            loaded,
            module,
            reads: Vec::new(),
            modules: BTreeSet::new(),
        }
    }

    /// Records the module a reference names, when it is not the current one.
    fn note_module(&mut self, module: &str) {
        if module != self.module.name {
            self.modules.insert(module.to_owned());
        }
    }

    /// Walks one expression and its subexpressions.
    fn walk(&mut self, expr: &Expr) {
        match &expr.kind {
            ExprKind::Name(name) => {
                if let Some(global) = resolve_name_global(self.loaded, self.module, name) {
                    self.reads.push((global.module, global.name));
                } else if let Some(binding) = self.module.import(name) {
                    self.note_module(&binding.module);
                }
            }
            ExprKind::Qualified(name) => self.walk_qualified(name),
            ExprKind::Call { callee, args, .. } => {
                self.walk(callee);
                for arg in args {
                    self.walk(&arg.value);
                }
            }
            ExprKind::Index { base, index } => {
                self.walk(base);
                self.walk(index);
            }
            ExprKind::Member { base, .. } => self.walk(base),
            ExprKind::Unary { operand, .. } => self.walk(operand),
            ExprKind::Binary { left, right, .. } => {
                self.walk(left);
                self.walk(right);
            }
            ExprKind::Assign { target, value, .. } => {
                self.walk(target);
                self.walk(value);
            }
            ExprKind::Conditional {
                condition,
                then_value,
                else_value,
            } => {
                self.walk(condition);
                self.walk(then_value);
                self.walk(else_value);
            }
            ExprKind::Cast { value, .. } | ExprKind::RawCast { value, .. } => self.walk(value),
            ExprKind::StructLiteral { fields, .. } => {
                for field in fields {
                    self.walk(&field.value);
                }
            }
            ExprKind::ArrayLiteral { elements, fill } => {
                for element in elements {
                    self.walk(element);
                }
                if let Some(fill) = fill {
                    self.walk(fill);
                }
            }
            ExprKind::Unsafe(inner) => self.walk(inner),
            ExprKind::IntrinsicCall { args, .. } => {
                for arg in args {
                    if let IntrinsicArg::Expr(inner) = arg {
                        self.walk(inner);
                    }
                }
            }
            ExprKind::Template(pieces) => {
                for piece in pieces {
                    if let TemplatePiece::Placeholder(inner) = piece {
                        self.walk(inner);
                    }
                }
            }
            _ => {}
        }
    }

    /// Walks a dotted name, resolving its module path or value binding head.
    ///
    /// The resolution is the front end's: a leading module path — at any
    /// depth — reaches the binding it declares, and otherwise the head is a
    /// same-module or imported binding. The read and its declaring module
    /// enter the dependency relation either way, so a deep reference orders
    /// its initializer after the binding it reads.
    fn walk_qualified(&mut self, name: &QualifiedName) {
        let Some(global) = resolve_segments(self.loaded, self.module, &name.segments) else {
            return;
        };
        self.reads.push((global.module.clone(), global.name.clone()));
        self.note_module(&global.module);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diag::Diagnostics;
    use crate::parse::Parser;
    use crate::span::SourceMap;

    /// Parses and loads the given files into one program.
    fn load(files: &[&str]) -> Loaded {
        let mut sources = SourceMap::new();
        let mut diagnostics = Diagnostics::new(false, None);
        let mut programs = Vec::new();
        for (index, text) in files.iter().enumerate() {
            let id = sources.add_file(format!("file{index}.clo"), (*text).to_owned());
            let program = Parser::new(&sources, id, &mut diagnostics).parse_program();
            programs.push((id, program));
        }
        crate::load::load(programs, &mut diagnostics)
    }

    /// The initializer expression of a module-level `let`.
    fn let_init<'a>(module: &'a Module, name: &str) -> &'a Expr {
        module
            .items
            .iter()
            .find_map(|item| match item {
                Item::GlobalLet(decl) if decl.name == name => Some(&decl.value),
                _ => None,
            })
            .expect("the global initializer exists")
    }

    #[test]
    fn a_full_module_path_read_resolves_to_its_global() {
        let loaded = load(&[
            "module test.PA;\nexport let a: i32 = test.PB.y + 1;\n",
            "module test.PB;\nexport let y: i32 = 41;\n",
        ]);
        let pa = loaded.module("test.PA").expect("module exists");
        let value = let_init(pa, "a");
        assert_eq!(
            global_reads(&loaded, pa, value),
            vec![("test.PB".to_owned(), "y".to_owned())]
        );
        assert!(named_modules(&loaded, pa, value).contains("test.PB"));
    }

    #[test]
    fn a_field_read_of_a_same_module_global_names_no_module() {
        let loaded = load(&[
            "module test;\nstruct Box { v: i32; }\nlet b: Box = Box { v: 1 };\nlet a: i32 = b.v + 1;\n",
        ]);
        let module = loaded.module("test").expect("module exists");
        let value = let_init(module, "a");
        assert_eq!(
            global_reads(&loaded, module, value),
            vec![("test".to_owned(), "b".to_owned())]
        );
        assert!(named_modules(&loaded, module, value).is_empty());
    }

    #[test]
    fn module_dependencies_include_imports_and_named_modules() {
        let loaded = load(&[
            "module test.Alpha;\nimport { helper } from test.Beta;\nexport let a: i32 = test.Gamma.z;\n",
            "module test.Beta;\nexport func helper(): i32 { return 1; }\n",
            "module test.Gamma;\nexport let z: i32 = 2;\n",
        ]);
        let deps = module_dependencies(&loaded);
        let alpha = deps.get("test.Alpha").expect("module exists");
        assert!(alpha.contains("test.Beta"), "imports are dependencies");
        assert!(alpha.contains("test.Gamma"), "named modules are dependencies");
        assert!(deps.get("test.Beta").expect("module").is_empty());
    }

    #[test]
    fn a_runtime_global_is_distinguished_from_a_const() {
        let loaded = load(&[
            "module test.Alpha;\nlet x: i32 = 1;\n",
            "module test.Beta;\nconst y: i32 = 2;\n",
        ]);
        assert!(has_runtime_global(&loaded, "test.Alpha"));
        assert!(!has_runtime_global(&loaded, "test.Beta"));
        assert!(runtime_span(&loaded, "test.Alpha").is_some());
        assert!(runtime_span(&loaded, "test.Beta").is_none());
    }
}

