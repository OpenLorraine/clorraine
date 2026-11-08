//! Declaration and local-name tables for the language-server features.
//!
//! The table is built from the parsed programs of one analysis set. It records
//! the exact name span, the full declaration span, and the module of every
//! top-level item, plus parameters and local bindings. Features use it instead
//! of re-walking the AST for each request.
//!
//! It is deliberately not a second resolver: it only answers "which declaration
//! is written here" and "which names are visible". Type checking stays in the
//! compiler.

use crate::ast::{Block, ElseBranch, ForInit, Item, Program, Stmt};
use crate::span::{FileId, SourceMap, Span};

/// What a symbol declares.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SymbolKind {
    /// A function with a body.
    Function,
    /// A `comptime` function.
    Comptime,
    /// A struct.
    Struct,
    /// An enum.
    Enum,
    /// An opaque handle.
    Handle,
    /// An `ffi` block.
    Ffi,
    /// A module-level constant.
    Constant,
    /// A module-level mutable binding.
    Global,
    /// A function parameter.
    Parameter,
    /// A local `let`, `const`, or loop binding.
    Local,
}

impl SymbolKind {
    /// The LSP `SymbolKind` number.
    pub fn lsp(self) -> i64 {
        match self {
            SymbolKind::Function | SymbolKind::Comptime => 12,
            SymbolKind::Struct => 23,
            SymbolKind::Enum => 10,
            SymbolKind::Handle => 5,
            SymbolKind::Ffi => 3,
            SymbolKind::Constant => 14,
            SymbolKind::Global => 13,
            SymbolKind::Parameter => 13,
            SymbolKind::Local => 13,
        }
    }

    /// A short human label used by hover.
    pub fn label(self) -> &'static str {
        match self {
            SymbolKind::Function => "function",
            SymbolKind::Comptime => "comptime function",
            SymbolKind::Struct => "struct",
            SymbolKind::Enum => "enum",
            SymbolKind::Handle => "handle",
            SymbolKind::Ffi => "ffi block",
            SymbolKind::Constant => "constant",
            SymbolKind::Global => "global",
            SymbolKind::Parameter => "parameter",
            SymbolKind::Local => "local",
        }
    }
}

/// One declaration or binding.
#[derive(Clone, Debug)]
pub struct Symbol {
    /// Exact identifier.
    pub name: String,
    /// What it declares.
    pub kind: SymbolKind,
    /// Span of the identifier.
    pub name_span: Span,
    /// Span of the whole declaration or statement.
    pub full_span: Span,
    /// Byte offset where the signature ends (body brace or full end).
    pub signature_end: u32,
    /// Declaring module.
    pub module: String,
    /// File that contains it.
    pub file: FileId,
    /// Enclosing function span for a local; `None` for a top-level item.
    pub function_span: Option<Span>,
    /// Scope in which a local is visible; `None` for a top-level item.
    pub scope_span: Option<Span>,
}

impl Symbol {
    /// The exact signature text for hover, with surrounding whitespace trimmed.
    pub fn signature(&self, sources: &SourceMap) -> String {
        let end = self.signature_end.max(self.full_span.start).min(self.full_span.end);
        let span = Span::new(self.full_span.file, self.full_span.start as usize, end as usize);
        sources.snippet(span).trim().to_owned()
    }
}

/// One import binding.
#[derive(Clone, Debug)]
pub struct ImportSymbol {
    /// Name bound in this file.
    pub local: String,
    /// Name requested from the target module.
    pub original: String,
    /// Target module.
    pub module: String,
    /// File that contains the import.
    pub file: FileId,
}

/// One module declaration.
#[derive(Clone, Debug)]
pub struct ModuleSymbol {
    /// Dotted module name.
    pub name: String,
    /// File that declares it.
    pub file: FileId,
    /// Span of the dotted name.
    pub name_span: Span,
}

/// The complete symbol table for one analysis set.
#[derive(Default)]
pub struct SymbolTable {
    /// Top-level declarations and locals.
    pub symbols: Vec<Symbol>,
    /// Import bindings.
    pub imports: Vec<ImportSymbol>,
    /// Module declarations.
    pub modules: Vec<ModuleSymbol>,
}

