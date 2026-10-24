//! Golden tests: the project's v1.9.1 vehicle fixtures, the Annex C examples ETSI publishes (ETSI GS CIM 009
//! v1.9.1) and the corrected copies of three of them read, written back, and converted between representations.
//!
//! A document in canonical form (see the design document) is written back exactly: the comparisons are of the
//! compact text, so they check the order of every member as well as its value.

use crate::fixtures::{assert_canonical, elements, fixture, json};
use jstrict::{Print, Unordered, Value as Json};
use ngsi_ld::{
    model::{
        DocumentErrorKind,
        Entity,
        Input,
        Lossless,
        MemberError,
        Output,
        Representation,
        SimplifiedEntity,
        evolution::{AggregatedTemporalEntity, SimplifiedTemporalEntity, TemporalEntity},
        geometry::Shape,
    },
    query::geoquery::GeoQuery,
    registration::CSourceRegistration,
    subscription::{Notification, Subscription},
};

/// Fixture loading and JSON comparison.
#[cfg(test)]
mod fixtures {
    use jstrict::{Print, Value as Json};
    use std::fs;

    /// Reads a fixture.
    pub(crate) fn fixture(path: &str) -> String {
        fs::read_to_string(format!("{}/tests/golden/{path}", env!("CARGO_MANIFEST_DIR"))).unwrap()
    }

    /// Parses JSON text.
    pub(crate) fn json(text: &str) -> Json {
        text.parse().unwrap()
    }

    /// The elements of an array fixture, each in compact form.
    pub(crate) fn elements(path: &str) -> Vec<String> {
        let Json::Array(items) = json(&fixture(path)) else {
            panic!("{path} is an array");
        };
        items.iter().map(|item| item.compact_print().to_string()).collect()
    }

    /// Asserts that `actual` is the document `expected` in compact form, member order included.
    pub(crate) fn assert_canonical(actual: &str, expected: &str) {
        pretty_assertions::assert_eq!(actual, json(expected).compact_print().to_string());
    }
}

#[test]
fn the_project_vehicle_in_normalized_form_reads_and_writes_back_unchanged() {
    let text = fixture("authored/vehicle-normalized.json");
    let vehicle = Entity::<Output>::parse(&text, Lossless::Normalized).unwrap();
    assert_canonical(&vehicle.render(Representation::Normalized).to_string(), &text);
}

#[test]
fn the_vehicle_written_concise_reads_back_as_the_same_entity() {
    let vehicle = Entity::<Output>::parse(&fixture("authored/vehicle-normalized.json"), Lossless::Normalized).unwrap();
    let concise = vehicle.render(Representation::Concise).to_string();
    assert_eq!(Entity::<Output>::parse(&concise, Lossless::Concise).unwrap(), vehicle);
}

#[test]
fn the_concise_vehicle_reads_as_the_normalized_one_and_is_what_the_writer_produces() {
    let normalized = Entity::<Output>::parse(&fixture("authored/vehicle-normalized.json"), Lossless::Normalized).unwrap();
    let text = fixture("authored/vehicle-concise.json");
    assert_eq!(Entity::<Output>::parse(&text, Lossless::Concise).unwrap(), normalized);
    assert_canonical(&normalized.render(Representation::Concise).to_string(), &text);
}

#[test]
fn the_vehicle_written_simplified_is_the_simplified_vehicle() {
    let vehicle = Entity::<Output>::parse(&fixture("authored/vehicle-normalized.json"), Lossless::Normalized).unwrap();
    assert_canonical(
        &vehicle.render(Representation::Simplified).to_string(),
        &fixture("authored/vehicle-simplified.json"),
    );
}

#[test]
fn the_simplified_vehicle_reads_and_writes_back_unchanged() {
    let text = fixture("authored/vehicle-simplified.json");
    let vehicle = SimplifiedEntity::parse(&text).unwrap();
    assert_canonical(&vehicle.render().to_string(), &text);
}

