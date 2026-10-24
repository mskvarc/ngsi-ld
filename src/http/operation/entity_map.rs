//! Entity maps (NGSI-LD clauses 4.5.25, 5.2.39 and 5.14): the `entityMap` and `entityMapLifetime` parameters and
//! the `NGSILD-EntityMap` request header of Retrieve Entity, Query Entities, Retrieve and Query Temporal Evolution
//! (Tables 6.4.3.2-1, 6.4.3.2-2, 6.5.3.1-1, 6.5.3.1-2 and 6.18.3.2-1, clause 5.7.3.3) and Purge Entities (Table
//! 6.4.3.3-2), and the operations on the entity map resources (clauses 6.32, 6.34 and 6.35).

use crate::{
    http::{QueryParameter, headers::NgsiLdEntityMap, query_string::QueryString},
    model::{Duration, DurationError},
    query::EntityMapReturn,
};
use jiff::Span;
use std::str::FromStr;

pub(super) mod create;
pub(super) mod create_temporal;
pub(super) mod delete;
pub(super) mod retrieve;
pub(super) mod update;

/// Why a value is not an entity map lifetime.
#[derive(Debug, Clone, thiserror::Error)]
#[non_exhaustive]
pub enum EntityMapLifetimeError {
    /// The value is not a duration.
    #[error("an entity map lifetime is a duration")]
    Duration {
        /// Why it is not.
        #[from]
        source: DurationError,
    },
    /// The duration is zero.
    #[error("an entity map lifetime is a positive duration")]
    NotPositive,
}

/// How long an entity map is asked to exist (the `entityMapLifetime` parameter): a positive ISO 8601 duration such as
/// `PT1H` or `P1D`. The broker sets the actual expiry, possibly overriding it (Table 6.4.3.2-1).
///
/// The duration keeps its units as written, so `PT90M` is printed back as `PT90M`, not `PT1H30M`.
///
/// ```
/// use ngsi_ld::http::operation::EntityMapLifetime;
///
/// let lifetime: EntityMapLifetime = "PT90M".parse()?;
/// assert_eq!(lifetime.to_string(), "PT90M");
/// assert!("-PT1H".parse::<EntityMapLifetime>().is_err());
/// assert!("PT0S".parse::<EntityMapLifetime>().is_err());
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, derive_more::Display)]
#[display("{_0}")]
pub struct EntityMapLifetime(Duration);

impl EntityMapLifetime {
    /// Returns the duration, with its units as written.
    pub(crate) const fn duration(self) -> Duration {
        self.0
    }

    /// Returns the duration.
    #[must_use]
    pub const fn span(self) -> Span {
        self.0.span()
    }
}

impl TryFrom<Duration> for EntityMapLifetime {
    type Error = EntityMapLifetimeError;

    fn try_from(duration: Duration) -> Result<Self, Self::Error> {
        if duration.is_zero() {
            Err(EntityMapLifetimeError::NotPositive)
        } else {
            Ok(EntityMapLifetime(duration))
        }
    }
}

impl TryFrom<Span> for EntityMapLifetime {
    type Error = EntityMapLifetimeError;

    fn try_from(span: Span) -> Result<Self, Self::Error> {
        Self::try_from(Duration::try_from(span)?)
    }
}

// A derived parser would skip the checks of `TryFrom<Duration>`.
impl FromStr for EntityMapLifetime {
    type Err = EntityMapLifetimeError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::try_from(text.parse::<Duration>()?)
    }
}

