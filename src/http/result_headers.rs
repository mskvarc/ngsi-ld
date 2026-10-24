//! The headers a page of a query and a retrieval read beside their body (NGSI-LD clauses 4.3.6.8, 6.3.10, 6.3.17
//! and 6.4.3.2): the entity map the broker used, the version it amended the payload to, the warnings of a
//! distributed operation and the part of the requested time range the answer covers.

use crate::http::{
    Coverage,
    headers::{NgsiLdEntityMap, NgsiLdWarnings, OptionalHeader, PreferenceApplied, SpecVersion},
    response::Success,
};

/// The headers a [`Page`](crate::http::Page) and a [`Retrieved`](crate::http::Retrieved) share,
/// read leniently: a malformed `NGSILD-EntityMap` or `NGSILD-Warning` is kept as [`OptionalHeader::Malformed`], and
/// a `Preference-Applied` without a readable `ngsi-ld` version is ignored, as RFC 7240 makes preferences advisory.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ResultHeaders {
    /// The entity map the broker used, from `NGSILD-EntityMap`.
    pub(crate) entity_map: OptionalHeader<NgsiLdEntityMap>,
    /// The version the payload conforms to, from `Preference-Applied`.
    pub(crate) preference_applied: Option<SpecVersion>,
    /// The warnings of a distributed operation, from `NGSILD-Warning`.
    pub(crate) warnings: OptionalHeader<NgsiLdWarnings>,
    /// The part of the requested time range the answer covers.
    pub(crate) coverage: Coverage,
}

impl ResultHeaders {
    /// Reads the headers of a successful response.
    pub(crate) fn read(response: &Success) -> ResultHeaders {
        ResultHeaders {
            entity_map: OptionalHeader::read(response.headers()),
            preference_applied: OptionalHeader::<PreferenceApplied>::read(response.headers())
                .into_valid()
                .map(|applied| applied.version()),
            warnings: OptionalHeader::read(response.headers()),
            coverage: Coverage::read(response),
        }
    }
}
