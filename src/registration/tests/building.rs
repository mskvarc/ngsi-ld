//! Building registrations, their information and management information, and the alias.

use crate::{
    model::{
        Context,
        ContextSourceAlias,
        ContextSourceAliasError,
        DatasetId,
        DateTime,
        EntityId,
        Input,
        MemberError,
        MemberProblem,
        OneOrMany,
        Scope,
        geometry::{Geometry, LinearRing, Polygon, Position, Shape},
    },
    query::Locality,
    registration::{
        CSourceRegistration,
        ContextSourceInfo,
        ContextSourceInfoError,
        ContextSourceInfoKey,
        ContextSourceInfoValue,
        ContextSourceMediaType,
        DistributedOperation,
        EntityInfo,
        ManagementMember,
        OperationGroup,
        Problem,
        RegistrationField,
        RegistrationId,
        RegistrationInfo,
        RegistrationInfoMember,
        RegistrationManagementInfo,
        RegistrationMember,
        RegistrationMode,
        SupportedOperation,
        TimeInterval,
        tests::{ENDPOINT, entity_type, problems, registration_text, round_trip, vehicle_speed, vehicles},
    },
    subscription::ReadErrorKind,
};
use iri_rs::{IriRefBuf, iri};
use pretty_assertions::assert_eq;

/// A `DateTime` from its lexical form.
fn time(text: &str) -> DateTime {
    text.parse().unwrap()
}

/// A position.
fn position(longitude: f64, latitude: f64) -> Position {
    Position::new(longitude, latitude).unwrap()
}

/// The polygon of Annex C.3.
fn polygon() -> Geometry {
    let ring = LinearRing::new(vec![
        position(100.0, 0.0),
        position(101.0, 0.0),
        position(101.0, 1.0),
        position(100.0, 1.0),
        position(100.0, 0.0),
    ])
    .unwrap();
    Geometry::from(Shape::Polygon(Polygon::new(vec![ring]).unwrap()))
}

/// A registration with every member set.
fn every_member() -> CSourceRegistration<Input> {
    let parking = RegistrationInfo::builder()
        .entities([
            EntityInfo::new(entity_type("OffStreetParking")).with_id_pattern(".*downtown$".parse().unwrap()),
            EntityInfo::new(OneOrMany::try_from(vec![entity_type("OffStreetParking"), entity_type("Parking")]).unwrap()),
        ])
        .property_names(["availableSpotNumber", "totalSpotNumber"])
        .relationship_names(["isNextToBuilding"])
        .build()
        .unwrap();
    let vehicle = RegistrationInfo::builder()
        .entities([vehicles().with_id(EntityId::from(iri!("urn:ngsi-ld:Vehicle:A456")))])
        .property_names(["brandName", "speed"])
        .build()
        .unwrap();
    CSourceRegistration::builder("http://my.csource.org:1026")
        .information([vehicle, parking])
        .id(iri!("urn:ngsi-ld:ContextSourceRegistration:csr1a3456"))
        .registration_name("Parkings")
        .description("Vehicles and parkings of the city")
        .context_source_info(("Authorization", "urn:ngsi-ld:request"))
        .context_source_info(("jsonldContext", "https://example.org/context.jsonld"))
        .context_source_alias("csource-1".parse::<ContextSourceAlias>().unwrap())
        .mode(RegistrationMode::Redirect)
        .operations([
            SupportedOperation::from(OperationGroup::RetrieveOps),
            SupportedOperation::from(DistributedOperation::QueryBatch),
        ])
        .management(
            RegistrationManagementInfo::builder()
                .locality(Locality::Local)
                .cache_duration("PT30S".parse().unwrap())
                .timeout(2000)
                .cooldown(500)
                .build()
                .unwrap(),
        )
        .tenant("t1".parse().unwrap())
        .scope(OneOrMany::try_from(vec!["/Madrid".parse::<Scope>().unwrap(), "/Madrid/Gardens".parse().unwrap()]).unwrap())
        .dataset_ids(["@none".parse::<DatasetId>().unwrap(), "urn:ngsi-ld:dataset:gps".parse().unwrap()])
        .location(polygon())
        .observation_space(Geometry::from(Shape::Point(position(100.5, 0.5))))
        .operation_space(Geometry::from(Shape::Point(position(100.5, 0.6))))
        .observation_interval(TimeInterval::between(time("2017-11-29T14:53:15Z"), time("2017-12-29T14:53:15Z")))
        .management_interval(TimeInterval::starting_at(time("2017-11-29T14:53:15Z")))
        .expires_at(time("2030-01-01T00:00:00Z"))
        .refresh_rate("PT1M".parse().unwrap())
        .context(Context::from(IriRefBuf::new("https://example.org/context.jsonld").unwrap()))
        .build()
        .unwrap()
}

