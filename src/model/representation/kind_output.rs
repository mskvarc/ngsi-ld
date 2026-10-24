//! Reading and writing the output-only members that belong to one attribute kind: the `previous*` members of
//! notifications (clauses 4.5.2.2, 4.5.3.2, 4.5.18.2, 4.5.20.2, 4.5.21.2, 4.5.22.2 and 4.5.24.2), the linked
//! entities of inline retrieval (clause 4.5.23.2) and the `lang` of the language filter (clause 4.15).
//!
//! A linked entity is read as a projected entity, whose `id` and `type` may be missing: the `pick` and `omit` of the
//! request apply to every entity of the payload, linked ones included (clause 4.21, Table 6.5.3.1-1), so a nested
//! projection such as `pick=isParked{name},id,type` returns linked entities without either member. The attribute
//! that holds them does not know the projection of the request, so one type serves every answer.
//!
//! Each member is written like the payload it shadows, so it is read and written with the payload's own rules.

use crate::model::{
    DocumentError,
    DocumentErrorKind,
    Entity,
    GeoPropertyOutputMembers,
    GeoValue,
    JsonPropertyOutputMembers,
    LanguageKey,
    LanguageMap,
    LanguagePropertyOutputMembers,
    ListPropertyOutputMembers,
    ListRelationshipOutputMembers,
    Lossless,
    Output,
    Projected,
    PropertyOutputMembers,
    RawJson,
    RelationshipOutputMembers,
    Value,
    VocabPropertyOutputMembers,
    VocabTerm,
    reader::{array, once, one_or_many, parsed, string, unexpected, value},
    representation::{
        member::{entity_id, geo_value},
        read::{instance::list_object, read_entity},
        write::{object_entry, write_context, write_entity},
    },
};
use jstrict::{Object, Value as Json};
use std::fmt::Debug;

/// A member only one attribute kind has, and only in a response. The specification closes the set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::EnumString, strum::IntoStaticStr)]
pub(crate) enum KindOutputMember {
    /// `previousValue`, of a Property or a `GeoProperty`.
    #[strum(serialize = "previousValue")]
    PreviousValue,
    /// `previousLanguageMap`, of a `LanguageProperty`.
    #[strum(serialize = "previousLanguageMap")]
    PreviousLanguageMap,
    /// `previousVocab`, of a `VocabProperty`.
    #[strum(serialize = "previousVocab")]
    PreviousVocab,
    /// `previousValueList`, of a `ListProperty`.
    #[strum(serialize = "previousValueList")]
    PreviousValueList,
    /// `previousJson`, of a `JsonProperty`.
    #[strum(serialize = "previousJson")]
    PreviousJson,
    /// `previousObject`, of a Relationship.
    #[strum(serialize = "previousObject")]
    PreviousObject,
    /// `previousObjectList`, of a `ListRelationship`.
    #[strum(serialize = "previousObjectList")]
    PreviousObjectList,
    /// `entity`, of a Relationship.
    #[strum(serialize = "entity")]
    Entity,
    /// `entityList`, of a `ListRelationship`.
    #[strum(serialize = "entityList")]
    EntityList,
    /// `lang`, of a Property the language filter made from a `LanguageProperty`.
    #[strum(serialize = "lang")]
    Lang,
}

impl KindOutputMember {
    /// Returns the member as it is spelled in a document.
    pub(crate) fn as_str(self) -> &'static str {
        self.into()
    }

    /// The error for a member the kind at hand prohibits (clauses 4.5.2.2, 4.5.3.2 and their counterparts).
    fn prohibited(self) -> DocumentError {
        unexpected(self.as_str())
    }
}

/// The output-only members of one attribute kind, read and written in normalized or concise representation.
pub(crate) trait KindOutputRepresentation: Debug + Clone + PartialEq + Eq + Default {
    /// Reads `member` into its slot, refusing a member the kind does not have and a member given twice.
    fn read(&mut self, member: KindOutputMember, json: Json, form: Lossless) -> Result<(), DocumentError>;

    /// Appends the members that are present, in canonical order.
    fn write(&self, object: &mut Object, form: Lossless);
}

/// Reads a member with `read` and stores it in `slot`, placing any error under the member's name.
fn read_into<T>(slot: &mut Option<T>, member: KindOutputMember, read: impl FnOnce() -> Result<T, DocumentError>) -> Result<(), DocumentError> {
    let name = member.as_str();
    let read = read().map_err(|error| error.within(name))?;
    once(slot, name, read)
}

/// Appends `json` under `member` when it is present.
fn push(object: &mut Object, member: KindOutputMember, json: Option<Json>) {
    if let Some(json) = json {
        object.push(member.as_str().into(), json);
    }
}

/// Writes an inline linked entity in the representation of the response, with its `@context` when it carried one,
/// and its `id` and `type` when the projection of the request kept them.
fn entity_json(entity: &Entity<Output, Projected>, form: Lossless) -> Json {
    let mut object = write_entity(entity, form);
    write_context(&mut object, entity.context.as_ref());
    Json::Object(object)
}

