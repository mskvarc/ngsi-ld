//! The temporal values of one attribute in simplified temporal representation (NGSI-LD clause 4.5.9): a list of
//! pairs of a payload and the time it was recorded at, in the member the attribute's kind names.

use crate::model::{
    AttributeKind,
    DateTime,
    DocumentError,
    DocumentErrorKind,
    EntityId,
    GeoValue,
    JsonType,
    LanguageMap,
    MemberError,
    OneOrMany,
    RawJson,
    Value,
    VocabTerm,
    keyword_alias::unalias_attribute,
    reader::{array, date_time, object, once, one_or_many, parsed, required, unexpected, value},
    representation::member::{entity_id, geo_value},
};
use jstrict::{Object, Value as Json, object::Entry};

/// One data point of a simplified temporal attribute.
///
/// It holds a payload and the time of the Temporal Property it was recorded at (`observedAt` unless the temporal query
/// names another, NGSI-LD clause 4.5.9).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemporalValue<T> {
    /// The payload.
    pub value: T,
    /// The time it was recorded at.
    pub time: DateTime,
}

/// The temporal values of one attribute in simplified temporal representation.
///
/// There is one variant per attribute kind (NGSI-LD clause 4.5.9).
///
/// The enum is exhaustive: clause 4.5.9 lists a simplified temporal form for each of the eight kinds, which the
/// specification closes. A payload may be the NGSI-LD Null a broker records for a deleted instance (clause 4.5.0).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemporalValues {
    /// A Property: `values`, each a value.
    Property(Vec<TemporalValue<Value>>),
    /// A `GeoProperty`: `values`, each a geometry.
    GeoProperty(Vec<TemporalValue<GeoValue>>),
    /// A `LanguageProperty`: `languageMaps`, each written `{"languageMap": ...}`.
    LanguageProperty(Vec<TemporalValue<LanguageMap>>),
    /// A `VocabProperty`: `vocabs`, each written `{"vocab": ...}`.
    VocabProperty(Vec<TemporalValue<OneOrMany<VocabTerm>>>),
    /// A `ListProperty`: `valueLists`, each an ordered array of values.
    ListProperty(Vec<TemporalValue<Vec<Value>>>),
    /// A `JsonProperty`: `jsons`, each written `{"json": ...}`.
    JsonProperty(Vec<TemporalValue<RawJson>>),
    /// A Relationship: `objects`, each one entity id or several.
    Relationship(Vec<TemporalValue<OneOrMany<EntityId>>>),
    /// A `ListRelationship`: `objectLists`, each an ordered array of entity ids.
    ListRelationship(Vec<TemporalValue<Vec<EntityId>>>),
}

/// The member that holds the temporal values of `kind` (clause 4.5.9).
const fn values_member(kind: AttributeKind) -> &'static str {
    match kind {
        AttributeKind::Property | AttributeKind::GeoProperty => "values",
        AttributeKind::LanguageProperty => "languageMaps",
        AttributeKind::VocabProperty => "vocabs",
        AttributeKind::ListProperty => "valueLists",
        AttributeKind::JsonProperty => "jsons",
        AttributeKind::Relationship => "objects",
        AttributeKind::ListRelationship => "objectLists",
    }
}

impl TemporalValues {
    /// The kind of the attribute.
    #[must_use]
    pub const fn kind(&self) -> AttributeKind {
        match self {
            Self::Property(_) => AttributeKind::Property,
            Self::GeoProperty(_) => AttributeKind::GeoProperty,
            Self::LanguageProperty(_) => AttributeKind::LanguageProperty,
            Self::VocabProperty(_) => AttributeKind::VocabProperty,
            Self::ListProperty(_) => AttributeKind::ListProperty,
            Self::JsonProperty(_) => AttributeKind::JsonProperty,
            Self::Relationship(_) => AttributeKind::Relationship,
            Self::ListRelationship(_) => AttributeKind::ListRelationship,
        }
    }

    /// Returns the number of data points.
    #[must_use]
    pub const fn len(&self) -> usize {
        match self {
            Self::Property(values) => values.len(),
            Self::GeoProperty(values) => values.len(),
            Self::LanguageProperty(values) => values.len(),
            Self::VocabProperty(values) => values.len(),
            Self::ListProperty(values) => values.len(),
            Self::JsonProperty(values) => values.len(),
            Self::Relationship(values) => values.len(),
            Self::ListRelationship(values) => values.len(),
        }
    }

    /// Whether there is no data point.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Writes the attribute: `type`, then the member of its kind.
    pub(crate) fn to_json(&self) -> Json {
        let ids = |ids: &[EntityId]| Json::Array(ids.iter().map(|id| Json::from(id.as_str())).collect());
        let pairs = match self {
            Self::Property(values) => write_pairs(values, Value::to_json),
            Self::GeoProperty(values) => write_pairs(values, GeoValue::to_json),
            Self::LanguageProperty(values) => write_pairs(values, |map| wrapped("languageMap", map.to_json())),
            Self::VocabProperty(values) => write_pairs(values, |vocab| wrapped("vocab", vocab.to_json(|term| Json::from(term.to_string())))),
            Self::ListProperty(values) => write_pairs(values, |list| Json::Array(list.iter().map(Value::to_json).collect())),
            // The raw JSON is carried verbatim, so the tree is copied as it is: the values are written from a borrow,
            // and the JSON tree owns its members.
            Self::JsonProperty(values) => write_pairs(values, |json| wrapped("json", json.to_json())),
            Self::Relationship(values) => write_pairs(values, |object| object.to_json(|id| Json::from(id.as_str()))),
            Self::ListRelationship(values) => write_pairs(values, |list| ids(list)),
        };
        temporal_attribute(self.kind(), values_member(self.kind()), pairs)
    }