impl SymbolTable {
    /// Builds the table from parsed programs.
    pub fn build(sources: &SourceMap, programs: &[(FileId, Program)]) -> Self {
        let mut table = Self::default();
        for (file, program) in programs {
            let module = program.module.name.to_dotted();
            if !module.is_empty() {
                table.modules.push(ModuleSymbol {
                    name: module.clone(),
                    file: *file,
                    name_span: program.module.name.span,
                });
            }
            for import in &program.imports {
                for item in &import.items {
                    table.imports.push(ImportSymbol {
                        local: item.alias.clone().unwrap_or_else(|| item.name.clone()),
                        original: item.name.clone(),
                        module: import.from.to_dotted(),
                        file: *file,
                    });
                }
            }
            for item in &program.items {
                table.item(sources, *file, &module, item);
            }
        }
        table.symbols.sort_by_key(|symbol| symbol.full_span.start);
        table
    }

    /// Adds one top-level item and its locals.
    fn item(&mut self, sources: &SourceMap, file: FileId, module: &str, item: &Item) {
        match item {
            Item::Func(decl) => {
                self.push(file, module, &decl.name, SymbolKind::Function, decl.name_span, decl.span, Some(decl.body.span.start as u32), None);
                for param in &decl.params {
                    self.push(file, module, &param.name, SymbolKind::Parameter, param.name_span, param.span, None, Some(decl.span));
                }
                self.block(file, module, &decl.body, decl.span);
            }
            Item::Comptime(decl) => {
                self.push(file, module, &decl.name, SymbolKind::Comptime, decl.name_span, decl.span, Some(decl.body.span.start as u32), None);
                for param in &decl.params {
                    self.push(file, module, &param.name, SymbolKind::Parameter, param.name_span, param.span, None, Some(decl.span));
                }
                self.block(file, module, &decl.body, decl.span);
            }
            Item::Struct(decl) => {
                let end = cut_at_brace(sources, decl.span);
                self.push(file, module, &decl.name, SymbolKind::Struct, decl.name_span, decl.span, Some(end), None);
            }
            Item::Enum(decl) => {
                let end = cut_at_brace(sources, decl.span);
                self.push(file, module, &decl.name, SymbolKind::Enum, decl.name_span, decl.span, Some(end), None);
            }
            Item::Handle(decl) => {
                self.push(file, module, &decl.name, SymbolKind::Handle, decl.name_span, decl.span, None, None);
            }
            Item::Ffi(decl) => {
                let end = cut_at_brace(sources, decl.span);
                self.push(file, module, &decl.name, SymbolKind::Ffi, decl.name_span, decl.span, Some(end), None);
            }
            Item::Intrinsic(decl) => {
                self.push(file, module, &decl.name, SymbolKind::Function, decl.name_span, decl.span, None, None);
            }
            Item::GlobalLet(decl) => {
                self.push(file, module, &decl.name, SymbolKind::Global, decl.name_span, decl.span, None, None);
            }
            Item::GlobalConst(decl) => {
                self.push(file, module, &decl.name, SymbolKind::Constant, decl.name_span, decl.span, None, None);
            }
        }
    }

    /// Adds one symbol.
    #[allow(clippy::too_many_arguments)]
    fn push(
        &mut self,
        file: FileId,
        module: &str,
        name: &str,
        kind: SymbolKind,
        name_span: Span,
        full_span: Span,
        signature_end: Option<u32>,
        function_span: Option<Span>,
    ) {
        let end = signature_end.unwrap_or(full_span.end);
        self.symbols.push(Symbol {
            name: name.to_owned(),
            kind,
            name_span,
            full_span,
            signature_end: end,
            module: module.to_owned(),
            file,
            function_span,
            scope_span: function_span,
        });
    }

    /// Walks a block and records its local bindings.
    fn block(&mut self, file: FileId, module: &str, block: &Block, function: Span) {
        for statement in &block.statements {
            self.statement(file, module, statement, function, block.span);
        }
    }

