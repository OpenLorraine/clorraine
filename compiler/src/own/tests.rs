//! Implementation tests for the ownership stage.
//!
//! The tests drive small programs through the shared front-end helper and
//! assert on the diagnostics and on the lowered IR. They cover the dataflow
//! joins, the out-parameter slot carve-out, and a snapshot of the lowered
//! control-flow graph.

use super::test_support::{compile, error_codes, reports};
use crate::ir::{Statement, Terminator};

/// A module header with a view helper used by the loan tests.
fn with_view_helper(body: &str) -> String {
    format!(
        "module t;\n\
         func viewOf(s: str): str {{ return s; }}\n\
         func consume(take s: String): void {{}}\n\
         func observe(s: str): void {{}}\n\
         {body}\n\
         func main(): i32 {{ return 0; }}\n"
    )
}

#[test]
fn a_move_on_one_branch_survives_the_join() {
    let source = with_view_helper(
        "func demo(flag: bool): void {\n\
             let s: String = $\"x{1}\";\n\
             if (flag) { consume(s); }\n\
             consume(s);\n\
         }",
    );
    assert_eq!(error_codes(&source), vec!["E10020"]);
}

#[test]
fn an_assignment_on_both_branches_is_initialized() {
    let source = "module t;\n\
        func demo(flag: bool): i32 {\n\
            let x: i32;\n\
            if (flag) { x = 1; } else { x = 2; }\n\
            return x;\n\
        }\n\
        func main(): i32 { return 0; }\n";
    assert!(error_codes(source).is_empty(), "{:?}", error_codes(source));
}

#[test]
fn an_assignment_on_one_branch_is_not_initialized() {
    let source = "module t;\n\
        func demo(flag: bool): i32 {\n\
            let x: i32;\n\
            if (flag) { x = 1; }\n\
            return x;\n\
        }\n\
        func main(): i32 { return 0; }\n";
    assert_eq!(error_codes(source), vec!["E10021"]);
}

#[test]
fn a_terminated_branch_does_not_poison_the_join() {
    // The early-return pattern: the `return` branch cannot reach the join, so
    // the narrowed `f` on the fall-through path survives.
    let source = "module t;\n\
        handle File freedBy CloseFile;\n\
        ffi lib {\n\
            func OpenFile(): File?;\n\
            func CloseFile(take f: File): void;\n\
        }\n\
        func use(f: File): void {}\n\
        func demo(): void {\n\
            let f: File? = OpenFile();\n\
            if (f == null) { return; }\n\
            use(f);\n\
        }\n\
        func main(): i32 { return 0; }\n";
    assert!(error_codes(source).is_empty(), "{:?}", error_codes(source));
}

#[test]
fn an_uninitialized_out_parameter_slot_is_rejected() {
    let source = "module t;\n\
        handle File freedBy CloseFile;\n\
        ffi lib {\n\
            func OpenInto(borrow out: File?): void;\n\
            func CloseFile(take f: File): void;\n\
        }\n\
        func demo(): void {\n\
            let f: File?;\n\
            OpenInto(borrow f);\n\
        }\n\
        func main(): i32 { return 0; }\n";
    assert_eq!(error_codes(source), vec!["E10021"]);
}

#[test]
fn an_initialized_out_parameter_slot_is_accepted() {
    let source = "module t;\n\
        handle File freedBy CloseFile;\n\
        ffi lib {\n\
            func OpenInto(borrow out: File?): void;\n\
            func CloseFile(take f: File): void;\n\
        }\n\
        func demo(): void {\n\
            let f: File? = null;\n\
            OpenInto(borrow f);\n\
            if (f != null) { CloseFile(f); }\n\
        }\n\
        func main(): i32 { return 0; }\n";
    assert!(error_codes(source).is_empty(), "{:?}", error_codes(source));
}

#[test]
fn a_borrow_out_slot_is_exempt_from_the_release_rule() {
    let source = "module t;\n\
        func fill(borrow out: String?): void {\n\
            out = $\"x{1}\";\n\
        }\n\
        func main(): i32 { return 0; }\n";
    assert!(!reports(source, "E10034"), "{:?}", error_codes(source));
}