    /// Reads an attribute: its `type` decides the kind, and the kind the one other member and its payloads.
    pub(crate) fn from_json(json: Json) -> Result<Self, DocumentError> {
        let (kind, pairs) = read_temporal_attribute(json, values_member)?;
        Ok(match kind {
            AttributeKind::Property => Self::Property(read_pairs(pairs, value)?),
            AttributeKind::GeoProperty => Self::GeoProperty(read_pairs(pairs, |json| geo_value(&json))?),
            AttributeKind::LanguageProperty => Self::LanguageProperty(read_pairs(pairs, |json| {
                LanguageMap::from_json(unwrap("languageMap", json)?).map_err(|source| DocumentError::top(DocumentErrorKind::LanguageMap { source }))
            })?),
            AttributeKind::VocabProperty => Self::VocabProperty(read_pairs(pairs, |json| {
                one_or_many(unwrap("vocab", json)?, |item| parsed::<VocabTerm, _>(item, "vocab"))
            })?),
            AttributeKind::ListProperty => Self::ListProperty(read_pairs(pairs, |json| array(json)?.into_iter().map(value).collect())?),
            AttributeKind::JsonProperty => Self::JsonProperty(read_pairs(pairs, |json| {
                let json = RawJson::from_json(unwrap("json", json)?);
                // A deleted `JsonProperty` holds the NGSI-LD Null (clause 4.5.0).
                if json.is_json_property_payload() || json.is_ngsi_ld_null() {
                    Ok(json)
                } else {
                    Err(DocumentError::top(DocumentErrorKind::JsonShape))
                }
            })?),
            AttributeKind::Relationship => Self::Relationship(read_pairs(pairs, |json| one_or_many(json, |item| entity_id(item, "objects")))?),
            AttributeKind::ListRelationship => Self::ListRelationship(read_pairs(pairs, |json| {
                array(json)?.into_iter().map(|item| entity_id(item, "objectLists")).collect()
            })?),
        })
    }
}

/// Writes `{member: json}`.
fn wrapped(member: &str, json: Json) -> Json {
    let mut object = Object::new();
    object.push(member.into(), json);
    Json::Object(object)
}

/// Reads the payload of `{member: payload}`, refusing any other member.
fn unwrap(member: &'static str, json: Json) -> Result<Json, DocumentError> {
    let object = object(json)?;
    let mut payload = None;
    for Entry { key, value } in object {
        if key.as_str() == member {
            once(&mut payload, member, value)?;
        } else {
            return Err(unexpected(key.as_str()));
        }
    }
    required(payload, member)
}

/// Writes the list of `[payload, time]` pairs.
pub(crate) fn write_pairs<T>(values: &[TemporalValue<T>], payload: impl Fn(&T) -> Json) -> Json {
    Json::Array(
        values
            .iter()
            .map(|point| Json::Array(vec![payload(&point.value), Json::from(point.time.to_string())].into()))
            .collect(),
    )
}

/// Reads a list of `[payload, time]` pairs, placing an error under the index of its pair.
pub(crate) fn read_pairs<T>(pairs: Vec<Json>, mut payload: impl FnMut(Json) -> Result<T, DocumentError>) -> Result<Vec<TemporalValue<T>>, DocumentError> {
    pairs
        .into_iter()
        .enumerate()
        .map(|(index, pair)| read_pair(pair, &mut payload).map_err(|error| error.at_index(index)))
        .collect()
}

/// Reads one `[payload, time]` pair.
fn read_pair<T>(pair: Json, payload: &mut impl FnMut(Json) -> Result<T, DocumentError>) -> Result<TemporalValue<T>, DocumentError> {
    let [value, time] = entry::<2>(pair)?;
    Ok(TemporalValue {
        value: payload(value)?,
        time: date_time(time, "time")?,
    })
}

/// Reads an array of exactly `N` elements, the entry of a simplified (two) or aggregated (three) temporal attribute.
pub(crate) fn entry<const N: usize>(json: Json) -> Result<[Json; N], DocumentError> {
    let items = array(json)?;
    let found = items.len();
    <[Json; N]>::try_from(items).map_err(|_| DocumentError::top(DocumentErrorKind::TemporalEntry { expected: N, found }))
}

/// Writes `{"type": kind, member: json}`.
pub(crate) fn temporal_attribute(kind: AttributeKind, member: &str, json: Json) -> Json {
    let mut object = Object::new();
    object.push("type".into(), Json::from(kind.as_str()));
    object.push(member.into(), json);
    Json::Object(object)
}

/// Reads `{"type": kind, member: [...]}`, where `member` is the one member the kind allows; returns the kind and the
/// elements of the member's array.
pub(crate) fn read_temporal_attribute(json: Json, member_of: impl Fn(AttributeKind) -> &'static str) -> Result<(AttributeKind, Vec<Json>), DocumentError> {
    let object = unalias_attribute(object(json)?)?;
    let kind = match object.get_unique("type").ok().flatten() {
        Some(Json::String(found)) => found
            .as_str()
            .parse::<AttributeKind>()
            .map_err(|_| DocumentError::top(DocumentErrorKind::UnknownAttributeType { found: found.to_string() }))?,
        Some(_) => {
            return Err(DocumentError::top(
                MemberError::WrongShape {
                    member: "type",
                    expected: JsonType::String,
                }
                .into(),
            ));
        }
        None => return Err(DocumentError::top(MemberError::MissingMember { member: "type" }.into())),
    };
    let expected = member_of(kind);
    let mut items = None;
    for Entry { key, value } in object {
        match key.as_str() {
            "type" => {}
            member if member == expected => once(&mut items, expected, array(value).map_err(|error| error.within(expected))?)?,
            other => return Err(unexpected(other)),
        }
    }
    Ok((kind, required(items, expected)?))
}
