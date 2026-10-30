//! The HTTP binding of the attribute operations and Purge Entities: their requests and their responses, against the
//! pure binding.
#![cfg(feature = "http")]

use crate::fixtures::{context, endpoint, endpoint_with, json, response, speed, vehicle_id};
use http::{HeaderValue, Method, StatusCode, header};
use jstrict::Unordered;
use ngsi_ld::{
    http::{
        BatchOutcome,
        ContextDelivery,
        RequestError,
        ResponseError,
        UpdateOutcome,
        operation::{
            AppendAttributes,
            ArgumentError,
            DeleteAttribute,
            InstanceSelection,
            Operation,
            Overwrite,
            PartialAttributeUpdate,
            PurgeEntities,
            ReplaceAttribute,
            UpdateAttributes,
        },
    },
    model::{Attribute, AttributeFragment, AttributeMembers, AttributeName, Change, DatasetId, EntityFragment, Lossless, Property},
    query::{Locality, SelectorType},
};
use pretty_assertions::assert_eq;

/// The NGSI-LD Null in partial update and merge patches.
#[cfg(test)]
#[path = "binding_attributes/null_patches.rs"]
mod null_patches;

/// Endpoints, fixtures and response construction.
#[cfg(test)]
mod fixtures {
    use bytes::Bytes;
    use http::{Request, Response, StatusCode};
    use iri_rs::IriRefBuf;
    use jstrict::Value as Json;
    use ngsi_ld::{
        http::{ContextDelivery, Endpoint, ResponseContext},
        model::{AttributeName, EntityId},
    };
    use std::str;
    use url::Url;

    /// The test endpoint.
    pub(crate) fn endpoint() -> Endpoint {
        Endpoint::builder()
            .base_url(Url::parse("http://broker.test/ngsi-ld/v1/").unwrap())
            .context(IriRefBuf::new("https://example.org/context.jsonld").unwrap())
            .build()
            .unwrap()
    }

    /// The test endpoint, delivering its context as `delivery`.
    pub(crate) fn endpoint_with(delivery: ContextDelivery) -> Endpoint {
        Endpoint::builder()
            .base_url(Url::parse("http://broker.test/ngsi-ld/v1/").unwrap())
            .context(IriRefBuf::new("https://example.org/context.jsonld").unwrap())
            .context_delivery(delivery)
            .build()
            .unwrap()
    }

    /// The context of a response to a request on the default tenant.
    pub(crate) fn context() -> ResponseContext {
        let request = Request::get("http://broker.test/ngsi-ld/v1/entities").body(Bytes::new()).unwrap();
        ResponseContext::new(&request)
    }

    /// The vehicle the requests address.
    pub(crate) fn vehicle_id() -> EntityId {
        "urn:ngsi-ld:Vehicle:A4567".parse().unwrap()
    }

    /// The attribute the requests address.
    pub(crate) fn speed() -> AttributeName {
        "speed".parse().unwrap()
    }

    /// Parses JSON text.
    pub(crate) fn json(text: &[u8]) -> Json {
        str::from_utf8(text).unwrap().parse().unwrap()
    }

    /// A response with a status and a body.
    pub(crate) fn response(status: u16, body: &str) -> Response<Bytes> {
        let mut response = Response::new(Bytes::copy_from_slice(body.as_bytes()));
        *response.status_mut() = StatusCode::from_u16(status).unwrap();
        response
    }
}

