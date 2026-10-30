//! The client of the broker under test, names unique to each test, and the documents several tests share.
//!
//! No `@context` is sent, so every term expands under the core context's default vocabulary (clause 4.3.4).

use cefact_units::UnitCode;
use iri_rs::iri;
use ngsi_ld::{
    client::{Client, ReqwestTransport},
    http::{CreateOutcome, headers::Prefer},
    model::{
        Attribute,
        AttributeName,
        Entity,
        EntityId,
        ErrorType,
        Input,
        Instances,
        ListProperty,
        ListRelationship,
        Output,
        Property,
        Relationship,
        Value,
        VocabTerm,
    },
};
use std::{env, process, time::SystemTime};
use url::Url;

/// The environment variable naming the API root of the broker under test.
const BROKER_URL_VARIABLE: &str = "NGSI_LD_TEST_BROKER_URL";

/// A client of the broker under test, on the default tenant.
pub(crate) fn client() -> Client<ReqwestTransport> {
    client_preferring(None)
}

/// A client of the broker under test, on the default tenant, stating `prefer` on every request (clause 6.3.6).
pub(crate) fn client_preferring(prefer: Option<Prefer>) -> Client<ReqwestTransport> {
    let base_url = env::var(BROKER_URL_VARIABLE).expect("NGSI_LD_TEST_BROKER_URL names the API root of the broker under test");
    Client::builder()
        .base_url(Url::parse(&base_url).expect("NGSI_LD_TEST_BROKER_URL is a URL"))
        .maybe_prefer(prefer)
        .transport(ReqwestTransport::try_default().unwrap())
        .build()
        .unwrap()
}

/// A name no other test and no earlier run uses: `label`, the process id and the time, in letters and digits only,
/// so it fits in a URN, an entity type name and a tenant name.
pub(crate) fn unique(label: &str) -> String {
    let nanos = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_nanos();
    format!("{label}p{}t{nanos}", process::id())
}

/// An entity id unique to the calling test.
pub(crate) fn entity_id(label: &str) -> EntityId {
    format!("urn:ngsi-ld:Vehicle:{}", unique(label)).parse().unwrap()
}

/// An attribute name.
pub(crate) fn name(text: &str) -> AttributeName {
    text.parse().unwrap()
}

/// A vehicle with one attribute of each kind the Annex C.2.2 vehicle has, a sub-Relationship, an `objectType`, an
/// `observedAt` and a `unitCode`, but no `valueType`.
pub(crate) fn every_kind_vehicle(id: &EntityId) -> Entity<Input> {
    Entity::builder()
        .id(id.as_str())
        .entity_type("Vehicle")
        .property("brandName", "Mercedes")
        .language_property("street", [("fr", "Grand Place"), ("nl", "Grote Markt")])
        .relationship(
            "isParked",
            Relationship::builder(EntityId::from(iri!("urn:ngsi-ld:OffStreetParking:Downtown1")))
                .object_type("OffStreetParking".parse::<VocabTerm>().unwrap())
                .observed_at("2017-07-29T12:00:04Z".parse().unwrap())
                .relationship("providedBy", iri!("urn:ngsi-ld:Person:Bob")),
        )
        .vocab_property("category", "non-commercial")
        .list_property(
            "tyreTreadDepths",
            ListProperty::builder(vec![Value::from(300), Value::from(300), Value::from(120), Value::from(6)]).unit_code(UnitCode::Mmt),
        )
        .relationship(
            "passengers",
            Relationship::builder(vec![
                EntityId::from(iri!("urn:ngsi-ld:Person:Alice")),
                EntityId::from(iri!("urn:ngsi-ld:Person:Bob")),
            ])
            .object_type("Person".parse::<VocabTerm>().unwrap()),
        )
        .list_relationship(
            "route",
            ListRelationship::builder(vec![
                EntityId::from(iri!("urn:ngsi-ld:City:Antwerp")),
                EntityId::from(iri!("urn:ngsi-ld:City:Rotterdam")),
                EntityId::from(iri!("urn:ngsi-ld:City:Amsterdam")),
            ])
            .object_type("City".parse::<VocabTerm>().unwrap()),
        )
        .build()
        .unwrap()
}

