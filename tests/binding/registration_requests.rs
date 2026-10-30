//! The requests of the single context source registration operations (clauses 5.9,
//! 5.10.1, 6.8.3.1 and 6.9): Register, Update, Retrieve and Delete, their path and their tenant.

use crate::{
    fixtures::{endpoint, endpoint_with, header_value, link_of},
    links::context_link,
    registration_fixtures::{R, VEHICLE_SOURCE, body_text, id, postpone, vehicle_source, vehicles},
};
use headers::Header;
use http::{Method, header};
use ngsi_ld::{
    http::{
        ContextDelivery,
        headers::NgsiLdTenant,
        operation::{DeleteRegistration, Operation, QueryRegistrations, RegisterContextSource, RetrieveRegistration, SystemAttributes, UpdateRegistration},
    },
    model::{EntityTypeName, Input, Tenant, UrlDraft},
    registration::{CSourceRegistration, EntityInfo, RegistrationInfo},
};
use pretty_assertions::assert_eq;

#[test]
fn register_posts_the_registration_with_the_json_headers_and_no_accept() {
    let request = RegisterContextSource {
        registration: &vehicle_source(),
    }
    .to_request(&endpoint())
    .unwrap();
    assert_eq!(request.method(), Method::POST);
    assert_eq!(request.uri().path(), "/ngsi-ld/v1/csourceRegistrations");
    assert_eq!(request.uri().query(), None);
    assert_eq!(header_value(&request, &header::CONTENT_TYPE), Some("application/json"));
    assert_eq!(link_of(&request), Some(context_link()));
    assert_eq!(header_value(&request, &header::CONTENT_LENGTH), Some(request.body().len().to_string().as_str()));
    assert_eq!(header_value(&request, &header::ACCEPT), None);
    assert_eq!(body_text(&request), VEHICLE_SOURCE);
}

#[test]
fn register_sends_the_endpoint_a_registration_was_built_with_as_it_was_given() {
    let registration = CSourceRegistration::<Input>::builder(UrlDraft::verbatim("http://localhost:8080"))
        .information([RegistrationInfo::builder()
            .entities([EntityInfo::new("Vehicle".parse::<EntityTypeName>().unwrap())])
            .property_names(["speed"])
            .build()
            .unwrap()])
        .build()
        .unwrap();
    let request = RegisterContextSource { registration: &registration }.to_request(&endpoint()).unwrap();
    assert!(
        body_text(&request).ends_with(r#""endpoint":"http://localhost:8080"}"#),
        "{}",
        body_text(&request)
    );
}

#[test]
fn under_ld_json_body_the_registration_carries_the_context_last() {
    let request = RegisterContextSource {
        registration: &vehicle_source(),
    }
    .to_request(&endpoint_with(ContextDelivery::LdJsonBody))
    .unwrap();
    assert_eq!(header_value(&request, &header::CONTENT_TYPE), Some("application/ld+json"));
    assert_eq!(link_of(&request), None);
    let expected = format!(
        r#"{},"@context":"https://example.org/context.jsonld"}}"#,
        VEHICLE_SOURCE.strip_suffix('}').unwrap()
    );
    assert_eq!(body_text(&request), expected);
}

#[test]
fn update_patches_the_fragment_without_an_id() {
    let target = id(R);
    let fragment = postpone();
    let update = UpdateRegistration {
        id: &target,
        fragment: &fragment,
    };
    let request = update.to_request(&endpoint()).unwrap();
    assert_eq!(request.method(), Method::PATCH);
    assert_eq!(request.uri().path(), format!("/ngsi-ld/v1/csourceRegistrations/{R}"));
    assert_eq!(request.uri().query(), None);
    assert_eq!(header_value(&request, &header::CONTENT_TYPE), Some("application/json"));
    assert_eq!(link_of(&request), Some(context_link()));
    assert_eq!(header_value(&request, &header::ACCEPT), None);
    assert_eq!(
        body_text(&request),
        r#"{"type":"ContextSourceRegistration","expiresAt":"2030-01-01T00:00:00Z"}"#
    );
    let request = update.to_request(&endpoint_with(ContextDelivery::LdJsonBody)).unwrap();
    assert_eq!(
        body_text(&request),
        r#"{"type":"ContextSourceRegistration","expiresAt":"2030-01-01T00:00:00Z","@context":"https://example.org/context.jsonld"}"#
    );
}

#[test]
fn retrieve_gets_the_registration_and_asks_for_json() {
    let target = id(R);
    let request = RetrieveRegistration::builder().id(&target).build().to_request(&endpoint()).unwrap();
    assert_eq!(request.method(), Method::GET);
    assert_eq!(request.uri().path(), format!("/ngsi-ld/v1/csourceRegistrations/{R}"));
    assert_eq!(request.uri().query(), None);
    assert_eq!(header_value(&request, &header::ACCEPT), Some("application/json"));
    assert_eq!(link_of(&request), Some(context_link()));
    assert_eq!(header_value(&request, &header::CONTENT_TYPE), None);
    assert!(request.body().is_empty());
    let request = RetrieveRegistration::builder()
        .id(&target)
        .system_attributes(SystemAttributes::Include)
        .build()
        .to_request(&endpoint())
        .unwrap();
    assert_eq!(request.uri().query(), Some("options=sysAttrs"));
}

#[test]
fn delete_sends_no_body_and_no_accept() {
    let target = id(R);
    let request = DeleteRegistration { id: &target }.to_request(&endpoint()).unwrap();
    assert_eq!(request.method(), Method::DELETE);
    assert_eq!(request.uri().path(), format!("/ngsi-ld/v1/csourceRegistrations/{R}"));
    assert_eq!(request.uri().query(), None);
    assert_eq!(link_of(&request), Some(context_link()));
    assert_eq!(header_value(&request, &header::ACCEPT), None);
    assert!(request.body().is_empty());
}

#[test]
fn a_registration_id_is_one_encoded_path_segment() {
    let target = id("https://example.org/csr/1#a");
    let fragment = postpone();
    let expected = "/ngsi-ld/v1/csourceRegistrations/https:%2F%2Fexample.org%2Fcsr%2F1%23a";
    let requests = [
        RetrieveRegistration::builder().id(&target).build().to_request(&endpoint()).unwrap(),
        UpdateRegistration {
            id: &target,
            fragment: &fragment,
        }
        .to_request(&endpoint())
        .unwrap(),
        DeleteRegistration { id: &target }.to_request(&endpoint()).unwrap(),
    ];
    for request in requests {
        assert_eq!(request.uri().path(), expected);
    }
}

#[test]
fn every_registration_operation_names_the_tenant() {
    let tenant: Tenant = "t1".parse().unwrap();
    let target = id(R);
    let registration = vehicle_source();
    let fragment = postpone();
    let entity_type = vehicles();
    let requests = [
        RegisterContextSource { registration: &registration }.to_request(&endpoint().with_tenant(tenant.clone())),
        UpdateRegistration {
            id: &target,
            fragment: &fragment,
        }
        .to_request(&endpoint().with_tenant(tenant.clone())),
        RetrieveRegistration::builder()
            .id(&target)
            .build()
            .to_request(&endpoint().with_tenant(tenant.clone())),
        QueryRegistrations {
            entity_type: Some(&entity_type),
            ..QueryRegistrations::default()
        }
        .to_request(&endpoint().with_tenant(tenant.clone())),
        DeleteRegistration { id: &target }.to_request(&endpoint().with_tenant(tenant)),
    ];
    for request in requests {
        assert_eq!(header_value(&request.unwrap(), NgsiLdTenant::name()), Some("t1"));
    }
}
