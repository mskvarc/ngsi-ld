//! The recursive walks of a [`Projection`] a derive would generate (copying, comparing, hashing, debug-printing and
//! dropping), written out so that each level of linked projections runs through [`grow`].
//!
//! Clause 4.21 does not bound how deep linked projections nest, and a broker returns the `pick` and `omit` of its
//! subscriptions, so a projection may be thousands of levels deep. Every level passes through a [`Projection`]; a
//! derived walk would recurse once per level on the current stack and overflow it, while these run each level on a
//! stack that grows with the depth instead.

use crate::{model::nesting::grow, query::projection::Projection};
use std::{
    fmt::{self, Debug, Formatter},
    hash::{Hash, Hasher},
    mem,
};

// No derive: a derived `Debug` recurses once per level on the current stack; see the module documentation.
impl Debug for Projection {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        grow(|| formatter.debug_tuple("Projection").field(&self.0).finish())
    }
}

// See the `Debug` implementation for why this is not derived.
impl Clone for Projection {
    fn clone(&self) -> Self {
        grow(|| Self(self.0.clone()))
    }
}

// See the `Debug` implementation for why this is not derived.
impl PartialEq for Projection {
    fn eq(&self, other: &Self) -> bool {
        grow(|| self.0 == other.0)
    }
}

// See the `Debug` implementation for why this is not derived. The terms are hashed as the derive hashes them, so
// equal projections hash alike.
impl Hash for Projection {
    fn hash<H: Hasher>(&self, state: &mut H) {
        grow(|| self.0.hash(state));
    }
}

// The derived drop of a deep projection would recurse once per level on the current stack; see the module
// documentation.
impl Drop for Projection {
    fn drop(&mut self) {
        let terms = mem::take(&mut self.0);
        grow(move || drop(terms));
    }
}
