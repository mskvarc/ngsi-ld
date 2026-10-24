//! The answer of a Retrieve Entity or Retrieve Temporal Evolution operation, and of the creation of an entity map
//! (NGSI-LD clauses 5.7.1, 5.7.3, 5.14.4, 5.14.5, 6.5.3.1, 6.19.3.1, 6.34.3 and 6.35.3).

use crate::{
    http::{
        Coverage,
        ResponseError,
        headers::{NgsiLdEntityMap, NgsiLdWarnings, OptionalHeader, SpecVersion, ngsild_warning::combine},
        operation::TemporalSelection,
        response::{Success, classify},
        result_headers::ResultHeaders,
    },
    model::{Completeness, Output, evolution::TemporalEntity},
    query::temporal_query::TemporalProperty,
};
use bytes::Bytes;
use http::{Response, StatusCode};

/// What Retrieve Entity and Retrieve Temporal Evolution return.
///
/// It holds the entity (or, for a flattened join, the entities), with the location of the Entity map the broker used
/// (NGSI-LD clause 6.5.3.1), the version it amended the payload to when it answered 203 (clause 4.3.6.8), the warnings
/// it attached when a distributed part of the retrieval was served from a cache or behaved abnormally (clause 6.3.17),
/// and, for a temporal evolution, the part of the requested time range a 206 Partial Content covers (clause 6.3.10).
///
/// The creation of an entity map (Tables 6.34.3.1-1 to 6.35.3.2-1) answers with the same parts: the map in the body,
/// and its location in `NGSILD-EntityMap`.
///
/// Like a [`Page`](crate::http::Page), it is read leniently: a malformed `NGSILD-EntityMap` is kept as
/// [`OptionalHeader::Malformed`] and a `Preference-Applied` without a readable `ngsi-ld` version is ignored, so the
/// entity is never lost to a header.
#[derive(Debug, Clone, PartialEq)]
pub struct Retrieved<T> {
    /// The entity or entities.
    body: T,
    /// The entity map, version, warnings and coverage the headers carry.
    headers: ResultHeaders,
}

impl<T> Retrieved<T> {
    /// The entity, or the entities of a flattened join.
    #[must_use]
    pub const fn body(&self) -> &T {
        &self.body
    }

    /// Takes the entity, or the entities of a flattened join.
    #[must_use]
    pub fn into_body(self) -> T {
        self.body
    }

    /// The location of the entity map the broker used, which it returns when the request asked for one
    /// (`NGSILD-EntityMap`, clause 6.5.3.1).
    #[must_use]
    pub const fn entity_map(&self) -> &OptionalHeader<NgsiLdEntityMap> {
        &self.headers.entity_map
    }

    /// The NGSI-LD version the broker says the payload conforms to (`Preference-Applied`, clause 6.3.6).
    #[must_use]
    pub const fn preference_applied(&self) -> Option<&SpecVersion> {
        self.headers.preference_applied.as_ref()
    }

    /// The warnings the broker attached when a distributed part of the retrieval was served from a cache or behaved
    /// abnormally (`NGSILD-Warning`, clause 6.3.17); [`OptionalHeader::Absent`] when it attached none.
    #[must_use]
    pub const fn warnings(&self) -> &OptionalHeader<NgsiLdWarnings> {
        &self.headers.warnings
    }

    /// The part of the requested time range the answer covers: [`Coverage::Partial`] when Retrieve Temporal
    /// Evolution answered 206 Partial Content (clause 6.3.10), [`Coverage::Whole`] otherwise.
    /// [`Coverage::following_query`] is the temporal query of the rest of the range.
    #[must_use]
    pub const fn coverage(&self) -> &Coverage {
        &self.headers.coverage
    }

    /// Reads a retrieval whose status is one of `statuses`, its body with `read`.
    pub(crate) fn decode(
        response: Response<Bytes>,
        statuses: &[StatusCode],
        read: impl FnOnce(&Success) -> Result<T, ResponseError>,
    ) -> Result<Retrieved<T>, ResponseError> {
        let response = classify(response)?;
        if !statuses.contains(&response.status()) {
            return Err(response.unexpected());
        }
        Ok(Retrieved {
            body: read(&response)?,
            headers: ResultHeaders::read(&response),
        })
    }
}

impl<C: Completeness> Retrieved<TemporalEntity<Output, C>> {
    /// This answer joined with `later`, the answer for the rest of the time range of a request that selected its
    /// instances with `selection` (clause 6.3.10): the instances of both (see [`TemporalEntity::absorb`]), only the
    /// last `lastN` of each attribute when the request has one (see [`TemporalEntity::absorb_last`]), the warnings
    /// of both, and the coverage of `later`; the entity map and the version are this answer's.
    pub(crate) fn followed_by(mut self, later: Retrieved<TemporalEntity<Output, C>>, selection: TemporalSelection<'_>) -> Retrieved<TemporalEntity<Output, C>> {
        match selection.last_n() {
            // Without a `timeproperty`, `lastN` counts by `observedAt` (clause 4.11).
            Some(last_n) => self
                .body
                .absorb_last(later.body, last_n, selection.time_property().unwrap_or(TemporalProperty::ObservedAt)),
            None => self.body.absorb(later.body),
        }
        self.headers.warnings = combine(self.headers.warnings, later.headers.warnings);
        self.headers.coverage = later.headers.coverage;
        self
    }
}
