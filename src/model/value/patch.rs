//! Merge patches of a Property value (NGSI-LD clause 5.5.12, IETF RFC 7396): Merge Entity merges the value of a
//! fragment's Property into the target's "up to an arbitrary depth", and an NGSI-LD Null inside it removes that member
//! of the target value.

use crate::model::{Value, null::NGSI_LD_NULL};
use indexmap::IndexMap;

/// What a merge patch does to one member of an object value.
///
/// The enum is exhaustive: RFC 7396 sets a member, merges an object into it, or removes it.
#[derive(Debug, Clone, PartialEq, Eq, derive_more::IsVariant)]
pub enum ValueChange {
    /// The member takes this value.
    Set(Value),
    /// The member, an object in the target, has this patch merged into it.
    Merge(ValuePatch),
    /// The member is removed: the patch writes the NGSI-LD Null for it (clause 5.5.12).
    Delete,
}

/// A merge patch of an object value.
///
/// It holds the members it sets, the objects it merges into at any depth, and the members it removes, in the order
/// given (NGSI-LD clause 5.5.12, RFC 7396). Members the patch does not name are kept. Member names are free-form user
/// data, as in [`Value::Object`].
///
/// The patch is the `value` of a Property fragment built with
/// [`Property::merge_builder`](crate::model::Property::merge_builder), which clause 5.5.12
/// example 2 sends to update `street` and remove `country`:
///
/// ```
/// use ngsi_ld::model::{Attribute, Lossless, Property, ValuePatch};
///
/// let address = ValuePatch::new()
///     .set("street", "Pariser Platz")
///     .delete("country");
/// let fragment = Attribute::from(Property::merge_builder(address).build()?);
/// assert_eq!(
///     fragment.render(Lossless::Normalized).to_string(),
///     r#"{"type":"Property","value":{"street":"Pariser Platz","country":"urn:ngsi-ld:null"}}"#
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ValuePatch(IndexMap<String, ValueChange>);

impl ValuePatch {
    /// A patch that changes nothing.
    #[must_use]
    pub fn new() -> ValuePatch {
        ValuePatch(IndexMap::new())
    }

    /// The same patch, setting `member` to `value`. A later change of the same member replaces this one.
    #[must_use]
    pub fn set(self, member: impl Into<String>, value: impl Into<Value>) -> Self {
        self.with(member, ValueChange::Set(value.into()))
    }

    /// The same patch, merging `patch` into the object `member` holds.
    #[must_use]
    pub fn merge(self, member: impl Into<String>, patch: ValuePatch) -> Self {
        self.with(member, ValueChange::Merge(patch))
    }

    /// The same patch, removing `member`.
    #[must_use]
    pub fn delete(self, member: impl Into<String>) -> Self {
        self.with(member, ValueChange::Delete)
    }

    /// Returns the change to `member`, if the patch names it.
    #[must_use]
    pub fn get(&self, member: &str) -> Option<&ValueChange> {
        self.0.get(member)
    }

    /// Iterates over the changes, in the order given.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = (&str, &ValueChange)> {
        self.0.iter().map(|(member, change)| (member.as_str(), change))
    }

    /// The number of members the patch names.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the patch names no member.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Records `change` for `member`, keeping the member's place when it is already named.
    fn with(mut self, member: impl Into<String>, change: ValueChange) -> Self {
        self.0.insert(member.into(), change);
        self
    }

    /// The values the patch sets, at every depth: none of them may be or hold the NGSI-LD Null, which only
    /// [`delete`](Self::delete) writes.
    pub(crate) fn set_values(&self) -> Vec<&Value> {
        self.0
            .values()
            .flat_map(|change| match change {
                ValueChange::Set(value) => vec![value],
                ValueChange::Merge(patch) => patch.set_values(),
                ValueChange::Delete => Vec::new(),
            })
            .collect()
    }

    /// The patch as the object value a fragment writes, each removed member holding the NGSI-LD Null.
    pub(crate) fn into_value(self) -> Value {
        Value::Object(
            self.0
                .into_iter()
                .map(|(member, change)| {
                    let value = match change {
                        ValueChange::Set(value) => value,
                        ValueChange::Merge(patch) => patch.into_value(),
                        ValueChange::Delete => Value::Text(NGSI_LD_NULL.as_str().to_owned()),
                    };
                    (member, value)
                })
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::model::{Value, ValueChange, ValuePatch};
    use pretty_assertions::assert_eq;

    #[test]
    fn a_patch_writes_its_changes_in_order_with_the_null_for_each_removed_member() {
        let patch = ValuePatch::new()
            .set("street", "Pariser Platz")
            .merge("settings", ValuePatch::new().set("a", 1).delete("b"))
            .delete("country");
        assert_eq!(
            patch.into_value().to_string(),
            r#"{"street":"Pariser Platz","settings":{"a":1,"b":"urn:ngsi-ld:null"},"country":"urn:ngsi-ld:null"}"#
        );
    }

    #[test]
    fn a_later_change_of_a_member_replaces_the_earlier_one_in_its_place() {
        let patch = ValuePatch::new().delete("a").set("b", Value::Boolean(true)).set("a", 2);
        assert_eq!(patch.len(), 2);
        assert_eq!(patch.get("a"), Some(&ValueChange::Set(Value::from(2))));
        assert_eq!(patch.iter().map(|(member, _)| member).collect::<Vec<_>>(), ["a", "b"]);
        assert!(ValuePatch::new().is_empty());
    }

    #[test]
    fn the_set_values_are_collected_at_every_depth() {
        let patch = ValuePatch::new().set("a", 1).merge("m", ValuePatch::new().set("b", 2).delete("c")).delete("d");
        assert_eq!(patch.set_values(), [&Value::from(1), &Value::from(2)]);
    }
}
