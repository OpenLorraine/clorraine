//! Implementation tests for the scope structure of the lowered IR.
//!
//! The ownership walk records a lexical scope for every block and tags each
//! drop, `defer`, and scope-end marker with the scope whose exit emits it. These
//! tests pin that structure so a future emitter can build the cleanup ladder
//! from the IR alone.

use super::test_support::{compile, with_view_helper};
use crate::ir::{Function, Statement};

/// The lowered `demo` function of a clean source string.
fn demo_function(source: &str) -> Function {
    let (_, output) = compile(source);
    let output = output.expect("the clean program lowers");
    output
        .functions
        .into_iter()
        .find(|function| function.name == "demo")
        .expect("the demo function is lowered")
}

#[test]
fn a_block_scope_is_recorded_with_its_parent() {
    // The function body block opens scope 1 under the parameter scope 0, and
    // the nested block opens scope 2 under scope 1.
    let source = with_view_helper(
        "func demo(): void {\n\
             { let s: String = $\"x{1}\"; observe(s); }\n\
         }",
    );
    let demo = demo_function(&source);
    let body = demo.scopes.first().expect("the body scope is recorded");
    assert_eq!(body.parent, None);
    let block = demo.scopes.get(1).expect("the body block scope is recorded");
    assert_eq!(block.parent, Some(0));
    let nested = demo.scopes.get(2).expect("the nested block scope is recorded");
    assert_eq!(nested.parent, Some(1));
    assert!(!nested.in_loop);
}

#[test]
fn a_scope_opened_in_a_loop_body_is_marked_in_loop() {
    let source = with_view_helper(
        "func demo(c: bool): void {\n\
             while (true) {\n\
                 let r: String = $\"x{1}\";\n\
                 if (c) { break; }\n\
                 observe(r);\n\
             }\n\
         }",
    );
    let demo = demo_function(&source);
    assert!(
        demo.scopes.iter().any(|scope| scope.in_loop),
        "the loop body scope must be marked in_loop"
    );
}

#[test]
fn a_normal_block_end_emits_a_scope_end_marker() {
    let source = with_view_helper(
        "func demo(): void {\n\
             observe($\"x{1}\");\n\
         }",
    );
    let demo = demo_function(&source);
    assert!(
        demo.blocks.iter().any(|block| block
            .statements
            .iter()
            .any(|statement| matches!(statement.statement, Statement::ScopeEnd { .. }))),
        "a block that falls through ends with a scope-end marker"
    );
}

#[test]
fn a_return_edge_drops_each_open_scope_and_emits_no_scope_end() {
    let source = with_view_helper(
        "func demo(): void {\n\
             let s: String = $\"x{1}\";\n\
             { let t: String = $\"y{2}\"; return; }\n\
         }",
    );
    let demo = demo_function(&source);
    let dropped: Vec<usize> = demo
        .blocks
        .iter()
        .flat_map(|block| block.statements.iter())
        .filter_map(|statement| match &statement.statement {
            Statement::Drop { scope, .. } => Some(*scope),
            _ => None,
        })
        .collect();
    assert!(
        dropped.contains(&1) && dropped.contains(&2),
        "the return edge drops both open scopes: {dropped:?}"
    );
    assert!(
        !demo.blocks.iter().any(|block| block
            .statements
            .iter()
            .any(|statement| matches!(statement.statement, Statement::ScopeEnd { .. }))),
        "a return edge must not emit a scope-end marker"
    );
}