/// The canonical text of `every_member`.
pub(super) const EVERY_MEMBER: &str = concat!(
    r#"{"id":"urn:ngsi-ld:ContextSourceRegistration:csr1a3456","type":"ContextSourceRegistration","registrationName":"Parkings","#,
    r#""description":"Vehicles and parkings of the city","information":[{"entities":[{"type":"Vehicle","id":"urn:ngsi-ld:Vehicle:A456"}],"#,
    r#""propertyNames":["brandName","speed"]},{"entities":[{"type":"OffStreetParking","idPattern":".*downtown$"},"#,
    r#"{"type":["OffStreetParking","Parking"]}],"propertyNames":["availableSpotNumber","totalSpotNumber"],"#,
    r#""relationshipNames":["isNextToBuilding"]}],"endpoint":"http://my.csource.org:1026/","contextSourceInfo":["#,
    r#"{"key":"Authorization","value":"urn:ngsi-ld:request"},{"key":"jsonldContext","value":"https://example.org/context.jsonld"}],"#,
    r#""contextSourceAlias":"csource-1","mode":"redirect","operations":["retrieveOps","queryBatch"],"#,
    r#""management":{"localOnly":true,"cacheDuration":"PT30S","timeout":2000,"cooldown":500},"tenant":"t1","#,
    r#""scope":["/Madrid","/Madrid/Gardens"],"datasetId":["@none","urn:ngsi-ld:dataset:gps"],"#,
    r#""location":{"type":"Polygon","coordinates":[[[100,0],[101,0],[101,1],[100,1],[100,0]]]},"#,
    r#""observationSpace":{"type":"Point","coordinates":[100.5,0.5]},"operationSpace":{"type":"Point","coordinates":[100.5,0.6]},"#,
    r#""observationInterval":{"startAt":"2017-11-29T14:53:15Z","endAt":"2017-12-29T14:53:15Z"},"#,
    r#""managementInterval":{"startAt":"2017-11-29T14:53:15Z"},"expiresAt":"2030-01-01T00:00:00Z","refreshRate":"PT1M","#,
    r#""@context":"https://example.org/context.jsonld"}"#
);

#[test]
fn the_least_registration_holds_its_information_and_endpoint() {
    let registration = CSourceRegistration::builder("http://my.csource.org:1026")
        .information([vehicle_speed()])
        .build()
        .unwrap();
    assert_eq!(registration.render().to_string(), registration_text(""));
    assert_eq!(registration.id(), None);
    assert_eq!(registration.endpoint().as_str(), ENDPOINT);
    assert_eq!(registration.mode(), None);
    assert_eq!(registration.operations(), None);
}

#[test]
fn every_member_is_written_in_canonical_order_and_read_back_equal() {
    let registration = every_member();
    assert_eq!(registration.render().to_string(), EVERY_MEMBER);
    assert_eq!(round_trip::<Input>(EVERY_MEMBER), registration);
    assert_eq!(
        registration.id(),
        Some(&RegistrationId::from(iri!("urn:ngsi-ld:ContextSourceRegistration:csr1a3456")))
    );
    assert_eq!(registration.information()[1].entities()[1].entity_type().as_slice().len(), 2);
}

#[test]
fn an_invalid_endpoint_and_an_empty_information_are_reported_together() {
    let result = CSourceRegistration::builder("not a url").information([]).build();
    let found = problems(result);
    assert!(
        matches!(
            found.as_slice(),
            [
                Problem::Member(MemberProblem::Empty {
                    member: RegistrationField::Information
                }),
                Problem::Member(MemberProblem::InvalidUrl {
                    member: RegistrationField::Endpoint,
                    ..
                })
            ]
        ),
        "{found:?}"
    );
}

