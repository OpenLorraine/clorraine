//! In-module tests for the parser.

use super::*;
use crate::ast::*;
use crate::diag::Diagnostics;

/// Parses a source string and returns the program with the diagnostic codes.
fn parse(source: &str) -> (Program, Vec<String>) {
    let mut sources = SourceMap::new();
    let id = sources.add_file("test.clo".to_owned(), source.to_owned());
    let mut diagnostics = Diagnostics::new(false, None);
    let program = Parser::new(&sources, id, &mut diagnostics).parse_program();
    let codes = diagnostics
        .items()
        .iter()
        .map(|diagnostic| diagnostic.code.to_owned())
        .collect();
    (program, codes)
}

/// First function item of a program.
fn first_func(program: &Program) -> &FuncDecl {
    program
        .items
        .iter()
        .find_map(|item| match item {
            Item::Func(func) => Some(func),
            _ => None,
        })
        .expect("expected a function item")
}

/// Initializer of the named `let` binding.
fn let_value<'a>(func: &'a FuncDecl, name: &str) -> &'a Expr {
    for statement in &func.body.statements {
        if let Stmt::Let(binding) = statement {
            if binding.name == name {
                return binding.value.as_ref().expect("let initializer");
            }
        }
    }
    panic!("no let binding named {name}");
}

#[test]
fn a_representative_program_parses_cleanly() {
    let source = r#"
module Demo.App;

import { println as out } from std.io;

struct Point { x: i32; y: i32; }

enum Mode : u8 { A, B = 5 }

handle File freedBy CloseFile;

func add(a: i32, b: i32): i32 { return a + b; }

func main(): i32 {
    let p = Point { x: 1, y: 2 };
    let values = [1, 2, 3];
    let total = 0;
    for (value in values) { total = total + value; }
    for (let i = 0; i < 3; i = i + 1) { total = total + i; }
    while (total > 0) { total = total - 1; }
    do { total = total + 1; } while (total < 1);
    if (total == 0) { total = 1; } else if (total == 1) { total = 2; } else { total = 3; }
    switch (total) { case 1: break; default: total = 0; }
    let f = (x: i32) => x + 1;
    let t = $"v={total}";
    defer { total = 0; }
    unsafe { let q = p; }
    return add(total, 1) + f(2);
}
"#;
    let (program, codes) = parse(source);
    assert!(codes.is_empty(), "unexpected diagnostics: {codes:?}");
    assert_eq!(program.module.name.to_dotted(), "Demo.App");
    assert_eq!(program.imports.len(), 1);
    assert_eq!(program.imports[0].items[0].alias.as_deref(), Some("out"));
    assert_eq!(program.items.len(), 5);
}

#[test]
fn generic_calls_and_comparisons_disambiguate() {
    let source = r#"
func shapes(a: i32, b: i32, c: i32, f: i32): i32 {
    let call = f<i32>(1);
    let chained = a < b > (c);
    let relation = a < b;
    let grouped = (a < b) > (c);
    return 0;
}
"#;
    let (program, codes) = parse(source);
    assert!(codes.is_empty(), "unexpected diagnostics: {codes:?}");
    let func = first_func(&program);
    let ExprKind::Call { type_args, .. } = &let_value(func, "call").kind else {
        panic!("expected a generic call");
    };
    assert_eq!(type_args.len(), 1);
    let ExprKind::Call { type_args, .. } = &let_value(func, "chained").kind else {
        panic!("`a < b > (c)` must parse as a generic call");
    };
    assert_eq!(type_args.len(), 1);
    let ExprKind::Binary { op: BinaryOp::Lt, .. } = &let_value(func, "relation").kind else {
        panic!("expected a relational comparison");
    };
    let ExprKind::Binary { op: BinaryOp::Gt, left, .. } = &let_value(func, "grouped").kind else {
        panic!("expected the parenthesized comparison");
    };
    assert!(matches!(left.kind, ExprKind::Binary { op: BinaryOp::Lt, .. }));
}

