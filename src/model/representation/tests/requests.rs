//! What a request document may hold: no NGSI-LD Null (clause 5.5.4), `GeoProperty` attributes only under the
//! names clause 5.2.4 reserves for them, and an `@context` that is kept (clause 4.5.1) but never written into a
//! request body (clause 6.3.5); and the canonical member order every document is written in.

use crate::model::{
    Attribute,
    AttributeKind,
    AttributesError,
    Context,
    DocumentError,
    DocumentErrorKind,
    Entity,
    EntityFragment,
    Input,
    Instances,
    Lossless,
    MemberError,
    NullMember,
    Output,
    ProblemKind,
    Representation,
    Value,
    representation::write::{write_entity, write_fragment},
};
use iri_rs::{IriRefBuf, iri};
use jstrict::{Print, Value as Json};
use pretty_assertions::assert_eq;

/// Reads a request entity with one attribute `x`, written in `form`.
fn request_with(attribute: &str, form: Lossless) -> Result<Entity<Input>, DocumentError> {
    Entity::<Input>::parse(&format!(r#"{{"id":"urn:ngsi-ld:T:1","type":"T","x":{attribute}}}"#), form)
}

/// Asserts that the request is refused for holding the NGSI-LD Null in `member`, under the attribute `x`.
fn assert_null_refused(result: Result<Entity<Input>, DocumentError>, expected: NullMember) {
    let error = result.unwrap_err();
    assert_eq!(error.path.to_string(), "x", "{error}");
    assert!(matches!(error.kind, DocumentErrorKind::NgsiLdNull { member } if member == expected), "{error}");
}

#[test]
fn a_request_refuses_the_null_in_every_payload_shape_of_clause_4_5_0() {
    for (attribute, member) in [
        (r#"{"type":"Property","value":"urn:ngsi-ld:null"}"#, NullMember::Value),
        (r#"{"type":"Property","value":{"a":{"b":"urn:ngsi-ld:null"}}}"#, NullMember::ValueMember),
        (r#"{"type":"Property","value":{"a":[{"b":"urn:ngsi-ld:null"}]}}"#, NullMember::ValueMember),
        (
            r#"{"type":"LanguageProperty","languageMap":{"@none":"urn:ngsi-ld:null"}}"#,
            NullMember::LanguageMap,
        ),
        (r#"{"type":"VocabProperty","vocab":"urn:ngsi-ld:null"}"#, NullMember::Vocab),
        (r#"{"type":"ListProperty","valueList":["urn:ngsi-ld:null"]}"#, NullMember::ValueList),
        (r#"{"type":"Relationship","object":"urn:ngsi-ld:null"}"#, NullMember::Object),
        (
            r#"{"type":"ListRelationship","objectList":[{"object":"urn:ngsi-ld:null"}]}"#,
            NullMember::ObjectList,
        ),
    ] {
        assert_null_refused(request_with(attribute, Lossless::Normalized), member);
    }
}

#[test]
fn a_concise_request_refuses_the_bare_null_and_the_null_object() {
    assert_null_refused(request_with(r#""urn:ngsi-ld:null""#, Lossless::Concise), NullMember::Value);
    assert_null_refused(request_with(r#"{"object":"urn:ngsi-ld:null"}"#, Lossless::Concise), NullMember::Object);
    assert_null_refused(
        request_with(r#"{"objectList":["urn:ngsi-ld:null"]}"#, Lossless::Concise),
        NullMember::ObjectList,
    );
}

#[test]
fn a_request_refuses_the_null_in_a_sub_attribute() {
    let error = request_with(
        r#"{"type":"Property","value":1,"providedBy":{"type":"Relationship","object":"urn:ngsi-ld:null"}}"#,
        Lossless::Normalized,
    )
    .unwrap_err();
    assert_eq!(error.path.to_string(), "x.providedBy");
    assert!(matches!(error.kind, DocumentErrorKind::NgsiLdNull { member: NullMember::Object }));
}

#[test]
fn a_request_keeps_the_null_where_it_is_one_value_among_others_or_raw_json() {
    for attribute in [
        r#"{"type":"Property","value":["urn:ngsi-ld:null"]}"#,
        r#"{"type":"ListProperty","valueList":["urn:ngsi-ld:null",1]}"#,
        r#"{"type":"LanguageProperty","languageMap":{"en":"urn:ngsi-ld:null"}}"#,
        r#"{"type":"JsonProperty","json":{"a":"urn:ngsi-ld:null"}}"#,
    ] {
        assert!(request_with(attribute, Lossless::Normalized).is_ok(), "{attribute}");
    }
}

#[test]
fn a_single_request_attribute_refuses_the_null() {
    let error = Attribute::<Input>::parse_input(r#""urn:ngsi-ld:null""#, Lossless::Concise).unwrap_err();
    assert!(matches!(error.kind, DocumentErrorKind::NgsiLdNull { member: NullMember::Value }));
}

#[test]
fn a_response_holds_the_null_of_a_deleted_attribute_and_cannot_be_turned_into_a_request() {
    let response = Entity::<Output>::parse(
        r#"{"id":"urn:ngsi-ld:T:1","type":"T","x":{"type":"Property","value":"urn:ngsi-ld:null"}}"#,
        Lossless::Normalized,
    )
    .unwrap();
    let error = response.into_input().unwrap_err();
    assert_eq!(error.problems().len(), 1);
    assert_eq!(error.problems()[0].within().map(ToString::to_string).collect::<Vec<_>>(), ["x"]);
    assert!(matches!(error.problems()[0].kind, ProblemKind::NgsiLdNull { member: NullMember::Value }));
}

#[test]
fn reading_location_as_a_property_is_refused_in_every_direction() {
    let text = r#"{"id":"urn:ngsi-ld:T:1","type":"T","location":{"type":"Property","value":"somewhere"}}"#;
    for error in [
        Entity::<Input>::parse(text, Lossless::Normalized).unwrap_err(),
        Entity::<Output>::parse(text, Lossless::Normalized).unwrap_err(),
    ] {
        assert_eq!(error.path.to_string(), "location");
        assert!(matches!(
            error.kind,
            DocumentErrorKind::Attributes {
                source: AttributesError::GeoPropertyRequired
            }
        ));
    }
    let concise = r#"{"id":"urn:ngsi-ld:T:1","type":"T","observationSpace":"somewhere"}"#;
    assert!(Entity::<Input>::parse(concise, Lossless::Concise).is_err());
}

#[test]
fn a_fragment_deleting_location_as_another_kind_is_refused() {
    let error = EntityFragment::builder()
        .delete_kind("operationSpace", AttributeKind::Property)
        .build()
        .unwrap_err();
    assert!(matches!(
        &error.problems()[0].kind,
        ProblemKind::Attributes {
            source: AttributesError::GeoPropertyRequired,
            ..
        }
    ));
    let fragment = EntityFragment::builder()
        .delete_kind("location", AttributeKind::GeoProperty)
        .delete("observationSpace")
        .build()
        .unwrap();
    assert_eq!(fragment.attributes.len(), 2);
}

#[test]
fn setting_location_as_a_property_in_a_fragment_is_refused() {
    let error = EntityFragment::builder().property("location", "somewhere").build().unwrap_err();
    assert!(matches!(
        &error.problems()[0].kind,
        ProblemKind::Attributes {
            source: AttributesError::GeoPropertyRequired,
            ..
        }
    ));
}

const WITH_CONTEXT: &str = r#"{"id":"urn:ngsi-ld:T:1","type":"T","speed":{"type":"Property","value":1},"@context":["https://example.org/context.jsonld","https://uri.etsi.org/ngsi-ld/v1/ngsi-ld-core-context-v1.9.jsonld"]}"#;

#[test]
fn a_request_entity_keeps_its_context_and_render_writes_it_last() {
    let entity = Entity::<Input>::parse(WITH_CONTEXT, Lossless::Normalized).unwrap();
    assert_eq!(entity.context.as_ref().map(|context| context.references().count()), Some(2));
    assert_eq!(entity.render(Representation::Normalized).to_string(), WITH_CONTEXT);
}

#[test]
fn a_request_body_never_carries_the_entity_context() {
    let entity = Entity::<Input>::parse(WITH_CONTEXT, Lossless::Normalized).unwrap();
    let body = write_entity(&entity, Lossless::Normalized);
    assert!(body.get("@context").next().is_none());
    let fragment = write_fragment(&entity.into_fragment(), Lossless::Concise);
    assert!(fragment.get("@context").next().is_none());
}

#[test]
fn a_built_entity_carries_the_context_it_is_given() {
    let context = Context::from(IriRefBuf::new("https://example.org/context.jsonld").unwrap());
    let entity = Entity::builder().id(iri!("urn:ngsi-ld:T:1")).entity_type("T").context(context).build().unwrap();
    assert_eq!(
        entity.render(Representation::Concise).to_string(),
        r#"{"id":"urn:ngsi-ld:T:1","type":"T","@context":"https://example.org/context.jsonld"}"#
    );
}

#[test]
fn an_entity_is_written_in_canonical_member_order_whatever_order_it_was_read_in() {
    let read = Entity::<Output>::parse(
        r#"{"@context":"https://example.org/context.jsonld","modifiedAt":"2020-01-02T00:00:00Z","b":{"value":2,"observedAt":"2020-01-01T00:00:00Z","type":"Property"},"createdAt":"2020-01-01T00:00:00Z","a":{"object":"urn:ngsi-ld:T:2","type":"Relationship","objectType":"T"},"scope":"/Madrid","type":"T","id":"urn:ngsi-ld:T:1"}"#,
        Lossless::Normalized,
    )
    .unwrap();
    let canonical: Json = r#"{"id":"urn:ngsi-ld:T:1","type":"T","scope":"/Madrid","b":{"type":"Property","value":2,"observedAt":"2020-01-01T00:00:00Z"},"a":{"type":"Relationship","objectType":"T","object":"urn:ngsi-ld:T:2"},"createdAt":"2020-01-01T00:00:00Z","modifiedAt":"2020-01-02T00:00:00Z","@context":"https://example.org/context.jsonld"}"#
        .parse()
        .unwrap();
    assert_eq!(read.render(Representation::Normalized).to_string(), canonical.compact_print().to_string());
}

#[test]
fn a_member_given_twice_is_refused() {
    for text in [
        r#"{"id":"urn:ngsi-ld:T:1","type":"T","@context":"https://example.org/a.jsonld","@context":"https://example.org/b.jsonld"}"#,
        r#"{"id":"urn:ngsi-ld:T:1","type":"T","createdAt":"2020-01-01T00:00:00Z","createdAt":"2020-01-01T00:00:00Z"}"#,
    ] {
        let error = Entity::<Output>::parse(text, Lossless::Normalized).unwrap_err();
        assert!(matches!(error.kind, DocumentErrorKind::Member(MemberError::DuplicateMember { .. })), "{error}");
    }
}

#[test]
fn into_input_drops_system_members() {
    let output = Entity::<Output>::parse(
        r#"{"id":"urn:ngsi-ld:T:1","type":"T","createdAt":"2020-01-01T00:00:00Z","speed":{"type":"Property","value":1,"modifiedAt":"2020-01-01T00:00:00Z"}}"#,
        Lossless::Normalized,
    )
    .unwrap();
    assert_eq!(
        output.into_input().unwrap().render(Representation::Normalized).to_string(),
        r#"{"id":"urn:ngsi-ld:T:1","type":"T","speed":{"type":"Property","value":1}}"#
    );
}

#[test]
fn a_single_request_attribute_holds_no_context_in_either_representation() {
    for form in [Lossless::Normalized, Lossless::Concise] {
        let error = Attribute::<Input>::parse_input(r#"{"@context":"https://example.org/context.jsonld","type":"Property","value":1}"#, form).unwrap_err();
        assert!(matches!(error.kind, DocumentErrorKind::Member(MemberError::UnexpectedMember { ref member }) if member == "@context"));
    }
}

#[test]
fn a_request_document_refuses_system_members() {
    let error = Entity::<Input>::parse(
        r#"{"id":"urn:ngsi-ld:T:1","type":"T","createdAt":"2020-01-01T00:00:00Z"}"#,
        Lossless::Normalized,
    )
    .unwrap_err();
    assert!(matches!(error.kind, DocumentErrorKind::Member(MemberError::UnexpectedMember { .. })));
}

#[test]
fn a_request_holds_the_json_null_literal_as_a_property_value_in_both_forms() {
    let literal = r#"{"@type":"@json","@value":null}"#;
    for (form, representation, attribute) in [
        (
            Lossless::Normalized,
            Representation::Normalized,
            format!(r#"{{"type":"Property","value":{literal}}}"#),
        ),
        (Lossless::Concise, Representation::Concise, literal.to_owned()),
    ] {
        let entity = request_with(&attribute, form).unwrap();
        let text = entity.render(representation).to_string();
        assert!(text.contains(literal), "{text}");
        assert_eq!(Entity::<Input>::parse(&text, form).unwrap(), entity);
    }
}

#[test]
fn a_property_built_with_the_json_null_literal_writes_it_and_reads_it_back() {
    let entity = Entity::builder()
        .id(iri!("urn:ngsi-ld:T:1"))
        .entity_type("T")
        .property("x", Value::JsonNull)
        .build()
        .unwrap();
    let normalized = r#"{"id":"urn:ngsi-ld:T:1","type":"T","x":{"type":"Property","value":{"@type":"@json","@value":null}}}"#;
    assert_eq!(entity.render(Representation::Normalized).to_string(), normalized);
    assert_eq!(Entity::<Input>::parse(normalized, Lossless::Normalized).unwrap(), entity);
}

#[test]
fn a_response_entity_holds_the_json_null_literal_spelled_with_the_term_type() {
    let canonical = r#"{"@type":"@json","@value":null}"#;
    for (form, representation) in [(Lossless::Normalized, Representation::Normalized), (Lossless::Concise, Representation::Concise)] {
        let text = r#"{"id":"urn:ngsi-ld:T:1","type":"T","x":{"type":"Property","value":{"@value":null,"type":"@json"}}}"#;
        let entity = Entity::<Output>::parse(text, form).unwrap();
        let written = entity.render(representation).to_string();
        assert!(written.contains(canonical), "{written}");
    }
}

#[test]
fn a_property_value_holding_a_non_null_json_literal_reads_as_an_object_in_both_forms() {
    let literal = r#"{"@type":"@json","@value":{"a":1,"b":[true]}}"#;
    for (form, representation, attribute) in [
        (
            Lossless::Normalized,
            Representation::Normalized,
            format!(r#"{{"type":"Property","value":{literal}}}"#),
        ),
        (Lossless::Concise, Representation::Concise, literal.to_owned()),
    ] {
        let entity = request_with(&attribute, form).unwrap();
        let Some(Attribute::Property(x)) = entity.attribute(&"x".parse().unwrap()).map(Instances::first) else {
            panic!("x is not a Property");
        };
        assert!(x.value().is_object());
        assert_eq!(x.value().to_string(), literal);
        let text = entity.render(representation).to_string();
        assert!(text.contains(literal), "{text}");
        assert_eq!(Entity::<Input>::parse(&text, form).unwrap(), entity);
    }
}

#[test]
fn a_response_property_value_holding_a_non_null_json_literal_spelled_with_the_term_type_is_written_as_received() {
    let text = concat!(
        r#"{"id":"urn:ngsi-ld:T:1","type":"T","#,
        r#""x":{"type":"Property","value":{"@value":{"a":1},"type":"@json"}}}"#
    );
    let entity = Entity::<Output>::parse(text, Lossless::Normalized).unwrap();
    assert_eq!(entity.render(Representation::Normalized).to_string(), text);
}

#[test]
fn a_response_property_value_holding_a_json_literal_under_an_expanded_term_is_an_object_read_by_path() {
    for literal in [
        r#"{"@value":{"a":1,"b":[true]},"type":"@json"}"#,
        r#"{"@type":"@json","@value":{"a":1,"b":[true]}}"#,
    ] {
        let text = format!(r#"{{"id":"urn:ngsi-ld:T:1","type":"T","x":{{"type":"Property","value":{{"https://example.org/settings":{literal}}}}}}}"#);
        let entity = Entity::<Output>::parse(&text, Lossless::Normalized).unwrap();
        let Some(Attribute::Property(x)) = entity.attribute(&"x".parse().unwrap()).map(Instances::first) else {
            panic!("x is not a Property");
        };
        let Value::Object(members) = x.value() else {
            panic!("the value of x is not an object: {}", x.value());
        };
        let Some(Value::Object(held)) = members.get("https://example.org/settings") else {
            panic!("the value of x holds no literal under the expanded term: {}", x.value());
        };
        let Some(Value::Object(content)) = held.get("@value") else {
            panic!("the literal holds no object under @value: {}", x.value());
        };
        assert_eq!(content.get("a"), Some(&Value::from(1)));
        assert_eq!(entity.render(Representation::Normalized).to_string(), text);
    }
}

#[test]
fn a_concise_attribute_that_is_a_value_object_spelled_with_the_term_type_is_normalized_and_refused() {
    let error = request_with(r#"{"@value":null,"type":"@json"}"#, Lossless::Concise).unwrap_err();
    assert!(
        matches!(error.kind, DocumentErrorKind::UnknownAttributeType { ref found } if found == r#""@json""#),
        "{error}"
    );
}

#[test]
fn an_object_type_is_a_term_so_it_may_hold_a_hyphen() {
    for (object_type, written) in [
        (r#""smart-building""#, r#""smart-building""#),
        (r#"["smart-building","smart-room"]"#, r#"["smart-building","smart-room"]"#),
    ] {
        let attribute = format!(r#"{{"type":"Relationship","object":"urn:ngsi-ld:T:2","objectType":{object_type}}}"#);
        let entity = request_with(&attribute, Lossless::Normalized).unwrap();
        let text = entity.render(Representation::Normalized).to_string();
        assert!(text.contains(&format!(r#""objectType":{written}"#)), "{text}");
    }
    assert!(
        request_with(
            r#"{"type":"Relationship","object":"urn:ngsi-ld:T:2","objectType":"smart building"}"#,
            Lossless::Normalized
        )
        .is_err()
    );
}