impl KindOutputRepresentation for PropertyOutputMembers {
    fn read(&mut self, member: KindOutputMember, json: Json, _: Lossless) -> Result<(), DocumentError> {
        match member {
            KindOutputMember::PreviousValue => read_into(&mut self.previous_value, member, || value(json)),
            KindOutputMember::Lang => read_into(&mut self.lang, member, || {
                string(json, "lang")?
                    .parse::<LanguageKey>()
                    .map_err(|source| DocumentError::top(DocumentErrorKind::LanguageTag { source }))
            }),
            KindOutputMember::PreviousLanguageMap
            | KindOutputMember::PreviousVocab
            | KindOutputMember::PreviousValueList
            | KindOutputMember::PreviousJson
            | KindOutputMember::PreviousObject
            | KindOutputMember::PreviousObjectList
            | KindOutputMember::Entity
            | KindOutputMember::EntityList => Err(member.prohibited()),
        }
    }

    fn write(&self, object: &mut Object, _: Lossless) {
        push(object, KindOutputMember::PreviousValue, self.previous_value.as_ref().map(Value::to_json));
        push(object, KindOutputMember::Lang, self.lang.as_ref().map(|lang| Json::from(lang.to_string())));
    }
}

impl KindOutputRepresentation for GeoPropertyOutputMembers {
    fn read(&mut self, member: KindOutputMember, json: Json, _: Lossless) -> Result<(), DocumentError> {
        match member {
            KindOutputMember::PreviousValue => read_into(&mut self.previous_value, member, || geo_value(&json)),
            KindOutputMember::PreviousLanguageMap
            | KindOutputMember::PreviousVocab
            | KindOutputMember::PreviousValueList
            | KindOutputMember::PreviousJson
            | KindOutputMember::PreviousObject
            | KindOutputMember::PreviousObjectList
            | KindOutputMember::Entity
            | KindOutputMember::EntityList
            | KindOutputMember::Lang => Err(member.prohibited()),
        }
    }

    fn write(&self, object: &mut Object, _: Lossless) {
        push(object, KindOutputMember::PreviousValue, self.previous_value.as_ref().map(GeoValue::to_json));
    }
}

impl KindOutputRepresentation for LanguagePropertyOutputMembers {
    fn read(&mut self, member: KindOutputMember, json: Json, _: Lossless) -> Result<(), DocumentError> {
        match member {
            KindOutputMember::PreviousLanguageMap => read_into(&mut self.previous_language_map, member, || {
                LanguageMap::from_json(json).map_err(|source| DocumentError::top(DocumentErrorKind::LanguageMap { source }))
            }),
            KindOutputMember::PreviousValue
            | KindOutputMember::PreviousVocab
            | KindOutputMember::PreviousValueList
            | KindOutputMember::PreviousJson
            | KindOutputMember::PreviousObject
            | KindOutputMember::PreviousObjectList
            | KindOutputMember::Entity
            | KindOutputMember::EntityList
            | KindOutputMember::Lang => Err(member.prohibited()),
        }
    }

    fn write(&self, object: &mut Object, _: Lossless) {
        push(
            object,
            KindOutputMember::PreviousLanguageMap,
            self.previous_language_map.as_ref().map(LanguageMap::to_json),
        );
    }
}

impl KindOutputRepresentation for VocabPropertyOutputMembers {
    fn read(&mut self, member: KindOutputMember, json: Json, _: Lossless) -> Result<(), DocumentError> {
        match member {
            KindOutputMember::PreviousVocab => read_into(&mut self.previous_vocab, member, || {
                one_or_many(json, |item| parsed::<VocabTerm, _>(item, "previousVocab"))
            }),
            KindOutputMember::PreviousValue
            | KindOutputMember::PreviousLanguageMap
            | KindOutputMember::PreviousValueList
            | KindOutputMember::PreviousJson
            | KindOutputMember::PreviousObject
            | KindOutputMember::PreviousObjectList
            | KindOutputMember::Entity
            | KindOutputMember::EntityList
            | KindOutputMember::Lang => Err(member.prohibited()),
        }
    }

    fn write(&self, object: &mut Object, _: Lossless) {
        let vocab = self.previous_vocab.as_ref().map(|vocab| vocab.to_json(|term| Json::from(term.to_string())));
        push(object, KindOutputMember::PreviousVocab, vocab);
    }
}

impl KindOutputRepresentation for ListPropertyOutputMembers {
    fn read(&mut self, member: KindOutputMember, json: Json, _: Lossless) -> Result<(), DocumentError> {
        match member {
            KindOutputMember::PreviousValueList => read_into(&mut self.previous_value_list, member, || {
                array(json)?.into_iter().map(value).collect::<Result<_, _>>()
            }),
            KindOutputMember::PreviousValue
            | KindOutputMember::PreviousLanguageMap
            | KindOutputMember::PreviousVocab
            | KindOutputMember::PreviousJson
            | KindOutputMember::PreviousObject
            | KindOutputMember::PreviousObjectList
            | KindOutputMember::Entity
            | KindOutputMember::EntityList
            | KindOutputMember::Lang => Err(member.prohibited()),
        }
    }

