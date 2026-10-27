//! The NGSI-LD Null in partial update and merge patches (clauses 4.5.2.2, 4.5.3.2, 5.5.8 and 5.5.12): a Partial
//! Attribute Update that deletes the instance, and fragments that delete an optional member or a sub-attribute of
//! the attribute they change.

use crate::fixtures::{endpoint, json, vehicle_id};
use cefact_units::UnitCode;
use iri_rs::iri;
use jstrict::Unordered;
use ngsi_ld::{
    http::operation::{MergeEntity, Operation, PartialAttributeUpdate},
    model::{
        Attribute,
        AttributeFragment,
        AttributeKind,
        AttributeName,
        Change,
        Complete,
        DatasetId,
        EntityFragment,
        EntityId,
        Input,
        InstanceDeletion,
        Lossless,
        MemberChange,
        Patch,
        PatchMember,
        Projected,
        Property,
        Relationship,
        Value,
        ValuePatch,
    },
};
use pretty_assertions::assert_eq;

/// The body of the Partial Attribute Update of `name` with `attribute`, in `form`.
fn body(name: &str, attribute: &Change<AttributeFragment, InstanceDeletion>, form: Lossless) -> jstrict::Value {
    let id = vehicle_id();
    let name: AttributeName = name.parse().unwrap();
    let request = PartialAttributeUpdate::builder()
        .id(&id)
        .name(&name)
        .attribute(attribute)
        .form(form)
        .build()
        .to_request(&endpoint())
        .unwrap();
    json(request.body())
}

/// `airQualityLevel` with the value 4, the unit `C62` and an observation time, as a fragment.
fn air_quality_level() -> Attribute<Patch> {
    Attribute::from(
        Property::builder(4)
            .unit_code(UnitCode::C62)
            .observed_at("2020-09-09T16:40:00Z".parse().unwrap())
            .build()
            .unwrap(),
    )
    .into_patch()
}

