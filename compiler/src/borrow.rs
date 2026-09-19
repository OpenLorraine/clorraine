//! Loans, conflicts, and view provenance for the ownership checker.
//!
//! A loan is a permission to access a place, created by a lend: a view-like
//! parameter, an implicit `String` to `str` widening, or a standard-library
//! helper that returns a view over storage the caller still owns. Loans come
//! only from lends; there is no borrow expression in the language.
//!
//! The model here is the conservative lexical approximation the version 1
//! checker uses: a loan is live from its creation until the end of the scope
//! that owns the derived view, so the checker never accepts a program a more
//! precise analysis would reject. The checker only needs to answer three
//! questions with a [`Loans`] set: may the owner be moved, may it be written,
//! and may a second loan of a given kind be created.

use crate::span::Span;
use crate::types::{Ty, TyKind};

/// The permission a loan grants.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoanKind {
    /// Many readers, no writer.
    Shared,
    /// A single writer, no other access.
    Exclusive,
}

/// One live loan of an owning place.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Loan {
    /// Index of the binding that owns the storage.
    pub owner: usize,
    /// The permission the loan grants.
    pub kind: LoanKind,
    /// Scope depth at which the loan begins; the loan ends when that scope
    /// ends.
    pub origin: usize,
    /// Source location of the lending expression, for diagnostics.
    pub span: Span,
}

/// The loans live at one program point.
#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub struct Loans {
    entries: Vec<Loan>,
}

impl Loans {
    /// Records a new loan, ignoring an identical live loan.
    ///
    /// Re-adding the same `(owner, kind, origin)` loan happens when a flow
    /// state is re-derived at a fixed point; keeping the set canonical makes
    /// the equality check that detects the fixed point meaningful.
    pub fn add(&mut self, owner: usize, kind: LoanKind, origin: usize, span: Span) {
        if self
            .entries
            .iter()
            .any(|loan| loan.owner == owner && loan.kind == kind && loan.origin == origin)
        {
            return;
        }
        self.entries.push(Loan {
            owner,
            kind,
            origin,
            span,
        });
    }

    /// Drops every loan whose origin is the scope that is now ending.
    pub fn end_scope(&mut self, scope: usize) {
        self.entries.retain(|loan| loan.origin < scope);
    }

    /// True when any loan of the owner is live.
    pub fn any_live(&self, owner: usize) -> bool {
        self.entries.iter().any(|loan| loan.owner == owner)
    }

    /// True when a shared loan of the owner is live.
    pub fn shared_live(&self, owner: usize) -> bool {
        self.entries
            .iter()
            .any(|loan| loan.owner == owner && loan.kind == LoanKind::Shared)
    }

    /// True when creating a loan of `kind` over `owner` conflicts with a live
    /// loan.
    ///
    /// A new loan conflicts when any loan is already live: a second loan of
    /// any kind overlaps, and an exclusive loan never coexists with another
    /// access. This is BR-2 and BR-3 in the conservative lexical model.
    pub fn conflict(&self, owner: usize, _kind: LoanKind) -> bool {
        self.any_live(owner)
    }

    /// True when the owner may not be moved because a loan is live (BR-4).
    pub fn blocks_move(&self, owner: usize) -> bool {
        self.any_live(owner)
    }

    /// True when the owner may not be written because a shared loan is live
    /// (BR-2).
    pub fn blocks_write(&self, owner: usize) -> bool {
        self.shared_live(owner)
    }

    /// Keeps the union of two branch states: a loan live on either path is
    /// treated as live after the join, which is conservative.
    pub fn merge(&mut self, other: &Loans) {
        for loan in &other.entries {
            let duplicate = self.entries.iter().any(|known| {
                known.owner == loan.owner
                    && known.kind == loan.kind
                    && known.origin == loan.origin
            });
            if !duplicate {
                self.entries.push(loan.clone());
            }
        }
    }
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
}

