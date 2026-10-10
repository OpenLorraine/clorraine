//! Loans, conflicts, and view provenance for the ownership checker.
//!
//! A loan is a permission to access a place, created by a lend: a view-like
//! parameter, an implicit `String` to `str` widening, or a standard-library
//! helper that returns a view over storage the caller still owns. Loans come
//! only from lends; there is no borrow expression in the language.
//!
//! A loan is live from its creation until its last use on any path; the
//! checker records loan events during the ownership walk and the liveness pass
//! decides the conflicts against the computed regions.

use crate::types::{Ty, TyKind};

/// The permission a loan grants.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LoanKind {
    /// Many readers, no writer.
    Shared,
    /// A single writer, no other access.
    Exclusive,
}

/// Where a view value came from.
///
/// Provenance decides lifetime elision, escape analysis, and whether a writable
/// view may derive from a read-only loan. It is attached to a view value when
/// it is produced and carried through bindings so a later use can reason about
/// the storage behind it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Provenance {
    /// The value carries no view storage (a scalar, an owner, a literal).
    None,
    /// The value points into static storage, such as a string literal.
    Static,
    /// The value is derived from the parameter with this index.
    Param(usize),
    /// The value is derived from the local binding with this index.
    Local(usize),
    /// The value is derived from the module-level binding with this name.
    Global(String),
    /// The value is derived from the compiler temporary with this id.
    ///
    /// A temporary the compiler materialized for a statement owns its storage
    /// until the statement ends, so a view that reads through it may not be
    /// used later.
    Temp(crate::ir::TempId),
}

/// True for a type that is a view over other storage.
///
/// A view is a cheap pointer-and-length value, so binding or copying it does
/// not move the storage it points into. A slice is a view; a fixed array is not
/// — it is inline storage the value owns.
pub fn is_view(ty: &Ty) -> bool {
    match ty.kind {
        TyKind::Str | TyKind::CString | TyKind::StaticStr => true,
        TyKind::Array { .. } => ty.is_slice(),
        _ => false,
    }
}

/// True for a view type whose lifetime participates in return elision.
///
/// `static str` is excluded: it points at static storage and needs no input
/// parameter (EL-4). A slice participates; a fixed array does not — except
/// when its element is itself an elided view: a `str[N]` carries exactly the
/// borrows the single `str` it repeats carries, so it participates in elision
/// and in the view-call loan and provenance rules.
pub fn is_elided_view(ty: &Ty) -> bool {
    match &ty.kind {
        TyKind::Str | TyKind::CString => true,
        TyKind::Array { element, .. } => ty.is_slice() || is_elided_view(element),
        _ => false,
    }
}

/// True for a type that may be a lifetime-elision candidate.
///
/// Views, handles, raw pointers, and `cstring` all borrow or reach storage, so
/// each carries its own elided lifetime (EL-1). The growable `Array` is a
/// compiler-known container that owns storage a returned view may borrow, so it
/// is a candidate too.
pub fn is_lend_candidate(ty: &Ty) -> bool {
    match &ty.kind {
        TyKind::Str
        | TyKind::CString
        | TyKind::StaticStr
        | TyKind::Array { .. }
        | TyKind::Raw(_)
        | TyKind::Handle(_) => true,
        TyKind::Struct(name, _) => name == "Array",
        _ => false,
    }
}

/// True for an owning handle type, whose value must be released.
pub fn is_handle(ty: &Ty) -> bool {
    matches!(ty.kind, TyKind::Handle(_))
}

/// True for an owned handle or aggregate, the two shapes an uncontracted
/// extern parameter may not receive.
///
/// A fixed array is an owned aggregate; a slice is a view and is allowed.
pub fn is_handle_or_aggregate(ty: &Ty) -> bool {
    match ty.kind {
        TyKind::Handle(_) | TyKind::Struct(..) => true,
        TyKind::Array { .. } => ty.is_fixed_array(),
        _ => false,
    }
}