#[test]
fn a_registration_information_needs_a_member_and_no_member_is_empty() {
    assert_eq!(problems(RegistrationInfo::builder().build()), [Problem::EmptyRegistrationInfo]);
    let none: [EntityInfo; 0] = [];
    assert_eq!(
        problems(RegistrationInfo::builder().entities(none).build()),
        [Problem::Member(MemberProblem::Empty {
            member: RegistrationField::InformationElement(RegistrationInfoMember::Entities)
        })]
    );
    let invalid = problems(RegistrationInfo::builder().property_names(["1x"]).build());
    assert!(
        matches!(
            invalid.as_slice(),
            [Problem::Member(MemberProblem::InvalidAttributeName {
                member: RegistrationField::InformationElement(RegistrationInfoMember::PropertyNames),
                index: 0,
                ..
            })]
        ),
        "{invalid:?}"
    );
    let no_name: [&str; 0] = [];
    assert_eq!(
        problems(RegistrationInfo::builder().relationship_names(no_name).build()),
        [Problem::Member(MemberProblem::Empty {
            member: RegistrationField::InformationElement(RegistrationInfoMember::RelationshipNames)
        })]
    );
}

#[test]
fn name_and_description_cannot_be_empty() {
    let result = CSourceRegistration::builder(ENDPOINT)
        .information([vehicle_speed()])
        .registration_name("")
        .description("")
        .build();
    assert_eq!(
        problems(result),
        [
            Problem::Member(MemberProblem::Empty {
                member: RegistrationField::Optional(RegistrationMember::RegistrationName)
            }),
            Problem::Member(MemberProblem::Empty {
                member: RegistrationField::Optional(RegistrationMember::Description)
            })
        ]
    );
}

#[test]
fn alias_is_an_http_token() {
    let text = r#"{"type":"ContextSourceRegistration","information":[{"entities":[{"type":"Vehicle"}]}],"endpoint":"http://my.csource.org:1026/","contextSourceAlias":"csource 1"}"#;
    let error = CSourceRegistration::<Input>::parse(text).unwrap_err();
    assert_eq!(error.path.to_string(), "contextSourceAlias");
    assert!(matches!(
        error.kind,
        ReadErrorKind::Member(MemberError::ContextSourceAlias {
            source: ContextSourceAliasError::InvalidCharacter
        })
    ));
    let read = CSourceRegistration::<Input>::parse(&text.replace("csource 1", "csource-1")).unwrap();
    assert_eq!(read.context_source_alias().map(AsRef::as_ref), Some("csource-1"));
}

#[test]
fn an_auxiliary_registration_supports_only_retrieval_operations() {
    let auxiliary = || {
        CSourceRegistration::builder(ENDPOINT)
            .information([vehicle_speed()])
            .mode(RegistrationMode::Auxiliary)
    };
    assert_eq!(problems(auxiliary().build()), [Problem::AuxiliaryOperations]);
    assert!(
        auxiliary()
            .operations([
                SupportedOperation::from(OperationGroup::RetrieveOps),
                SupportedOperation::from(DistributedOperation::QueryEntity)
            ])
            .build()
            .is_ok()
    );
    assert_eq!(
        problems(auxiliary().operations([OperationGroup::FederationOps]).build()),
        [Problem::AuxiliaryOperations]
    );
    let none: [OperationGroup; 0] = [];
    assert_eq!(problems(auxiliary().operations(none).build()), [Problem::AuxiliaryOperations]);
}

#[test]
fn an_exclusive_registration_names_an_entity_id_and_attributes() {
    let exclusive = |information| {
        CSourceRegistration::builder(ENDPOINT)
            .information([information])
            .mode(RegistrationMode::Exclusive)
            .build()
    };
    assert_eq!(problems(exclusive(vehicle_speed())), [Problem::ExclusiveWithoutEntityIdAndAttributes]);
    let one_vehicle = || vehicles().with_id(EntityId::from(iri!("urn:ngsi-ld:Vehicle:A456")));
    let named = RegistrationInfo::builder().entities([one_vehicle()]).property_names(["speed"]).build().unwrap();
    assert!(exclusive(named).is_ok());
    let without_attributes = RegistrationInfo::builder().entities([one_vehicle()]).build().unwrap();
    assert_eq!(problems(exclusive(without_attributes)), [Problem::ExclusiveWithoutEntityIdAndAttributes]);
}

