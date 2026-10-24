//! What the `attrs` of a temporal request names (NGSI-LD clauses 4.18 and 5.7.4): an attribute, or the entity's
//! `scope`, which the temporal representation reifies.

use crate::model::{AttributeName, NameError, ReservedMember};
use std::str::FromStr;

/// A name the `attrs` of Retrieve or Query Temporal Evolution takes.
///
/// It is an attribute, or `scope` (NGSI-LD Tables 6.18.3.2-1 and 6.19.3.1-1).
///
/// Clause 4.18 represents the scope of an entity as a Property that is non-reified in the normalized representation and
/// reified in the temporal one, so a temporal request asks for its evolution like an attribute's: clause C.5.16.2 sends
/// `attrs=speed,scope`. Outside the temporal API `scope` is never an attribute name, so [`AttributeName`] refuses it.
///
/// The enum is exhaustive: clause 4.18 makes the scope the only non-reified member the temporal representation
/// reifies.
///
/// ```
/// use ngsi_ld::model::evolution::TemporalAttributeName;
///
/// let speed: TemporalAttributeName = "speed".parse()?;
/// assert!(speed.is_attribute());
/// assert_eq!(
///     "scope".parse::<TemporalAttributeName>()?,
///     TemporalAttributeName::Scope
/// );
/// assert!("createdAt".parse::<TemporalAttributeName>().is_err());
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display, derive_more::From, derive_more::IsVariant)]
pub enum TemporalAttributeName {
    /// An attribute.
    #[display("{_0}")]
    Attribute(AttributeName),
    /// `scope` (clause 4.18).
    #[display("scope")]
    #[from(skip)]
    Scope,
}

// No derive reads `scope` apart from the attribute names, which refuse it.
impl FromStr for TemporalAttributeName {
    type Err = NameError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        if text == ReservedMember::Scope.as_str() {
            Ok(Self::Scope)
        } else {
            text.parse().map(Self::Attribute)
        }
    }
}

draft!(
    /// A name of the `attrs` of a temporal request given to a builder.
    ///
    /// It is a [`TemporalAttributeName`], or text validated when the builder finishes, so every invalid name is
    /// reported together.
    ///
    /// ```
    /// use ngsi_ld::model::evolution::{TemporalAttributeName, TemporalAttributeNameDraft};
    ///
    /// let names: [TemporalAttributeNameDraft; 2] =
    ///     [TemporalAttributeName::Scope.into(), "speed".into()];
    /// # let _ = names;
    /// ```
    TemporalAttributeNameDraft(TemporalAttributeName, NameError)
);

// No derive converts through the variant the draft holds.
impl From<AttributeName> for TemporalAttributeNameDraft {
    fn from(name: AttributeName) -> Self {
        Self::from(TemporalAttributeName::Attribute(name))
    }
}

#[cfg(test)]
mod tests {
    use crate::model::{AttributeName, NameError, ReservedMember, evolution::TemporalAttributeName};
    use pretty_assertions::assert_eq;

    #[test]
    fn scope_is_read_apart_and_written_back() {
        let scope: TemporalAttributeName = "scope".parse().unwrap();
        assert_eq!(scope, TemporalAttributeName::Scope);
        assert_eq!(scope.to_string(), "scope");
    }

    #[test]
    fn an_attribute_name_is_read_and_written_as_itself() {
        for text in ["speed", "https://example.org/speed", "ex:speed", "Scope"] {
            let name: TemporalAttributeName = text.parse().unwrap();
            assert_eq!(name, TemporalAttributeName::Attribute(text.parse::<AttributeName>().unwrap()));
            assert_eq!(name.to_string(), text);
        }
    }

    #[test]
    fn every_other_reserved_member_and_invalid_name_is_refused() {
        assert_eq!(
            "createdAt".parse::<TemporalAttributeName>(),
            Err(NameError::Reserved {
                member: ReservedMember::CreatedAt
            })
        );
        assert_eq!("id".parse::<TemporalAttributeName>(), Err(NameError::Reserved { member: ReservedMember::Id }));
        assert_eq!("".parse::<TemporalAttributeName>(), Err(NameError::Empty));
        assert!("1speed".parse::<TemporalAttributeName>().is_err());
    }
}