#[test]
fn struct_literals_and_blocks_disambiguate() {
    let source = r#"
struct Point { x: i32; y: i32; }
func main(): i32 {
    Point { x: 1, y: 2 };
    { let y = 1; }
    return 0;
}
"#;
    let (program, codes) = parse(source);
    assert!(codes.is_empty(), "unexpected diagnostics: {codes:?}");
    let func = first_func(&program);
    let Stmt::Expr(expr) = &func.body.statements[0] else {
        panic!("expected the struct-literal expression statement");
    };
    let ExprKind::StructLiteral { name, fields, .. } = &expr.kind else {
        panic!("expected a struct literal");
    };
    assert_eq!(name.to_dotted(), "Point");
    assert_eq!(fields.len(), 2);
    let Stmt::Block(block) = &func.body.statements[1] else {
        panic!("expected a block statement");
    };
    assert_eq!(block.statements.len(), 1);
}

#[test]
fn both_for_forms_disambiguate() {
    let source = r#"
func main(): i32 {
    let values = [1, 2, 3];
    for (value in values) { }
    for (borrow value in values) { }
    for (let i = 0; i < 3; i = i + 1) { }
    for (;;) { }
    return 0;
}
"#;
    let (program, codes) = parse(source);
    assert!(codes.is_empty(), "unexpected diagnostics: {codes:?}");
    let func = first_func(&program);
    let Stmt::ForIn(element) = &func.body.statements[1] else {
        panic!("expected the element form");
    };
    assert!(!element.borrow);
    let Stmt::ForIn(borrowed) = &func.body.statements[2] else {
        panic!("expected the borrowing element form");
    };
    assert!(borrowed.borrow);
    let Stmt::ForC(counted) = &func.body.statements[3] else {
        panic!("expected the three-clause form");
    };
    assert!(matches!(counted.init, Some(ForInit::Let(_))));
    let Stmt::ForC(infinite) = &func.body.statements[4] else {
        panic!("expected the infinite form");
    };
    assert!(infinite.init.is_none());
    assert!(infinite.condition.is_none());
    assert!(infinite.step.is_empty());
}

#[test]
fn lambdas_groupings_and_array_literals_disambiguate() {
    let source = r#"
func main(): i32 {
    let f = (x: i32) => x + 1;
    let g = (1 + 2);
    let h = [1, 2];
    let k = [borrow x](y: i32) => { return y; };
    let m = [];
    let n = [1, 2; 0];
    return 0;
}
"#;
    let (program, codes) = parse(source);
    assert!(codes.is_empty(), "unexpected diagnostics: {codes:?}");
    let func = first_func(&program);
    let ExprKind::Lambda { captures, params, .. } = &let_value(func, "f").kind else {
        panic!("expected a lambda");
    };
    assert!(captures.is_empty());
    assert_eq!(params.len(), 1);
    assert!(matches!(let_value(func, "g").kind, ExprKind::Binary { .. }));
    let ExprKind::ArrayLiteral { elements, fill } = &let_value(func, "h").kind else {
        panic!("expected an array literal");
    };
    assert_eq!(elements.len(), 2);
    assert!(fill.is_none());
    let ExprKind::Lambda { captures, .. } = &let_value(func, "k").kind else {
        panic!("expected a capturing lambda");
    };
    assert_eq!(captures.len(), 1);
    assert_eq!(captures[0].mode, Mode::Borrow);
    assert!(matches!(
        &let_value(func, "m").kind,
        ExprKind::ArrayLiteral { elements, .. } if elements.is_empty()
    ));
    let ExprKind::ArrayLiteral { elements, fill } = &let_value(func, "n").kind else {
        panic!("expected an array literal");
    };
    assert_eq!(elements.len(), 2);
    assert!(fill.is_some());
}