#[test]
fn a_partially_initialized_struct_field_is_tracked() {
    let source = "module t;\n\
        struct Pair { a: i32; b: i32; }\n\
        func demo(): i32 {\n\
            let p: Pair;\n\
            p.a = 1;\n\
            return p.b;\n\
        }\n\
        func main(): i32 { return 0; }\n";
    assert_eq!(error_codes(source), vec!["E10021"]);
}

#[test]
fn reassigning_a_moved_field_reinitializes_the_struct() {
    let source = "module t;\n\
        struct Pair { left: String; right: String; }\n\
        func observePair(p: Pair): void {}\n\
        func demo(): void {\n\
            let p = Pair { left: $\"a{1}\", right: $\"b{2}\" };\n\
            let l: String = p.left;\n\
            p.left = $\"c{3}\";\n\
            observePair(p);\n\
            observe(l);\n\
        }\n\
        func observe(s: String): void {}\n\
        func main(): i32 { return 0; }\n";
    assert!(error_codes(source).is_empty(), "{:?}", error_codes(source));
}

#[test]
fn a_switch_merges_clauses_with_a_default_aware_join() {
    let source = "module t;\n\
        func demo(n: i32): i32 {\n\
            let x: i32;\n\
            switch (n) {\n\
                case 1: x = 1; break;\n\
                default: break;\n\
            }\n\
            return x;\n\
        }\n\
        func main(): i32 { return 0; }\n";
    assert_eq!(error_codes(source), vec!["E10021"]);
}

#[test]
fn a_switch_with_every_clause_assigning_is_initialized() {
    let source = "module t;\n\
        func demo(n: i32): i32 {\n\
            let x: i32;\n\
            switch (n) {\n\
                case 1: x = 1; break;\n\
                default: x = 2; break;\n\
            }\n\
            return x;\n\
        }\n\
        func main(): i32 { return 0; }\n";
    assert!(error_codes(source).is_empty(), "{:?}", error_codes(source));
}

#[test]
fn a_loop_move_is_seen_on_the_next_iteration() {
    let source = with_view_helper(
        "func demo(): void {\n\
             let s: String = $\"x{1}\";\n\
             let i: i32 = 0;\n\
             while (i < 3) { consume(s); i = i + 1; }\n\
         }",
    );
    assert_eq!(error_codes(&source), vec!["E10020"]);
}

#[test]
fn a_deferred_write_does_not_leak_into_the_enclosing_flow() {
    let source = "module t;\n\
        func init(): i32 { return 5; }\n\
        func demo(): i32 {\n\
            let x: i32;\n\
            defer { x = init(); }\n\
            return x;\n\
        }\n\
        func main(): i32 { return 0; }\n";
    assert_eq!(error_codes(source), vec!["E10021"]);
}

#[test]
fn a_deferred_move_is_checked_at_the_exit_edge() {
    let source = with_view_helper(
        "func demo(): void {\n\
             let s: String = $\"x{1}\";\n\
             defer { consume(s); }\n\
             consume(s);\n\
         }",
    );
    assert_eq!(error_codes(&source), vec!["E10020"]);
}

#[test]
fn the_lowered_ir_has_a_branch_a_drop_and_a_join() {
    let source = with_view_helper(
        "func demo(flag: bool): void {\n\
             let s: String = $\"x{1}\";\n\
             if (flag) { observe($\"a{2}\"); return; }\n\
             observe($\"b{3}\");\n\
         }",
    );
    let (_, output) = compile(&source);
    let output = output.expect("the clean program lowers");
    let demo = output
        .functions
        .iter()
        .find(|function| function.name == "demo")
        .expect("the demo function is lowered");

    assert!(demo.blocks.len() >= 4, "expected a branch and a join");
    assert!(demo
        .blocks
        .iter()
        .any(|block| matches!(block.terminator, Terminator::Branch { .. })));
    assert!(demo.blocks.iter().any(|block| block
        .statements
        .iter()
        .any(|statement| matches!(statement, Statement::Drop(_)))));
    assert!(demo
        .blocks
        .iter()
        .any(|block| matches!(block.terminator, Terminator::Return(_))));
}

