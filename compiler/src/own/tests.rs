//! Implementation tests for the ownership stage.
//!
//! The tests drive small programs through the shared front-end helper and
//! assert on the diagnostics and on the lowered IR. They cover the dataflow
//! joins, the out-parameter slot carve-out, and a snapshot of the lowered
//! control-flow graph.

use super::test_support::{compile, error_codes, reports, with_view_helper};
use crate::ir::{Statement, Terminator};

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
        .any(|statement| matches!(statement.statement, Statement::Drop { .. }))));
    assert!(demo
        .blocks
        .iter()
        .any(|block| matches!(block.terminator, Terminator::Return { .. })));
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
                    .any(|statement| matches!(statement.statement, Statement::Drop { .. }))
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
fn returning_a_borrow_parameter_is_rejected() {
    let source = "module t;\n\
        handle File freedBy Close;\n\
        ffi lib { func Close(take f: File): void; }\n\
        func leak(borrow f: File): File { return f; }\n\
        func main(): i32 { return 0; }\n";
    assert_eq!(error_codes(source), vec!["E10034"]);
}

#[test]
fn an_element_loop_move_is_seen_on_the_next_iteration() {
    // The element loop is analyzed to a fixed point, so the move in the body
    // comes back around and the second iteration's reads are reported.
    let source = with_view_helper(
        "func demo(xs: i32[]): void {\n\
             let s: String = $\"x{1}\";\n\
             for (e in xs) { observe(s); consume(s); }\n\
         }",
    );
    assert!(reports(&source, "E10020"), "{:?}", error_codes(&source));
}

#[test]
fn a_default_only_switch_initializes_after_the_switch() {
    let source = "module t;\n\
        func demo(n: i32): i32 {\n\
            let x: i32;\n\
            switch (n) { default: x = 2; }\n\
            return x;\n\
        }\n\
        func main(): i32 { return 0; }\n";
    assert!(error_codes(source).is_empty(), "{:?}", error_codes(source));
}

#[test]
fn a_falling_default_clause_initializes_after_the_switch() {
    let source = "module t;\n\
        func demo(n: i32): i32 {\n\
            let x: i32;\n\
            switch (n) { case 1: x = 1; default: x = 2; }\n\
            return x;\n\
        }\n\
        func main(): i32 { return 0; }\n";
    assert!(error_codes(source).is_empty(), "{:?}", error_codes(source));
}

#[test]
fn a_terminated_loop_body_does_not_poison_the_entry() {
    let source = with_view_helper(
        "func demo(): void {\n\
             let s: String = $\"x{1}\";\n\
             while (true) { consume(s); break; }\n\
         }",
    );
    assert!(error_codes(&source).is_empty(), "{:?}", error_codes(&source));
}

#[test]
fn a_terminated_loop_body_still_initializes_on_exit() {
    let source = "module t;\n\
        func demo(): i32 {\n\
            let x: i32;\n\
            while (true) { x = 1; break; }\n\
            return x;\n\
        }\n\
        func main(): i32 { return 0; }\n";
    assert!(error_codes(source).is_empty(), "{:?}", error_codes(source));
}

#[test]
fn a_non_nullable_borrow_lend_keeps_a_caller_narrowing() {
    // A `borrow T` of a non-nullable `T` cannot write `null`, so the earlier
    // null check still holds after the call.
    let source = "module t;\n\
        handle File freedBy CloseFile;\n\
        ffi lib {\n\
            func OpenFile(): File?;\n\
            func CloseFile(take f: File): void;\n\
        }\n\
        func touch(borrow f: File): void {}\n\
        func use(f: File): void {}\n\
        func demo(): void {\n\
            let f: File? = OpenFile();\n\
            if (f == null) { return; }\n\
            touch(borrow f);\n\
            use(f);\n\
        }\n\
        func main(): i32 { return 0; }\n";
    assert!(error_codes(source).is_empty(), "{:?}", error_codes(source));
}

#[test]
fn a_struct_written_field_by_field_may_be_read_whole() {
    let source = "module t;\n\
        struct Pair { a: i32; b: i32; }\n\
        func observe(p: Pair): void {}\n\
        func demo(): void {\n\
            let p: Pair;\n\
            p.a = 1;\n\
            p.b = 2;\n\
            observe(p);\n\
        }\n\
        func main(): i32 { return 0; }\n";
    assert!(error_codes(source).is_empty(), "{:?}", error_codes(source));
}

#[test]
fn a_partially_written_struct_may_not_be_read_whole() {
    let source = "module t;\n\
        struct Pair { a: i32; b: i32; }\n\
        func observe(p: Pair): void {}\n\
        func demo(): void {\n\
            let p: Pair;\n\
            p.a = 1;\n\
            observe(p);\n\
        }\n\
        func main(): i32 { return 0; }\n";
    assert_eq!(error_codes(source), vec!["E10021"]);
}

