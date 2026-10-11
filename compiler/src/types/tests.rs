use super::*;

/// A span for a synthetic expression; only `int_literal_value` reads it.
fn span() -> crate::span::Span {
    let mut sources = crate::span::SourceMap::new();
    let id = sources.add_file("test.clo".to_owned(), String::new());
    crate::span::Span::new(id, 0, 0)
}

/// An integer literal expression.
fn int_literal(value: u64) -> Expr {
    Expr {
        kind: ExprKind::Int(value),
        span: span(),
    }
}

/// A `-operand` expression.
fn negated(operand: Expr) -> Expr {
    Expr {
        kind: ExprKind::Unary {
            op: crate::ast::UnaryOp::Neg,
            operand: Box::new(operand),
        },
        span: span(),
    }
}

/// A non-nullable type with the given kind.
fn ty(kind: TyKind) -> Ty {
    Ty {
        kind,
        nullable: false,
    }
}

#[test]
fn implicit_conversion_matrix_is_exactly_the_t1a_widenings() {
    use ScalarType::*;
    let scalars = [I8, I16, I32, U8, U16, U32, F32, F64, Bool, Usize, Isize];
    let allowed = [
        (U8, U16),
        (U8, U32),
        (U16, U32),
        (I8, I16),
        (I8, I32),
        (I16, I32),
        (F32, F64),
    ];
    for from in scalars {
        for to in scalars {
            let expected = from == to || allowed.contains(&(from, to));
            assert_eq!(
                implicit_conversion(&Ty::scalar(from), &Ty::scalar(to)),
                expected,
                "{} -> {}",
                scalar_name(from),
                scalar_name(to)
            );
        }
    }
}

#[test]
fn views_handles_and_aggregates_convert_only_where_allowed() {
    let str_ty = ty(TyKind::Str);
    let static_str = ty(TyKind::StaticStr);
    let cstring = ty(TyKind::CString);
    assert!(implicit_conversion(&static_str, &str_ty));
    assert!(!implicit_conversion(&str_ty, &static_str));
    assert!(implicit_conversion(&Ty::handle("String"), &str_ty));
    assert!(!implicit_conversion(&Ty::handle("File"), &str_ty));
    assert!(!implicit_conversion(&str_ty, &cstring));
    // A `static str` is converted to `cstring` only as a literal call argument
    // (checked in the call stage), not by the general type rule.
    assert!(!implicit_conversion(&static_str, &cstring));
    assert!(implicit_conversion(
        &Ty::struct_("Point"),
        &Ty::struct_("Point")
    ));
    assert!(!implicit_conversion(
        &Ty::struct_("Point"),
        &Ty::struct_("Other")
    ));
    assert!(implicit_conversion(&Ty::enum_("E"), &Ty::enum_("E")));
    assert!(!implicit_conversion(&Ty::enum_("E"), &Ty::i32_ty()));
    assert!(!implicit_conversion(
        &Ty::array(Ty::i32_ty(), vec![Some(3)]),
        &str_ty
    ));
}

#[test]
fn unknown_and_parameter_types_convert_freely() {
    let param = ty(TyKind::Param("T".to_owned()));
    assert!(implicit_conversion(&Ty::unknown(), &Ty::i32_ty()));
    assert!(implicit_conversion(&Ty::i32_ty(), &Ty::unknown()));
    assert!(implicit_conversion(&param, &Ty::void()));
    assert!(implicit_conversion(&Ty::void(), &param));
    assert!(param.is_param());
    assert!(!param.is_unknown());
}

#[test]
fn nullability_does_not_change_implicit_conversion() {
    let nullable = Ty {
        kind: TyKind::Scalar(ScalarType::I32),
        nullable: true,
    };
    assert!(implicit_conversion(&nullable, &Ty::i32_ty()));
    assert!(implicit_conversion(&Ty::i32_ty(), &nullable));
}

#[test]
fn numeric_unification_picks_the_wider_operand() {
    use ScalarType::*;
    let u8_ty = Ty::scalar(U8);
    let u16_ty = Ty::scalar(U16);
    assert_eq!(unify_numeric(&u8_ty, &u16_ty), Some(u16_ty.clone()));
    assert_eq!(unify_numeric(&u16_ty, &u8_ty), Some(u16_ty.clone()));
    assert_eq!(
        unify_numeric(&Ty::scalar(F32), &Ty::scalar(F64)),
        Some(Ty::scalar(F64))
    );
    assert_eq!(
        unify_numeric(&Ty::scalar(I8), &Ty::scalar(I32)),
        Some(Ty::i32_ty())
    );
    assert_eq!(
        unify_numeric(&Ty::i32_ty(), &Ty::i32_ty()),
        Some(Ty::i32_ty())
    );
}