#[test]
fn unsafe_block_and_prefix_operator_differ() {
    let source = r#"
func main(): i32 {
    unsafe { }
    let p = unsafe q;
    return 0;
}
"#;
    let (program, codes) = parse(source);
    assert!(codes.is_empty(), "unexpected diagnostics: {codes:?}");
    let func = first_func(&program);
    assert!(matches!(func.body.statements[0], Stmt::Unsafe(_)));
    assert!(matches!(let_value(func, "p").kind, ExprKind::Unsafe(_)));
}

#[test]
fn raw_binds_to_the_next_type_atom() {
    let source = "func f(mem: raw u8[4], p: raw u8?): void { }";
    let (program, codes) = parse(source);
    assert!(codes.is_empty(), "unexpected diagnostics: {codes:?}");
    let func = first_func(&program);
    let TypeKind::Array { element, extents } = &func.params[0].ty.kind else {
        panic!("expected an array of raw pointers");
    };
    assert!(matches!(element.kind, TypeKind::Raw(_)));
    assert_eq!(extents.len(), 1);
    assert!(matches!(func.params[1].ty.kind, TypeKind::Raw(_)));
    assert!(func.params[1].ty.nullable);
}

#[test]
fn function_types_and_extents_parse() {
    let source = "func apply(f: func(i32): bool, xs: u8[], ys: u8[4], zs: u8[][8]): void { }";
    let (program, codes) = parse(source);
    assert!(codes.is_empty(), "unexpected diagnostics: {codes:?}");
    let func = first_func(&program);
    let TypeKind::Function { params, .. } = &func.params[0].ty.kind else {
        panic!("expected a function pointer type");
    };
    assert_eq!(params.len(), 1);
    let TypeKind::Array { extents, .. } = &func.params[1].ty.kind else {
        panic!("expected a slice type");
    };
    assert!(extents[0].size.is_none());
    let TypeKind::Array { extents, .. } = &func.params[2].ty.kind else {
        panic!("expected a fixed array type");
    };
    assert!(extents[0].size.is_some());
    let TypeKind::Array { extents, .. } = &func.params[3].ty.kind else {
        panic!("expected a multi-extent array type");
    };
    assert_eq!(extents.len(), 2);
}

#[test]
fn diagnostic_only_starts_report_their_codes() {
    let source = r#"
module M;
union Value { a: i32; }
func demo(): void { volatile let x: u32 = 0; }
func fmt(): str { let x = 1; return $"{x:04x}"; }
"#;
    let (_, codes) = parse(source);
    assert_eq!(codes, ["E10204", "E10208", "E10501"]);
}

#[test]
fn keyword_and_reserved_declaration_names_report_their_codes() {
    let (_, keyword_codes) = parse("func main(): i32 { let if = 1; return 0; }");
    assert!(keyword_codes.contains(&"E20005".to_owned()));
    let reserved = parse("func sizeof(x: i32): i32 { return x; }");
    assert_eq!(reserved.1, ["E20006"]);
    let intrinsic = parse("intrinsic func readVolatile16(addr: raw u16): u16;");
    assert!(intrinsic.1.is_empty(), "intrinsic names may be reserved");
}

#[test]
fn a_stray_token_reports_once_and_parsing_continues() {
    let source = r#"
module M;
}
func main(): i32 { return 0; }
"#;
    let (program, codes) = parse(source);
    assert_eq!(codes, ["E20000"]);
    assert_eq!(program.items.len(), 1);
    assert_eq!(first_func(&program).name, "main");
}

#[test]
fn template_placeholders_are_parsed_expressions() {
    let source = r#"func main(): i32 { let x = 1; let t = $"v={x + 1}"; return 0; }"#;
    let (program, codes) = parse(source);
    assert!(codes.is_empty(), "unexpected diagnostics: {codes:?}");
    let func = first_func(&program);
    let ExprKind::Template(pieces) = &let_value(func, "t").kind else {
        panic!("expected a template");
    };
    assert!(matches!(pieces[0], TemplatePiece::Literal(_)));
    let TemplatePiece::Placeholder(expr) = &pieces[1] else {
        panic!("expected a placeholder");
    };
    assert!(matches!(expr.kind, ExprKind::Binary { op: BinaryOp::Add, .. }));
}