#[test]
fn a_generic_struct_holding_a_handle_is_move_only() {
    // `Box<String>` owns its `String`, so the second use after a move is the
    // moved-value error; reading the declared `T` would call it duplicable and
    // let the handle be copied.
    let source = with_view_helper(
        "struct Box<T> { value: T; }\n\
         func dropBox(take b: Box<String>): void {}\n\
         func demo(): void {\n\
             let a = Box<String> { value: $\"x{1}\" };\n\
             let b = a;\n\
             dropBox(a);\n\
         }",
    );
    assert_eq!(error_codes(&source), vec!["E10020"]);
}

#[test]
fn a_generic_struct_holding_a_copy_value_is_duplicable() {
    // The same shape with an `i32` argument copies, so `a` may be used again.
    let source = with_view_helper(
        "struct Box<T> { value: T; }\n\
         func demo(): void {\n\
             let a = Box<i32> { value: 1 };\n\
             let b = a;\n\
             let c = a;\n\
         }",
    );
    assert!(error_codes(&source).is_empty(), "{:?}", error_codes(&source));
}

#[test]
fn a_duplicable_literal_field_is_copied_not_moved() {
    // An `i32` field is duplicable, so consuming it in a literal copies it and
    // the source stays usable.
    let source = "module t;\n\
        struct Pair<T> { first: T; second: T; }\n\
        func demo(): void {\n\
            let p = Pair<i32> { first: 1, second: 2 };\n\
            let q = Pair<i32> { first: p.first, second: p.second };\n\
        }\n\
        func main(): i32 { return 0; }\n";
    assert!(error_codes(source).is_empty(), "{:?}", error_codes(source));
}

#[test]
fn a_type_parameter_literal_field_is_not_moved() {
    // A field still written in terms of `T` is left to the instantiation, so
    // rebuilding a generic struct from its own fields is accepted.
    let source = "module t;\n\
        struct Pair<T> { first: T; second: T; }\n\
        func rebuild<T>(p: Pair<T>): Pair<T> {\n\
            return Pair<T> { first: p.first, second: p.second };\n\
        }\n\
        func main(): i32 { return 0; }\n";
    assert!(error_codes(source).is_empty(), "{:?}", error_codes(source));
}


/// A source with a nested shadow and a sibling shadow of the same name.
const SHADOWED_LOCALS: &str = "module t;\n\
    func demo(): i32 {\n\
        let x: i32 = 1;\n\
        if (x > 0) {\n\
            let x: i32 = 2;\n\
        }\n\
        {\n\
            let x: i32 = 3;\n\
        }\n\
        return x;\n\
    }\n\
    func main(): i32 { return 0; }\n";

#[test]
fn shadowing_bindings_get_unique_and_deterministic_ir_spellings() {
    // The first binding of a source name keeps that name; every later binding
    // appends its declaration index, so a nested shadow and a sibling shadow
    // both stay distinct. Compiling again must reproduce the same spellings,
    // because speculative passes redeclare at the same indices.
    let spelling = |source: &str| {
        let (diagnostics, output) = compile(source);
        assert_eq!(
            diagnostics.error_count(),
            0,
            "the program types: {:?}",
            error_codes(source)
        );
        let function = output
            .expect("the ownership stage ran")
            .functions
            .into_iter()
            .find(|function| function.name == "demo")
            .expect("demo is lowered");
        function
            .locals
            .into_iter()
            .map(|local| local.name)
            .collect::<Vec<_>>()
    };
    assert_eq!(spelling(SHADOWED_LOCALS), vec!["x", "x#1", "x#2"]);
    assert_eq!(spelling(SHADOWED_LOCALS), vec!["x", "x#1", "x#2"]);
}

#[test]
fn a_shadowing_initializer_moves_the_outer_binding() {
    // The initializer's name ties to the outer binding, so the outer value
    // moves once into the shadowing local and both bindings stay sound.
    let source = with_view_helper(
        "func demo(): void {\n\
             let s: String = $\"x{1}\";\n\
             let s: String = s;\n\
             observe(s);\n\
         }",
    );
    assert!(error_codes(&source).is_empty(), "{:?}", error_codes(&source));
}

#[test]
fn a_moved_outer_binding_is_not_usable_after_a_shadowing_move() {
    // The shadowing initializer moves the outer `s`, so using the outer
    // binding after the inner block is a use of a moved value (E10020).
    let source = with_view_helper(
        "func demo(): void {\n\
             let s: String = $\"x{1}\";\n\
             {\n\
                 let s: String = s;\n\
             }\n\
             observe(s);\n\
         }",
    );
    assert_eq!(error_codes(&source), vec!["E10020"]);
}