    /// Records one statement's locals and recurses.
    fn statement(
        &mut self,
        file: FileId,
        module: &str,
        statement: &Stmt,
        function: Span,
        scope: Span,
    ) {
        match statement {
            Stmt::Let(decl) => {
                self.push_local(file, module, &decl.name, decl.name_span, decl.span, function, scope);
                if let Some(value) = &decl.value {
                    self.expr(file, module, value, function);
                }
            }
            Stmt::Const(decl) => {
                self.push_local(file, module, &decl.name, decl.name_span, decl.span, function, scope);
                self.expr(file, module, &decl.value, function);
            }
            Stmt::If(decl) => {
                self.expr(file, module, &decl.condition, function);
                self.block(file, module, &decl.then_block, function);
                self.else_branch(file, module, &decl.else_branch, function);
            }
            Stmt::While(decl) => {
                self.expr(file, module, &decl.condition, function);
                self.block(file, module, &decl.body, function);
            }
            Stmt::DoWhile(decl) => {
                self.block(file, module, &decl.body, function);
                self.expr(file, module, &decl.condition, function);
            }
            Stmt::ForC(decl) => {
                match &decl.init {
                    Some(ForInit::Let(binding)) => {
                        self.push_local(file, module, &binding.name, binding.name_span, binding.span, function, decl.span);
                        if let Some(value) = &binding.value {
                            self.expr(file, module, value, function);
                        }
                    }
                    Some(ForInit::Const(binding)) => {
                        self.push_local(file, module, &binding.name, binding.name_span, binding.span, function, decl.span);
                        self.expr(file, module, &binding.value, function);
                    }
                    Some(ForInit::Expr(expressions)) => {
                        for expression in expressions {
                            self.expr(file, module, expression, function);
                        }
                    }
                    None => {}
                }
                if let Some(condition) = &decl.condition {
                    self.expr(file, module, condition, function);
                }
                for step in &decl.step {
                    self.expr(file, module, step, function);
                }
                self.block(file, module, &decl.body, function);
            }
            Stmt::ForIn(decl) => {
                self.push_local(file, module, &decl.name, decl.name_span, decl.span, function, scope);
                self.expr(file, module, &decl.iterable, function);
                self.block(file, module, &decl.body, function);
            }
            Stmt::Switch(decl) => {
                self.expr(file, module, &decl.scrutinee, function);
                for clause in &decl.clauses {
                    if let crate::ast::SwitchLabel::Case(value) = &clause.label {
                        self.expr(file, module, value, function);
                    }
                    for statement in &clause.statements {
                        self.statement(file, module, statement, function, clause.span);
                    }
                }
            }
            Stmt::Defer(block) | Stmt::Unsafe(block) | Stmt::Block(block) => {
                self.block(file, module, block, function);
            }
            Stmt::Expr(expression) => self.expr(file, module, expression, function),
            Stmt::Return { value: Some(value), .. } => {
                self.expr(file, module, value, function);
            }
            Stmt::Return { value: None, .. }
            | Stmt::Break(_)
            | Stmt::Continue(_) => {}
        }
    }

