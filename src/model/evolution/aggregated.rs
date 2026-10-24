//! The aggregated temporal representation of an entity (NGSI-LD clause 4.5.19, `format=aggregatedValues`): each
//! attribute reduced, per aggregation method, to one value per period of the temporal query.

use crate::model::{
    AttributeKind,
    AttributeName,
    Complete,
    Completeness,
    Context,
    DateTime,
    DocumentError,
    DocumentErrorKind,
    EntityId,
    EntityTypeName,
    OneOrMany,
    Projected,
    RawJson,
    Value,
    entity::member::EntityMember,
    evolution::simplified::values::entry,
    reader::{array, attribute_name, context, date_time, duplicate, entity_types, object, once, parse_json, required, string, unexpected, value},
    representation::{
        member::{entity_id, identifying},
        write::{write_context, write_identity},
    },
};
use indexmap::IndexMap;
use jstrict::{Object, Value as Json, object::Entry};

pub(super) mod output;

/// An aggregation method (NGSI-LD clause 4.5.19.1), the key of a member of an aggregated temporal attribute and
/// an item of the `aggrMethods` parameter.
///
/// The enum is exhaustive: clause 4.5.19.1 closes the set.
///
/// ```
/// use ngsi_ld::model::evolution::AggregationMethod;
///
/// assert_eq!(
///     "distinctCount".parse::<AggregationMethod>(),
///     Ok(AggregationMethod::DistinctCount)
/// );
/// assert_eq!(AggregationMethod::Sumsq.to_string(), "sumsq");
/// assert!("median".parse::<AggregationMethod>().is_err());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::Display, strum::EnumString, strum::IntoStaticStr, strum::EnumIter)]
#[strum(serialize_all = "camelCase")]
pub enum AggregationMethod {
    /// `avg`: the average of the values inside the period.
    Avg,
    /// `distinctCount`: the number of distinct values inside the period.
    DistinctCount,
    /// `max`: the greatest value inside the period.
    Max,
    /// `min`: the least value inside the period.
    Min,
    /// `stddev`: the standard deviation of the values inside the period.
    Stddev,
    /// `sum`: the sum of the values inside the period.
    Sum,
    /// `sumsq`: the sum of the squares of the values inside the period.
    Sumsq,
    /// `totalCount`: the number of times the value was updated inside the period.
    TotalCount,
}

/// The kind of an aggregated temporal attribute: clause 4.5.19 aggregates Properties and Relationships only.
///
/// The enum is exhaustive: the specification closes the set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AggregatedKind {
    /// A Property.
    Property,
    /// A Relationship.
    Relationship,
}

impl AggregatedKind {
    /// The attribute kind.
    #[must_use]
    pub const fn attribute_kind(self) -> AttributeKind {
        match self {
            Self::Property => AttributeKind::Property,
            Self::Relationship => AttributeKind::Relationship,
        }
    }
}

/// An attribute kind clause 4.5.19 does not aggregate: it aggregates Properties and Relationships only.
///
/// The struct is `#[non_exhaustive]`: its field is readable, and only the crate builds it.
///
/// ```
/// use ngsi_ld::model::{
///     AttributeKind,
///     evolution::{AggregatedKind, NotAggregatable},
/// };
///
/// assert_eq!(
///     AggregatedKind::try_from(AttributeKind::Relationship),
///     Ok(AggregatedKind::Relationship)
/// );
/// let refused = AggregatedKind::try_from(AttributeKind::GeoProperty)
///     .err()
///     .map(|error: NotAggregatable| error.kind);
/// assert_eq!(refused, Some(AttributeKind::GeoProperty));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
#[error("a {kind} has no aggregated temporal representation")]
#[non_exhaustive]
pub struct NotAggregatable {
    /// The kind refused.
    pub kind: AttributeKind,
}

/// Fails with [`NotAggregatable`] for every kind but Property and Relationship (clause 4.5.19).
impl TryFrom<AttributeKind> for AggregatedKind {
    type Error = NotAggregatable;