/// True for a type that is a view over other storage.
///
/// A view is a cheap pointer-and-length value, so binding or copying it does
/// not move the storage it points into.
pub fn is_view(ty: &Ty) -> bool {
    matches!(
        ty.kind,
        TyKind::Str | TyKind::CString | TyKind::StaticStr | TyKind::Array { .. }
    )
}

/// True for a view type whose lifetime participates in return elision.
///
/// `static str` is excluded: it points at static storage and needs no input
/// parameter (EL-4).
pub fn is_elided_view(ty: &Ty) -> bool {
    matches!(
        ty.kind,
        TyKind::Str | TyKind::CString | TyKind::Array { .. }
    )
}

/// True for a type that may be a lifetime-elision candidate.
///
/// Views, handles, raw pointers, and `cstring` all borrow or reach storage, so
/// each carries its own elided lifetime (EL-1).
pub fn is_lend_candidate(ty: &Ty) -> bool {
    matches!(
        ty.kind,
        TyKind::Str
            | TyKind::CString
            | TyKind::StaticStr
            | TyKind::Array { .. }
            | TyKind::Raw(_)
            | TyKind::Handle(_)
    )
}

/// True for an owning handle type, whose value must be released.
pub fn is_handle(ty: &Ty) -> bool {
    matches!(ty.kind, TyKind::Handle(_))
}

/// True for an owned handle or aggregate, the two shapes an uncontracted
/// extern parameter may not receive.
pub fn is_handle_or_aggregate(ty: &Ty) -> bool {
    matches!(ty.kind, TyKind::Handle(_) | TyKind::Struct(_))
}

/// True for a type that can carry the `?` empty state.
///
/// Nullability is meaningful for handles, raw pointers, `cstring`, and views;
/// on any other type the suffix is rejected.
pub fn is_nullable_capable(ty: &Ty) -> bool {
    matches!(
        ty.kind,
        TyKind::Handle(_)
            | TyKind::Raw(_)
            | TyKind::CString
            | TyKind::Str
            | TyKind::StaticStr
            | TyKind::Array { .. }
            | TyKind::Unknown
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::own::test_support::{error_codes, reports};
    use crate::span::Span;

    /// A synthetic span; the loan set never renders it in these tests.
    fn span() -> Span {
        let mut sources = crate::span::SourceMap::new();
        let id = sources.add_file("test.clo".to_owned(), String::new());
        Span::new(id, 0, 0)
    }

    #[test]
    fn any_second_loan_conflicts_in_the_conservative_model() {
        let mut loans = Loans::default();
        loans.add(0, LoanKind::Shared, 1, span());
        assert!(loans.conflict(0, LoanKind::Shared), "v1 rejects a second loan");
        assert!(loans.conflict(0, LoanKind::Exclusive), "a writer excludes");
        assert!(loans.blocks_write(0), "a shared loan blocks a write");
        assert!(loans.blocks_move(0), "any loan blocks a move");
        assert!(!loans.any_live(1), "another owner is unaffected");
    }

    #[test]
    fn an_exclusive_loan_excludes_a_second_loan_of_either_kind() {
        let mut loans = Loans::default();
        loans.add(0, LoanKind::Exclusive, 1, span());
        assert!(loans.conflict(0, LoanKind::Shared));
        assert!(loans.conflict(0, LoanKind::Exclusive));
    }

    #[test]
    fn ending_a_scope_retires_the_loans_it_owns() {
        let mut loans = Loans::default();
        loans.add(0, LoanKind::Shared, 2, span());
        loans.add(0, LoanKind::Exclusive, 3, span());
        loans.end_scope(3);
        assert!(loans.any_live(0), "the inner loan is gone but the outer stays");
        loans.end_scope(2);
        assert!(!loans.any_live(0), "the outer loan is gone");
    }

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
    fn a_second_view_of_one_owner_is_a_conflict_in_v1() {
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
        assert!(reports(source, "E10032"), "{:?}", error_codes(source));
    }
}
