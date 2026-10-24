//! Multi-instance attributes (clause 4.5.5) written and read back in both lossless forms, and the bare deletion
//! that cannot stand among instances (clauses 4.5.0 and 5.4).

use crate::model::{
    Attribute,
    AttributeKind,
    AttributesError,
    BuildError,
    Change,
    DatasetId,
    Deletion,
    Entity,
    EntityFragment,
    GeoProperty,
    Input,
    InstanceDeletion,
    Instances,
    InstancesError,
    Lossless,
    Output,
    Patch,
    ProblemKind,
    Property,
    Representation,
    geometry::{Geometry, Position, Shape},
};
use iri_rs::iri;
use pretty_assertions::assert_eq;

fn gps() -> DatasetId {
    iri!("urn:ngsi-ld:dataset:gps").into()
}

fn point(longitude: f64, latitude: f64) -> Geometry {
    Geometry::from(Shape::Point(Position::new(longitude, latitude).unwrap()))
}

/// Writes the entity in `form` and reads it back as a request entity.
fn round_trip(entity: &Entity<Input>, representation: Representation, form: Lossless) -> Entity<Input> {
    Entity::<Input>::parse(&entity.render(representation).to_string(), form).unwrap()
}

#[test]
fn two_named_instances_read_back_from_both_forms() {
    let vehicle = Entity::builder()
        .id(iri!("urn:ngsi-ld:Vehicle:A4567"))
        .entity_type("Vehicle")
        .property("speed", Property::builder(80).dataset_id(gps()))
        .property("speed", Property::builder(79).dataset_id(iri!("urn:ngsi-ld:dataset:speedometer")))
        .build()
        .unwrap();
    assert_eq!(round_trip(&vehicle, Representation::Normalized, Lossless::Normalized), vehicle);
    assert_eq!(round_trip(&vehicle, Representation::Concise, Lossless::Concise), vehicle);
}

#[test]
fn a_default_and_a_named_property_are_both_objects_in_concise_form_and_read_back() {
    let vehicle = Entity::builder()
        .id(iri!("urn:ngsi-ld:Vehicle:A4567"))
        .entity_type("Vehicle")
        .property("speed", 80)
        .property("speed", Property::builder(79).dataset_id(gps()))
        .build()
        .unwrap();
    assert_eq!(
        vehicle.render(Representation::Concise).to_string(),
        r#"{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Vehicle","speed":[{"value":80},{"value":79,"datasetId":"urn:ngsi-ld:dataset:gps"}]}"#
    );
    assert_eq!(round_trip(&vehicle, Representation::Concise, Lossless::Concise), vehicle);
}

#[test]
fn a_default_and_a_named_geo_property_are_both_objects_in_concise_form_and_read_back() {
    let vehicle = Entity::builder()
        .id(iri!("urn:ngsi-ld:Vehicle:A4567"))
        .entity_type("Vehicle")
        .geo_property("location", point(8.0, 40.0))
        .geo_property("location", GeoProperty::builder(point(8.5, 40.5)).dataset_id(gps()))
        .build()
        .unwrap();
    assert_eq!(
        vehicle.render(Representation::Concise).to_string(),
        r#"{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Vehicle","location":[{"value":{"type":"Point","coordinates":[8,40]}},{"value":{"type":"Point","coordinates":[8.5,40.5]},"datasetId":"urn:ngsi-ld:dataset:gps"}]}"#
    );
    let read = round_trip(&vehicle, Representation::Concise, Lossless::Concise);
    assert_eq!(read, vehicle);
    let kinds: Vec<AttributeKind> = read.attribute(&"location".parse().unwrap()).unwrap().iter().map(Attribute::kind).collect();
    assert_eq!(kinds, [AttributeKind::GeoProperty, AttributeKind::GeoProperty]);
}

#[test]
fn a_sub_attribute_with_two_instances_is_written_as_objects_in_concise_form() {
    let speed = Property::builder(80)
        .property("accuracy", 1)
        .property("accuracy", Property::builder(2).dataset_id(gps()))
        .build()
        .unwrap();
    let vehicle = Entity::builder()
        .id(iri!("urn:ngsi-ld:Vehicle:A4567"))
        .entity_type("Vehicle")
        .property("speed", speed)
        .build()
        .unwrap();
    assert_eq!(
        vehicle.render(Representation::Concise).to_string(),
        r#"{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Vehicle","speed":{"value":80,"accuracy":[{"value":1},{"value":2,"datasetId":"urn:ngsi-ld:dataset:gps"}]}}"#
    );
    assert_eq!(round_trip(&vehicle, Representation::Concise, Lossless::Concise), vehicle);
}

#[test]
fn a_fragment_writes_every_instance_of_a_multi_instance_change_as_an_object_in_concise_form() {
    let fragment = EntityFragment::builder()
        .property("speed", 80)
        .delete_instance("speed", AttributeKind::Property, gps())
        .build()
        .unwrap();
    assert_eq!(
        fragment.render(Lossless::Concise).to_string(),
        r#"{"speed":[{"value":80},{"value":"urn:ngsi-ld:null","datasetId":"urn:ngsi-ld:dataset:gps"}]}"#
    );
}

/// Asserts that the only problem is a bare deletion standing among other instances.
fn assert_bare_deletion_among_instances(fragment: &Result<EntityFragment, BuildError>) {
    let error = fragment.as_ref().unwrap_err();
    assert_eq!(error.problems().len(), 1, "{error}");
    assert!(
        matches!(
            &error.problems()[0].kind,
            ProblemKind::Attributes {
                source: AttributesError::Instances {
                    source: InstancesError::BareDeletionAmongInstances
                },
                ..
            }
        ),
        "{error}"
    );
}

