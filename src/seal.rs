//! The key to the hooks of the crate's sealed traits.
//!
//! A method of a trait no caller can name is still callable through any public trait that has it as a supertrait or
//! as the bound of an associated type. Every such method takes a [`Seal`], a type public in a module private to the
//! crate that only the crate can build, so no caller can call it and its signature stays free to change.

/// Proof that a call comes from inside the crate, taken by every method of a sealed trait.
#[derive(Debug, Clone, Copy)]
pub struct Seal(());

impl Seal {
    /// The seal, which only the crate can build.
    pub(crate) const fn new() -> Seal {
        Seal(())
    }
}