    /// Walks an expression and records lambda parameters and bodies.
    fn expr(&mut self, file: FileId, module: &str, expression: &crate::ast::Expr, function: Span) {
        use crate::ast::{ExprKind, IntrinsicArg, LambdaBody, TemplatePiece};
        match &expression.kind {
            ExprKind::Call { callee, args, .. } => {
                self.expr(file, module, callee, function);
                for argument in args {
                    self.expr(file, module, &argument.value, function);
                }
            }
            ExprKind::Index { base, index } => {
                self.expr(file, module, base, function);
                self.expr(file, module, index, function);
            }
            ExprKind::Member { base, .. } => self.expr(file, module, base, function),
            ExprKind::Unary { operand, .. } => self.expr(file, module, operand, function),
            ExprKind::Binary { left, right, .. } => {
                self.expr(file, module, left, function);
                self.expr(file, module, right, function);
            }
            ExprKind::Assign { target, value, .. } => {
                self.expr(file, module, target, function);
                self.expr(file, module, value, function);
            }
            ExprKind::Conditional {
                condition,
                then_value,
                else_value,
            } => {
                self.expr(file, module, condition, function);
                self.expr(file, module, then_value, function);
                self.expr(file, module, else_value, function);
            }
            ExprKind::Cast { value, .. } | ExprKind::RawCast { value, .. } => {
                self.expr(file, module, value, function);
            }
            ExprKind::StructLiteral { fields, .. } => {
                for field in fields {
                    self.expr(file, module, &field.value, function);
                }
            }
            ExprKind::ArrayLiteral { elements, fill } => {
                for element in elements {
                    self.expr(file, module, element, function);
                }
                if let Some(fill) = fill {
                    self.expr(file, module, fill, function);
                }
            }
            ExprKind::Lambda { params, body, .. } => {
                for parameter in params {
                    self.symbols.push(Symbol {
                        name: parameter.name.clone(),
                        kind: SymbolKind::Parameter,
                        name_span: parameter.name_span,
                        full_span: parameter.span,
                        signature_end: parameter.span.end,
                        module: module.to_owned(),
                        file,
                        function_span: Some(expression.span),
                        scope_span: Some(expression.span),
                    });
                }
                match body {
                    LambdaBody::Expr(value) => self.expr(file, module, value, expression.span),
                    LambdaBody::Block(block) => {
                        self.block(file, module, block, expression.span)
                    }
                }
            }
            ExprKind::Unsafe(inner) => self.expr(file, module, inner, function),
            ExprKind::IntrinsicCall { args, .. } => {
                for argument in args {
                    if let IntrinsicArg::Expr(value) = argument {
                        self.expr(file, module, value, function);
                    }
                }
            }
            ExprKind::Template(pieces) => {
                for piece in pieces {
                    if let TemplatePiece::Placeholder(value) = piece {
                        self.expr(file, module, value, function);
                    }
                }
            }
            ExprKind::Int(_)
            | ExprKind::Float(_)
            | ExprKind::Char(_)
            | ExprKind::Str(_)
            | ExprKind::Bool(_)
            | ExprKind::Null
            | ExprKind::Name(_)
            | ExprKind::Qualified(_)
            | ExprKind::Error => {}
        }
    }

    /// Adds a local binding.
    fn push_local(
        &mut self,
        file: FileId,
        module: &str,
        name: &str,
        name_span: Span,
        full_span: Span,
        function: Span,
        scope: Span,
    ) {
        self.symbols.push(Symbol {
            name: name.to_owned(),
            kind: SymbolKind::Local,
            name_span,
            full_span,
            signature_end: full_span.end,
            module: module.to_owned(),
            file,
            function_span: Some(function),
            scope_span: Some(scope),
        });
    }

    /// Recurses into an `else` branch.
    fn else_branch(
        &mut self,
        file: FileId,
        module: &str,
        branch: &Option<ElseBranch>,
        function: Span,
    ) {
        match branch {
            Some(ElseBranch::If(decl)) => {
                self.expr(file, module, &decl.condition, function);
                self.block(file, module, &decl.then_block, function);
                self.else_branch(file, module, &decl.else_branch, function);
            }
            Some(ElseBranch::Block(block)) => self.block(file, module, block, function),
            None => {}
        }
    }

    /// The symbol in `file` whose name span contains `offset`, if any.
    pub fn at(&self, file: FileId, offset: u32) -> Option<&Symbol> {
        self.symbols.iter().find(|symbol| {
            symbol.file == file
                && symbol.name_span.start <= offset
                && offset < symbol.name_span.end
        })
    }

    /// The innermost local in `file` with `name` that is visible at `offset`.
    pub fn local_at(&self, file: FileId, name: &str, offset: u32) -> Option<&Symbol> {
        self.symbols
            .iter()
            .filter(|symbol| {
                symbol.file == file
                    && (symbol.kind == SymbolKind::Local
                        || symbol.kind == SymbolKind::Parameter)
            })
            .filter(|symbol| symbol.name == name)
            .filter(|symbol| {
                symbol
                    .scope_span
                    .map(|span| span.start <= offset && offset < span.end)
                    .unwrap_or(false)
            })
            .filter(|symbol| symbol.name_span.start <= offset)
            .max_by_key(|symbol| symbol.name_span.start)
    }