#[test]
fn a_partial_update_deletes_the_instance_with_the_null_in_the_shape_of_its_kind() {
    for (kind, expected) in [
        (AttributeKind::Property, r#"{"type":"Property","value":"urn:ngsi-ld:null"}"#),
        (AttributeKind::GeoProperty, r#"{"type":"GeoProperty","value":"urn:ngsi-ld:null"}"#),
        (
            AttributeKind::LanguageProperty,
            r#"{"type":"LanguageProperty","languageMap":{"@none":"urn:ngsi-ld:null"}}"#,
        ),
        (AttributeKind::VocabProperty, r#"{"type":"VocabProperty","vocab":"urn:ngsi-ld:null"}"#),
        (AttributeKind::ListProperty, r#"{"type":"ListProperty","valueList":["urn:ngsi-ld:null"]}"#),
        (AttributeKind::JsonProperty, r#"{"type":"JsonProperty","json":"urn:ngsi-ld:null"}"#),
        (AttributeKind::Relationship, r#"{"type":"Relationship","object":"urn:ngsi-ld:null"}"#),
        (
            AttributeKind::ListRelationship,
            r#"{"type":"ListRelationship","objectList":["urn:ngsi-ld:null"]}"#,
        ),
    ] {
        let deletion = Change::from(InstanceDeletion::new(kind));
        assert_eq!(body("name", &deletion, Lossless::Normalized), json(expected.as_bytes()), "{kind}");
        let read = Change::<AttributeFragment, InstanceDeletion>::parse(expected, Lossless::Normalized).unwrap();
        assert_eq!(read, deletion, "{kind}");
    }
}

#[test]
fn a_partial_update_deletes_the_instance_its_dataset_id_names_in_either_form() {
    let gps = DatasetId::from(iri!("urn:ngsi-ld:dataset:gps"));
    let deletion = Change::from(InstanceDeletion::new(AttributeKind::Relationship).with_dataset_id(gps));
    assert_eq!(
        Unordered(body("locatedAt", &deletion, Lossless::Normalized)),
        Unordered(json(
            br#"{"type":"Relationship","object":"urn:ngsi-ld:null","datasetId":"urn:ngsi-ld:dataset:gps"}"#
        ))
    );
    assert_eq!(
        Unordered(body("locatedAt", &deletion, Lossless::Concise)),
        Unordered(json(br#"{"object":"urn:ngsi-ld:null","datasetId":"urn:ngsi-ld:dataset:gps"}"#))
    );
    assert_eq!(
        body("name", &Change::from(InstanceDeletion::new(AttributeKind::Property)), Lossless::Concise),
        json(br#"{"value":"urn:ngsi-ld:null"}"#)
    );
}

#[test]
fn a_partial_update_deletes_one_optional_member_and_keeps_the_others() {
    let observed = Change::from(air_quality_level().delete_member(PatchMember::ObservedAt).unwrap());
    assert_eq!(
        body("airQualityLevel", &observed, Lossless::Normalized),
        json(br#"{"type":"Property","value":4,"unitCode":"C62","observedAt":"urn:ngsi-ld:null"}"#)
    );
    let unit = Change::from(air_quality_level().delete_member(PatchMember::UnitCode).unwrap());
    assert_eq!(
        body("airQualityLevel", &unit, Lossless::Normalized),
        json(br#"{"type":"Property","value":4,"unitCode":"urn:ngsi-ld:null","observedAt":"2020-09-09T16:40:00Z"}"#)
    );
    let expires = Change::from(air_quality_level().delete_member(PatchMember::ExpiresAt).unwrap());
    assert!(
        body("airQualityLevel", &expires, Lossless::Normalized)
            .as_object()
            .unwrap()
            .contains_key("expiresAt")
    );
}

#[test]
fn a_partial_update_deletes_a_sub_relationship_with_the_null_as_its_object() {
    let fragment = Attribute::from(Property::builder(4).build().unwrap())
        .into_patch()
        .delete_sub_attribute("hasWeatherObserved", InstanceDeletion::new(AttributeKind::Relationship))
        .unwrap();
    assert_eq!(
        body("airQualityLevel", &Change::from(fragment), Lossless::Normalized),
        json(br#"{"type":"Property","value":4,"hasWeatherObserved":{"type":"Relationship","object":"urn:ngsi-ld:null"}}"#)
    );
}

#[test]
fn a_partial_update_body_that_deletes_members_reads_back_as_the_fragment_sent() {
    let text = r#"{"type":"Relationship","object":"urn:ngsi-ld:Sensor:1","objectType":"urn:ngsi-ld:null","observedAt":"urn:ngsi-ld:null"}"#;
    let Change::Set(AttributeFragment::WithPayload(Attribute::Relationship(relationship))) =
        Change::<AttributeFragment, InstanceDeletion>::parse(text, Lossless::Normalized).unwrap()
    else {
        panic!("a Relationship fragment");
    };
    assert_eq!(relationship.object_type(), Some(MemberChange::Delete));
    assert_eq!(relationship.meta().observed_at, Some(MemberChange::Delete));
    assert!(Change::<AttributeFragment, InstanceDeletion>::parse(r#""urn:ngsi-ld:null""#, Lossless::Normalized).is_err());
}

#[test]
fn a_merge_fragment_deletes_the_unit_code_of_a_property_it_names() {
    let id = vehicle_id();
    let fragment = EntityFragment::for_entity(iri!("urn:ngsi-ld:Building:1"), "Building")
        .patch("airQualityLevel", air_quality_level().delete_member(PatchMember::UnitCode).unwrap())
        .build()
        .unwrap();
    let request = MergeEntity::builder().id(&id).fragment(&fragment).build().to_request(&endpoint()).unwrap();
    assert_eq!(
        json(request.body()),
        json(
            br#"{"id":"urn:ngsi-ld:Building:1","type":"Building","airQualityLevel":{"type":"Property","value":4,"unitCode":"urn:ngsi-ld:null","observedAt":"2020-09-09T16:40:00Z"}}"#
        )
    );
}

#[test]
fn a_fragment_that_deletes_members_round_trips_through_its_reader() {
    let text = r#"{"airQualityLevel":{"type":"Property","value":4,"unitCode":"urn:ngsi-ld:null","observedAt":"urn:ngsi-ld:null","hasWeatherObserved":{"type":"Relationship","object":"urn:ngsi-ld:null"}}}"#;
    for form in [Lossless::Normalized, Lossless::Concise] {
        let fragment = EntityFragment::<Projected>::parse(text, Lossless::Normalized).unwrap();
        let rendered = fragment.render(form).to_string();
        assert_eq!(EntityFragment::<Projected>::parse(&rendered, form).unwrap(), fragment, "{rendered}");
    }
    let fragment = EntityFragment::<Projected>::parse(text, Lossless::Normalized).unwrap();
    assert_eq!(fragment.render(Lossless::Normalized).to_string(), text);
}

#[test]
fn a_request_attribute_never_holds_the_null_as_an_optional_member() {
    let text = r#"{"type":"Property","value":4,"observedAt":"urn:ngsi-ld:null"}"#;
    assert!(Attribute::<Input>::parse_input(text, Lossless::Normalized).is_err());
    let sensor = Relationship::builder(EntityId::from(iri!("urn:ngsi-ld:Sensor:1"))).build().unwrap();
    assert!(Attribute::from(sensor).into_patch().delete_member(PatchMember::UnitCode).is_err());
}

/// The Merge Entity of `fragment` into the vehicle, in normalized form.
fn merge_body(fragment: &EntityFragment<Complete>) -> jstrict::Value {
    let id = vehicle_id();
    let request = MergeEntity::builder().id(&id).fragment(fragment).build().to_request(&endpoint()).unwrap();
    json(request.body())
}

#[test]
fn a_merge_removes_a_member_of_a_property_value_as_clause_5_5_12_example_2_does() {
    let address = ValuePatch::new().set("street", "Pariser Platz").delete("country");
    let fragment = EntityFragment::for_entity(iri!("urn:ngsi-ld:Building:1"), "Building")
        .patch("address", Attribute::from(Property::merge_builder(address).build().unwrap()))
        .build()
        .unwrap();
    assert_eq!(
        merge_body(&fragment),
        json(br#"{"id":"urn:ngsi-ld:Building:1","type":"Building","address":{"type":"Property","value":{"street":"Pariser Platz","country":"urn:ngsi-ld:null"}}}"#)
    );
}

#[test]
fn a_merge_removes_a_member_alone_and_one_nested_at_any_depth() {
    let details = Property::merge_builder(ValuePatch::new().delete("deprecated")).build().unwrap();
    let config = Property::merge_builder(ValuePatch::new().merge("settings", ValuePatch::new().set("a", 1).delete("b")))
        .observed_at("2024-05-01T10:00:00Z".parse().unwrap())
        .build()
        .unwrap();
    let fragment = EntityFragment::for_entity(iri!("urn:ngsi-ld:Building:1"), "Building")
        .patch("buildingDetails", Attribute::from(details))
        .patch("config", Attribute::from(config))
        .build()
        .unwrap();
    assert_eq!(
        merge_body(&fragment),
        json(
            br#"{"id":"urn:ngsi-ld:Building:1","type":"Building","buildingDetails":{"type":"Property","value":{"deprecated":"urn:ngsi-ld:null"}},"config":{"type":"Property","value":{"settings":{"a":1,"b":"urn:ngsi-ld:null"}},"observedAt":"2024-05-01T10:00:00Z"}}"#
        )
    );
}

#[test]
fn a_merged_value_reads_back_as_the_fragment_sent() {
    let address = || {
        let patch = ValuePatch::new().set("street", "Pariser Platz").delete("country");
        Attribute::from(Property::merge_builder(patch).unit_code(UnitCode::C62).build().unwrap())
    };
    for form in [Lossless::Normalized, Lossless::Concise] {
        let text = address().render(form).to_string();
        assert_eq!(
            Change::<AttributeFragment, InstanceDeletion>::parse(&text, form).unwrap(),
            Change::Set(address().into()),
            "{text}"
        );
    }
}

#[test]
fn a_merged_value_sets_no_null_of_its_own() {
    let spelled = Property::merge_builder(ValuePatch::new().set("country", "urn:ngsi-ld:null")).build();
    assert!(spelled.is_err());
    let nested = ValuePatch::new().merge("a", ValuePatch::new().set("b", Value::Array(vec![])).set("c", "urn:ngsi-ld:null"));
    assert!(Property::merge_builder(nested).build().is_err());
    assert!(Property::merge_builder(ValuePatch::new()).build().is_ok());
}

#[test]
fn a_merged_value_is_sent_as_written_by_a_partial_update() {
    let patch = Property::merge_builder(ValuePatch::new().delete("country")).build().unwrap();
    assert_eq!(
        body("address", &Change::from(Attribute::from(patch)), Lossless::Normalized),
        json(br#"{"type":"Property","value":{"country":"urn:ngsi-ld:null"}}"#)
    );
}
