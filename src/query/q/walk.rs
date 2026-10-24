//! The recursive walks of a [`Query`](crate::query::q::Query) a derive would generate (copying, comparing,
//! hashing, debug-printing and dropping), written out so that each level of grouping runs through [`grow`].
//!
//! Clause 4.9 does not bound how deep groups nest, and a broker returns the queries of its subscriptions and
//! registrations, so a query may be thousands of levels deep. Every level of grouping passes through the
//! [`Operands`] of an AND or an OR; a derived walk would recurse once per level on the current stack and overflow
//! it, while these run each level on a stack that grows with the depth instead.

use crate::{model::nesting::grow, query::q::Operands};
use std::{
    fmt::{self, Debug, Formatter},
    hash::{Hash, Hasher},
    mem,
};

// No derive: a derived `Debug` recurses once per level on the current stack; see the module documentation.
impl Debug for Operands {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        grow(|| formatter.debug_tuple("Operands").field(&self.0).finish())
    }
}

// See the `Debug` implementation for why this is not derived.
impl Clone for Operands {
    fn clone(&self) -> Self {
        grow(|| Self(self.0.clone()))
    }
}

// See the `Debug` implementation for why this is not derived.
impl PartialEq for Operands {
    fn eq(&self, other: &Self) -> bool {
        grow(|| self.0 == other.0)
    }
}

// See the `Debug` implementation for why this is not derived. The operands are hashed as the derive hashes them, so
// equal operands hash alike.
impl Hash for Operands {
    fn hash<H: Hasher>(&self, state: &mut H) {
        grow(|| self.0.hash(state));
    }
}

// The derived drop of a deep query would recurse once per level on the current stack; see the module documentation.
impl Drop for Operands {
    fn drop(&mut self) {
        let operands = mem::take(&mut self.0);
        grow(move || drop(operands));
    }
}