    /// A module declaration by name.
    pub fn module(&self, name: &str) -> Option<&ModuleSymbol> {
        self.modules.iter().find(|module| module.name == name)
    }

    /// The module declared by a file, when the file declares one.
    pub fn module_for_file(&self, file: FileId) -> Option<&ModuleSymbol> {
        self.modules.iter().find(|module| module.file == file)
    }

    /// A top-level declaration by module and name.
    pub fn declaration(&self, module: &str, name: &str) -> Option<&Symbol> {
        self.symbols.iter().find(|symbol| {
            symbol.kind != SymbolKind::Local
                && symbol.kind != SymbolKind::Parameter
                && symbol.module == module
                && symbol.name == name
        })
    }

    /// Every module-level declaration in one module, in source order.
    pub fn module_declarations(&self, module: &str) -> impl Iterator<Item = &Symbol> + '_ {
        let module = module.to_owned();
        self.symbols.iter().filter(move |symbol| {
            symbol.module == module
                && symbol.kind != SymbolKind::Local
                && symbol.kind != SymbolKind::Parameter
        })
    }

    /// Every module-level declaration in one file, in source order.
    pub fn file_declarations(&self, file: FileId) -> impl Iterator<Item = &Symbol> {
        self.symbols.iter().filter(move |symbol| {
            symbol.file == file
                && symbol.kind != SymbolKind::Local
                && symbol.kind != SymbolKind::Parameter
        })
    }

    /// The import binding visible in one file.
    pub fn import_in_file(&self, file: FileId, local: &str) -> Option<&ImportSymbol> {
        self.imports
            .iter()
            .find(|import| import.file == file && import.local == local)
    }

    /// Modules grouped by name, for completion.
    pub fn module_names(&self) -> impl Iterator<Item = &String> {
        self.modules.iter().map(|module| &module.name)
    }
}

/// Returns the offset of the first `{` in a declaration, or its end.
fn cut_at_brace(sources: &SourceMap, span: Span) -> u32 {
    let text = sources.snippet(span);
    match text.find('{') {
        Some(index) => span.start + index as u32,
        None => span.end,
    }
}

/// The identifier token that contains, or is adjacent to, `offset`.
///
/// A position at the end of an identifier still selects it, which is what an
/// editor sends when the caret sits after the last character.
pub fn word_at(text: &str, offset: usize) -> Option<(usize, String)> {
    let bytes = text.as_bytes();
    if bytes.is_empty() {
        return None;
    }
    let mut start = offset.min(bytes.len());
    while start > 0 && is_word_byte(bytes[start - 1]) {
        start -= 1;
    }
    let mut end = offset.min(bytes.len());
    while end < bytes.len() && is_word_byte(bytes[end]) {
        end += 1;
    }
    if start == end {
        if start == 0 {
            return None;
        }
        // The caret is after a non-word character; try the token before it.
        let mut back = start;
        while back > 0 && !is_word_byte(bytes[back - 1]) {
            back -= 1;
        }
        if back == start {
            return None;
        }
        return word_at(text, back);
    }
    Some((start, text[start..end].to_owned()))
}