#[test]
fn a_shadowing_move_from_a_borrow_parameter_is_rejected() {
    // A `borrow` parameter does not own its value, so a shadowing initializer
    // cannot move it out; the whole move is E10034.
    let source = with_view_helper(
        "func demo(borrow s: String): void {\n\
             let s: String = s;\n\
         }",
    );
    assert_eq!(error_codes(&source), vec!["E10034"]);
}

#[test]
fn a_shadowing_view_self_borrow_keeps_the_outer_loan() {
    // The inner view re-derives from the outer view, so the owner stays
    // borrowed; writing the owner while the view is used is E10031.
    let source = with_view_helper(
        "func demo(): void {\n\
             let x: String = $\"x{1}\";\n\
             let v: str = x;\n\
             {\n\
                 let v: str = v;\n\
                 x = $\"y{2}\";\n\
                 observe(v);\n\
             }\n\
         }",
    );
    assert_eq!(error_codes(&source), vec!["E10031"]);
}

#[test]
fn an_element_of_a_call_result_cannot_be_moved() {
    // The call owns its storage only until the statement ends, so an element
    // of the result has no owner the destination could take over.
    let source = "module t;\n\
        import { String, stringOf } from std.text;\n\
        func mk(): String[2] {\n\
            return [stringOf(\"ab\"), stringOf(\"cde\")];\n\
        }\n\
        func demo(): void {\n\
            let s: String = mk()[0];\n\
        }\n\
        func main(): i32 { return 0; }\n";
    assert_eq!(error_codes(source), vec!["E10026"]);
}

#[test]
fn a_nested_element_of_a_call_result_cannot_be_moved() {
    // An index chain rooted in a call resolves to no place at all, so the
    // element-move limit applies at any depth.
    let source = "module t;\n\
        import { String, stringOf } from std.text;\n\
        func mk(): String[2][2] {\n\
            return [[stringOf(\"ab\"), stringOf(\"cd\")], [stringOf(\"ef\"), stringOf(\"gh\")]];\n\
        }\n\
        func demo(): void {\n\
            let s: String = mk()[0][0];\n\
        }\n\
        func main(): i32 { return 0; }\n";
    assert_eq!(error_codes(source), vec!["E10026"]);
}

#[test]
fn a_place_element_moved_to_a_projected_target_is_rejected() {
    // The element belongs to a local, but the target is a projected field, so
    // the store is a partial move outside a top-level field.
    let source = "module t;\n\
        import { String, stringOf } from std.text;\n\
        struct Holder { s: String; }\n\
        func demo(): void {\n\
            let a: String[2] = [stringOf(\"a\"), stringOf(\"bb\")];\n\
            let h: Holder = Holder { s: stringOf(\"x\") };\n\
            h.s = a[0];\n\
        }\n\
        func main(): i32 { return 0; }\n";
    assert_eq!(error_codes(source), vec!["E10026"]);
}

#[test]
fn a_top_level_field_can_still_move_to_a_projected_target() {
    // A top-level field is the one partial move v1 allows, so the move is
    // recorded without reporting the element-move limit.
    let source = "module t;\n\
        import { String, stringOf } from std.text;\n\
        struct Holder { s: String; }\n\
        func demo(): void {\n\
            let h: Holder = Holder { s: stringOf(\"x\") };\n\
            let b: Holder = Holder { s: stringOf(\"y\") };\n\
            b.s = h.s;\n\
        }\n\
        func main(): i32 { return 0; }\n";
    assert!(error_codes(source).is_empty(), "{:?}", error_codes(source));
}

#[test]
fn a_move_through_a_borrow_element_view_is_rejected() {
    // The view never owns its element, so a `take` argument reading it is the
    // v1 element-move limit rather than a lend of the view.
    let source = with_view_helper(
        "func demo(): void {\n\
             let xs: String[2] = [$\"a{1}\", $\"bb{2}\"];\n\
             for (borrow e in xs) {\n\
                 consume(e);\n\
             }\n\
         }",
    );
    assert_eq!(error_codes(&source), vec!["E10026"]);
}

#[test]
fn a_copy_form_element_binding_owns_its_copy() {
    // Without `borrow`, each iteration copies the duplicable row, so reading
    // the copy is an ordinary read of an owned local.
    let source = with_view_helper(
        "func demo(): void {\n\
             let m: i32[2][3] = [[1, 2, 3], [4, 5, 6]];\n\
             let sum: i32 = 0;\n\
             for (row in m) {\n\
                 let copy: i32[3] = row;\n\
                 sum = sum + copy[0];\n\
             }\n\
         }",
    );
    assert!(error_codes(&source).is_empty(), "{:?}", error_codes(&source));
}
