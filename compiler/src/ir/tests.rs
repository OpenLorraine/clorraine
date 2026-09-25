//! Unit tests for the typed control-flow IR.
//!
//! The tests build small functions with the [`Builder`] and pin the shape of
//! the emitted blocks, statements, and scope records without involving the
//! ownership walk or the C emitter.

use super::*;
use crate::span::Span;

/// A parameter of `i32`.
fn i32_param(name: &str) -> Param {
    Param {
        name: name.to_owned(),
        ty: Ty::i32_ty(),
        mode: Mode::None,
    }
}

#[test]
fn a_straight_line_function_keeps_one_block_with_an_explicit_exit() {
    let mut builder = Builder::new("demo", vec![i32_param("s")], Ty::i32_ty());
    let local = Place::local("t").with_type(Ty::i32_ty());
    builder.emit(Statement::Move {
        target: local.clone(),
        source: Place::param("s").with_type(Ty::i32_ty()),
    });
    builder.emit(Statement::Drop {
        place: local.clone(),
        scope: 0,
        order: 0,
    });
    builder.terminate(Terminator::Return(None));
    let function = builder.finish();

    assert_eq!(function.entry, 0);
    assert_eq!(function.blocks.len(), 1);
    assert_eq!(function.params, vec![i32_param("s")]);
    assert_eq!(function.ret, Ty::i32_ty());
    assert!(matches!(
        function.blocks[0].statements[0].statement,
        Statement::Move { .. }
    ));
    assert!(matches!(
        function.blocks[0].statements[1].statement,
        Statement::Drop { .. }
    ));
    assert!(matches!(
        function.blocks[0].terminator,
        Terminator::Return(None)
    ));
}

#[test]
fn a_statement_and_terminator_carry_their_source_span() {
    let mut builder = Builder::new("demo", Vec::new(), Ty::void());
    let span = Span::default();
    builder.set_span(span);
    builder.emit(Statement::Nop);
    builder.terminate(Terminator::Return(None));
    let function = builder.finish();
    assert_eq!(function.blocks[0].statements[0].span, span);
    assert_eq!(function.blocks[0].terminator_span, span);
}

#[test]
fn locals_and_temps_are_collected_in_order() {
    let mut builder = Builder::new("demo", Vec::new(), Ty::void());
    builder.add_local("a", Ty::i32_ty());
    builder.add_local("b", Ty::boolean());
    let first = builder.add_temp(Ty::i32_ty());
    let second = builder.add_temp(Ty::boolean());
    let function = builder.finish();
    assert_eq!(first, 0);
    assert_eq!(second, 1);
    assert_eq!(function.locals.len(), 2);
    assert_eq!(function.locals[0].name, "a");
    assert_eq!(function.locals[1].ty, Ty::boolean());
    assert_eq!(function.temps, vec![Ty::i32_ty(), Ty::boolean()]);
}

#[test]
fn branching_creates_separate_blocks_and_a_join() {
    let mut builder = Builder::new("pick", vec![i32_param("flag")], Ty::void());
    let then_block = builder.new_block();
    let else_block = builder.new_block();
    let join = builder.new_block();
    builder.emit(Statement::Nop);
    builder.terminate(Terminator::Branch {
        condition: Value::bool(Ty::boolean(), true),
        on_true: then_block,
        on_false: else_block,
        shape: BranchShape::Plain,
    });
    builder.switch_to(then_block);
    builder.terminate(Terminator::Goto(join));
    builder.switch_to(else_block);
    builder.terminate(Terminator::Goto(join));
    builder.switch_to(join);
    builder.terminate(Terminator::Return(None));

    let function = builder.finish();
    assert_eq!(function.blocks.len(), 4);
    assert!(matches!(
        function.blocks[0].terminator,
        Terminator::Branch { .. }
    ));
    assert!(matches!(
        function.blocks[3].terminator,
        Terminator::Return(None)
    ));
}

#[test]
fn a_projected_place_reports_its_shape_and_type() {
    let place = Place {
        base: PlaceBase::Local("pair".to_owned()),
        projections: vec![Projection::Field("left".to_owned())],
        ty: Ty::i32_ty(),
    };
    assert!(!place.is_plain());
    assert_eq!(place.ty, Ty::i32_ty());
    assert_eq!(Place::local("pair").is_plain(), true);
}

#[test]
fn scopes_are_recorded_with_their_parent_and_loop_flag() {
    let mut builder = Builder::new("demo", Vec::new(), Ty::void());
    let body = builder.add_scope(None, false, Span::default());
    let loop_body = builder.add_scope(Some(body), true, Span::default());
    assert_eq!(body, 0);
    assert_eq!(loop_body, 1);
    let function = builder.finish();
    assert_eq!(
        function.scopes,
        vec![
            ScopeInfo {
                parent: None,
                in_loop: false,
                close_span: Span::default(),
            },
            ScopeInfo {
                parent: Some(0),
                in_loop: true,
                close_span: Span::default(),
            },
        ]
    );
}

#[test]
fn instances_deduplicate_by_function_and_type_arguments() {
    let mut mono = Monomorphization::default();
    let first = Instance {
        function: "identity".to_owned(),
        args: vec![crate::types::Ty::i32_ty()],
    };
    assert!(mono.record(first.clone()));
    assert!(!mono.record(first.clone()));
    assert!(mono.record(Instance {
        function: "identity".to_owned(),
        args: vec![crate::types::Ty::scalar(crate::ast::ScalarType::U8)],
    }));
    assert_eq!(mono.instances().len(), 2);
    assert_eq!(first.dedupe_key(), "identity<i32>");
}

#[test]
fn struct_instances_deduplicate_by_name_and_arguments() {
    let mut mono = Monomorphization::default();
    let first = StructInstance {
        name: "Pair".to_owned(),
        args: vec![crate::types::Ty::i32_ty()],
    };
    assert!(mono.record_struct(first.clone()));
    assert!(!mono.record_struct(first.clone()));
    assert!(mono.record_struct(StructInstance {
        name: "Pair".to_owned(),
        args: vec![crate::types::Ty::usize_ty()],
    }));
    assert_eq!(mono.struct_instances().len(), 2);
    assert_eq!(first.dedupe_key(), "Pair<i32>");
}

#[test]
fn copy_obligations_name_their_instantiation() {
    let mut mono = Monomorphization::default();
    mono.require_copy(CopyObligation {
        function: "append".to_owned(),
        instance: "append<u8>".to_owned(),
        parameter: "value".to_owned(),
        ty: "u8".to_owned(),
    });
    assert_eq!(mono.obligations().len(), 1);
    assert_eq!(mono.obligations()[0].instance, "append<u8>");
}
