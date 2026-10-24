//! The entity type and attribute discovery documents (clauses 5.2.24 to 5.2.28): reading, writing and each refusal.

use crate::model::{
    AttributeKind,
    AttributeName,
    DocumentError,
    DocumentErrorKind,
    EntityTypeName,
    JsonType,
    MemberError,
    NameError,
    discovery::{AttributeDetails, AttributeInfo, AttributeList, DiscoveryType, EntityTypeDetails, EntityTypeInfo, EntityTypeList},
};
use iri_rs::iri;
use pretty_assertions::assert_eq;
use std::str::FromStr;

/// `V`.
const V: &str = "https://example.org/Vehicle";

/// An `EntityTypeList`, one type a term and one a Fully Qualified Name.
const TYPE_LIST: &str = r#"{"id":"urn:ngsi-ld:EntityTypeList:1","type":"EntityTypeList","typeList":["Vehicle","https://example.org/OffStreetParking"]}"#;

/// An `EntityType` with its attribute names.
const TYPE_DETAILS: &str = r#"{"id":"https://example.org/Vehicle","type":"EntityType","attributeNames":["speed","isParked"],"typeName":"Vehicle"}"#;

/// An `EntityTypeInfo` with the details of two attributes.
const TYPE_INFO: &str = concat!(
    r#"{"id":"https://example.org/Vehicle","type":"EntityTypeInfo","attributeDetails":["#,
    r#"{"id":"https://example.org/speed","type":"Attribute","attributeName":"speed","attributeTypes":["Property"]},"#,
    r#"{"id":"https://example.org/location","type":"Attribute","attributeName":"location","attributeTypes":["GeoProperty"]}],"#,
    r#""entityCount":2,"typeName":"Vehicle"}"#
);

/// An `AttributeList`, one attribute a term and one a Fully Qualified Name.
const ATTRIBUTE_LIST: &str = r#"{"id":"urn:ngsi-ld:AttributeList:1","type":"AttributeList","attributeList":["speed","https://example.org/brandName"]}"#;

/// An `Attribute` with its count, its attribute types and its entity types.
const ATTRIBUTE_INFO: &str = concat!(
    r#"{"id":"https://example.org/speed","type":"Attribute","attributeName":"speed","attributeCount":3,"#,
    r#""attributeTypes":["Property","GeoProperty"],"typeNames":["Vehicle","Bus"]}"#
);