#[test]
fn the_parking_entity_round_trips_in_each_form_the_annex_shows() {
    let normalized_text = fixture("etsi/OffStreetParking_normalized_C.2.3.json");
    let parking = Entity::<Output>::parse(&normalized_text, Lossless::Normalized).unwrap();
    assert_canonical(&parking.render(Representation::Normalized).to_string(), &normalized_text);

    let concise_text = fixture("etsi/OffStreetParking_concise_C.2.3.json");
    assert_eq!(Entity::<Output>::parse(&concise_text, Lossless::Concise).unwrap(), parking);
    assert_canonical(&parking.render(Representation::Concise).to_string(), &concise_text);

    let simplified_text = fixture("etsi/OffStreetParking_simplified_C.2.3.json");
    assert_canonical(&parking.render(Representation::Simplified).to_string(), &simplified_text);
}

#[test]
fn the_annex_multi_attribute_vehicle_round_trips_and_simplifies_into_a_dataset_object() {
    let text = fixture("etsi/Vehicle_MultiAttribute_C.2.2.json");
    let vehicle = Entity::<Output>::parse(&text, Lossless::Normalized).unwrap();
    // Deliberately order-insensitive: the annex writes each instance's `datasetId` after its `source`
    // sub-attribute, while the canonical order puts the members every kind shares before the sub-attributes. The
    // order of the attributes and of the instances is still checked, by the comparison of the simplified form.
    assert_eq!(Unordered(json(&vehicle.render(Representation::Normalized).to_string())), Unordered(json(&text)));
    assert_canonical(
        &vehicle.render(Representation::Simplified).to_string(),
        r#"{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Vehicle","speed":{"dataset":{"urn:ngsi-ld:Property:speedometerA4567-speed":55,"urn:ngsi-ld:Property:gpsBxyz123-speed":54.5}},"@context":[{"Vehicle":"http://example.org/Vehicle","speed":"http://example.org/speed","source":"http://example.org/hasSource"},"https://uri.etsi.org/ngsi-ld/v1/ngsi-ld-core-context-v1.6.jsonld"]}"#,
    );
}

#[test]
fn the_annex_typed_date_time_value_round_trips() {
    let text = fixture("etsi/Vehicle_datetime_C.6.json");
    let vehicle = Entity::<Output>::parse(&text, Lossless::Normalized).unwrap();
    assert_canonical(&vehicle.render(Representation::Normalized).to_string(), &text);
}

#[test]
fn the_annex_query_answer_reads_as_simplified_entities_and_writes_back_unchanged() {
    let Json::Array(entities) = json(&fixture("etsi/EntityList_C.5.4.3.json")) else {
        panic!("the answer is an array");
    };
    let ids: Vec<String> = entities
        .iter()
        .map(|entity| {
            let text = entity.compact_print().to_string();
            let read = SimplifiedEntity::parse(&text).unwrap();
            assert_eq!(read.render().to_string(), text);
            read.id.to_string()
        })
        .collect();
    assert_eq!(ids, ["urn:ngsi-ld:Vehicle:B9211", "urn:ngsi-ld:Vehicle:A456"]);
}

#[test]
fn the_annex_subscription_reads_with_its_latitude_of_100_degrees() {
    let subscription = Subscription::<Input>::parse(&fixture("etsi/Subscription_C.4.json")).unwrap();
    let Some(Shape::Point(origin)) = subscription.geo_q().map(GeoQuery::geometry) else {
        panic!("expected a point geoquery");
    };
    assert_eq!(origin.latitude().to_string(), "100");
}

#[test]
fn the_corrected_copy_of_the_annex_subscription_reads_and_writes_back_unchanged() {
    let text = fixture("etsi-corrected/subscription-c4.json");
    let subscription = Subscription::<Input>::parse(&text).unwrap();
    assert_eq!(subscription.entities()[0].selector_type().to_string(), "Vehicle");
    assert_eq!(subscription.watched_attributes()[0].as_ref(), "speed");
    assert_eq!(subscription.q().map(ToString::to_string).as_deref(), Some("speed>50"));
    assert_eq!(
        subscription.geo_q().map(|query| query.relation().to_string()).as_deref(),
        Some("near;maxDistance==2000")
    );
    assert_eq!(subscription.notification().format(), Some(Representation::Simplified));
    assert_canonical(&subscription.render().to_string(), &text);
}