/// How a request uses an entity map (NGSI-LD clauses 4.5.25, 5.7.1.3, 5.7.2.3, 5.7.3.3 and 5.7.4.3): whether the
/// location of the map used is returned (`entityMap`), the suggested lifetime of a map the broker creates
/// (`entityMapLifetime`), and an existing map to use (the `NGSILD-EntityMap` header).
///
/// The clauses list the three as independent inputs, so a request may name a map to use and still ask for the
/// location of the map used, which differs when the named map has expired. `entityMap` is the same three-way
/// choice as the member of a `Query` document ([`EntityMapReturn`]), and the lifetime is "only applicable if
/// entityMap is set to true" (Table 6.4.3.2-1): the broker ignores it otherwise. A response that names the map
/// used does so in its `NGSILD-EntityMap` header.
///
/// ```
/// use headers::Header;
/// use iri_rs::IriRefBuf;
/// use ngsi_ld::{
///     http::{
///         Endpoint,
///         headers::NgsiLdEntityMap,
///         operation::{EntityMapRequest, Operation, QueryEntities},
///     },
///     model::Lossless,
///     query::SelectorType,
/// };
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let map = NgsiLdEntityMap::try_new(IriRefBuf::new(
///     "/ngsi-ld/v1/entityMaps/urn:ngsi-ld:EntityMap:1",
/// )?)?;
/// let reuse = EntityMapRequest::reuse(map);
/// let vehicles: SelectorType = "Vehicle".parse()?;
/// let query = QueryEntities::builder()
///     .entity_type(&vehicles)
///     .entity_map(&reuse)
///     .format(Lossless::Normalized)
///     .build();
/// let request = query.to_request(&endpoint)?;
/// assert_eq!(
///     request.headers()[NgsiLdEntityMap::name()],
///     "/ngsi-ld/v1/entityMaps/urn:ngsi-ld:EntityMap:1"
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct EntityMapRequest {
    /// Whether the location of the map used is returned (`entityMap`).
    pub returned: EntityMapReturn,
    /// The suggested lifetime of a map the broker creates (`entityMapLifetime`).
    pub lifetime: Option<EntityMapLifetime>,
    /// The existing map to use (the `NGSILD-EntityMap` request header); the broker creates a new one when it is gone
    /// or expired.
    pub location: Option<NgsiLdEntityMap>,
}

impl EntityMapRequest {
    /// `entityMap=true`: asks for the location of the map used.
    #[must_use]
    pub const fn returned() -> EntityMapRequest {
        EntityMapRequest {
            returned: EntityMapReturn::Returned,
            lifetime: None,
            location: None,
        }
    }

    /// `entityMap=true&entityMapLifetime=...`: asks for the location of the map used, suggesting how long a map
    /// the broker creates exists.
    #[must_use]
    pub const fn returned_for(lifetime: EntityMapLifetime) -> EntityMapRequest {
        EntityMapRequest {
            returned: EntityMapReturn::Returned,
            lifetime: Some(lifetime),
            location: None,
        }
    }

    /// The `NGSILD-EntityMap` header alone: uses the map at `location`.
    #[must_use]
    pub const fn reuse(location: NgsiLdEntityMap) -> EntityMapRequest {
        EntityMapRequest {
            returned: EntityMapReturn::Unspecified,
            lifetime: None,
            location: Some(location),
        }
    }

    /// Also names the map at `location` to use, in the `NGSILD-EntityMap` header.
    #[must_use]
    pub fn reusing(self, location: NgsiLdEntityMap) -> Self {
        Self {
            location: Some(location),
            ..self
        }
    }

    /// The map to name in the `NGSILD-EntityMap` request header.
    pub(crate) const fn header(&self) -> Option<&NgsiLdEntityMap> {
        self.location.as_ref()
    }

    /// Adds the parameters to a query string.
    pub(crate) fn push_query(&self, query: &mut QueryString) {
        query.push_some(QueryParameter::EntityMap, self.returned.flag());
        query.push_some(QueryParameter::EntityMapLifetime, self.lifetime.as_ref());
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        http::{
            headers::NgsiLdEntityMap,
            operation::{EntityMapLifetime, EntityMapLifetimeError, EntityMapRequest, entity_selection::ResultAssembly},
            query_string::QueryString,
        },
        model::DurationError,
        query::{EntityMapReturn, EntitySplitting},
        snapshot::query_document::writer::{QueryMembers, query_object},
    };
    use iri_rs::IriRefBuf;
    use jiff::ToSpan;
    use jstrict::{Print, Value as Json};
    use pretty_assertions::assert_eq;

    #[test]
    fn an_iso_8601_duration_round_trips_with_its_units() {
        for text in ["PT1H", "P1D", "PT90M", "P1DT12H", "PT0.5S"] {
            assert_eq!(text.parse::<EntityMapLifetime>().unwrap().to_string(), text);
        }
        assert_eq!(EntityMapLifetime::try_from(2.hours()).unwrap().to_string(), "PT2H");
    }

    #[test]
    fn zero_negative_and_friendly_durations_are_rejected() {
        assert!(matches!("PT0S".parse::<EntityMapLifetime>(), Err(EntityMapLifetimeError::NotPositive)));
        assert!(matches!(
            "-PT1H".parse::<EntityMapLifetime>(),
            Err(EntityMapLifetimeError::Duration {
                source: DurationError::Negative
            })
        ));
        assert!(matches!(
            "1 hour".parse::<EntityMapLifetime>(),
            Err(EntityMapLifetimeError::Duration {
                source: DurationError::Syntax { .. }
            })
        ));
    }

    /// The map at `/ngsi-ld/v1/entityMaps/urn:ngsi-ld:EntityMap:1`.
    fn map() -> NgsiLdEntityMap {
        NgsiLdEntityMap::try_new(IriRefBuf::new("/ngsi-ld/v1/entityMaps/urn:ngsi-ld:EntityMap:1".to_owned()).unwrap()).unwrap()
    }

    /// The query string and the `Query` body a request writes.
    fn written(request: &EntityMapRequest) -> (String, String) {
        let mut query = QueryString::default();
        request.push_query(&mut query);
        let assembly = ResultAssembly {
            ordering: None,
            split_entities: EntitySplitting::Unspecified,
            entity_map: Some(request),
        };
        let body = query_object(assembly.query_members(QueryMembers::default()));
        (query.to_string(), Json::Object(body).compact_print().to_string())
    }

    #[test]
    fn returning_a_map_sets_the_flag_and_the_lifetime() {
        let request = EntityMapRequest::returned_for("PT1H".parse().unwrap());
        assert_eq!(
            written(&request),
            (
                "entityMap=true&entityMapLifetime=PT1H".to_owned(),
                r#"{"type":"Query","entityMap":true,"entityMapLifetime":"PT1H"}"#.to_owned()
            )
        );
        assert_eq!(request.header(), None);
        assert_eq!(
            written(&EntityMapRequest::returned()),
            ("entityMap=true".to_owned(), r#"{"type":"Query","entityMap":true}"#.to_owned())
        );
    }

    #[test]
    fn the_flag_and_the_header_are_independent() {
        let both = EntityMapRequest::returned().reusing(map());
        assert_eq!(written(&both), ("entityMap=true".to_owned(), r#"{"type":"Query","entityMap":true}"#.to_owned()));
        assert_eq!(both.header(), Some(&map()));
        let reuse = EntityMapRequest::reuse(map());
        assert_eq!(written(&reuse), (String::new(), r#"{"type":"Query"}"#.to_owned()));
        assert_eq!(reuse.header(), Some(&map()));
    }

    #[test]
    fn an_explicit_false_is_written_and_an_unspecified_flag_is_not() {
        let refused = EntityMapRequest {
            returned: EntityMapReturn::NotReturned,
            ..EntityMapRequest::default()
        };
        assert_eq!(
            written(&refused),
            ("entityMap=false".to_owned(), r#"{"type":"Query","entityMap":false}"#.to_owned())
        );
        assert_eq!(written(&EntityMapRequest::default()), (String::new(), r#"{"type":"Query"}"#.to_owned()));
    }
}