    fn try_from(kind: AttributeKind) -> Result<Self, Self::Error> {
        match kind {
            AttributeKind::Property => Ok(Self::Property),
            AttributeKind::Relationship => Ok(Self::Relationship),
            other @ (AttributeKind::GeoProperty
            | AttributeKind::LanguageProperty
            | AttributeKind::VocabProperty
            | AttributeKind::ListProperty
            | AttributeKind::JsonProperty
            | AttributeKind::ListRelationship) => Err(NotAggregatable { kind: other }),
        }
    }
}

/// The value an aggregation method gives over one period (clause 4.5.19): the value, then the start and the end of
/// the period.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AggregatedValue {
    /// The value the method gives over the period.
    pub value: Value,
    /// The start of the period.
    pub start: DateTime,
    /// The end of the period.
    pub end: DateTime,
}

/// One attribute in aggregated temporal representation (clause 4.5.19): its kind and, per aggregation method, one
/// value per period, the methods in document order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AggregatedAttribute {
    /// `type`.
    pub kind: AggregatedKind,
    /// The values of each method, by period.
    pub methods: IndexMap<AggregationMethod, Vec<AggregatedValue>>,
}

impl AggregatedAttribute {
    /// Writes the attribute: `type`, then one member per method.
    fn to_json(&self) -> Json {
        let mut object = Object::new();
        object.push("type".into(), Json::from(self.kind.attribute_kind().as_str()));
        for (method, periods) in &self.methods {
            let periods = periods
                .iter()
                .map(|period| Json::Array(vec![period.value.to_json(), Json::from(period.start.to_string()), Json::from(period.end.to_string())].into()));
            object.push(<&str>::from(*method).into(), Json::Array(periods.collect()));
        }
        Json::Object(object)
    }

    /// Reads an attribute: its `type`, then one member per method, each an array of `[value, start, end]`.
    fn from_json(json: Json) -> Result<Self, DocumentError> {
        let object = object(json)?;
        let mut kind = None;
        let mut methods = IndexMap::new();
        for Entry { key, value: member } in object {
            let key = key.as_str();
            if key == "type" {
                once(&mut kind, key, aggregated_kind(member)?)?;
                continue;
            }
            let method = key.parse::<AggregationMethod>().map_err(|_| unexpected(key))?;
            let periods = array(member)?
                .into_iter()
                .enumerate()
                .map(|(index, period)| aggregated_value(period).map_err(|error| error.at_index(index)))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| error.within(key))?;
            if methods.insert(method, periods).is_some() {
                return Err(duplicate(key));
            }
        }
        Ok(Self {
            kind: required(kind, "type")?,
            methods,
        })
    }
}

/// Reads the `type` of an aggregated attribute.
fn aggregated_kind(json: Json) -> Result<AggregatedKind, DocumentError> {
    let found = string(json, "type")?;
    let kind = found
        .parse::<AttributeKind>()
        .map_err(|_| DocumentError::top(DocumentErrorKind::UnknownAttributeType { found }))?;
    AggregatedKind::try_from(kind).map_err(|error| DocumentError::top(DocumentErrorKind::AggregatedKind { found: error.kind }))
}

/// Reads one period: `[value, start, end]`.
fn aggregated_value(json: Json) -> Result<AggregatedValue, DocumentError> {
    let [aggregate, start, end] = entry::<3>(json)?;
    Ok(AggregatedValue {
        value: value(aggregate)?,
        start: date_time(start, "start")?,
        end: date_time(end, "end")?,
    })
}