    fn write(&self, object: &mut Object, _: Lossless) {
        let list = self
            .previous_value_list
            .as_ref()
            .map(|list| Json::Array(list.iter().map(Value::to_json).collect()));
        push(object, KindOutputMember::PreviousValueList, list);
    }
}

impl KindOutputRepresentation for JsonPropertyOutputMembers {
    fn read(&mut self, member: KindOutputMember, json: Json, _: Lossless) -> Result<(), DocumentError> {
        match member {
            KindOutputMember::PreviousJson => read_into(&mut self.previous_json, member, || {
                let json = RawJson::from_json(json);
                if json.is_json_property_payload() {
                    Ok(json)
                } else {
                    Err(DocumentError::top(DocumentErrorKind::JsonShape))
                }
            }),
            KindOutputMember::PreviousValue
            | KindOutputMember::PreviousLanguageMap
            | KindOutputMember::PreviousVocab
            | KindOutputMember::PreviousValueList
            | KindOutputMember::PreviousObject
            | KindOutputMember::PreviousObjectList
            | KindOutputMember::Entity
            | KindOutputMember::EntityList
            | KindOutputMember::Lang => Err(member.prohibited()),
        }
    }

    fn write(&self, object: &mut Object, _: Lossless) {
        // The raw JSON is carried verbatim, so the tree is copied as it is.
        push(
            object,
            KindOutputMember::PreviousJson,
            // Clone: the attribute is written from a borrow, and the JSON tree owns its members.
            self.previous_json.as_ref().map(RawJson::to_json),
        );
    }
}

impl KindOutputRepresentation for RelationshipOutputMembers {
    fn read(&mut self, member: KindOutputMember, json: Json, form: Lossless) -> Result<(), DocumentError> {
        match member {
            KindOutputMember::PreviousObject => read_into(&mut self.previous_object, member, || {
                one_or_many(json, |item| entity_id(item, "previousObject"))
            }),
            KindOutputMember::Entity => read_into(&mut self.entity, member, || {
                one_or_many(json, |item| read_entity::<Output, Projected>(item, form))
            }),
            KindOutputMember::PreviousValue
            | KindOutputMember::PreviousLanguageMap
            | KindOutputMember::PreviousVocab
            | KindOutputMember::PreviousValueList
            | KindOutputMember::PreviousJson
            | KindOutputMember::PreviousObjectList
            | KindOutputMember::EntityList
            | KindOutputMember::Lang => Err(member.prohibited()),
        }
    }

    fn write(&self, object: &mut Object, form: Lossless) {
        let previous = self.previous_object.as_ref().map(|object| object.to_json(|id| Json::from(id.as_str())));
        push(object, KindOutputMember::PreviousObject, previous);
        let entity = self.entity.as_ref().map(|entity| entity.to_json(|entity| entity_json(entity, form)));
        push(object, KindOutputMember::Entity, entity);
    }
}

impl KindOutputRepresentation for ListRelationshipOutputMembers {
    fn read(&mut self, member: KindOutputMember, json: Json, form: Lossless) -> Result<(), DocumentError> {
        match member {
            KindOutputMember::PreviousObjectList => read_into(&mut self.previous_object_list, member, || {
                array(json)?.into_iter().map(list_object).collect::<Result<_, _>>()
            }),
            KindOutputMember::EntityList => read_into(&mut self.entity_list, member, || {
                array(json)?
                    .into_iter()
                    .map(|item| read_entity::<Output, Projected>(item, form))
                    .collect::<Result<_, _>>()
            }),
            KindOutputMember::PreviousValue
            | KindOutputMember::PreviousLanguageMap
            | KindOutputMember::PreviousVocab
            | KindOutputMember::PreviousValueList
            | KindOutputMember::PreviousJson
            | KindOutputMember::PreviousObject
            | KindOutputMember::Entity
            | KindOutputMember::Lang => Err(member.prohibited()),
        }
    }

    fn write(&self, object: &mut Object, form: Lossless) {
        // `previousObjectList` is written like `objectList`: `{"object": id}` entries in normalized form, bare URIs
        // in concise form (Table 5.2.37-2).
        let previous = self.previous_object_list.as_ref().map(|list| {
            let entries = list.iter().map(|id| match form {
                Lossless::Normalized => object_entry(id),
                Lossless::Concise => Json::from(id.as_str()),
            });
            Json::Array(entries.collect())
        });
        push(object, KindOutputMember::PreviousObjectList, previous);
        let entities = self
            .entity_list
            .as_ref()
            .map(|list| Json::Array(list.iter().map(|entity| entity_json(entity, form)).collect()));
        push(object, KindOutputMember::EntityList, entities);
    }
}
