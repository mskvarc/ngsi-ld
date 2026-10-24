//! The entity type and attribute discovery operations (clauses 5.7.5 to 5.7.10 and 6.25 to 6.28).

use crate::{
    fixtures::{context, endpoint, header_value, link_of, response},
    links::context_link,
};
use bytes::Bytes;
use headers::Header;
use http::{Method, Request, header};
use ngsi_ld::{
    http::{
        BodyError,
        ResponseError,
        headers::NgsiLdTenant,
        operation::{
            Operation,
            RetrieveAttributeDetails,
            RetrieveAttributeInfo,
            RetrieveAttributes,
            RetrieveEntityTypeDetails,
            RetrieveEntityTypeInfo,
            RetrieveEntityTypes,
        },
    },
    model::{
        AttributeKind,
        AttributeName,
        DocumentError,
        DocumentErrorKind,
        EntityTypeName,
        ErrorType,
        MemberError,
        NameError,
        Tenant,
        discovery::DiscoveryType,
    },
    query::Locality,
};
use pretty_assertions::assert_eq;
use std::str::FromStr;

/// An `EntityTypeList`, one type a term and one a Fully Qualified Name.
const TYPE_LIST: &str = r#"{"id":"urn:ngsi-ld:EntityTypeList:1","type":"EntityTypeList","typeList":["Vehicle","https://example.org/OffStreetParking"]}"#;

/// An `EntityType` with its attribute names.
const TYPE_DETAILS: &str = r#"{"id":"https://example.org/Vehicle","type":"EntityType","attributeNames":["speed","isParked"],"typeName":"Vehicle"}"#;

/// An `EntityTypeInfo` with the details of one attribute.
const TYPE_INFO: &str = concat!(
    r#"{"id":"https://example.org/Vehicle","type":"EntityTypeInfo","attributeDetails":["#,
    r#"{"id":"https://example.org/speed","type":"Attribute","attributeName":"speed","attributeTypes":["Property"]}],"#,
    r#""entityCount":2,"typeName":"Vehicle"}"#
);

/// An `AttributeList`, one attribute a term and one a Fully Qualified Name.
const ATTRIBUTE_LIST: &str = r#"{"id":"urn:ngsi-ld:AttributeList:1","type":"AttributeList","attributeList":["speed","https://example.org/brandName"]}"#;

/// An `Attribute` with the entity types that use it.
const ATTRIBUTE_DETAILS: &str = r#"{"id":"https://example.org/speed","type":"Attribute","attributeName":"speed","typeNames":["Vehicle"]}"#;

/// An `Attribute` with its count, its attribute types and its entity types.
const ATTRIBUTE_INFO: &str = concat!(
    r#"{"id":"https://example.org/speed","type":"Attribute","attributeName":"speed","attributeCount":3,"#,
    r#""attributeTypes":["Property","GeoProperty"],"typeNames":["Vehicle","Bus"]}"#
);

/// A 404 `ResourceNotFound` problem.
const NOT_FOUND: &str = r#"{"type":"https://uri.etsi.org/ngsi-ld/errors/ResourceNotFound","title":"unknown"}"#;

/// Parses a name.
fn name<T: FromStr<Err = NameError>>(text: &str) -> T {
    text.parse().unwrap()
}

/// The request an operation builds against the binding endpoint, on the default tenant.
fn built(operation: &impl Operation) -> Request<Bytes> {
    operation.to_request(&endpoint()).unwrap()
}

/// Checks a discovery GET: the method, the path, the query string, `Accept`, the `Link` header and no body.
fn assert_discovery_get(request: &Request<Bytes>, path: &str, query: Option<&str>) {
    assert_eq!(request.method(), Method::GET);
    assert_eq!(request.uri().path(), path);
    assert_eq!(request.uri().query(), query);
    assert_eq!(header_value(request, &header::ACCEPT), Some("application/json"));
    assert_eq!(link_of(request), Some(context_link()));
    assert_eq!(header_value(request, &header::CONTENT_TYPE), None);
    assert!(request.body().is_empty());
}

/// The document error inside a body error.
fn document_error(error: ResponseError) -> DocumentError {
    let ResponseError::Body { source, .. } = error else {
        panic!("expected a body error, got {error:?}")
    };
    let BodyError::Document(error) = *source else {
        panic!("expected a document error, got {source:?}")
    };
    error
}