#[test]
fn the_lowered_ir_has_a_switch_terminator() {
    let source = "module t;\n\
        func demo(n: i32): void {\n\
            switch (n) {\n\
                case 1: observe(1); break;\n\
                default: observe(2); break;\n\
            }\n\
        }\n\
        func observe(n: i32): void {}\n\
        func main(): i32 { return 0; }\n";
    let (_, output) = compile(source);
    let output = output.expect("the clean program lowers");
    let demo = output
        .functions
        .iter()
        .find(|function| function.name == "demo")
        .expect("the demo function is lowered");
    assert!(demo
        .blocks
        .iter()
        .any(|block| matches!(block.terminator, Terminator::Switch { .. })));
}

#[test]
fn a_break_edge_runs_the_scopes_it_leaves() {
    let source = with_view_helper(
        "func demo(c: bool): void {\n\
             let s: String = $\"x{1}\";\n\
             while (true) {\n\
                 if (c) { break; }\n\
                 observe($\"y{2}\");\n\
             }\n\
             consume(s);\n\
         }",
    );
    // The loop never moves `s`, so the break path and the code after it are
    // clean; the lowered graph still has the break's exit edge.
    assert!(error_codes(&source).is_empty(), "{:?}", error_codes(&source));
    let (_, output) = compile(&source);
    let output = output.expect("the clean program lowers");
    let demo = output
        .functions
        .iter()
        .find(|function| function.name == "demo")
        .expect("the demo function is lowered");
    assert!(demo
        .blocks
        .iter()
        .any(|block| matches!(block.terminator, Terminator::Goto(_))));
}

#[test]
fn a_break_edge_drops_the_locals_it_leaves() {
    let source = with_view_helper(
        "func demo(c: bool): void {\n\
             while (true) {\n\
                 let r: String = $\"x{1}\";\n\
                 if (c) { break; }\n\
                 observe(r);\n\
             }\n\
         }",
    );
    let (_, output) = compile(&source);
    let output = output.expect("the clean program lowers");
    let demo = output
        .functions
        .iter()
        .find(|function| function.name == "demo")
        .expect("the demo function is lowered");
    assert!(
        demo.blocks.iter().any(|block| {
            matches!(block.terminator, Terminator::Goto(_))
                && block
                    .statements
                    .iter()
                    .any(|statement| matches!(statement, Statement::Drop(_)))
        }),
        "the break edge must drop the loop body's local"
    );
}

#[test]
fn a_borrow_lend_clears_a_caller_narrowing() {
    let source = "module t;\n\
        handle File freedBy CloseFile;\n\
        ffi lib {\n\
            func OpenFile(): File?;\n\
            func Reset(borrow f: File?): void;\n\
            func CloseFile(take f: File): void;\n\
        }\n\
        func demo(): void {\n\
            let f: File? = OpenFile();\n\
            if (f != null) { Reset(f); CloseFile(f); }\n\
        }\n\
        func main(): i32 { return 0; }\n";
    assert_eq!(error_codes(source), vec!["E10010"]);
}

#[test]
fn a_const_global_may_be_lent_but_a_mutable_one_may_not() {
    let source = "module t;\n\
        const NAME: static str = \"x\";\n\
        func sink(s: str): void {}\n\
        func demo(): void { sink(NAME); }\n\
        func main(): i32 { return 0; }\n";
    assert!(error_codes(source).is_empty(), "{:?}", error_codes(source));

    let mutable = "module t;\n\
        let MUT: str = \"y\";\n\
        func sink(s: str): void {}\n\
        func demo(): void { sink(MUT); }\n\
        func main(): i32 { return 0; }\n";
    assert_eq!(error_codes(mutable), vec!["E10038"]);
}

#[test]
fn a_double_move_reports_once() {
    let source = with_view_helper(
        "func demo(): void {\n\
             let s: String = $\"x{1}\";\n\
             consume(s);\n\
             consume(s);\n\
         }",
    );
    assert_eq!(
        error_codes(&source).iter().filter(|code| *code == "E10020").count(),
        1
    );
}

#[test]
fn two_exclusive_lend_arguments_conflict() {
    let source = "module t;\n\
        func two(borrow a: String, borrow b: String): void {}\n\
        func demo(): void {\n\
            let s: String = $\"x{1}\";\n\
            two(s, s);\n\
        }\n\
        func main(): i32 { return 0; }\n";
    assert_eq!(error_codes(source), vec!["E10032"]);
}

