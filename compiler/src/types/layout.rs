//! The target ABI's type layout: the sizes an array extent must match.
//!
//! A fixed array is emitted with the extent its type carries, so an extent
//! that came from `sizeof` must equal the size the target C compiler gives the
//! type. [`TypeTable::size_of`] models the declared widths without the
//! target's alignment padding and assumes 32-bit pointers; this module
//! measures the same shapes the way the emitted C lays them out, so the extent
//! position and the value position agree on every profile.

use crate::ast::ScalarType;
use crate::profile::Profile;

use super::substitute_instance;
use super::table::{scalar_size, TypeTable};
use super::Ty;
use super::TyKind;

/// The layout facts of one target ABI.
///
/// The compiler emits C for two families of targets: the host C ABI (64-bit
/// pointers, natural alignment) and the m68k Amiga ABI (32-bit pointers,
/// scalar alignment capped at two bytes). The declared-width scalars and the
/// emitted `usize` — an `unsigned int` — are the same on both, so only the
/// pointer size and the alignment cap vary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AbiLayout {
    /// The byte size of a pointer.
    pointer: usize,
    /// The largest alignment a scalar gets.
    scalar_align_cap: usize,
}

impl Default for AbiLayout {
    fn default() -> Self {
        Self::host()
    }
}

impl AbiLayout {
    /// The host C ABI: 64-bit pointers and natural alignment.
    pub const fn host() -> Self {
        Self {
            pointer: 8,
            scalar_align_cap: 8,
        }
    }

    /// The m68k Amiga ABI: 32-bit pointers, two-byte scalar alignment.
    pub const fn m68k() -> Self {
        Self {
            pointer: 4,
            scalar_align_cap: 2,
        }
    }

    /// The layout of one profile.
    ///
    /// The host profile targets the host C compiler; the two Amiga profiles
    /// share the m68k ABI.
    pub fn for_profile(profile: Profile) -> Self {
        match profile {
            Profile::Host => Self::host(),
            Profile::HostedAmiga | Profile::Freestanding => Self::m68k(),
        }
    }

    /// The byte size of a type, when the table models its shape.
    pub fn size_of(&self, table: &TypeTable, ty: &Ty) -> Option<usize> {
        self.size_at(table, ty, 0)
    }

    /// The byte alignment of a type, when the table models its shape.
    pub fn align_of(&self, table: &TypeTable, ty: &Ty) -> Option<usize> {
        self.align_at(table, ty, 0)
    }

    /// The byte offset of one struct field under this ABI.
    ///
    /// Each preceding field starts at its own alignment and advances by its
    /// padded size, so the result is the same offset the emitted C
    /// `offsetof` reports.
    pub fn offset_of(&self, table: &TypeTable, base: &Ty, field: &str) -> Option<usize> {
        let TyKind::Struct(name, args) = &base.kind else {
            return None;
        };
        let params = table.struct_type_params(name).unwrap_or(&[]);
        let mut offset = 0usize;
        for (field_name, field_ty) in table.struct_fields(name)? {
            let field_ty = substitute_instance(field_ty, params, args);
            let align = self.align_at(table, &field_ty, 0)?;
            offset = self.round_up(offset, align)?;
            if field_name == field {
                return Some(offset);
            }
            offset = offset.checked_add(self.size_at(table, &field_ty, 0)?)?;
        }
        None
    }

