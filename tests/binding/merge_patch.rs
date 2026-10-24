//! PATCH bodies sent as `application/merge-patch+json`, which clause 6.3.4 admits for PATCH alone and makes a broker
//! read as `application/json`.

use crate::{
    fixtures::{endpoint_with, header_value, json, link_of, vehicle, vehicle_id},
    links::context_link,
};
use http::{Method, header};
use jstrict::Unordered;
use ngsi_ld::{
    http::{
        ContextDelivery,
        operation::{CreateEntity, DeleteEntity, MergeEntity, Operation, ReplaceEntity},
    },
    model::{EntityFragment, Lossless},
};
use pretty_assertions::assert_eq;

#[test]
fn a_patch_body_is_sent_as_a_merge_patch_with_the_context_in_a_link_header() {
    let id = vehicle_id();
    let fragment = EntityFragment::builder().property("speed", 0).delete("isParked").build().unwrap();
    let request = MergeEntity::builder()
        .id(&id)
        .fragment(&fragment)
        .form(Lossless::Concise)
        .build()
        .to_request(&endpoint_with(ContextDelivery::MergePatchWithLink))
        .unwrap();
    assert_eq!(request.method(), Method::PATCH);
    assert_eq!(header_value(&request, &header::CONTENT_TYPE), Some("application/merge-patch+json"));
    assert_eq!(link_of(&request), Some(context_link()));
    assert_eq!(
        Unordered(json(request.body())),
        Unordered(json(br#"{"speed": 0, "isParked": "urn:ngsi-ld:null"}"#))
    );
}

#[test]
fn a_post_or_put_body_of_a_merge_patch_endpoint_is_sent_as_json() {
    let endpoint = endpoint_with(ContextDelivery::MergePatchWithLink);
    let entity = vehicle();
    let create = CreateEntity::builder().entity(&entity).build().to_request(&endpoint).unwrap();
    assert_eq!(create.method(), Method::POST);
    assert_eq!(header_value(&create, &header::CONTENT_TYPE), Some("application/json"));
    assert_eq!(link_of(&create), Some(context_link()));
    let replace = ReplaceEntity::builder().entity(&entity).build().to_request(&endpoint).unwrap();
    assert_eq!(replace.method(), Method::PUT);
    assert_eq!(header_value(&replace, &header::CONTENT_TYPE), Some("application/json"));
    assert!(!json(replace.body()).as_object().unwrap().contains_key("@context"));
}

#[test]
fn a_request_without_a_body_of_a_merge_patch_endpoint_carries_no_content_type() {
    let id = vehicle_id();
    let request = DeleteEntity::builder()
        .id(&id)
        .build()
        .to_request(&endpoint_with(ContextDelivery::MergePatchWithLink))
        .unwrap();
    assert_eq!(header_value(&request, &header::CONTENT_TYPE), None);
    assert_eq!(link_of(&request), Some(context_link()));
}