#[test]
fn the_context_source_information_keys_of_clause_4_3_6_6_take_their_values() {
    let with = |key: &str, value: &str| {
        CSourceRegistration::builder(ENDPOINT)
            .information([vehicle_speed()])
            .context_source_info((key, value))
            .build()
    };
    assert_eq!(
        problems(with("accept", "text/plain")),
        [Problem::ContextSourceInfo {
            index: 0,
            source: ContextSourceInfoError::MediaType {
                key: ContextSourceInfoKey::Accept
            }
        }]
    );
    let read = |key: &str, value: &str| ContextSourceInfo::parse(key, value);
    assert_eq!(
        read("contentType", "application/ld+json"),
        Ok(ContextSourceInfo::ContentType(ContextSourceInfoValue::Given(ContextSourceMediaType::LdJson)))
    );
    assert!(matches!(read("jsonldContext", "not a url"), Err(ContextSourceInfoError::Url { .. })));
    assert_eq!(
        read("jsonldContext", "urn:ngsi-ld:request"),
        Ok(ContextSourceInfo::JsonldContext(ContextSourceInfoValue::FromRequest))
    );
    assert!(matches!(
        read("ngsildConformance", "1.8"),
        Ok(ContextSourceInfo::NgsildConformance(ContextSourceInfoValue::Given(_)))
    ));
    assert!(matches!(read("ngsildConformance", "one"), Err(ContextSourceInfoError::Conformance { .. })));
    let Ok(ContextSourceInfo::Other(header)) = read("Authorization", "Bearer abc") else {
        panic!("an unknown key is another pair");
    };
    assert_eq!(
        (header.key(), header.value()),
        ("Authorization", &ContextSourceInfoValue::Given("Bearer abc".to_owned()))
    );
    let Ok(ContextSourceInfo::Other(forwarded)) = read("Authorization", "urn:ngsi-ld:request") else {
        panic!("an unknown key is another pair");
    };
    assert_eq!(forwarded.value(), &ContextSourceInfoValue::FromRequest);
}

#[test]
fn every_typed_context_source_information_pair_is_written_and_read_back() {
    let pairs = [
        ("accept", "application/json"),
        ("contentType", "application/ld+json"),
        ("jsonldContext", "https://example.org/context.jsonld"),
        ("jsonldContext", "urn:ngsi-ld:request"),
        ("ngsildConformance", "1.8"),
        ("Authorization", "Bearer abc"),
    ];
    let registration = pairs
        .iter()
        .fold(CSourceRegistration::builder(ENDPOINT).information([vehicle_speed()]), |builder, &pair| {
            builder.context_source_info(pair)
        })
        .build()
        .unwrap();
    let text = registration.render().to_string();
    let read = CSourceRegistration::<Input>::parse(&text).unwrap();
    assert_eq!(read.context_source_info(), registration.context_source_info());
    assert_eq!(read.context_source_info().map(<[_]>::len), Some(pairs.len()));
}

#[test]
fn management_timeout_and_cooldown_are_greater_than_zero() {
    let found = problems(RegistrationManagementInfo::builder().timeout(0).cooldown(-1).build());
    assert!(
        matches!(
            found.as_slice(),
            [
                Problem::Member(MemberProblem::NotPositive {
                    member: RegistrationField::Management(ManagementMember::Timeout),
                    ..
                }),
                Problem::Member(MemberProblem::NotPositive {
                    member: RegistrationField::Management(ManagementMember::Cooldown),
                    ..
                })
            ]
        ),
        "{found:?}"
    );
}

#[test]
fn an_entity_type_given_as_text_names_the_entities_and_is_checked_when_built() {
    let information = RegistrationInfo::builder().entities(["Vehicle"]).build().unwrap();
    assert_eq!(information.entities()[0].entity_type(), &OneOrMany::one(entity_type("Vehicle")));
    let invalid = problems(RegistrationInfo::builder().entities(["Vehicle", "bad name"]).build());
    assert!(
        matches!(
            invalid.as_slice(),
            [Problem::Member(MemberProblem::InvalidEntityTypeName {
                member: RegistrationField::InformationElement(RegistrationInfoMember::Entities),
                index: 1,
                ..
            })]
        ),
        "{invalid:?}"
    );
}
