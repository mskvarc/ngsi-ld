//! The members a document writes as arrays, which JSON-LD compaction writes bare when they hold one value.
//!
//! Compaction replaces a one-element array by its element unless the term's container is `@set` or `@list`
//! (JSON-LD 1.1 API, compaction algorithm step 12.8.2), and most array members of the core context have neither.
//! The readers follow the data types of the specification, which write those members as arrays (for instance
//! Table 5.2.12-1 for `entities` and `watchedAttributes`), so the arrays are put back before a compacted document is
//! read. The two forms are the same JSON-LD. A member whose data type allows one value or an array (an entity's
//! `type` and `scope`, a Relationship's `object`, a `VocabProperty`'s `vocab`, a Property's `value`) is read as
//! compaction writes it, so a one-element array there comes back as its element.

use jstrict::Value as Json;
use std::mem;

/// The shape of the array members of a JSON value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    /// The value is read as compaction writes it.
    Kept,
    /// The value is an array, each element of this shape.
    Array(&'static Shape),
    /// The value is an object whose named members have these shapes.
    Object(&'static [(&'static str, Shape)]),
}

/// An array of values read as compaction writes them.
pub(crate) const ARRAY: Shape = Shape::Array(&Shape::Kept);

impl Shape {
    /// Puts back the arrays of `json` this shape names.
    pub(crate) fn restore(self, json: &mut Json) {
        match self {
            Self::Kept => {}
            Self::Array(item) => {
                if !json.is_array() {
                    let single = mem::replace(json, Json::Null);
                    *json = Json::Array(vec![single].into());
                }
                if let Json::Array(items) = json {
                    items.iter_mut().for_each(|element| item.restore(element));
                }
            }
            Self::Object(members) => {
                if let Json::Object(object) = json {
                    for &(name, shape) in members {
                        object.get_mut(name).for_each(|value| shape.restore(value));
                    }
                }
            }
        }
    }
}