#[test]
fn numeric_unification_rejects_mixed_sign_and_non_widenings() {
    use ScalarType::*;
    assert_eq!(unify_numeric(&Ty::i32_ty(), &Ty::scalar(U32)), None);
    assert_eq!(unify_numeric(&Ty::scalar(U8), &Ty::scalar(I8)), None);
    assert_eq!(unify_numeric(&Ty::scalar(U16), &Ty::scalar(I32)), None);
    assert_eq!(unify_numeric(&Ty::scalar(F64), &Ty::scalar(I32)), None);
    assert_eq!(unify_numeric(&Ty::boolean(), &Ty::scalar(I32)), None);
    let param = ty(TyKind::Param("T".to_owned()));
    assert_eq!(
        unify_numeric(&Ty::unknown(), &Ty::scalar(U16)),
        Some(Ty::scalar(U16))
    );
    assert_eq!(unify_numeric(&Ty::scalar(U16), &param), Some(Ty::scalar(U16)));
}

#[test]
fn explicit_scalar_casts_cover_narrowing_and_cross_family_pairs() {
    use ScalarType::*;
    assert!(scalar_cast_allowed(&Ty::scalar(U16), &Ty::scalar(U8)));
    assert!(scalar_cast_allowed(&Ty::scalar(F64), &Ty::i32_ty()));
    assert!(scalar_cast_allowed(&Ty::boolean(), &Ty::scalar(U8)));
    assert!(scalar_cast_allowed(&Ty::scalar(U8), &Ty::boolean()));
    assert!(scalar_cast_allowed(&Ty::enum_("E"), &Ty::scalar(U16)));
    assert!(scalar_cast_allowed(
        &ty(TyKind::Raw(Box::new(Ty::i32_ty()))),
        &Ty::usize_ty()
    ));
}

#[test]
fn explicit_casts_reject_non_scalar_conversions() {
    assert!(!scalar_cast_allowed(&Ty::struct_("Point"), &Ty::i32_ty()));
    assert!(!scalar_cast_allowed(&Ty::i32_ty(), &Ty::enum_("E")));
    assert!(!scalar_cast_allowed(&Ty::enum_("E"), &Ty::struct_("S")));
    assert!(!scalar_cast_allowed(&ty(TyKind::Str), &Ty::i32_ty()));
    assert!(!scalar_cast_allowed(&Ty::i32_ty(), &ty(TyKind::Str)));
    assert!(scalar_cast_allowed(&Ty::unknown(), &Ty::struct_("S")));
    assert!(scalar_cast_allowed(
        &ty(TyKind::Param("T".to_owned())),
        &Ty::void()
    ));
}

#[test]
fn integer_literal_fit_boundaries_are_inclusive() {
    use ScalarType::*;
    assert!(int_fits(-128, I8));
    assert!(int_fits(127, I8));
    assert!(!int_fits(-129, I8));
    assert!(!int_fits(128, I8));
    assert!(int_fits(0, U8));
    assert!(int_fits(255, U8));
    assert!(!int_fits(-1, U8));
    assert!(!int_fits(256, U8));
    assert!(int_fits(-32768, I16));
    assert!(int_fits(32767, I16));
    assert!(!int_fits(32768, I16));
    assert!(int_fits(65535, U16));
    assert!(!int_fits(65536, U16));
    assert!(int_fits(-2147483648, I32));
    assert!(int_fits(2147483647, I32));
    assert!(!int_fits(2147483648, I32));
    assert!(int_fits(4294967295, U32));
    assert!(!int_fits(4294967296, U32));
    assert!(int_fits(-2147483648, Isize));
    assert!(!int_fits(2147483648, Isize));
    assert!(int_fits(4294967295, Usize));
    assert!(!int_fits(-1, Usize));
    assert!(int_fits(-999, F32));
    assert!(int_fits(999, Bool));
}

#[test]
fn integer_literal_value_reads_plain_and_negated_literals() {
    assert_eq!(int_literal_value(&int_literal(7)), Some(7));
    assert_eq!(int_literal_value(&negated(int_literal(7))), Some(-7));
    assert_eq!(int_literal_value(&negated(int_literal(0))), Some(0));
    let boolean = Expr {
        kind: ExprKind::Bool(true),
        span: span(),
    };
    assert_eq!(int_literal_value(&boolean), None);
    assert_eq!(int_literal_value(&negated(boolean)), None);
    let named = Expr {
        kind: ExprKind::Name("x".to_owned()),
        span: span(),
    };
    assert_eq!(int_literal_value(&named), None);
}

