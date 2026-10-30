//! The HTTP binding of each operation: the exact request it builds, how each response is decoded, and the unchecked
//! path. Everything runs against the pure binding, with no network.
#![cfg(feature = "http")]

use ngsi_ld::{
    http::operation::{InlineJoin, QueryEntities, RetrieveEntity},
    model::{EntityId, Projected},
    query::{SelectorType, q},
};

/// `Link` fields built from typed link-values.
#[cfg(test)]
#[path = "support/links.rs"]
mod links;

/// Endpoints, fixtures and response construction.
#[cfg(test)]
mod fixtures {
    use crate::links::CONTEXT;
    use bytes::Bytes;
    use headers::HeaderMapExt;
    use http::{HeaderName, HeaderValue, Request, Response, StatusCode};
    use iri_rs::IriRefBuf;
    use jstrict::Value as Json;
    use ngsi_ld::{
        http::{ContextDelivery, Endpoint, ResponseContext, headers::Link},
        model::{Entity, EntityId, Input, Lossless, Output, Representation},
    };
    use std::{fs, str};
    use url::Url;

    /// The test endpoint, delivering the context in a `Link` header.
    pub(crate) fn endpoint() -> Endpoint {
        endpoint_with(ContextDelivery::JsonWithLink)
    }

    /// The test endpoint with a chosen context delivery.
    pub(crate) fn endpoint_with(delivery: ContextDelivery) -> Endpoint {
        Endpoint::builder()
            .base_url(Url::parse("http://broker.test/ngsi-ld/v1/").unwrap())
            .context(IriRefBuf::new(CONTEXT).unwrap())
            .context_delivery(delivery)
            .build()
            .unwrap()
    }

    /// The context of a response to a request on the default tenant, for the decoding tests that do not depend on
    /// the request.
    pub(crate) fn context() -> ResponseContext {
        let request = Request::get("http://broker.test/ngsi-ld/v1/entities").body(Bytes::new()).unwrap();
        ResponseContext::new(&request)
    }

    /// The project's vehicle fixture, as a client sends it.
    pub(crate) fn vehicle() -> Entity<Input> {
        let text = fs::read_to_string(format!("{}/tests/golden/authored/vehicle-normalized.json", env!("CARGO_MANIFEST_DIR"))).unwrap();
        Entity::<Output>::parse(&text, Lossless::Normalized).unwrap().into_input().unwrap()
    }

    /// Parses JSON text.
    pub(crate) fn json(text: &[u8]) -> Json {
        str::from_utf8(text).unwrap().parse().unwrap()
    }

    /// A response with a status, headers and a body.
    pub(crate) fn response(status: u16, headers: &[(&HeaderName, &str)], body: &str) -> Response<Bytes> {
        let mut response = Response::new(Bytes::copy_from_slice(body.as_bytes()));
        *response.status_mut() = StatusCode::from_u16(status).unwrap();
        for &(name, value) in headers {
            response.headers_mut().append(name, HeaderValue::from_str(value).unwrap());
        }
        response
    }

    /// The value of a header, as text.
    pub(crate) fn header_value<'a>(request: &'a Request<Bytes>, name: &HeaderName) -> Option<&'a str> {
        request.headers().get(name).map(|value| value.to_str().unwrap())
    }

    /// The `Link` header of a request, read as the typed header.
    pub(crate) fn link_of(request: &Request<Bytes>) -> Option<Link> {
        request.headers().typed_get()
    }

    /// The vehicle as the body of a request, without its context.
    pub(crate) fn vehicle_body() -> Json {
        let mut body = json(vehicle().render(Representation::Normalized).to_string().as_bytes());
        // The document the fixture is read from names its context; a request body under `json-with-link` does not.
        body.as_object_mut().unwrap().remove("@context").for_each(drop);
        body
    }

    pub(crate) fn vehicle_id() -> EntityId {
        "urn:ngsi-ld:Vehicle:A4567".parse().unwrap()
    }
}

/// The Retrieve Entity and Query Entities parameters.
#[cfg(test)]
#[path = "binding/query_parameters.rs"]
mod query_parameters;

/// The flags and inputs a client may state explicitly: `details=false`, `reload=false`, the members beside an added
/// context, the entity map inputs of Purge Entities and the temporal operations, and the temporal representation
/// parameter.
#[cfg(test)]
#[path = "binding/request_flags.rs"]
mod request_flags;

/// The `GeoJSON` representation.
#[cfg(test)]
#[path = "binding/geo_json.rs"]
mod geo_json;

/// Projected entities.
#[cfg(test)]
#[path = "binding/projected_entities.rs"]
mod projected_entities;

/// Projected temporal entities.
#[cfg(test)]
#[path = "binding/projected_temporal.rs"]
mod projected_temporal;

/// The entity map on the following pages of a query.
#[cfg(test)]
#[path = "binding/entity_map_pages.rs"]
mod entity_map_pages;

/// Merge Entity with a simplified fragment.
#[cfg(test)]
#[path = "binding/merge_simplified.rs"]
mod merge_simplified;

/// The query strings of the listings and the entity map creations, read back in typed form.
#[cfg(test)]
#[path = "binding/received_listings.rs"]
mod received_listings;

/// The query strings of forwarded write operations, read back in typed form.
#[cfg(test)]
#[path = "binding/received_writes.rs"]
mod received_writes;

/// PATCH bodies sent as `application/merge-patch+json` (clause 6.3.4).
#[cfg(test)]
#[path = "binding/merge_patch.rs"]
mod merge_patch;