/// `text` with `"@context":"https://example.org/context.jsonld"` added as its last member.
fn with_context(text: &str) -> String {
    format!(r#"{},"@context":"https://example.org/context.jsonld"}}"#, text.strip_suffix('}').unwrap())
}

/// The names `texts` parse to.
fn names<T: FromStr<Err = NameError>>(texts: &[&str]) -> Vec<T> {
    texts.iter().map(|text| text.parse().unwrap()).collect()
}

/// The path of an error, joined with dots.
fn path(error: &DocumentError) -> String {
    error.path.to_string()
}

#[test]
fn an_entity_type_list_keeps_short_and_fully_qualified_names() {
    let list = EntityTypeList::parse(TYPE_LIST).unwrap();
    assert_eq!(list.id.as_str(), "urn:ngsi-ld:EntityTypeList:1");
    assert_eq!(list.type_list, names::<EntityTypeName>(&["Vehicle", "https://example.org/OffStreetParking"]));
    assert_eq!(list.render().to_string(), TYPE_LIST);
}

#[test]
fn an_entity_type_of_the_detailed_list_names_its_attributes() {
    let details = EntityTypeDetails::parse(TYPE_DETAILS).unwrap();
    assert_eq!(details.id.as_str(), V);
    assert_eq!(details.attribute_names, names::<AttributeName>(&["speed", "isParked"]));
    assert_eq!(details.type_name.as_ref(), "Vehicle");
    assert_eq!(details.render().to_string(), TYPE_DETAILS);
}

#[test]
fn the_entity_type_information_counts_entities_and_names_attribute_kinds() {
    let info = EntityTypeInfo::parse(TYPE_INFO).unwrap();
    assert_eq!(info.entity_count, 2);
    assert_eq!(info.type_name.as_ref(), "Vehicle");
    let kinds: Vec<_> = info.attribute_details.iter().map(|attribute| attribute.attribute_types.clone()).collect();
    assert_eq!(kinds, [Some(vec![AttributeKind::Property]), Some(vec![AttributeKind::GeoProperty])]);
    assert_eq!(info.attribute_details[1].attribute_name.as_ref(), "location");
    assert_eq!(info.render().to_string(), TYPE_INFO);
}

#[test]
fn an_attribute_list_round_trips() {
    let list = AttributeList::parse(ATTRIBUTE_LIST).unwrap();
    assert_eq!(list.attribute_list, names::<AttributeName>(&["speed", "https://example.org/brandName"]));
    assert_eq!(list.render().to_string(), ATTRIBUTE_LIST);
}

#[test]
fn an_attribute_of_the_detailed_list_keeps_absent_and_empty_type_names_apart() {
    let with = r#"{"id":"https://example.org/speed","type":"Attribute","attributeName":"speed","typeNames":["Vehicle"]}"#;
    let without = r#"{"id":"https://example.org/speed","type":"Attribute","attributeName":"speed"}"#;
    let empty = r#"{"id":"https://example.org/speed","type":"Attribute","attributeName":"speed","typeNames":[]}"#;
    let read = [with, without, empty].map(|text| AttributeDetails::parse(text).unwrap());
    assert_eq!(read[0].type_names, Some(names::<EntityTypeName>(&["Vehicle"])));
    assert_eq!(read[1].type_names, None);
    assert_eq!(read[2].type_names, Some(Vec::new()));
    assert_eq!(read.map(|details| details.render().to_string()), [with, without, empty]);
}

#[test]
fn the_attribute_information_reads_every_optional_member_or_none() {
    let info = AttributeInfo::parse(ATTRIBUTE_INFO).unwrap();
    assert_eq!(info.attribute_count, Some(3));
    assert_eq!(info.attribute_types, Some(vec![AttributeKind::Property, AttributeKind::GeoProperty]));
    assert_eq!(info.type_names, Some(names::<EntityTypeName>(&["Vehicle", "Bus"])));
    assert_eq!(info.render().to_string(), ATTRIBUTE_INFO);
    let minimal = r#"{"id":"https://example.org/speed","type":"Attribute","attributeName":"speed"}"#;
    let info = AttributeInfo::parse(minimal).unwrap();
    assert_eq!(
        (info.attribute_count, info.attribute_types.is_none(), info.type_names.is_none()),
        (None, true, true)
    );
    assert_eq!(info.render().to_string(), minimal);
}

#[test]
fn a_context_is_kept_and_written_last() {
    let list = with_context(TYPE_LIST);
    assert_eq!(EntityTypeList::parse(&list).unwrap().render().to_string(), list);
    let info = with_context(TYPE_INFO);
    let read = EntityTypeInfo::parse(&info).unwrap();
    assert_eq!(read.context.as_ref().map(|context| context.references().count()), Some(1));
    assert_eq!(read.render().to_string(), info);
    let attribute = with_context(ATTRIBUTE_INFO);
    assert_eq!(AttributeInfo::parse(&attribute).unwrap().render().to_string(), attribute);
    let details = with_context(TYPE_DETAILS);
    assert_eq!(EntityTypeDetails::parse(&details).unwrap().render().to_string(), details);
}

#[test]
fn a_document_of_another_type_is_refused() {
    let list = EntityTypeList::parse(&ATTRIBUTE_LIST.replace("attributeList", "typeList")).unwrap_err();
    assert!(matches!(
        list.kind,
        DocumentErrorKind::DiscoveryType { expected: DiscoveryType::EntityTypeList, ref found } if found == "AttributeList"
    ));
    let info = AttributeInfo::parse(TYPE_DETAILS).unwrap_err();
    assert!(matches!(
        info.kind,
        DocumentErrorKind::DiscoveryType { expected: DiscoveryType::Attribute, ref found } if found == "EntityType"
    ));
    let untyped = EntityTypeList::parse(r#"{"id":"urn:ngsi-ld:EntityTypeList:1","typeList":[]}"#).unwrap_err();
    assert!(matches!(untyped.kind, DocumentErrorKind::Member(MemberError::MissingMember { member: "type" })));
}

#[test]
fn a_missing_mandatory_member_is_named() {
    let missing = |error: DocumentError| {
        let DocumentErrorKind::Member(MemberError::MissingMember { member }) = error.kind else {
            panic!("expected a missing member, got {error:?}")
        };
        member
    };
    assert_eq!(
        missing(EntityTypeList::parse(r#"{"id":"urn:ngsi-ld:EntityTypeList:1","type":"EntityTypeList"}"#).unwrap_err()),
        "typeList"
    );
    assert_eq!(
        missing(EntityTypeInfo::parse(&TYPE_INFO.replace(r#""entityCount":2,"#, "")).unwrap_err()),
        "entityCount"
    );
    let without_details = r#"{"id":"https://example.org/Vehicle","type":"EntityTypeInfo","entityCount":2,"typeName":"Vehicle"}"#;
    assert_eq!(missing(EntityTypeInfo::parse(without_details).unwrap_err()), "attributeDetails");
    assert_eq!(
        missing(AttributeInfo::parse(r#"{"id":"https://example.org/speed","type":"Attribute"}"#).unwrap_err()),
        "attributeName"
    );
    assert_eq!(
        missing(EntityTypeDetails::parse(&TYPE_DETAILS.replace(r#""id":"https://example.org/Vehicle","#, "")).unwrap_err()),
        "id"
    );
}

#[test]
fn a_count_that_is_not_a_whole_number_of_zero_or_more_is_refused() {
    for count in ["-1", "1.5", r#""2""#] {
        let error = EntityTypeInfo::parse(&TYPE_INFO.replace(r#""entityCount":2"#, &format!(r#""entityCount":{count}"#))).unwrap_err();
        assert!(
            matches!(
                error.kind,
                DocumentErrorKind::Member(MemberError::WrongShape {
                    member: "entityCount",
                    expected: JsonType::Count
                })
            ),
            "{count}: {error:?}"
        );
        assert_eq!(path(&error), "entityCount");
    }
    let error = AttributeInfo::parse(&ATTRIBUTE_INFO.replace(r#""attributeCount":3"#, r#""attributeCount":-3"#)).unwrap_err();
    assert!(matches!(
        error.kind,
        DocumentErrorKind::Member(MemberError::WrongShape {
            member: "attributeCount",
            expected: JsonType::Count
        })
    ));
}

#[test]
fn an_unknown_attribute_kind_is_refused_where_it_is() {
    let error = AttributeInfo::parse(&ATTRIBUTE_INFO.replace(r#"["Property","GeoProperty"]"#, r#"["Property","Foo"]"#)).unwrap_err();
    assert!(matches!(error.kind, DocumentErrorKind::UnknownAttributeType { ref found } if found == "Foo"));
    assert_eq!(path(&error), "attributeTypes");
    let error = EntityTypeInfo::parse(&TYPE_INFO.replacen(r#"["Property"]"#, r#"["Property","Foo"]"#, 1)).unwrap_err();
    assert!(matches!(error.kind, DocumentErrorKind::UnknownAttributeType { ref found } if found == "Foo"));
    assert_eq!(path(&error), "attributeDetails.0.attributeTypes");
}

#[test]
fn a_member_the_shape_does_not_allow_is_refused() {
    let unexpected = |error: &DocumentError| {
        let DocumentErrorKind::Member(MemberError::UnexpectedMember { member }) = &error.kind else {
            panic!("expected an unexpected member, got {error:?}")
        };
        member.clone()
    };
    let counted = AttributeDetails::parse(r#"{"id":"https://example.org/speed","type":"Attribute","attributeName":"speed","attributeCount":3}"#).unwrap_err();
    assert_eq!(unexpected(&counted), "attributeCount");
    let typed = EntityTypeInfo::parse(&TYPE_INFO.replacen(
        r#""attributeTypes":["Property"]"#,
        r#""attributeTypes":["Property"],"typeNames":["Vehicle"]"#,
        1,
    ))
    .unwrap_err();
    assert_eq!((unexpected(&typed), path(&typed)), ("typeNames".to_owned(), "attributeDetails.0".to_owned()));
    let foo = EntityTypeList::parse(&TYPE_LIST.replace(r#""type":"EntityTypeList","#, r#""type":"EntityTypeList","foo":1,"#)).unwrap_err();
    assert_eq!(unexpected(&foo), "foo");
}

#[test]
fn invalid_names_identifiers_and_shapes_are_refused_at_their_member() {
    let name = EntityTypeList::parse(r#"{"id":"urn:ngsi-ld:EntityTypeList:1","type":"EntityTypeList","typeList":["1Vehicle"]}"#).unwrap_err();
    assert!(matches!(name.kind, DocumentErrorKind::Member(MemberError::Name { .. })));
    assert_eq!(path(&name), "typeList");
    let id = EntityTypeList::parse(r#"{"id":"not a uri","type":"EntityTypeList","typeList":[]}"#).unwrap_err();
    assert!(matches!(id.kind, DocumentErrorKind::Member(MemberError::Identifier { .. })));
    assert_eq!(path(&id), "id");
    let shape = EntityTypeDetails::parse(&TYPE_DETAILS.replace(r#"["speed","isParked"]"#, "[7]")).unwrap_err();
    assert!(matches!(
        shape.kind,
        DocumentErrorKind::Member(MemberError::WrongShape {
            member: "attributeNames",
            expected: JsonType::String
        })
    ));
    assert_eq!(path(&shape), "attributeNames");
}

#[test]
fn a_member_written_twice_is_refused() {
    let error = EntityTypeList::parse(r#"{"id":"urn:ngsi-ld:EntityTypeList:1","type":"EntityTypeList","typeList":[],"typeList":[]}"#).unwrap_err();
    assert!(matches!(error.kind, DocumentErrorKind::Member(MemberError::DuplicateMember { ref member }) if member == "typeList"));
}

#[test]
fn documents_built_by_their_constructors_render_in_canonical_order() {
    let list = EntityTypeList::new(
        iri!("urn:ngsi-ld:EntityTypeList:1").into(),
        names(&["Vehicle", "https://example.org/OffStreetParking"]),
    );
    assert_eq!(list.render().to_string(), TYPE_LIST);
    let mut speed = AttributeInfo::new(iri!("https://example.org/speed").into(), "speed".parse().unwrap());
    speed.attribute_count = Some(3);
    assert_eq!(
        speed.render().to_string(),
        r#"{"id":"https://example.org/speed","type":"Attribute","attributeName":"speed","attributeCount":3}"#
    );
}