#[test]
fn the_project_vehicle_reads_and_writes_back_in_a_notification_of_each_form() {
    let normalized_text = fixture("authored/notification-normalized.json");
    let normalized = Notification::<Vec<Entity<Output>>>::parse(&normalized_text, Lossless::Normalized).unwrap();
    assert_canonical(&normalized.render(Lossless::Normalized).to_string(), &normalized_text);
    let concise_text = fixture("authored/notification-concise.json");
    let concise = Notification::<Vec<Entity<Output>>>::parse(&concise_text, Lossless::Concise).unwrap();
    assert_canonical(&concise.render(Lossless::Concise).to_string(), &concise_text);
    assert_eq!(concise.data(), normalized.data());
    let simplified_text = fixture("authored/notification-simplified.json");
    let simplified = Notification::<Vec<SimplifiedEntity>>::parse(&simplified_text).unwrap();
    assert_canonical(&simplified.render().to_string(), &simplified_text);
}

#[test]
fn the_annex_temporal_evolution_reads_and_writes_back_unchanged() {
    for text in elements("etsi/EntityTemporal_C.5.5.3.json") {
        let entity = TemporalEntity::<Output>::parse(&text, Lossless::Normalized).unwrap();
        assert_eq!(entity.attributes.iter().map(|(_, evolution)| evolution.len()).sum::<usize>(), 3);
        assert_eq!(entity.render(Lossless::Normalized).to_string(), text);
    }
}

#[test]
fn the_annex_simplified_temporal_evolution_reads_and_writes_back_unchanged() {
    for text in elements("etsi/EntityTemporal_simplified_C.5.6.3.json") {
        let entity = SimplifiedTemporalEntity::parse(&text).unwrap();
        assert_eq!(entity.attributes.len(), 1);
        assert_eq!(entity.render().to_string(), text);
    }
}

#[test]
fn the_annex_aggregated_temporal_evolution_reads_and_writes_back_unchanged() {
    for text in elements("etsi/Vehicle_temporal_aggregated_C.5.14.3.json") {
        let entity = AggregatedTemporalEntity::parse(&text).unwrap();
        assert_eq!(entity.attributes.values().next().map(|attribute| attribute.methods.len()), Some(2));
        assert_eq!(entity.render().to_string(), text);
    }
}

#[test]
fn the_annex_temporal_scope_is_refused_for_its_unwrapped_pair_and_the_corrected_copy_round_trips() {
    let published = elements("etsi/Vehicle_temporal_scope_C.5.16.3.json");
    let error = SimplifiedTemporalEntity::parse(&published[0]).unwrap_err();
    assert_eq!(error.path.to_string(), "scope.0");
    assert!(matches!(error.kind, DocumentErrorKind::Member(MemberError::NotAnArray)), "{error}");
    let corrected = elements("etsi-corrected/temporal-scope-c516.json");
    assert_eq!(corrected.len(), 2);
    for text in corrected {
        let entity = SimplifiedTemporalEntity::parse(&text).unwrap();
        assert_eq!(entity.scope.len(), 1);
        assert_eq!(entity.render().to_string(), text);
    }
}

#[test]
fn the_annex_registration_reads_its_member_of_an_earlier_version_as_a_context_source_property() {
    // `timestamp` is no member of Table 5.2.9-1 in this version, so it is a Context Source Property (clause 4.9).
    let registration = CSourceRegistration::<Input>::parse(&fixture("etsi/ContextSourceRegistration_C.3.json")).unwrap();
    let properties: Vec<(&str, String)> = registration
        .properties()
        .iter()
        .map(|(name, value)| (name.as_ref(), value.to_string()))
        .collect();
    assert_eq!(properties, [("timestamp", r#"{"startAt":" 2017-11-29T14:53:15Z"}"#.to_owned())]);
}

#[test]
fn the_corrected_copy_of_the_annex_registration_reads_and_writes_back_unchanged() {
    let text = fixture("etsi-corrected/registration-c3.json");
    let registration = CSourceRegistration::<Input>::parse(&text).unwrap();
    assert_eq!(registration.information().len(), 2);
    assert_eq!(registration.information()[1].entities().len(), 2);
    assert_eq!(registration.information()[0].relationship_names()[0].as_ref(), "isParked");
    assert_eq!(registration.endpoint().as_str(), "http://my.csource.org:1026/");
    assert!(registration.location().is_some());
    assert_eq!(registration.management_interval().map(|interval| interval.end_at()), Some(None));
    assert_canonical(&registration.render().to_string(), &text);
}