#[test]
fn substitution_replaces_parameters_inside_arrays_pointers_and_struct_args() {
    use std::collections::BTreeMap;
    let mut bindings = BTreeMap::new();
    bindings.insert("T".to_owned(), Ty::i32_ty());
    let param = || ty(TyKind::Param("T".to_owned()));

    assert_eq!(substitute(&param(), &bindings), Ty::i32_ty());

    // An array view substitutes its element and keeps both written extents.
    assert_eq!(
        substitute(&Ty::array(param(), vec![Some(2), None]), &bindings),
        Ty::array(Ty::i32_ty(), vec![Some(2), None])
    );

    // The nullability flag survives the recursion.
    let mut nullable = Ty::array(param(), vec![None]);
    nullable.nullable = true;
    let mut expected = Ty::array(Ty::i32_ty(), vec![None]);
    expected.nullable = true;
    assert_eq!(substitute(&nullable, &bindings), expected);

    assert_eq!(
        substitute(&Ty::raw(param()), &bindings),
        Ty::raw(Ty::i32_ty())
    );
    assert_eq!(
        substitute(&Ty::struct_with("Array", vec![param()]), &bindings),
        Ty::struct_with("Array", vec![Ty::i32_ty()])
    );
}

#[test]
fn a_fixed_array_element_folds_into_the_extents() {
    // A fixed row under a fixed outer extent is one flat type, the shape the C
    // declarator and the row stores assume.
    let nested = Ty::array(Ty::array(Ty::i32_ty(), vec![Some(3)]), vec![Some(2)]);
    assert_eq!(nested, Ty::array(Ty::i32_ty(), vec![Some(2), Some(3)]));
    assert_eq!(nested.display(), "i32[2][3]");
    // Folding an already-flat type is a no-op with the new extent outermost.
    let flat = Ty::array(Ty::i32_ty(), vec![Some(2), Some(3)]);
    assert_eq!(
        Ty::array(flat, vec![Some(4)]),
        Ty::array(Ty::i32_ty(), vec![Some(4), Some(2), Some(3)])
    );
}

#[test]
fn a_slice_element_or_outer_extent_never_folds() {
    // An array of slices is a different type from a slice of rows.
    let slices = Ty::array(Ty::array(Ty::i32_ty(), vec![None]), vec![Some(2)]);
    assert_eq!(slices.display(), "i32[][2]");
    assert_eq!(
        slices.iterated_element(),
        Ty::array(Ty::i32_ty(), vec![None])
    );
    // A slice outer extent keeps its fixed-array element inside the slice.
    let rows = Ty::array(Ty::array(Ty::i32_ty(), vec![Some(3)]), vec![None]);
    assert!(!rows.is_fixed_array());
    assert_eq!(
        rows.iterated_element(),
        Ty::array(Ty::i32_ty(), vec![Some(3)])
    );
}

#[test]
fn substitution_folds_a_fixed_array_binding_flat() {
    use std::collections::BTreeMap;
    let mut bindings = BTreeMap::new();
    bindings.insert("T".to_owned(), Ty::array(Ty::i32_ty(), vec![Some(3)]));
    let substituted = substitute(
        &Ty::array(ty(TyKind::Param("T".to_owned())), vec![Some(2)]),
        &bindings,
    );
    assert_eq!(
        substituted,
        Ty::array(Ty::i32_ty(), vec![Some(2), Some(3)])
    );
}

#[test]
fn substitution_leaves_unbound_and_parameter_free_types_alone() {
    use std::collections::BTreeMap;
    let mut bindings = BTreeMap::new();
    bindings.insert("T".to_owned(), Ty::i32_ty());
    let unbound = ty(TyKind::Param("U".to_owned()));
    assert_eq!(substitute(&unbound, &bindings), unbound);
    assert_eq!(substitute(&Ty::i32_ty(), &bindings), Ty::i32_ty());
    assert_eq!(substitute(&ty(TyKind::Str), &bindings), ty(TyKind::Str));
    // An empty substitution is a no-op even for a parameter.
    assert_eq!(substitute(&unbound, &BTreeMap::new()), unbound);
}

