//! Initializer-cycle and staged-body tests for the semantic checker.
//!
//! These cases drive the shared initializer-read graph and the `comptime`
//! staged-feature code through the public `check` entry point, so they live
//! apart from the general expression and statement tests.

use super::tests::{assert_accepts, assert_codes, check_sources, codes};

/// A `comptime` body is frozen syntax whose semantics are staged out.
///
/// The declaration is otherwise well-formed, so the staged-feature code is the
/// only diagnostic; the body is not type-checked.
#[test]
fn a_comptime_body_reports_its_staged_feature_code() {
    assert_codes(
        r#"module test;
comptime func tableSize(n: i32): i32 { return n * 4; }
func main(): i32 { return 0; }
"#,
        &["E90001"],
    );
}

/// Two same-module globals whose initializers read each other form a cycle.
///
/// There is no initialization order that satisfies both reads, so the stage
/// reports the cycle instead of ordering the module's initializers.
#[test]
fn a_same_module_initializer_cycle_is_rejected() {
    assert_codes(
        r#"module test;
let a: i32 = b + 1;
let b: i32 = a + 1;
func main(): i32 { return 0; }
"#,
        &["E11100"],
    );
}

/// A global may read a global initialized before it without forming a cycle.
#[test]
fn an_acyclic_initializer_read_is_accepted() {
    assert_accepts(
        r#"module test;
let a: i32 = 1;
let b: i32 = a + 1;
func main(): i32 { return 0; }
"#,
    );
}

/// An initializer cycle across two modules is rejected as well.
///
/// Each module reads the other's global, so neither module can be initialized
/// first; the reported cycle names both modules.
#[test]
fn a_cross_module_initializer_cycle_is_rejected() {
    let alpha = "module test.Alpha;\nimport { y } from test.Beta;\nlet x: i32 = y + 1;\n";
    let beta = "module test.Beta;\nimport { x } from test.Alpha;\nlet y: i32 = x + 1;\n";
    let diagnostics = check_sources(&[alpha, beta]);
    assert_eq!(codes(&diagnostics), ["E11100"]);
    assert!(
        diagnostics[0].1.contains("test.Alpha") && diagnostics[0].1.contains("test.Beta"),
        "the cycle must name both modules, got {:?}",
        diagnostics[0].1
    );
}

/// A field read of a same-module global is still an initializer edge.
///
/// `b.v` parses as one dotted name, not a member node, so the cycle detector
/// must resolve its head as the global `b` and treat the rest as a field path.
#[test]
fn a_same_module_field_cycle_is_rejected() {
    assert_codes(
        r#"module test;
struct Crate { v: i32; }
let a: i32 = b.v + 1;
let b: Crate = Crate { v: a };
func main(): i32 { return 0; }
"#,
        &["E11100"],
    );
}

/// A full module path read is a module dependency edge.
///
/// `test.Beta.y` names the module `test.Beta`, and the reverse read closes the
/// module cycle; the cycle is reported once and names both modules.
#[test]
fn a_full_path_cross_module_cycle_is_rejected_once() {
    let alpha =
        "module test.Alpha;\nexport let x: i32 = test.Beta.y + 1;\n";
    let beta =
        "module test.Beta;\nexport let y: i32 = test.Alpha.x + 1;\n";
    let diagnostics = check_sources(&[alpha, beta]);
    assert_eq!(codes(&diagnostics), ["E11100"]);
    assert!(
        diagnostics[0].1.contains("test.Alpha") && diagnostics[0].1.contains("test.Beta"),
        "the cycle must name both modules, got {:?}",
        diagnostics[0].1
    );
}

/// A mutual import with runtime globals is a module cycle.
///
/// The two initializers read nothing from each other, but the imports alone fix
/// an order that cannot be satisfied, and at least one module has a runtime
/// `let`, so the cycle is E11100.
#[test]
fn a_mutual_import_with_runtime_globals_is_rejected_once() {
    let alpha = "module test.Alpha;\nimport { y } from test.Beta;\nlet x: i32 = 5;\n";
    let beta = "module test.Beta;\nimport { x } from test.Alpha;\nlet y: i32 = 6;\n";
    let diagnostics = check_sources(&[alpha, beta]);
    assert_eq!(codes(&diagnostics), ["E11100"]);
    assert!(
        diagnostics[0].1.contains("test.Alpha") && diagnostics[0].1.contains("test.Beta"),
        "the cycle must name both modules, got {:?}",
        diagnostics[0].1
    );
}

/// A call to a function that reads a global creates no initializer edge.
///
/// The function body runs later, so its read must not make the caller's
/// initializer depend on the global.
#[test]
fn a_call_to_a_function_that_reads_a_global_is_accepted() {
    assert_accepts(
        r#"module test;
let base: i32 = 5;
func read(): i32 { return base; }
let derived: i32 = read();
func main(): i32 { return 0; }
"#,
    );
}

/// A cycle of `const`-only modules is allowed.
///
/// A `const` folds to a compile-time value, so a const-only module cycle adds
/// no runtime step and needs no initialization order. The imports alone create
/// the module cycle, but neither module has a runtime `let`.
#[test]
fn a_const_only_module_cycle_is_accepted() {
    let alpha = "module test.Alpha;\nimport { y } from test.Beta;\nconst x: i32 = 1;\n";
    let beta = "module test.Beta;\nimport { x } from test.Alpha;\nconst y: i32 = 2;\n";
    let diagnostics = check_sources(&[alpha, beta]);
    assert!(
        diagnostics.is_empty(),
        "a const-only module cycle must be accepted, got {diagnostics:?}"
    );
}
