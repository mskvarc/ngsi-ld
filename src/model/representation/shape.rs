//! The members that make a concise reader take an object for an attribute rather than for a value (NGSI-LD
//! clauses 4.5.2.3, 4.5.3.3, 4.5.18.3, 4.5.20.3, 4.5.21.3, 4.5.22.3 and 4.5.24.3). The reader infers kinds from
//! them, and the writer never writes a bare value that a reader would take for an attribute.

use crate::model::{AttributeKind, ReservedMember};
use jstrict::Value as Json;

/// A member that decides the kind of a concise attribute without a `type`, in the order the reader's rule 4
/// lists them. The specification closes the set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::EnumString, strum::IntoStaticStr, strum::EnumIter)]
pub(crate) enum KindMember {
    /// `object`: a Relationship.
    #[strum(serialize = "object")]
    Object,
    /// `objectList`: a `ListRelationship`.
    #[strum(serialize = "objectList")]
    ObjectList,
    /// `languageMap`: a `LanguageProperty`.
    #[strum(serialize = "languageMap")]
    LanguageMap,
    /// `vocab`: a `VocabProperty`.
    #[strum(serialize = "vocab")]
    Vocab,
    /// `valueList`: a `ListProperty`.
    #[strum(serialize = "valueList")]
    ValueList,
    /// `json`: a `JsonProperty`.
    #[strum(serialize = "json")]
    Json,
}

impl KindMember {
    /// The kind the member decides.
    pub(crate) const fn kind(self) -> AttributeKind {
        match self {
            Self::Object => AttributeKind::Relationship,
            Self::ObjectList => AttributeKind::ListRelationship,
            Self::LanguageMap => AttributeKind::LanguageProperty,
            Self::Vocab => AttributeKind::VocabProperty,
            Self::ValueList => AttributeKind::ListProperty,
            Self::Json => AttributeKind::JsonProperty,
        }
    }

    /// Returns the member as it is spelled in a document.
    pub(crate) fn as_str(self) -> &'static str {
        self.into()
    }
}

/// Whether a member makes a concise reader take its object for an attribute: `type`, `value` or a kind member.
fn shapes_attribute_member(member: &str) -> bool {
    matches!(ReservedMember::find(member), Some(ReservedMember::Type | ReservedMember::Value)) || member.parse::<KindMember>().is_ok()
}

/// Whether a concise reader takes `json` for an attribute rather than for a value.
pub(crate) fn shapes_attribute(json: &Json) -> bool {
    json.as_object()
        .is_some_and(|object| object.iter().any(|entry| shapes_attribute_member(entry.key.as_str())))
}
