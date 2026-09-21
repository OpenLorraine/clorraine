//! The identity of a module-level binding.
//!
//! A simple name is not unique: two modules may each declare a global with the
//! same name. The name resolver knows which module a reference names, so every
//! global place carries the module along with the name. The emitter can then
//! spell the right mangled symbol without guessing from a bare name.

/// Identifies a module-level `let` or `const` by its module and simple name.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct GlobalId {
    /// Dotted module the binding is declared in.
    pub module: String,
    /// Simple name of the binding.
    pub name: String,
}

impl GlobalId {
    /// Builds an identity from a module and a simple name.
    pub fn new(module: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            module: module.into(),
            name: name.into(),
        }
    }
}