/// Whether `error` is a problem of type `expected`.
fn is_problem(error: &ResponseError, expected: &ErrorType) -> bool {
    matches!(error, ResponseError::Problem { details, .. } if details.error_type.as_ref() == Some(expected))
}

#[test]
fn retrieve_available_entity_types_is_a_get_of_types() {
    assert_discovery_get(&built(&RetrieveEntityTypes::default()), "/ngsi-ld/v1/types", None);
}

#[test]
fn the_detailed_request_asks_for_details_and_every_request_takes_local() {
    assert_discovery_get(&built(&RetrieveEntityTypeDetails::default()), "/ngsi-ld/v1/types", Some("details=true"));
    let local = RetrieveEntityTypeDetails::builder().locality(Locality::Local).build();
    assert_discovery_get(&built(&local), "/ngsi-ld/v1/types", Some("details=true&local=true"));
    let local = RetrieveEntityTypes {
        locality: Locality::Local,
        ..RetrieveEntityTypes::default()
    };
    assert_eq!(built(&local).uri().query(), Some("local=true"));
    let distributed = RetrieveEntityTypes {
        locality: Locality::Distributed,
        ..RetrieveEntityTypes::default()
    };
    assert_eq!(built(&distributed).uri().query(), Some("local=false"));
}

#[test]
fn entity_type_information_names_the_type_in_one_path_segment() {
    let vehicle: EntityTypeName = name("Vehicle");
    let short = RetrieveEntityTypeInfo::builder().entity_type(&vehicle).build();
    assert_discovery_get(&built(&short), "/ngsi-ld/v1/types/Vehicle", None);
    let fully_qualified: EntityTypeName = name("https://example.org/Vehicle");
    let long = RetrieveEntityTypeInfo::builder().entity_type(&fully_qualified).build();
    assert_discovery_get(&built(&long), "/ngsi-ld/v1/types/https:%2F%2Fexample.org%2FVehicle", None);
    let local = RetrieveEntityTypeInfo::builder().entity_type(&vehicle).locality(Locality::Local).build();
    assert_eq!(built(&local).uri().query(), Some("local=true"));
}

#[test]
fn retrieve_available_attributes_is_a_get_of_attributes() {
    assert_discovery_get(&built(&RetrieveAttributes::default()), "/ngsi-ld/v1/attributes", None);
    assert_discovery_get(&built(&RetrieveAttributeDetails::default()), "/ngsi-ld/v1/attributes", Some("details=true"));
    let local = RetrieveAttributes {
        locality: Locality::Local,
        ..RetrieveAttributes::default()
    };
    assert_eq!(built(&local).uri().query(), Some("local=true"));
}

#[test]
fn attribute_information_names_the_attribute_in_one_path_segment() {
    let speed: AttributeName = name("speed");
    let short = RetrieveAttributeInfo::builder().attribute(&speed).build();
    assert_discovery_get(&built(&short), "/ngsi-ld/v1/attributes/speed", None);
    let fully_qualified: AttributeName = name("https://example.org/speed");
    let long = RetrieveAttributeInfo::builder().attribute(&fully_qualified).build();
    assert_discovery_get(&built(&long), "/ngsi-ld/v1/attributes/https:%2F%2Fexample.org%2Fspeed", None);
}

#[test]
fn every_discovery_request_names_its_tenant() {
    let tenant: Tenant = "t1".parse().unwrap();
    let vehicle: EntityTypeName = name("Vehicle");
    let speed: AttributeName = name("speed");
    let requests = [
        RetrieveEntityTypes::default().to_request(&endpoint().with_tenant(tenant.clone())),
        RetrieveEntityTypeDetails::default().to_request(&endpoint().with_tenant(tenant.clone())),
        RetrieveEntityTypeInfo::builder()
            .entity_type(&vehicle)
            .build()
            .to_request(&endpoint().with_tenant(tenant.clone())),
        RetrieveAttributes::default().to_request(&endpoint().with_tenant(tenant.clone())),
        RetrieveAttributeDetails::default().to_request(&endpoint().with_tenant(tenant.clone())),
        RetrieveAttributeInfo::builder()
            .attribute(&speed)
            .build()
            .to_request(&endpoint().with_tenant(tenant)),
    ];
    for request in requests {
        assert_eq!(header_value(&request.unwrap(), NgsiLdTenant::name()), Some("t1"));
    }
}