#[test]
fn update_attributes_sends_patch_to_the_attrs_resource_with_the_fragment() {
    let id = vehicle_id();
    let fragment = EntityFragment::builder().property("speed", 55).build().unwrap();
    let request = UpdateAttributes::builder().id(&id).fragment(&fragment).build().to_request(&endpoint()).unwrap();
    assert_eq!(request.method(), Method::PATCH);
    assert_eq!(request.uri().path(), "/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567/attrs");
    assert_eq!(
        Unordered(json(request.body())),
        Unordered(json(br#"{"speed": {"type": "Property", "value": 55}}"#))
    );
}

#[test]
fn append_attributes_sends_post_and_no_overwrite_only_when_asked() {
    let id = vehicle_id();
    let fragment = EntityFragment::builder().property("speed", 55).build().unwrap();
    let append = AppendAttributes::builder().id(&id).fragment(&fragment).build();
    let request = append.to_request(&endpoint()).unwrap();
    assert_eq!(request.method(), Method::POST);
    assert_eq!(request.uri().path(), "/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567/attrs");
    assert_eq!(request.uri().query(), None);
    let keep = AppendAttributes {
        overwrite: Overwrite::Keep,
        ..append
    };
    assert_eq!(keep.to_request(&endpoint()).unwrap().uri().query(), Some("options=noOverwrite"));
}

#[test]
fn partial_attribute_update_sends_patch_to_the_attribute_with_its_members() {
    let id = vehicle_id();
    let name = speed();
    let attribute = Change::Set(AttributeFragment::from(
        Attribute::Property(Property::builder(55).build().unwrap()).into_patch(),
    ));
    let request = PartialAttributeUpdate::builder()
        .id(&id)
        .name(&name)
        .attribute(&attribute)
        .form(Lossless::Concise)
        .build()
        .to_request(&endpoint())
        .unwrap();
    assert_eq!(request.method(), Method::PATCH);
    assert_eq!(request.uri().path(), "/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567/attrs/speed");
    // A concise attribute body is still an object, never a bare value, so it can carry an `@context`.
    assert_eq!(Unordered(json(request.body())), Unordered(json(br#"{"value": 55}"#)));
}

#[test]
fn partial_attribute_update_sends_a_fragment_that_sets_no_value() {
    let id = vehicle_id();
    let name = speed();
    let send = |members: AttributeMembers, form| {
        let attribute = Change::Set(AttributeFragment::from(members));
        PartialAttributeUpdate::builder()
            .id(&id)
            .name(&name)
            .attribute(&attribute)
            .form(form)
            .build()
            .to_request(&endpoint())
            .unwrap()
    };
    let observed = || {
        AttributeMembers::builder()
            .observed_at("2022-03-14T12:51:02Z".parse().unwrap())
            .property("accuracy", 3)
            .build()
            .unwrap()
    };
    let request = send(observed(), Lossless::Concise);
    assert_eq!(request.method(), Method::PATCH);
    assert_eq!(
        Unordered(json(request.body())),
        Unordered(json(br#"{"observedAt": "2022-03-14T12:51:02Z", "accuracy": 3}"#))
    );
    assert_eq!(
        Unordered(json(send(observed(), Lossless::Normalized).body())),
        Unordered(json(br#"{"observedAt": "2022-03-14T12:51:02Z", "accuracy": {"type": "Property", "value": 3}}"#))
    );
    for form in [Lossless::Normalized, Lossless::Concise] {
        assert_eq!(json(send(AttributeMembers::builder().build().unwrap(), form).body()), json(b"{}"));
    }
}

#[test]
fn replace_attribute_sends_put_to_the_attribute() {
    let id = vehicle_id();
    let name = speed();
    let attribute = Attribute::Property(Property::builder(55).build().unwrap());
    let request = ReplaceAttribute::builder()
        .id(&id)
        .name(&name)
        .attribute(&attribute)
        .build()
        .to_request(&endpoint())
        .unwrap();
    assert_eq!(request.method(), Method::PUT);
    assert_eq!(request.uri().path(), "/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567/attrs/speed");
    assert_eq!(Unordered(json(request.body())), Unordered(json(br#"{"type": "Property", "value": 55}"#)));
}

#[test]
fn replace_attribute_as_json_names_the_context_in_a_link_and_leaves_it_out_of_the_body() {
    let id = vehicle_id();
    let name = speed();
    let attribute = Attribute::Property(Property::builder(55).build().unwrap());
    let request = ReplaceAttribute::builder()
        .id(&id)
        .name(&name)
        .attribute(&attribute)
        .build()
        .to_request(&endpoint_with(ContextDelivery::JsonWithLink))
        .unwrap();
    assert_eq!(
        request.headers().get(header::CONTENT_TYPE).map(HeaderValue::as_bytes),
        Some(b"application/json".as_slice())
    );
    assert!(request.headers().contains_key(header::LINK));
    assert!(json(request.body()).as_object().unwrap().get("@context").next().is_none());
}

#[test]
fn replace_attribute_as_ld_json_carries_the_context_at_the_top_of_the_attribute_and_sends_no_link() {
    let id = vehicle_id();
    let name = speed();
    let attribute = Attribute::Property(Property::builder(55).build().unwrap());
    let request = ReplaceAttribute::builder()
        .id(&id)
        .name(&name)
        .attribute(&attribute)
        .build()
        .to_request(&endpoint_with(ContextDelivery::LdJsonBody))
        .unwrap();
    assert_eq!(
        request.headers().get(header::CONTENT_TYPE).map(HeaderValue::as_bytes),
        Some(b"application/ld+json".as_slice())
    );
    assert!(!request.headers().contains_key(header::LINK));
    assert_eq!(
        Unordered(json(request.body())),
        Unordered(json(br#"{"@context": "https://example.org/context.jsonld", "type": "Property", "value": 55}"#))
    );
}

#[test]
fn to_b21_delete_attribute_selects_the_default_instance_a_dataset_or_all() {
    let id = vehicle_id();
    let name = speed();
    let gps: DatasetId = "urn:ngsi-ld:dataset:gps".parse().unwrap();
    let delete = |instances| {
        DeleteAttribute::builder()
            .id(&id)
            .name(&name)
            .instances(instances)
            .build()
            .to_request(&endpoint())
            .unwrap()
    };
    let default = delete(InstanceSelection::Default);
    assert_eq!(default.method(), Method::DELETE);
    assert_eq!(default.uri().path(), "/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567/attrs/speed");
    assert_eq!(default.uri().query(), None);
    assert!(default.body().is_empty());
    assert_eq!(
        delete(InstanceSelection::Dataset(&gps)).uri().query(),
        Some("datasetId=urn%3Angsi-ld%3Adataset%3Agps")
    );
    // Clause 4.5.5: `@none` names the default instance wherever a datasetId is given.
    assert_eq!(
        delete(InstanceSelection::Dataset(&DatasetId::DefaultInstance)).uri().query(),
        Some("datasetId=%40none")
    );
    assert_eq!(delete(InstanceSelection::All).uri().query(), Some("deleteAll=true"));
    assert_eq!(delete(InstanceSelection::Selected(None)).uri().query(), Some("deleteAll=false"));
    assert_eq!(
        delete(InstanceSelection::Selected(Some(&gps))).uri().query(),
        Some("datasetId=urn%3Angsi-ld%3Adataset%3Agps&deleteAll=false")
    );
}

#[test]
fn purge_sends_delete_to_entities_with_its_selection() {
    let entity_type: SelectorType = "Vehicle".parse().unwrap();
    let drop: [AttributeName; 1] = [speed()];
    let purge = PurgeEntities::builder().entity_type(&entity_type).drop(&drop).build();
    let request = purge.to_request(&endpoint()).unwrap();
    assert_eq!(request.method(), Method::DELETE);
    assert_eq!(request.uri().path(), "/ngsi-ld/v1/entities");
    assert_eq!(request.uri().query(), Some("type=Vehicle&drop=speed"));
}

#[test]
fn a_purge_without_type_or_q_is_refused_unless_local() {
    let unbounded = PurgeEntities::builder().build();
    assert!(matches!(
        unbounded.to_request(&endpoint()),
        Err(RequestError::Argument(ArgumentError::UnboundedQuery))
    ));
    let local = PurgeEntities {
        locality: Locality::Local,
        entity_map: None,
        ..unbounded
    };
    assert_eq!(local.to_request(&endpoint()).unwrap().uri().query(), Some("local=true"));
}

#[test]
fn a_207_to_an_attribute_operation_is_a_partial_update_result() {
    let id = vehicle_id();
    let fragment = EntityFragment::builder().property("speed", 55).build().unwrap();
    let update = UpdateAttributes::builder().id(&id).fragment(&fragment).build();
    let outcome = update
        .decode(
            &context(),
            response(
                207,
                r#"{"updated": [], "notUpdated": [{"attributeName": "speed", "reason": "the attribute does not exist"}]}"#,
            ),
        )
        .unwrap();
    let UpdateOutcome::Partial(result) = outcome else {
        panic!("expected a partial result, got {outcome:?}")
    };
    assert_eq!(result.updated, []);
    assert_eq!(result.not_updated[0].attribute_name, speed());
}

#[test]
fn a_204_to_an_attribute_operation_is_done() {
    let id = vehicle_id();
    let name = speed();
    let delete = DeleteAttribute::builder().id(&id).name(&name).instances(InstanceSelection::All).build();
    assert_eq!(delete.decode(&context(), response(204, "")).unwrap(), UpdateOutcome::Done);
}

#[test]
fn a_207_to_purge_is_a_partial_batch_result() {
    let entity_type: SelectorType = "Vehicle".parse().unwrap();
    let purge = PurgeEntities::builder().entity_type(&entity_type).build();
    let outcome = purge
        .decode(&context(), response(207, r#"{"success": ["urn:ngsi-ld:Vehicle:A4567"], "errors": []}"#))
        .unwrap();
    let BatchOutcome::Partial(result) = outcome else {
        panic!("expected a partial result, got {outcome:?}")
    };
    assert_eq!(result.success.len(), 1);
}

#[test]
fn a_200_to_an_attribute_operation_is_an_undefined_status() {
    let id = vehicle_id();
    let name = speed();
    let delete = DeleteAttribute::builder().id(&id).name(&name).build();
    assert!(matches!(
        delete.decode(&context(), response(200, "")),
        Err(ResponseError::UnexpectedStatus { status: StatusCode::OK, .. })
    ));
}