#[test]
fn module_volatile_let_is_valid_and_misplaced_volatile_recovers() {
    let valid = parse("volatile let shared: u32 = 0;");
    assert!(valid.1.is_empty(), "unexpected diagnostics: {:?}", valid.1);
    let misplaced = parse("volatile func helper(): void { }");
    assert_eq!(misplaced.1, ["E10208"]);
    assert_eq!(misplaced.0.items.len(), 1);
}

#[test]
fn comptime_functions_parse_fully() {
    let source = r#"
comptime func tableSize(n: i32): i32 { return n * 4; }
"#;
    let (program, codes) = parse(source);
    assert!(codes.is_empty(), "unexpected diagnostics: {codes:?}");
    assert!(matches!(program.items[0], Item::Comptime(_)));
}

#[test]
fn nested_type_arguments_parse() {
    let source = r#"
func main(): i32 {
    let value: Box<Box<i32>> = other;
    let made = make<Box<i32>>(1);
    return 0;
}
"#;
    let (program, codes) = parse(source);
    assert!(codes.is_empty(), "unexpected diagnostics: {codes:?}");
    let func = first_func(&program);
    let Stmt::Let(binding) = &func.body.statements[0] else {
        panic!("expected a let binding");
    };
    let TypeKind::Named { args, .. } = &binding.ty.as_ref().expect("declared type").kind else {
        panic!("expected a named type");
    };
    assert_eq!(args.len(), 1);
    let ExprKind::Call { type_args, .. } = &let_value(func, "made").kind else {
        panic!("expected a generic call");
    };
    assert_eq!(type_args.len(), 1);
}

#[test]
fn a_stray_close_angle_is_relational() {
    let source = r#"
func compare(a: i32, b: i32, c: i32): i32 {
    let chained = a < b > c;
    let nested = a < b > (c);
    return 0;
}
"#;
    let (program, codes) = parse(source);
    assert!(codes.is_empty(), "unexpected diagnostics: {codes:?}");
    let func = first_func(&program);
    let ExprKind::Binary { op: BinaryOp::Gt, left, .. } = &let_value(func, "chained").kind else {
        panic!("`a < b > c` must parse as a comparison chain");
    };
    assert!(matches!(left.kind, ExprKind::Binary { op: BinaryOp::Lt, .. }));
    let ExprKind::Call { .. } = &let_value(func, "nested").kind else {
        panic!("`a < b > (c)` must parse as a generic call");
    };
}

#[test]
fn lexical_errors_do_not_cascade() {
    assert_eq!(parse("func main(): i32 { let s: str = \"no close;\n return 0; }").1, ["E20002"]);
    assert_eq!(parse("func main(): i32 { let n = 0x; return 0; }").1, ["E20004"]);
    assert_eq!(parse("func main(): i32 { let c: u8 = 'ab'; return 0; }").1, ["E20007"]);
}

#[test]
fn imports_and_qualified_names_parse() {
    let source = r#"
module A.B;
import { readFile as load, File } from std.io;
func main(): i32 {
    let f = std.io.open("name");
    return 0;
}
"#;
    let (program, codes) = parse(source);
    assert!(codes.is_empty(), "unexpected diagnostics: {codes:?}");
    assert_eq!(program.imports[0].items.len(), 2);
    assert_eq!(program.imports[0].items[0].alias.as_deref(), Some("load"));
    let func = first_func(&program);
    let ExprKind::Call { callee, .. } = &let_value(func, "f").kind else {
        panic!("expected a call");
    };
    let ExprKind::Qualified(name) = &callee.kind else {
        panic!("expected a qualified name");
    };
    assert_eq!(name.to_dotted(), "std.io.open");
}