/// True for a type that can carry the `?` empty state.
///
/// Nullability is meaningful for handles, raw pointers, `cstring`, and views; a
/// fixed array is inline storage with no empty state. On any other type the
/// suffix is rejected.
pub fn is_nullable_capable(ty: &Ty) -> bool {
    match ty.kind {
        TyKind::Handle(_)
        | TyKind::Raw(_)
        | TyKind::CString
        | TyKind::Str
        | TyKind::StaticStr
        | TyKind::Unknown => true,
        TyKind::Array { .. } => ty.is_slice(),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::own::test_support::{error_codes, reports};

    #[test]
    fn a_move_while_a_view_is_live_is_rejected() {
        let source = "module t;\n\
            func viewOf(s: str): str { return s; }\n\
            func consume(take s: String): void {}\n\
            func demo(): void {\n\
                let s: String = $\"x{1}\";\n\
                let v: str = viewOf(s);\n\
                consume(s);\n\
                observe(v);\n\
            }\n\
            func observe(s: str): void {}\n\
            func main(): i32 { return 0; }\n";
        assert!(reports(source, "E10022"), "{:?}", error_codes(source));
    }

    #[test]
    fn a_write_while_a_view_is_live_is_rejected() {
        let source = "module t;\n\
            func viewOf(s: str): str { return s; }\n\
            func demo(): void {\n\
                let s: String = $\"x{1}\";\n\
                let v: str = viewOf(s);\n\
                s = $\"y{2}\";\n\
                observe(v);\n\
            }\n\
            func observe(s: str): void {}\n\
            func main(): i32 { return 0; }\n";
        assert!(reports(source, "E10031"), "{:?}", error_codes(source));
    }

    #[test]
    fn a_shared_reborrow_of_a_borrow_parameter_is_accepted() {
        let source = "module t;\n\
            import { slice } from std.mem;\n\
            func observe(buf: u8[]): void {}\n\
            func demo(borrow b: u8[]): void {\n\
                observe(slice(b, 0, 1));\n\
            }\n\
            func main(): i32 { return 0; }\n";
        assert!(error_codes(source).is_empty(), "{:?}", error_codes(source));
    }

    #[test]
    fn a_field_view_borrows_the_whole_struct_conservatively() {
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
        assert!(reports(source, "E10031"), "{:?}", error_codes(source));
    }

    #[test]
    fn two_shared_views_of_one_owner_are_accepted() {
        let source = "module t;\n\
            import { String, stringOf, bytes } from std.text;\n\
            func demo(): void {\n\
                let s: String = stringOf(\"x\");\n\
                let v: u8[] = bytes(s);\n\
                let w: u8[] = bytes(s);\n\
                observe(v);\n\
                observe(w);\n\
            }\n\
            func observe(v: u8[]): void {}\n\
            func main(): i32 { return 0; }\n";
        assert!(error_codes(source).is_empty(), "{:?}", error_codes(source));
    }

    #[test]
    fn a_fixed_array_is_an_aggregate_but_a_slice_is_a_view() {
        let fixed = Ty::array(Ty::i32_ty(), vec![Some(4)]);
        let slice = Ty::array(Ty::i32_ty(), vec![None]);
        assert!(is_handle_or_aggregate(&fixed));
        assert!(!is_handle_or_aggregate(&slice));
        assert!(!is_view(&fixed));
        assert!(is_view(&slice));
        assert!(!is_nullable_capable(&fixed));
        assert!(is_nullable_capable(&slice));
    }

    #[test]
    fn a_fixed_array_stays_a_lend_candidate() {
        let fixed = Ty::array(Ty::i32_ty(), vec![Some(2)]);
        let slice = Ty::array(Ty::i32_ty(), vec![None]);
        assert!(is_lend_candidate(&fixed));
        assert!(is_lend_candidate(&slice));
    }
}