#[test]
fn type_predicates_and_display_match_the_surface_spelling() {
    use ScalarType::*;
    assert!(Ty::scalar(I32).is_integer());
    assert!(!Ty::scalar(F32).is_integer());
    assert!(!Ty::boolean().is_integer());
    assert!(Ty::scalar(F32).is_numeric());
    assert!(!Ty::boolean().is_numeric());
    assert!(Ty::boolean().is_bool());
    assert!(Ty::void().is_void());
    assert!(Ty::unknown().is_unknown());
    assert_eq!(Ty::scalar(I32).display(), "i32");
    assert_eq!(Ty::scalar(U16).display(), "u16");
    assert_eq!(Ty::void().display(), "void");
    assert_eq!(
        Ty {
            kind: TyKind::Scalar(U8),
            nullable: true
        }
        .display(),
        "u8?"
    );
    assert_eq!(ty(TyKind::StaticStr).display(), "static str");
    assert_eq!(ty(TyKind::CString).display(), "cstring");
    assert_eq!(ty(TyKind::Raw(Box::new(Ty::i32_ty()))).display(), "raw i32");
    assert_eq!(Ty::array(Ty::scalar(U8), vec![Some(4)]).display(), "u8[4]");
    assert_eq!(Ty::array(Ty::scalar(U8), vec![None]).display(), "u8[]");
    assert_eq!(
        Ty::array(Ty::scalar(U8), vec![Some(2), Some(3)]).display(),
        "u8[2][3]"
    );
    assert_eq!(Ty::handle("String").display(), "String");
    assert_eq!(Ty::unknown().display(), "?");
    let signature = FuncTy {
        params: vec![FuncParam {
            ty: Ty::i32_ty(),
            mode: Mode::None,
        }],
        returns: Box::new(Ty::boolean()),
    };
    assert_eq!(Ty::func(signature).display(), "func(i32): bool");
}

#[test]
fn the_shape_predicates_split_slices_from_fixed_arrays() {
    use ScalarType::U8;
    // A slice has at least one omitted extent.
    assert!(Ty::array(Ty::scalar(U8), vec![None]).is_slice());
    assert!(!Ty::array(Ty::scalar(U8), vec![None]).is_fixed_array());
    // A fixed array has every extent present.
    assert!(Ty::array(Ty::scalar(U8), vec![Some(4)]).is_fixed_array());
    assert!(!Ty::array(Ty::scalar(U8), vec![Some(4)]).is_slice());
    // A multi-dimensional fixed array is fixed; a partially omitted one is a
    // slice.
    assert!(Ty::array(Ty::scalar(U8), vec![Some(3), Some(2)]).is_fixed_array());
    assert!(Ty::array(Ty::scalar(U8), vec![None, Some(2)]).is_slice());
    assert!(!Ty::array(Ty::scalar(U8), vec![None, Some(2)]).is_fixed_array());
    // A scalar is neither shape. An empty extent list never comes from source,
    // and the predicates stay total on it: no omitted extent means not a slice.
    assert!(!Ty::scalar(U8).is_slice());
    assert!(!Ty::scalar(U8).is_fixed_array());
    assert!(!Ty::array(Ty::scalar(U8), vec![]).is_slice());
}

#[test]
fn a_multi_extent_array_iterates_its_rows() {
    // A fixed grid iterates rows: only the first extent is stripped.
    let grid = Ty::array(Ty::i32_ty(), vec![Some(2), Some(3)]);
    assert_eq!(
        grid.iterated_element(),
        Ty::array(Ty::i32_ty(), vec![Some(3)])
    );
    // A slice of rows yields the same fixed row type.
    let rows = Ty::array(Ty::i32_ty(), vec![None, Some(3)]);
    assert_eq!(
        rows.iterated_element(),
        Ty::array(Ty::i32_ty(), vec![Some(3)])
    );
    // Three extents keep the two inner extents on the row.
    let cube = Ty::array(Ty::i32_ty(), vec![Some(2), Some(2), Some(2)]);
    assert_eq!(
        cube.iterated_element(),
        Ty::array(Ty::i32_ty(), vec![Some(2), Some(2)])
    );
}

#[test]
fn a_single_extent_array_or_slice_iterates_its_element() {
    let list = Ty::array(Ty::i32_ty(), vec![Some(2)]);
    assert_eq!(list.iterated_element(), Ty::i32_ty());
    let slice = Ty::array(Ty::i32_ty(), vec![None]);
    assert_eq!(slice.iterated_element(), Ty::i32_ty());
}

#[test]
fn a_non_array_iterates_the_unknown_type() {
    // The callers report or ignore the unknown element, so the derivation
    // stays total for every other type.
    assert!(Ty::i32_ty().iterated_element().is_unknown());
    assert!(Ty::boolean().iterated_element().is_unknown());
}

/// The null shape is an unresolved type that is nullable, and only that.
#[test]
fn the_null_shape_is_an_unresolved_nullable_type() {
    let null_shape = Ty {
        kind: TyKind::Unknown,
        nullable: true,
    };
    assert!(is_null_shaped(&null_shape));
    assert!(!is_null_shaped(&Ty::unknown()));
    let nullable_handle = Ty {
        kind: TyKind::Handle("String".to_owned()),
        nullable: true,
    };
    assert!(!is_null_shaped(&nullable_handle));
}