#[test]
fn the_entity_type_list_is_read_and_an_array_is_refused() {
    let list = RetrieveEntityTypes::default().decode(&context(), response(200, &[], TYPE_LIST)).unwrap();
    assert_eq!(
        list.type_list,
        [name::<EntityTypeName>("Vehicle"), name("https://example.org/OffStreetParking")]
    );
    let error = RetrieveEntityTypes::default().decode(&context(), response(200, &[], "[]")).unwrap_err();
    assert!(matches!(document_error(error).kind, DocumentErrorKind::Member(MemberError::NotAnObject)));
}

#[test]
fn the_detailed_entity_type_list_is_an_array_read_element_by_element() {
    let details = RetrieveEntityTypeDetails::default()
        .decode(
            &context(),
            response(200, &[], &format!("[{TYPE_DETAILS},{}]", TYPE_DETAILS.replace("Vehicle", "Bus"))),
        )
        .unwrap();
    let names: Vec<&str> = details.iter().map(|details| details.type_name.as_ref()).collect();
    assert_eq!(names, ["Vehicle", "Bus"]);
    let error = RetrieveEntityTypeDetails::default()
        .decode(&context(), response(200, &[], TYPE_DETAILS))
        .unwrap_err();
    assert!(
        matches!(&error, ResponseError::Body { source, .. } if matches!(**source, BodyError::NotAnArray)),
        "{error:?}"
    );
    let error = RetrieveEntityTypeDetails::default()
        .decode(&context(), response(200, &[], &format!("[{TYPE_DETAILS},{ATTRIBUTE_DETAILS}]")))
        .unwrap_err();
    let error = document_error(error);
    assert!(matches!(
        error.kind,
        DocumentErrorKind::DiscoveryType {
            expected: DiscoveryType::EntityType,
            ..
        }
    ));
    assert_eq!(error.path.to_string(), "1");
}

#[test]
fn the_entity_type_information_is_read_and_an_unknown_type_is_a_problem() {
    let vehicle: EntityTypeName = name("Vehicle");
    let retrieve = RetrieveEntityTypeInfo::builder().entity_type(&vehicle).build();
    let info = retrieve.decode(&context(), response(200, &[], TYPE_INFO)).unwrap();
    assert_eq!(info.entity_count, 2);
    assert_eq!(info.attribute_details[0].attribute_types, Some(vec![AttributeKind::Property]));
    let error = retrieve
        .decode(&context(), response(404, &[(&header::CONTENT_TYPE, "application/json")], NOT_FOUND))
        .unwrap_err();
    assert!(is_problem(&error, &ErrorType::ResourceNotFound), "{error:?}");
}

#[test]
fn the_attribute_documents_are_read_and_an_unknown_attribute_is_a_problem() {
    let list = RetrieveAttributes::default().decode(&context(), response(200, &[], ATTRIBUTE_LIST)).unwrap();
    assert_eq!(list.attribute_list.len(), 2);
    let details = RetrieveAttributeDetails::default()
        .decode(&context(), response(200, &[], &format!("[{ATTRIBUTE_DETAILS}]")))
        .unwrap();
    assert_eq!(details.len(), 1);
    assert_eq!(details[0].type_names, Some(vec![name::<EntityTypeName>("Vehicle")]));
    let speed: AttributeName = name("speed");
    let retrieve = RetrieveAttributeInfo::builder().attribute(&speed).build();
    let info = retrieve.decode(&context(), response(200, &[], ATTRIBUTE_INFO)).unwrap();
    assert_eq!(info.attribute_count, Some(3));
    assert_eq!(info.render().to_string(), ATTRIBUTE_INFO);
    let error = retrieve
        .decode(&context(), response(404, &[(&header::CONTENT_TYPE, "application/json")], NOT_FOUND))
        .unwrap_err();
    assert!(is_problem(&error, &ErrorType::ResourceNotFound), "{error:?}");
}

#[test]
fn only_200_is_a_success_of_a_discovery_request() {
    let error = RetrieveEntityTypes::default().decode(&context(), response(204, &[], "")).unwrap_err();
    assert!(matches!(error, ResponseError::UnexpectedStatus { .. }), "{error:?}");
    let speed: AttributeName = name("speed");
    let error = RetrieveAttributeInfo::builder()
        .attribute(&speed)
        .build()
        .decode(&context(), response(201, &[], ATTRIBUTE_INFO))
        .unwrap_err();
    assert!(matches!(error, ResponseError::UnexpectedStatus { .. }), "{error:?}");
}
