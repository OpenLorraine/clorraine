use super::*;
use crate::diag::Diagnostics;
use crate::load;
use crate::parse::Parser;
use crate::span::SourceMap;

/// Parses, loads, and resolves the given files, returning `(code, message)`
/// for every diagnostic in report order.
fn resolve_sources(files: &[&str]) -> Vec<(&'static str, String)> {
    let mut sources = SourceMap::new();
    let mut diagnostics = Diagnostics::new(false, None);
    let mut programs = Vec::new();
    for (index, text) in files.iter().enumerate() {
        let id = sources.add_file(format!("file{index}.clo"), (*text).to_owned());
        let program = Parser::new(&sources, id, &mut diagnostics).parse_program();
        programs.push((id, program));
    }
    let loaded = load::load(programs, &mut diagnostics);
    resolve(&loaded, &mut diagnostics);
    diagnostics
        .items()
        .iter()
        .map(|diagnostic| (diagnostic.code, diagnostic.message.clone()))
        .collect()
}

/// Codes of the diagnostics in a list.
fn codes(diagnostics: &[(&'static str, String)]) -> Vec<&'static str> {
    diagnostics.iter().map(|(code, _)| *code).collect()
}

#[test]
fn shadowing_prefers_the_innermost_binding() {
    let diagnostics = resolve_sources(&[r#"
module app;

let LIMIT: i32 = 100;

func pick(LIMIT: i32): i32 {
let doubled: i32 = LIMIT * 2;
let LIMIT: i32 = doubled;
return LIMIT;
}

func main(): i32 {
return pick(5);
}
"#]);
    assert!(diagnostics.is_empty(), "unexpected diagnostics: {diagnostics:?}");
}

#[test]
fn use_before_declaration_in_a_block_is_undeclared() {
    let diagnostics = resolve_sources(&[r#"
module app;

func demo(): i32 {
let first: i32 = second;
let second: i32 = 2;
return first + second;
}

func main(): i32 {
return 0;
}
"#]);
    assert_eq!(
        diagnostics,
        [("E10219", "'second' is not declared".to_owned())]
    );
}

#[test]
fn module_level_declarations_are_order_independent() {
    let diagnostics = resolve_sources(&[r#"
module app;

let seed: i32 = compute();

func compute(): i32 {
return 1;
}

func main(): i32 {
return seed;
}
"#]);
    assert!(diagnostics.is_empty(), "unexpected diagnostics: {diagnostics:?}");
}

#[test]
fn undeclared_module_initializer_reports() {
    let diagnostics = resolve_sources(&[r#"
module app;

let seed: i32 = missing + 1;

func main(): i32 {
return seed;
}
"#]);
    assert_eq!(codes(&diagnostics), ["E10219"]);
    assert_eq!(diagnostics[0].1, "'missing' is not declared");
}

#[test]
fn qualified_access_resolves_without_an_import() {
    let diagnostics = resolve_sources(&[r#"
module app;

func sink(b: std.mem.Buffer): void {}

func main(): i32 {
return 0;
}
"#]);
    assert!(diagnostics.is_empty(), "unexpected diagnostics: {diagnostics:?}");
}

#[test]
fn qualified_access_of_a_missing_export_reports() {
    let diagnostics = resolve_sources(&[r#"
module app;

func sink(b: std.mem.Missing): void {}

func main(): i32 {
return 0;
}
"#]);
    assert_eq!(
        diagnostics,
        [("E10219", "'std.mem.Missing' is not declared".to_owned())]
    );
}

#[test]
fn type_parameters_resolve_inside_their_declaration() {
    let diagnostics = resolve_sources(&[r#"
module app;

func identity<T>(x: T): T {
return x;
}

func main(): i32 {
return identity<i32>(1);
}
"#]);
    assert!(diagnostics.is_empty(), "unexpected diagnostics: {diagnostics:?}");
}

#[test]
fn for_loops_bind_their_variables() {
    let diagnostics = resolve_sources(&[r#"
module app;

func main(): i32 {
let values = [1, 2, 3];
for (value in values) {
    let copied = value;
}
for (let i = 0; i < 3; i = i + 1) {
    let current = i;
}
return 0;
}
"#]);
    assert!(diagnostics.is_empty(), "unexpected diagnostics: {diagnostics:?}");
}

#[test]
fn member_access_checks_only_the_receiver() {
    let resolved = resolve_sources(&[r#"
module app;

struct Holder { data: i32; }

func main(): i32 {
let holder = Holder { data: 1 };
holder.push(1);
return 0;
}
"#]);
    assert!(resolved.is_empty(), "unexpected diagnostics: {resolved:?}");

    let unresolved = resolve_sources(&[r#"
module app;

func main(): i32 {
missing.field;
return 0;
}
"#]);
    assert_eq!(
        unresolved,
        [("E10219", "'missing.field' is not declared".to_owned())]
    );
}

#[test]
fn nested_block_shadowing_does_not_leak() {
    let diagnostics = resolve_sources(&[r#"
module app;

func main(): i32 {
{
    let inner: i32 = 1;
}
return inner;
}
"#]);
    assert_eq!(
        diagnostics,
        [("E10219", "'inner' is not declared".to_owned())]
    );
}

#[test]
fn ffi_signatures_and_deleters_resolve() {
    let diagnostics = resolve_sources(&[r#"
module app;

handle File freedBy CloseFile;

ffi lib {
func CloseFile(take f: File): void;
func Read(f: File, buf: u8[]): i32;
}

func main(): i32 {
return 0;
}
"#]);
    assert!(diagnostics.is_empty(), "unexpected diagnostics: {diagnostics:?}");
}

#[test]
fn own_module_qualified_access_accepts_private_declarations() {
    let diagnostics = resolve_sources(&[r#"
module app;

func helper(): i32 {
return 1;
}

func main(): i32 {
return app.helper();
}
"#]);
    assert!(diagnostics.is_empty(), "unexpected diagnostics: {diagnostics:?}");
}

#[test]
fn alias_imports_bind_the_renamed_name() {
    let diagnostics = resolve_sources(&[
        "module dep;\nexport func helper(): i32 { return 1; }\n",
        "module app;\nimport { helper as h } from dep;\nfunc main(): i32 { return h(); }\n",
    ]);
    assert!(diagnostics.is_empty(), "unexpected diagnostics: {diagnostics:?}");
}

#[test]
fn import_used_only_in_an_ffi_signature_is_not_unused() {
    let diagnostics = resolve_sources(&[
        "module dep;\nexport struct Token { id: i32; }\n",
        "module app;\nimport { Token } from dep;\nffi lib { func consume(t: Token): void; }\nfunc main(): i32 { return 0; }\n",
    ]);
    assert!(diagnostics.is_empty(), "unexpected diagnostics: {diagnostics:?}");
}

#[test]
fn qualified_deleter_resolves_in_its_own_module() {
    let diagnostics = resolve_sources(&[r#"
module app;

handle File freedBy app.Close;

ffi lib {
    func Close(f: File): void;
}

func main(): i32 {
    return 0;
}
"#]);
    assert!(diagnostics.is_empty(), "unexpected diagnostics: {diagnostics:?}");
}