#[test]
fn a_bare_deletion_followed_by_another_instance_of_the_name_is_refused() {
    let fragment = EntityFragment::builder()
        .delete("speed")
        .property("speed", Property::builder(79).dataset_id(gps()))
        .build();
    assert_bare_deletion_among_instances(&fragment);
}

#[test]
fn an_instance_followed_by_a_bare_deletion_of_the_name_is_refused() {
    let fragment = EntityFragment::builder()
        .property("speed", Property::builder(79).dataset_id(gps()))
        .delete("speed")
        .build();
    assert_bare_deletion_among_instances(&fragment);
}

#[test]
fn a_bare_sub_attribute_deletion_beside_an_instance_of_that_sub_attribute_is_refused() {
    let fragment = EntityFragment::builder()
        .property(
            "batteryLevel",
            Property::builder(7).property("providedBy", Property::builder("x").dataset_id(gps())),
        )
        .delete_sub_attribute("batteryLevel", "providedBy")
        .build();
    assert_bare_deletion_among_instances(&fragment);
}

#[test]
fn instances_refuse_a_bare_deletion_among_other_instances() {
    let named = Change::Delete(Deletion::Instance(InstanceDeletion::new(AttributeKind::Property).with_dataset_id(gps())));
    let refused = Instances::<Change<Attribute<Patch>>>::try_from_vec(vec![Change::Delete(Deletion::Member), named]);
    assert_eq!(refused, Err(InstancesError::BareDeletionAmongInstances));
}

fn vehicle_with_speeds(first: Option<DatasetId>, second: Option<DatasetId>) -> Result<Entity<Input>, BuildError> {
    Entity::builder()
        .id(iri!("urn:ngsi-ld:Vehicle:A4567"))
        .entity_type("Vehicle")
        .property("speed", Property::builder(80).maybe_dataset_id(first))
        .property("speed", Property::builder(79).maybe_dataset_id(second))
        .build()
}

#[test]
fn two_instances_are_written_as_an_array_with_their_dataset_ids() {
    let vehicle = vehicle_with_speeds(
        Some(iri!("urn:ngsi-ld:dataset:gps").into()),
        Some(iri!("urn:ngsi-ld:dataset:speedometer").into()),
    )
    .unwrap();
    assert_eq!(
        vehicle.render(Representation::Normalized).to_string(),
        r#"{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Vehicle","speed":[{"type":"Property","value":80,"datasetId":"urn:ngsi-ld:dataset:gps"},{"type":"Property","value":79,"datasetId":"urn:ngsi-ld:dataset:speedometer"}]}"#
    );
    assert_eq!(
        vehicle.render(Representation::Concise).to_string(),
        r#"{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Vehicle","speed":[{"value":80,"datasetId":"urn:ngsi-ld:dataset:gps"},{"value":79,"datasetId":"urn:ngsi-ld:dataset:speedometer"}]}"#
    );
}

#[test]
fn the_default_instance_has_no_dataset_id() {
    let vehicle = vehicle_with_speeds(None, Some(iri!("urn:ngsi-ld:dataset:gps").into())).unwrap();
    assert_eq!(
        vehicle.render(Representation::Normalized).to_string(),
        r#"{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Vehicle","speed":[{"type":"Property","value":80},{"type":"Property","value":79,"datasetId":"urn:ngsi-ld:dataset:gps"}]}"#
    );
    assert_eq!(
        vehicle.render(Representation::Concise).to_string(),
        r#"{"id":"urn:ngsi-ld:Vehicle:A4567","type":"Vehicle","speed":[{"value":80},{"value":79,"datasetId":"urn:ngsi-ld:dataset:gps"}]}"#
    );
}

#[test]
fn two_default_instances_are_rejected() {
    let error = vehicle_with_speeds(None, Some(DatasetId::DefaultInstance)).unwrap_err();
    assert!(matches!(
        &error.problems()[0].kind,
        ProblemKind::Attributes {
            source: AttributesError::Instances {
                source: InstancesError::DefaultTwice
            },
            ..
        }
    ));
}

#[test]
fn a_repeated_dataset_id_is_rejected() {
    let gps: DatasetId = iri!("urn:ngsi-ld:dataset:gps").into();
    let error = vehicle_with_speeds(Some(gps.clone()), Some(gps)).unwrap_err();
    assert!(matches!(
        &error.problems()[0].kind,
        ProblemKind::Attributes {
            source: AttributesError::Instances {
                source: InstancesError::DuplicateDatasetId { .. }
            },
            ..
        }
    ));
}

#[test]
fn simplified_multi_attribute_uses_the_default_keyword() {
    let entity = Entity::<Output>::parse(
        r#"{"id":"urn:ngsi-ld:T:1","type":"T","name":[{"type":"Property","value":"David"},{"type":"Property","value":"Bowie","datasetId":"urn:ngsi-ld:datasetId:001"}]}"#,
        Lossless::Normalized,
    )
    .unwrap();
    assert_eq!(
        entity.render(Representation::Simplified).to_string(),
        r#"{"id":"urn:ngsi-ld:T:1","type":"T","name":{"dataset":{"@none":"David","urn:ngsi-ld:datasetId:001":"Bowie"}}}"#
    );
}
