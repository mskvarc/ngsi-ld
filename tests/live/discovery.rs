//! Entity type and attribute discovery (clauses 6.25 to 6.28) and the stored `@context`s (clauses 6.29 and 6.30).
//!
//! The discovery tests work in a tenant of their own, so the types and attributes they find are exactly those of
//! the entities they create, whatever the other tests create meanwhile.

use crate::broker::{client, name, unique};
use ngsi_ld::{
    client::{Client, ReqwestTransport},
    http::operation::ContextKindFilter,
    model::{AttributeKind, Context, ContextEntry, ContextId, Entity, EntityId, EntityTypeName, ErrorType, InlineContext, Input, StoredContextKind, Tenant},
};
use percent_encoding::percent_decode_str;
use pretty_assertions::assert_eq;

/// A tenant unique to the calling test.
fn tenant(label: &str) -> Tenant {
    unique(label).parse().unwrap()
}

/// An entity `number` of `entity_type` with a Property `50` under each of `properties` and a Relationship under
/// each of `relationships`.
fn entity(entity_type: &str, number: u32, properties: &[&str], relationships: &[&str]) -> Entity<Input> {
    let mut builder = Entity::builder()
        .id(format!("urn:ngsi-ld:Vehicle:{entity_type}n{number}").as_str())
        .entity_type(entity_type);
    for property in properties {
        builder = builder.property(*property, 50);
    }
    for relationship in relationships {
        builder = builder.relationship(*relationship, "urn:ngsi-ld:Person:Alice".parse::<EntityId>().unwrap());
    }
    builder.build().unwrap()
}

/// Creates `entities` on `tenant`.
async fn create_on(client: &Client<ReqwestTransport>, tenant: &Tenant, entities: &[Entity<Input>]) {
    for entity in entities {
        let outcome = client.entities().create(entity).tenant(tenant).send().await.unwrap();
        assert!(outcome.is_created(), "{outcome:?}");
    }
}

/// Deletes `entities` from `tenant`.
async fn remove_from(client: &Client<ReqwestTransport>, tenant: &Tenant, entities: &[Entity<Input>]) {
    for entity in entities {
        if let Err(error) = client.entities().delete(&entity.id).tenant(tenant).send().await {
            assert_eq!(error.error_type(), Some(&ErrorType::ResourceNotFound), "{error:?}");
        }
    }
}

/// `names`, sorted: a broker may list names in any order.
fn sorted<T: AsRef<str>>(names: &[T]) -> Vec<&str> {
    let mut names: Vec<&str> = names.iter().map(AsRef::as_ref).collect();
    names.sort_unstable();
    names
}

