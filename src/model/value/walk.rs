//! The recursive walks of a [`Value`] a derive would generate (copying, comparing, debug-printing and dropping),
//! written out so that each level of nesting runs through [`grow`].
//!
//! Clause 3.1 does not bound how deep a Value nests, so a broker may return a Property whose value is an object
//! thousands of levels deep. A derived walk recurses once per level on the current stack and overflows it; these run
//! each level on a stack that grows with the depth instead.

use crate::model::{Value, nesting::grow};
use std::{
    fmt::{self, Debug, Formatter},
    mem,
};

// No derive: a derived `Debug` recurses once per level on the current stack; see the module documentation.
impl Debug for Value {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        grow(|| match self {
            Self::Boolean(boolean) => formatter.debug_tuple("Boolean").field(boolean).finish(),
            Self::Number(number) => formatter.debug_tuple("Number").field(number).finish(),
            Self::Text(text) => formatter.debug_tuple("Text").field(text).finish(),
            Self::Array(items) => formatter.debug_tuple("Array").field(items).finish(),
            Self::Object(members) => formatter.debug_tuple("Object").field(members).finish(),
            Self::Typed(typed) => formatter.debug_tuple("Typed").field(typed).finish(),
            Self::JsonNull => formatter.write_str("JsonNull"),
        })
    }
}

// See the `Debug` implementation for why this is not derived.
impl Clone for Value {
    fn clone(&self) -> Self {
        grow(|| match self {
            Self::Boolean(boolean) => Self::Boolean(*boolean),
            Self::Number(number) => Self::Number(number.clone()),
            Self::Text(text) => Self::Text(text.clone()),
            Self::Array(items) => Self::Array(items.clone()),
            Self::Object(members) => Self::Object(members.clone()),
            Self::Typed(typed) => Self::Typed(typed.clone()),
            Self::JsonNull => Self::JsonNull,
        })
    }
}

// See the `Debug` implementation for why this is not derived. Objects compare as the derive compares them: as maps,
// whatever their member order.
impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        grow(|| match (self, other) {
            (Self::Boolean(left), Self::Boolean(right)) => left == right,
            (Self::Number(left), Self::Number(right)) => left == right,
            (Self::Text(left), Self::Text(right)) => left == right,
            (Self::Array(left), Self::Array(right)) => left == right,
            (Self::Object(left), Self::Object(right)) => left == right,
            (Self::Typed(left), Self::Typed(right)) => left == right,
            (Self::JsonNull, Self::JsonNull) => true,
            (
                Self::Boolean(_) | Self::Number(_) | Self::Text(_) | Self::Array(_) | Self::Object(_) | Self::Typed(_) | Self::JsonNull,
                Self::Boolean(_) | Self::Number(_) | Self::Text(_) | Self::Array(_) | Self::Object(_) | Self::Typed(_) | Self::JsonNull,
            ) => false,
        })
    }
}

// The derived drop of a deep value would recurse once per level on the current stack; see the module documentation.
impl Drop for Value {
    fn drop(&mut self) {
        match self {
            Self::Array(items) => {
                let items = mem::take(items);
                grow(move || drop(items));
            }
            Self::Object(members) => {
                let members = mem::take(members);
                grow(move || drop(members));
            }
            Self::Boolean(_) | Self::Number(_) | Self::Text(_) | Self::Typed(_) | Self::JsonNull => {}
        }
    }
}