/// True for identifier characters.
fn is_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// The dotted name around an offset, for module references.
pub fn dotted_at(text: &str, offset: usize) -> Option<String> {
    let bytes = text.as_bytes();
    let mut start = offset.min(bytes.len());
    while start > 0 && (is_word_byte(bytes[start - 1]) || bytes[start - 1] == b'.') {
        start -= 1;
    }
    let mut end = offset.min(bytes.len());
    while end < bytes.len() && (is_word_byte(bytes[end]) || bytes[end] == b'.') {
        end += 1;
    }
    let value = text[start..end].trim_matches('.').to_owned();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::{self, SourceFile};

    fn parse_one(text: &str) -> (SourceMap, Vec<(FileId, Program)>) {
        let parsed = analysis::parse(vec![SourceFile {
            path: "app.scope.clo".to_owned(),
            text: text.to_owned(),
        }]);
        (parsed.sources, parsed.programs)
    }

    #[test]
    fn should_pick_the_innermost_local_visible_at_the_offset() {
        let text = "module app.scope;\nfunc f(): void {\n    let x: i32 = 1;\n    {\n        let x: i32 = 2;\n    }\n}\n";
        let (sources, programs) = parse_one(text);
        let table = SymbolTable::build(&sources, &programs);
        let second = text.rfind("x: i32 = 2").expect("second x");
        let symbol = table
            .local_at(programs[0].0, "x", second as u32)
            .expect("second local");
        assert_eq!(symbol.name_span.start as usize, second);
    }

    #[test]
    fn should_not_match_a_symbol_from_another_file() {
        let first = "module app.one;\nfunc f(): void {\n    let shared: i32 = 1;\n}\n";
        let second = "module app.two;\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\nfunc g(): void {\n    let shared: i32 = 2;\n}\n";
        let parsed = analysis::parse(vec![
            SourceFile {
                path: "one.clo".to_owned(),
                text: first.to_owned(),
            },
            SourceFile {
                path: "two.clo".to_owned(),
                text: second.to_owned(),
            },
        ]);
        let table = SymbolTable::build(&parsed.sources, &parsed.programs);
        let second_file = parsed.files["two.clo"];
        let second_offset = second.find("shared").expect("shared") as u32;
        let symbol = table.at(second_file, second_offset).expect("symbol");
        assert_eq!(symbol.file, second_file);
        assert_eq!(symbol.name_span.start, second_offset);
        let first_file = parsed.files["one.clo"];
        assert!(table.at(first_file, second_offset).is_none());
        assert!(table
            .local_at(first_file, "shared", second_offset)
            .is_none());
        assert!(table
            .local_at(second_file, "shared", second_offset)
            .is_some());
    }
}

#[cfg(test)]
mod lambda_tests {
    use super::*;
    use crate::analysis::{self, SourceFile};

    #[test]
    fn should_scope_a_lambda_in_a_switch_scrutinee() {
        let text = "module app.lambda;\nfunc main(): i32 {\n    let x: i32 = 1;\n    switch (((x: i32) => x + 1)(1)) {\n        case 2: return 0;\n        default: return 1;\n    }\n}\n";
        let parsed = analysis::parse(vec![SourceFile {
            path: "app/switch.clo".to_owned(),
            text: text.to_owned(),
        }]);
        assert!(
            parsed.diagnostics.items().is_empty(),
            "switch lambda fixture must parse: {:?}",
            parsed.diagnostics.items()
        );
        let table = SymbolTable::build(&parsed.sources, &parsed.programs);
        let file = parsed.files["app/switch.clo"];
        let param = text.find("(x: i32)").expect("lambda parameter") + 1;
        let body = text.rfind("x + 1").expect("lambda body");
        assert_eq!(
            table.local_at(file, "x", param as u32).unwrap().name_span.start as usize,
            param
        );
        assert_eq!(
            table.local_at(file, "x", body as u32).unwrap().name_span.start as usize,
            param
        );
    }

    #[test]
    fn should_scope_lambda_parameters_and_bodies_to_the_lambda() {
        let text = "module app.lambda;\nfunc main(): i32 {\n    let x: i32 = 1;\n    let f: func(i32): i32 = (x: i32) => x + 1;\n    return f(x);\n}\n";
        let parsed = analysis::parse(vec![SourceFile {
            path: "app/lambda.clo".to_owned(),
            text: text.to_owned(),
        }]);
        assert!(
            parsed.diagnostics.items().is_empty(),
            "lambda fixture must parse: {:?}",
            parsed.diagnostics.items()
        );
        let table = SymbolTable::build(&parsed.sources, &parsed.programs);
        let file = parsed.files["app/lambda.clo"];
        let param = text.find("(x: i32)").expect("lambda parameter") + 1;
        let body = text.rfind("x + 1").expect("lambda body");
        let outer = text.find("x: i32 = 1").expect("outer local");
        assert_eq!(table.local_at(file, "x", param as u32).unwrap().name_span.start as usize, param);
        assert_eq!(table.local_at(file, "x", body as u32).unwrap().name_span.start as usize, param);
        assert_eq!(table.local_at(file, "x", outer as u32).unwrap().name_span.start as usize, outer);
    }
}