/// A context defining one term unique to the calling test.
fn hosted_context(label: &str) -> Context {
    let term = unique(label);
    let inline: InlineContext = format!(r#"{{"{term}":"https://example.org/{term}"}}"#).parse().unwrap();
    Context::Single(ContextEntry::Inline(inline))
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn the_types_and_attributes_of_created_entities_are_discovered() {
    let client = client();
    let tenant = tenant("discovery");
    let (vehicle, bus) = ("DiscoveredVehicle", "DiscoveredBus");
    let entities = [
        entity(vehicle, 1, &["speed"], &["owner"]),
        entity(vehicle, 2, &["speed"], &["owner"]),
        entity(bus, 1, &["speed", "seats"], &[]),
    ];
    create_on(&client, &tenant, &entities).await;
    let vehicle_type: EntityTypeName = vehicle.parse().unwrap();
    let types = client.entity_types().list().tenant(&tenant).send().await;
    let type_details = client.entity_types().list_details().tenant(&tenant).send().await;
    let type_info = client.entity_types().retrieve(&vehicle_type).tenant(&tenant).send().await;
    let attributes = client.attributes().list().tenant(&tenant).send().await;
    let attribute_details = client.attributes().list_details().tenant(&tenant).send().await;
    let attribute_info = client.attributes().retrieve(&name("speed")).tenant(&tenant).send().await;
    remove_from(&client, &tenant, &entities).await;

    assert_eq!(sorted(&types.unwrap().type_list), [bus, vehicle]);
    let type_details = type_details.unwrap();
    let names: Vec<(&str, Vec<&str>)> = {
        let mut names: Vec<_> = type_details
            .iter()
            .map(|details| (details.type_name.as_ref(), sorted(&details.attribute_names)))
            .collect();
        names.sort_unstable();
        names
    };
    assert_eq!(names, [(bus, vec!["seats", "speed"]), (vehicle, vec!["owner", "speed"])]);

    let info = type_info.unwrap();
    assert_eq!(info.type_name, vehicle_type);
    assert_eq!(info.entity_count, 2);
    let mut kinds: Vec<(&str, Option<Vec<AttributeKind>>)> = info
        .attribute_details
        .iter()
        .map(|details| (details.attribute_name.as_ref(), details.attribute_types.clone()))
        .collect();
    kinds.sort_unstable_by_key(|(name, _)| *name);
    assert_eq!(
        kinds,
        [
            ("owner", Some(vec![AttributeKind::Relationship])),
            ("speed", Some(vec![AttributeKind::Property]))
        ]
    );

    assert_eq!(sorted(&attributes.unwrap().attribute_list), ["owner", "seats", "speed"]);
    let attribute_details = attribute_details.unwrap();
    let speed = attribute_details.iter().find(|details| details.attribute_name == name("speed")).unwrap();
    assert_eq!(speed.type_names.as_deref().map(sorted), Some(vec![bus, vehicle]));

    let info = attribute_info.unwrap();
    assert_eq!(info.attribute_name, name("speed"));
    assert_eq!(info.attribute_count, Some(3));
    assert_eq!(info.attribute_types, Some(vec![AttributeKind::Property]));
    assert_eq!(info.type_names.as_deref().map(sorted), Some(vec![bus, vehicle]));
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn a_single_entity_type_is_listed_in_detail_as_an_array() {
    let client = client();
    let tenant = tenant("singletype");
    let entities = [entity("LoneVehicle", 1, &["speed"], &[])];
    create_on(&client, &tenant, &entities).await;
    let type_details = client.entity_types().list_details().tenant(&tenant).send().await;
    remove_from(&client, &tenant, &entities).await;

    // Table 6.25.3.1-2: the detailed entity type list is an array of EntityType (clause 5.2.25), whose
    // attributeNames is an array (Table 5.2.25-1), however many elements each holds. Observed: a single EntityType
    // is answered as a bare object, and a single attribute name as a bare string.
    let type_details = type_details.unwrap();
    assert_eq!(type_details.len(), 1);
    assert_eq!(type_details[0].type_name.as_ref(), "LoneVehicle");
    assert_eq!(sorted(&type_details[0].attribute_names), ["speed"]);
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn an_unknown_type_or_attribute_is_resource_not_found() {
    let client = client();
    let unknown_type: EntityTypeName = unique("UnknownType").parse().unwrap();
    let type_error = client.entity_types().retrieve(&unknown_type).send().await.unwrap_err();
    let attribute_error = client.attributes().retrieve(&name(&unique("unknownAttribute"))).send().await.unwrap_err();

    // Tables 6.26.3.1-1 and 6.28.3.1-1: 404 ResourceNotFound.
    assert_eq!(type_error.error_type(), Some(&ErrorType::ResourceNotFound), "{type_error:?}");
    assert_eq!(attribute_error.error_type(), Some(&ErrorType::ResourceNotFound), "{attribute_error:?}");
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn a_context_is_added_listed_served_and_deleted() {
    let client = client();
    let context = hosted_context("hostedTerm");
    let created = client.contexts().add(&context).send().await.unwrap();
    // Table 6.29.3.1-1: the 201 names the stored context in its Location.
    let id = created.id().expect("the 201 names the stored context in its Location");
    let hosted = client.contexts().list().kind(ContextKindFilter::Hosted).send().await;
    let served = client.contexts().serve(&id).send().await;
    let deleted = client.contexts().delete(&id).send().await;
    let after_delete = client.contexts().serve(&id).send().await;

    // Clause 5.13.3.1: each listed URL downloads its context; the URL of a hosted context names it by its local
    // identifier, the last segment of the path.
    let named: Vec<ContextId> = hosted
        .unwrap()
        .iter()
        .filter_map(|url| url.path_segments()?.next_back())
        .filter_map(|segment| percent_decode_str(segment).decode_utf8().ok()?.parse().ok())
        .collect();
    assert!(named.contains(&id), "{named:?}");
    assert_eq!(served.unwrap(), context);
    deleted.unwrap();
    let error = after_delete.unwrap_err();
    assert_eq!(error.error_type(), Some(&ErrorType::ResourceNotFound), "{error:?}");
}

#[tokio::test]
#[ignore = "needs a live broker: set NGSI_LD_TEST_BROKER_URL"]
async fn the_metadata_of_a_hosted_context_is_listed_and_served() {
    let client = client();
    let context = hosted_context("describedTerm");
    let id = client.contexts().add(&context).send().await.unwrap().id().unwrap();
    let served = client.contexts().serve_details(&id).send().await;
    let listed = client.contexts().list_details().kind(ContextKindFilter::Hosted).send().await;
    client.contexts().delete(&id).send().await.unwrap();

    // Clause 5.13.3.5 lists the members of the metadata: URL, localId, kind, createdAt, expiresAt, lastUsage,
    // numberOfHits and extraInfo. Observed: the metadata also carries the context itself as `body`, and the list
    // holds a hosted context whose `localId` is `)$%^&`, which is no URI (clause 5.13.2.4).
    let served = served.unwrap();
    assert_eq!((served.local_id.clone(), served.kind), (id.clone(), StoredContextKind::Hosted));
    let listed = listed.unwrap();
    assert!(listed.iter().any(|metadata| metadata.local_id == id), "{listed:?}");
}