/// The temporal evolution of an entity in aggregated temporal representation (NGSI-LD clause 4.5.19): `id`,
/// `type`, each attribute's [`AggregatedAttribute`], and the `@context`.
///
/// Only a broker writes it, as the answer to `format=aggregatedValues` (clause 6.3.12). `C` says whether the entity
/// holds its `id` and `type`, which a projection may remove (clause 4.21).
///
/// ```
/// use ngsi_ld::model::evolution::{AggregatedTemporalEntity, AggregationMethod};
///
/// let vehicle = AggregatedTemporalEntity::parse(
///     r#"{
///       "id": "urn:ngsi-ld:Vehicle:B9211",
///       "type": "Vehicle",
///       "speed": {
///         "type": "Property",
///         "max": [[120, "2018-08-01T12:00:00Z", "2018-08-01T12:04:00Z"]]
///       }
///     }"#,
/// )?;
/// let speed = vehicle.attributes.values().next();
/// assert_eq!(
///     speed
///         .and_then(|speed| speed.methods.get(&AggregationMethod::Max))
///         .map(Vec::len),
///     Some(1)
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct AggregatedTemporalEntity<C: Completeness = Complete> {
    /// `id`; optional only in a projected entity.
    pub id: C::Member<EntityId>,
    /// `type`; optional only in a projected entity.
    pub entity_type: C::Member<OneOrMany<EntityTypeName>>,
    /// The aggregated attributes, in document order.
    pub attributes: IndexMap<AttributeName, AggregatedAttribute>,
    /// The `@context` the document carried; [`render`](Self::render) writes it last.
    pub context: Option<Context>,
}

impl<C: Completeness> AggregatedTemporalEntity<C> {
    /// Writes the entity in aggregated temporal representation: `id`, `type`, the attributes in their order, and the
    /// `@context` last when the entity has one.
    #[must_use]
    pub fn render(&self) -> RawJson {
        let mut object = Object::new();
        write_identity::<C>(&mut object, &self.id, &self.entity_type);
        for (name, attribute) in &self.attributes {
            object.push(name.as_ref().into(), attribute.to_json());
        }
        write_context(&mut object, self.context.as_ref());
        RawJson::from_json(Json::Object(object))
    }

    /// Reads an entity in aggregated temporal representation from JSON already parsed, such as an element of the
    /// array Query Temporal Evolution answers with.
    pub(crate) fn from_json(json: Json) -> Result<Self, DocumentError> {
        let object = object(json)?;
        let mut id = None;
        let mut entity_type = None;
        let mut entity_context = None;
        let mut attributes = IndexMap::new();
        for Entry { key, value: member } in object {
            let key = key.as_str();
            match key.parse::<EntityMember>() {
                Ok(EntityMember::Id) => once(&mut id, key, entity_id(member, "id")?)?,
                Ok(EntityMember::Type) => once(&mut entity_type, key, entity_types(member)?)?,
                Ok(EntityMember::Context) => once(&mut entity_context, key, context(member)?)?,
                // Clause 4.5.19 lists `id`, `type` and `@context` besides the attributes.
                Ok(EntityMember::Scope | EntityMember::ExpiresAt) => {
                    return Err(unexpected(key));
                }
                Err(_) => {
                    let name = attribute_name(key)?;
                    let attribute = AggregatedAttribute::from_json(member).map_err(|error| error.within(key))?;
                    if attributes.insert(name, attribute).is_some() {
                        return Err(duplicate(key));
                    }
                }
            }
        }
        Ok(Self {
            id: identifying::<C, _>(id, "id")?,
            entity_type: identifying::<C, _>(entity_type, "type")?,
            attributes,
            context: entity_context,
        })
    }
}

impl AggregatedTemporalEntity {
    /// Reads an entity in aggregated temporal representation from JSON text, keeping its `@context`.
    ///
    /// # Errors
    ///
    /// Returns [`DocumentError`] naming the member at fault, including a missing `id` or `type`, an attribute of a
    /// kind clause 4.5.19 does not aggregate, and a member that names no aggregation method.
    pub fn parse(text: &str) -> Result<Self, DocumentError> {
        Self::from_json(parse_json(text)?)
    }
}

impl AggregatedTemporalEntity<Projected> {
    /// Reads an entity a projection may have reduced (clause 4.21) in aggregated temporal representation; `id` and
    /// `type` may be missing.
    ///
    /// # Errors
    ///
    /// Returns [`DocumentError`] naming the member at fault.
    pub fn parse_projected(text: &str) -> Result<Self, DocumentError> {
        Self::from_json(parse_json(text)?)
    }
}