#[test]
fn a_projected_nullable_field_is_reported() {
    let source = "module t;\n\
        handle File freedBy CloseFile;\n\
        ffi lib { func CloseFile(take f: File): void; }\n\
        struct Holder { f: File?; }\n\
        func use(f: File): void {}\n\
        func demo(h: Holder): void { use(h.f); }\n\
        func main(): i32 { return 0; }\n";
    assert_eq!(error_codes(source), vec!["E10010"]);
}

#[test]
fn a_writable_stdlib_view_from_a_read_only_borrow_is_rejected() {
    let source = "module t;\n\
        import { Buffer, bufferBytes } from std.mem;\n\
        func bad(b: Buffer): void {\n\
            let v: u8[] = bufferBytes(b);\n\
        }\n\
        func main(): i32 { return 0; }\n";
    assert_eq!(error_codes(source), vec!["E10041"]);
}

#[test]
fn a_second_writable_stdlib_view_conflicts() {
    let source = "module t;\n\
        import { Buffer, bufferNew, bufferBytes } from std.mem;\n\
        func demo(): void {\n\
            let b: Buffer = bufferNew(16);\n\
            let v: u8[] = bufferBytes(b);\n\
            let w: u8[] = bufferBytes(b);\n\
            observe(v);\n\
            observe(w);\n\
        }\n\
        func observe(v: u8[]): void {}\n\
        func main(): i32 { return 0; }\n";
    assert_eq!(error_codes(source), vec!["E10032"]);
}

#[test]
fn a_projected_field_view_escapes_a_local() {
    let source = "module t;\n\
        import { String, stringOf, bytes } from std.text;\n\
        struct Holder { name: String; }\n\
        func bad(): u8[] {\n\
            let h: Holder = Holder { name: stringOf(\"x\") };\n\
            return bytes(h.name);\n\
        }\n\
        func main(): i32 { return 0; }\n";
    assert_eq!(error_codes(source), vec!["E10035"]);
}

#[test]
fn a_member_view_keeps_its_loan_alive() {
    let source = "module t;\n\
        import { String, stringOf, bytes } from std.text;\n\
        struct Holder { name: String; }\n\
        func demo(): void {\n\
            let h: Holder = Holder { name: stringOf(\"x\") };\n\
            let v: u8[] = bytes(h.name);\n\
            h.name = stringOf(\"y\");\n\
            observe(v);\n\
        }\n\
        func observe(v: u8[]): void {}\n\
        func main(): i32 { return 0; }\n";
    assert_eq!(error_codes(source), vec!["E10031"]);
}

#[test]
fn raw_member_access_requires_unsafe() {
    let source = "module t;\n\
        struct Inner { x: u32; }\n\
        struct Outer { inner: raw Inner; }\n\
        func demo(o: Outer): u32 { return o.inner.x; }\n\
        func main(): i32 { return 0; }\n";
    assert_eq!(error_codes(source), vec!["E10600"]);
}

#[test]
fn raw_compound_assignment_requires_unsafe() {
    let source = "module t;\n\
        func demo(): void {\n\
            let p: raw u32 = raw u32(0x1000);\n\
            p += 4;\n\
        }\n\
        func main(): i32 { return 0; }\n";
    assert_eq!(error_codes(source), vec!["E10602"]);
}

#[test]
fn a_lambda_reports_its_staged_feature() {
    let source = "module t;\n\
        let gp: raw u32 = raw u32(0x1000);\n\
        func use(f: func(i32): u32): u32 { return f(1); }\n\
        func demo(): u32 {\n\
            let f = (x: i32) => *gp;\n\
            return use(f);\n\
        }\n\
        func main(): i32 { return 0; }\n";
    assert_eq!(error_codes(source), vec!["E90000"]);
}

#[test]
fn returning_a_borrow_parameter_is_rejected() {
    let source = "module t;\n\
        handle File freedBy Close;\n\
        ffi lib { func Close(take f: File): void; }\n\
        func leak(borrow f: File): File { return f; }\n\
        func main(): i32 { return 0; }\n";
    assert_eq!(error_codes(source), vec!["E10034"]);
}