/// The vehicle of Annex C.2.2 under the id `id`: [`every_kind_vehicle`] with the `valueType` of its
/// `ListProperty`.
pub(crate) fn annex_vehicle(id: &EntityId) -> Entity<Input> {
    let mut vehicle = every_kind_vehicle(id);
    let tyre_tread_depths = ListProperty::builder(vec![Value::from(300), Value::from(300), Value::from(120), Value::from(6)])
        .value_type("Integer".parse().unwrap())
        .unit_code(UnitCode::Mmt)
        .build()
        .unwrap();
    vehicle
        .attributes
        .set(name("tyreTreadDepths"), Instances::one(Attribute::from(tyre_tread_depths)))
        .unwrap();
    vehicle
}

/// A parked vehicle: `brandName`, `speed` and the Relationship `isParked`.
pub(crate) fn parked_vehicle(id: &EntityId) -> Entity<Input> {
    Entity::builder()
        .id(id.as_str())
        .entity_type("Vehicle")
        .property("brandName", "Mercedes")
        .property("speed", 0)
        .relationship("isParked", iri!("urn:ngsi-ld:OffStreetParking:Downtown1"))
        .build()
        .unwrap()
}

/// The default instance of the Property `attribute`, if the entity has it as a Property.
pub(crate) fn property<'e>(entity: &'e Entity<Output>, attribute: &str) -> Option<&'e Property<Output>> {
    match entity.attribute(&name(attribute))?.default_instance()? {
        Attribute::Property(property) => Some(property),
        Attribute::GeoProperty(_)
        | Attribute::LanguageProperty(_)
        | Attribute::VocabProperty(_)
        | Attribute::ListProperty(_)
        | Attribute::JsonProperty(_)
        | Attribute::Relationship(_)
        | Attribute::ListRelationship(_) => None,
    }
}

/// The value of the default instance of the Property `attribute`, if the entity has it as a Property.
pub(crate) fn property_value<'e>(entity: &'e Entity<Output>, attribute: &str) -> Option<&'e Value> {
    property(entity, attribute).map(Property::value)
}

/// The names of the entity's attributes, sorted: a JSON object's member order carries no meaning, so a broker
/// may answer in any order.
pub(crate) fn attribute_names(entity: &Entity<Output>) -> Vec<&str> {
    let mut names: Vec<&str> = entity.attributes.names().map(AsRef::as_ref).collect();
    names.sort_unstable();
    names
}

/// Creates `entity`, which must succeed with 201 (clause 6.4.3.1).
pub(crate) async fn create(client: &Client<ReqwestTransport>, entity: &Entity<Input>) {
    let outcome = client.entities().create(entity).send().await.unwrap();
    assert!(outcome.is_created(), "{outcome:?}");
}

/// Creates `count` vehicles of an entity type unique to the calling test, with the speeds 10, 20, 30, ... and a
/// `brandName` and a `color` each; returns the type and the vehicles.
pub(crate) async fn create_fleet(client: &Client<ReqwestTransport>, label: &str, count: u32) -> (String, Vec<Entity<Input>>) {
    let vehicle_type = unique(label);
    let mut vehicles = Vec::new();
    for number in 1..=count {
        let id = format!("urn:ngsi-ld:Vehicle:{vehicle_type}n{number}");
        let vehicle = Entity::builder()
            .id(id.as_str())
            .entity_type(vehicle_type.as_str())
            .property("brandName", format!("Brand{number}"))
            .property("color", "red")
            .property("speed", 10 * number)
            .build()
            .unwrap();
        create(client, &vehicle).await;
        vehicles.push(vehicle);
    }
    (vehicle_type, vehicles)
}

/// Deletes every entity of `entities` that is still there.
pub(crate) async fn remove_all(client: &Client<ReqwestTransport>, entities: &[Entity<Input>]) {
    for entity in entities {
        remove(client, &entity.id).await;
    }
}

/// Deletes the entity `id` if it is still there, so a test leaves nothing behind.
pub(crate) async fn remove(client: &Client<ReqwestTransport>, id: &EntityId) {
    match client.entities().delete(id).send().await {
        Ok(_) => {}
        Err(error) => assert_eq!(error.error_type(), Some(&ErrorType::ResourceNotFound), "{error:?}"),
    }
}

/// Whether a create outcome names a readable location; clause 6.4.3.1 requires one.
pub(crate) const fn has_location(outcome: &CreateOutcome) -> bool {
    match outcome {
        CreateOutcome::Created { location } | CreateOutcome::PartiallyCreated { location, .. } => location.is_valid(),
    }
}