/// The batch operations.
#[cfg(test)]
#[path = "binding/batch.rs"]
mod batch;

/// The geoquery and scope query parameters.
#[cfg(test)]
#[path = "binding/geo_and_scope_queries.rs"]
mod geo_and_scope_queries;

/// The subscription operations.
#[cfg(test)]
#[path = "binding/subscriptions.rs"]
mod subscriptions;

/// The documents and helpers the context source registration tests share.
#[cfg(test)]
#[path = "binding/registration_fixtures.rs"]
mod registration_fixtures;

/// The requests of the single context source registration operations.
#[cfg(test)]
#[path = "binding/registration_requests.rs"]
mod registration_requests;

/// Query Context Source Registrations.
#[cfg(test)]
#[path = "binding/registration_queries.rs"]
mod registration_queries;

/// The responses of the context source registration operations.
#[cfg(test)]
#[path = "binding/registration_responses.rs"]
mod registration_responses;

/// The Context Source Registration Subscription operations.
#[cfg(test)]
#[path = "binding/registration_subscriptions.rs"]
mod registration_subscriptions;

/// The documents and helpers the temporal operation tests share.
#[cfg(test)]
#[path = "binding/temporal_fixtures.rs"]
mod temporal_fixtures;

/// The requests of the temporal operations.
#[cfg(test)]
#[path = "binding/temporal_requests.rs"]
mod temporal_requests;

/// The answers of the temporal operations.
#[cfg(test)]
#[path = "binding/temporal_responses.rs"]
mod temporal_responses;

/// The entity type and attribute discovery operations.
#[cfg(test)]
#[path = "binding/discovery.rs"]
mod discovery;

/// The stored `@context` operations.
#[cfg(test)]
#[path = "binding/contexts.rs"]
mod contexts;

/// The entity map and source identity operations.
#[cfg(test)]
#[path = "binding/entity_maps.rs"]
mod entity_maps;

/// The snapshot operations and the snapshot specification.
#[cfg(test)]
#[path = "binding/snapshots.rs"]
mod snapshots;

/// The deprecated `attrs` parameter.
#[cfg(test)]
#[path = "binding/attrs_parameter.rs"]
mod attrs_parameter;

/// The entity selectors, `type=*`, id patterns, representation spellings and projected inline joins of Query Entities
/// and Retrieve Entity.
#[cfg(test)]
#[path = "binding/query_selection.rs"]
mod query_selection;

/// Query Entities and Retrieve Entity requests read back in typed form, as a context source receives them.
#[cfg(test)]
#[path = "binding/received_queries.rs"]
mod received_queries;

/// Every type and whole id patterns beyond Query Entities, the temporal query strings read back in typed form, and
/// POST bodies that are exactly the `Query` document rendered.
#[cfg(test)]
#[path = "binding/received_temporal_queries.rs"]
mod received_temporal_queries;

/// The `local` parameter of clause 6.3.18, and the implicit locality of an operation on a snapshot.
#[cfg(test)]
#[path = "binding/locality.rs"]
mod locality;

/// The `Prefer` header of clause 6.3.6.
#[cfg(test)]
#[path = "binding/preferences.rs"]
mod preferences;

/// The `@context`, `Accept` and further headers an endpoint states on every request, and the head of every error.
#[cfg(test)]
#[path = "binding/request_overrides.rs"]
mod request_overrides;

/// Reading a notification request a broker sent.
#[cfg(test)]
#[path = "binding/notification_delivery.rs"]
mod notification_delivery;

/// Create or Update Temporal Evolution in simplified temporal representation.
#[cfg(test)]
#[path = "binding/simplified_temporal_input.rs"]
mod simplified_temporal_input;

/// The `NGSILD-Warning` header of clause 6.3.17.
#[cfg(test)]
#[path = "binding/warnings.rs"]
mod warnings;

/// The requests of the entity operations.
#[cfg(test)]
#[path = "binding/entity_requests.rs"]
mod entity_requests;

/// The responses of the entity operations.
#[cfg(test)]
#[path = "binding/entity_responses.rs"]
mod entity_responses;

/// The unchecked path.
#[cfg(test)]
#[path = "binding/unchecked.rs"]
mod unchecked;

/// Retrieves the entity `id` in `format`, as complete entities, with no other parameter.
fn retrieve_entity<F>(id: &EntityId, format: F) -> RetrieveEntity<'_, F> {
    RetrieveEntity::builder().id(id).format(format).build()
}

/// Retrieves the entity `id` in `format`, as projected entities, with no other parameter.
fn retrieve_projected<F>(id: &EntityId, format: F) -> RetrieveEntity<'_, F, Option<InlineJoin>, Projected> {
    RetrieveEntity::projected_builder().id(id).format(format).build()
}

/// Queries complete entities in `format`, with no other parameter than `entity_type` and `q`.
fn query_vehicles<'a, F>(entity_type: Option<&'a SelectorType>, q: Option<&'a q::Query>, format: F) -> QueryEntities<'a, F> {
    QueryEntities::builder().maybe_entity_type(entity_type).maybe_q(q).format(format).build()
}

/// Queries projected entities in `format`, with no other parameter than `entity_type` and `q`.
fn query_projected<'a, F>(entity_type: Option<&'a SelectorType>, q: Option<&'a q::Query>, format: F) -> QueryEntities<'a, F, Projected> {
    QueryEntities::projected_builder()
        .maybe_entity_type(entity_type)
        .maybe_q(q)
        .format(format)
        .build()
}