    /// Depth-limited worker for [`AbiLayout::size_of`].
    ///
    /// A malformed self-referential declaration would otherwise recurse without
    /// bound; the limit makes the size unknown instead of overflowing.
    fn size_at(&self, table: &TypeTable, ty: &Ty, depth: usize) -> Option<usize> {
        if depth > 32 {
            return None;
        }
        match &ty.kind {
            TyKind::Void => Some(0),
            TyKind::Scalar(scalar) => scalar_size(*scalar),
            TyKind::Raw(_) | TyKind::Func(_) | TyKind::Handle(_) => Some(self.pointer),
            // The emitted `str` view is a pointer-and-length struct; a
            // `cstring` is the bare C string pointer.
            TyKind::Str | TyKind::StaticStr => self.struct_size(
                table,
                &[Ty::raw(Ty::scalar(ScalarType::U8)), Ty::usize_ty()],
                depth,
            ),
            TyKind::CString => Some(self.pointer),
            TyKind::Enum(name) => table
                .enum_backing(name)
                .and_then(|backing| self.size_at(table, backing, depth + 1)),
            TyKind::Array { element, extents } => {
                // An omitted extent is a slice view: a pointer and a length,
                // whatever the element and later extents are.
                if extents.iter().any(Option::is_none) {
                    return self.struct_size(
                        table,
                        &[Ty::raw(Ty::scalar(ScalarType::U8)), Ty::usize_ty()],
                        depth,
                    );
                }
                let mut total = self.size_at(table, element, depth + 1)?;
                for extent in extents {
                    total = total.checked_mul((*extent)?)?;
                }
                Some(total)
            }
            TyKind::Struct(name, args) => {
                let params = table.struct_type_params(name).unwrap_or(&[]);
                let fields: Vec<Ty> = table
                    .struct_fields(name)?
                    .iter()
                    .map(|(_, field)| super::substitute_instance(field, params, args))
                    .collect();
                self.struct_size(table, &fields, depth)
            }
            _ => None,
        }
    }

    /// The size of a struct whose fields are laid out in declaration order.
    ///
    /// Each field starts at its own alignment, and the whole struct is padded
    /// to its largest field alignment, exactly as the emitted C struct is.
    fn struct_size(&self, table: &TypeTable, fields: &[Ty], depth: usize) -> Option<usize> {
        if depth > 32 {
            return None;
        }
        let mut offset = 0usize;
        let mut max_align = 1usize;
        for field in fields {
            let size = self.size_at(table, field, depth + 1)?;
            let align = self.align_at(table, field, depth + 1)?;
            max_align = max_align.max(align);
            offset = self.round_up(offset, align)?.checked_add(size)?;
        }
        self.round_up(offset, max_align)
    }

    /// The byte alignment of a type under the ABI.
    fn align_at(&self, table: &TypeTable, ty: &Ty, depth: usize) -> Option<usize> {
        if depth > 32 {
            return None;
        }
        match &ty.kind {
            TyKind::Void => Some(1),
            TyKind::Scalar(scalar) => {
                let size = scalar_size(*scalar)?;
                Some(size.min(self.scalar_align_cap).max(1))
            }
            TyKind::Raw(_) | TyKind::Func(_) | TyKind::Handle(_) | TyKind::CString => {
                Some(self.pointer.min(self.scalar_align_cap).max(1))
            }
            TyKind::Str | TyKind::StaticStr => {
                let pointer = self.pointer.min(self.scalar_align_cap).max(1);
                let length = 4usize.min(self.scalar_align_cap).max(1);
                Some(pointer.max(length))
            }
            TyKind::Enum(name) => table
                .enum_backing(name)
                .and_then(|backing| self.align_at(table, backing, depth + 1)),
            TyKind::Array { element, extents } => {
                if extents.iter().any(Option::is_none) {
                    let pointer = self.pointer.min(self.scalar_align_cap).max(1);
                    let length = 4usize.min(self.scalar_align_cap).max(1);
                    Some(pointer.max(length))
                } else {
                    self.align_at(table, element, depth + 1)
                }
            }
            TyKind::Struct(name, args) => {
                let params = table.struct_type_params(name).unwrap_or(&[]);
                let fields = table.struct_fields(name)?;
                let mut max_align = 1usize;
                for (_, field) in fields {
                    let field_ty = super::substitute_instance(field, params, args);
                    max_align = max_align.max(self.align_at(table, &field_ty, depth + 1)?);
                }
                Some(max_align)
            }
            _ => None,
        }
    }

    /// Rounds a byte offset up to an alignment.
    fn round_up(&self, offset: usize, align: usize) -> Option<usize> {
        let align = align.max(1);
        let remainder = offset % align;
        if remainder == 0 {
            Some(offset)
        } else {
            offset.checked_add(align - remainder)
        }
    }
}
