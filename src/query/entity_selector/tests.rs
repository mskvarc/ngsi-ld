//! Entity selectors (NGSI-LD clause 5.2.33).

use crate::{
    model::{DocumentError, EntityId, EntityTypeName, JsonType, MemberError, OneOrMany},
    query::{EntitySelector, EntitySelectorError, Problem, SelectorType, builders::SelectorTypeBuilder},
};
use pretty_assertions::assert_eq;

/// Reads a selector and checks that it is written back as the same text.
fn round_trip(text: &str) -> EntitySelector {
    let selector = EntitySelector::parse(text).unwrap();
    assert_eq!(selector.render().to_string(), text);
    selector
}

#[test]
fn one_id_round_trips_as_a_string() {
    let selector = round_trip(r#"{"type":"Vehicle","id":"urn:ngsi-ld:Vehicle:A4567"}"#);
    assert_eq!(selector.selector_type().to_string(), "Vehicle");
    assert_eq!(
        selector.ids().and_then(|ids| ids.as_one()).map(EntityId::as_str),
        Some("urn:ngsi-ld:Vehicle:A4567")
    );
}

#[test]
fn one_id_in_an_array_round_trips_as_an_array() {
    let selector = round_trip(r#"{"type":"Vehicle","id":["urn:ngsi-ld:Vehicle:A4567"]}"#);
    assert!(selector.ids().is_some_and(OneOrMany::is_many));
}

#[test]
fn a_type_selection_and_an_id_pattern_round_trip() {
    let selector = round_trip(r#"{"type":"(Home;Vehicle)|Motorhome","idPattern":"urn:ngsi-ld:Vehicle:.*"}"#);
    assert!(selector.selector_type().is_selection());
    assert_eq!(selector.id_pattern().map(AsRef::as_ref), Some("urn:ngsi-ld:Vehicle:.*"));
}

#[test]
fn a_selector_without_type_is_refused() {
    assert!(matches!(
        EntitySelector::parse(r#"{"id":"urn:ngsi-ld:Vehicle:A4567"}"#).map_err(|error| error.kind),
        Err(EntitySelectorError::Member(MemberError::MissingMember { member: "type" }))
    ));
}

#[test]
fn the_selector_of_every_type_is_written_as_a_star() {
    assert_eq!(EntitySelector::all().render().to_string(), r#"{"type":"*"}"#);
    assert_eq!(round_trip(r#"{"type":"*"}"#).selector_type(), &SelectorType::All);
}

#[test]
fn an_entity_type_name_converts_into_a_selector_type_builder_owned_or_borrowed() {
    let name: EntityTypeName = "Vehicle".parse().unwrap();
    assert_eq!(SelectorTypeBuilder::from(&name).build().unwrap().to_string(), "Vehicle");
    assert_eq!(
        SelectorTypeBuilder::from(name).build().unwrap(),
        SelectorTypeBuilder::from("Vehicle").build().unwrap()
    );
}

#[test]
fn an_entity_type_name_holding_an_operator_is_refused_when_the_selector_type_is_built() {
    let name: EntityTypeName = "https://example.org/a,b".parse().unwrap();
    let error = SelectorTypeBuilder::from(name).build().unwrap_err();
    assert!(matches!(error.problems(), [Problem::InvalidEntityType { .. }]), "{error:?}");
}

/// Whether an error is the one a case expects.
type ErrorCheck = fn(&EntitySelectorError) -> bool;

#[test]
fn invalid_members_are_refused_by_name() {
    let cases: [(&str, &str, ErrorCheck); 8] = [
        (
            r#"{"type":"Vehicle","foo":1}"#,
            "",
            |error| matches!(error, EntitySelectorError::Member(MemberError::UnexpectedMember { member }) if member == "foo"),
        ),
        (
            r#"{"type":"Vehicle","type":"Car"}"#,
            "",
            |error| matches!(error, EntitySelectorError::Member(MemberError::DuplicateMember { member }) if member == "type"),
        ),
        (r#"{"type":5}"#, "type", |error| {
            matches!(
                error,
                EntitySelectorError::Member(MemberError::WrongShape {
                    member: "type",
                    expected: JsonType::String
                })
            )
        }),
        (r#"{"type":"1Vehicle"}"#, "type", |error| matches!(error, EntitySelectorError::Type { .. })),
        (r#"{"type":"Vehicle","id":[]}"#, "id", |error| matches!(error, EntitySelectorError::EmptyIds)),
        (r#"{"type":"Vehicle","id":"not a uri"}"#, "id", |error| {
            matches!(error, EntitySelectorError::Member(MemberError::Identifier { .. }))
        }),
        (r#"{"type":"Vehicle","id":["urn:ngsi-ld:Vehicle:A4567","not a uri"]}"#, "id.1", |error| {
            matches!(error, EntitySelectorError::Member(MemberError::Identifier { .. }))
        }),
        (r#"{"type":"Vehicle","idPattern":""}"#, "idPattern", |error| {
            matches!(error, EntitySelectorError::IdPattern { .. })
        }),
    ];
    for (text, path, expected) in cases {
        let error: DocumentError<EntitySelectorError> = EntitySelector::parse(text).unwrap_err();
        assert!(expected(&error.kind), "{text}: {error:?}");
        assert_eq!(error.path.to_string(), path, "{text}");
    }
    assert!(matches!(
        EntitySelector::parse("[]").map_err(|error| error.kind),
        Err(EntitySelectorError::Member(MemberError::NotAnObject))
    ));
    assert!(matches!(
        EntitySelector::parse("{").map_err(|error| error.kind),
        Err(EntitySelectorError::Member(MemberError::NotJson))
    ));
}
