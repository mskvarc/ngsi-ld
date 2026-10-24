//! Context Source Properties (clause 4.9), the members a registration carries beyond Tables 5.2.9-1
//! and 5.2.9-2.

use crate::{
    model::{Input, MemberChange, MemberError, NameError, Output, Value, ValueError},
    registration::{
        CSourceRegistration,
        ContextSourcePropertyName,
        ContextSourcePropertyNameError,
        Problem,
        RegistrationDirection,
        RegistrationFragment,
        tests::{ENDPOINT, problems, read_error, registration_text, round_trip, vehicle_speed},
    },
    subscription::ReadErrorKind,
};
use pretty_assertions::assert_eq;

/// A property name.
fn name(text: &str) -> ContextSourcePropertyName {
    text.parse().unwrap()
}

/// The names of the properties of a registration, in order.
fn names<D: RegistrationDirection>(registration: &CSourceRegistration<D>) -> Vec<&str> {
    registration.properties().keys().map(AsRef::as_ref).collect()
}

#[test]
fn properties_are_read_in_order_with_any_value_and_round_trip() {
    let text =
        registration_text(r#""csourceProperty1":"aValue","csourceProperty2":42,"jsonldContext":{"speed":"https://example.org/speed"},"ex:flags":[true,false]"#);
    let registration = round_trip::<Input>(&text);
    assert_eq!(names(&registration), ["csourceProperty1", "csourceProperty2", "jsonldContext", "ex:flags"]);
    assert_eq!(registration.properties().get(&name("csourceProperty1")), Some(&Value::from("aValue")));
    assert_eq!(registration.properties().get(&name("csourceProperty2")), Some(&Value::from(42)));
    assert_eq!(
        registration.properties().get(&name("jsonldContext")).map(ToString::to_string).as_deref(),
        Some(r#"{"speed":"https://example.org/speed"}"#)
    );
}

#[test]
fn a_retrieved_registration_writes_its_properties_before_the_members_only_a_broker_writes() {
    let text = registration_text(r#""csourceProperty1":"aValue","status":"ok","createdAt":"2024-04-01T10:00:00Z""#).replacen(
        '{',
        r#"{"id":"urn:ngsi-ld:ContextSourceRegistration:csr1","#,
        1,
    );
    let registration = round_trip::<Output>(&text);
    assert_eq!(names(&registration), ["csourceProperty1"]);
    let reordered =
        registration_text(r#""status":"ok","csourceProperty1":"aValue""#).replacen('{', r#"{"id":"urn:ngsi-ld:ContextSourceRegistration:csr1","#, 1);
    let read = CSourceRegistration::<Output>::parse(&reordered).unwrap();
    assert_eq!(
        read.render().to_string(),
        registration_text(r#""csourceProperty1":"aValue","status":"ok""#).replacen('{', r#"{"id":"urn:ngsi-ld:ContextSourceRegistration:csr1","#, 1)
    );
    assert_eq!(read.into_input().properties().len(), 1);
}

#[test]
fn a_member_only_a_broker_writes_or_an_invalid_name_is_no_property_of_a_request() {
    for member in [r#""status":"ok""#, r#""createdAt":"2024-04-01T10:00:00Z""#, r#""1x":1"#, r#""a-b":1"#] {
        let error = read_error::<Input>(&registration_text(member));
        assert!(matches!(error.kind, ReadErrorKind::Member(_)), "{error}");
        assert_eq!(error.path.to_string(), "", "{error}");
    }
    let null = read_error::<Input>(&registration_text(r#""csourceProperty1":null"#));
    assert!(
        matches!(null.kind, ReadErrorKind::Member(MemberError::Value { source: ValueError::Null })),
        "{null}"
    );
    assert_eq!(null.path.to_string(), "csourceProperty1");
}

#[test]
fn a_property_name_is_an_attribute_name_no_registration_member_has() {
    for accepted in ["csourceProperty1", "jsonldContext", "ex:brand", "https://example.org/brand"] {
        assert_eq!(name(accepted).to_string(), accepted);
    }
    for member in [
        "information",
        "endpoint",
        "mode",
        "registrationName",
        "refreshRate",
        "timesSent",
        "lastFailure",
        "status",
    ] {
        assert_eq!(
            member.parse::<ContextSourcePropertyName>(),
            Err(ContextSourcePropertyNameError::ReservedByRegistration),
            "{member}"
        );
    }
    for reserved in ["id", "type", "@context", "createdAt", "scope", "datasetId"] {
        assert!(
            matches!(
                reserved.parse::<ContextSourcePropertyName>(),
                Err(ContextSourcePropertyNameError::Name {
                    source: NameError::Reserved { .. }
                })
            ),
            "{reserved}"
        );
    }
    assert!(matches!(
        "1x".parse::<ContextSourcePropertyName>(),
        Err(ContextSourcePropertyNameError::Name {
            source: NameError::InvalidStart { .. }
        })
    ));
}

#[test]
fn the_builder_adds_properties_in_order_and_reports_invalid_and_repeated_names() {
    let registration = CSourceRegistration::builder(ENDPOINT)
        .information([vehicle_speed()])
        .refresh_rate("PT1M".parse().unwrap())
        .property("csourceProperty1", "aValue")
        .property(name("csourceProperty2"), 7)
        .build()
        .unwrap();
    assert_eq!(
        registration.render().to_string(),
        registration_text(r#""refreshRate":"PT1M","csourceProperty1":"aValue","csourceProperty2":7"#)
    );
    assert_eq!(
        problems(
            CSourceRegistration::builder(ENDPOINT)
                .information([vehicle_speed()])
                .property("csourceProperty1", 1)
                .property("endpoint", 2)
                .property("csourceProperty1", 3)
                .build()
        ),
        [
            Problem::ContextSourceProperty {
                index: 1,
                source: ContextSourcePropertyNameError::ReservedByRegistration
            },
            Problem::DuplicateContextSourceProperty {
                name: name("csourceProperty1")
            },
        ]
    );
}

#[test]
fn a_fragment_sets_and_deletes_properties_after_its_members() {
    let fragment = RegistrationFragment::builder()
        .description("Vehicles")
        .property("csourceProperty1", "anotherValue")
        .delete_property("csourceProperty2")
        .build()
        .unwrap();
    assert_eq!(
        fragment.render().to_string(),
        r#"{"type":"ContextSourceRegistration","description":"Vehicles","csourceProperty1":"anotherValue","csourceProperty2":"urn:ngsi-ld:null"}"#
    );
    assert_eq!(fragment.properties().get(&name("csourceProperty2")), Some(&MemberChange::Delete));
    assert_eq!(
        problems(
            RegistrationFragment::builder()
                .property("csourceProperty1", 1)
                .delete_property("csourceProperty1")
                .property("mode", 2)
                .build()
        ),
        [
            Problem::DuplicateContextSourceProperty {
                name: name("csourceProperty1")
            },
            Problem::ContextSourceProperty {
                index: 2,
                source: ContextSourcePropertyNameError::ReservedByRegistration
            },
        ]
    );
}
