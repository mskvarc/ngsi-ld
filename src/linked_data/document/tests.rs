use crate::{
    linked_data::{
        document::shape::{ARRAY, Shape},
        fixtures::{A, B, DEFAULT_CONTEXT, active, core, inline},
    },
    model::{Entity, Input, Lossless, name::mapping::CarriesContext},
    registration::CSourceRegistration,
    seal::Seal,
    subscription::Subscription,
};
use jstrict::{Print, Value as Json};
use pretty_assertions::assert_eq;

/// A vehicle under `A`, normalized.
const VEHICLE: &str = r#"{"id":"urn:ngsi-ld:Vehicle:A1","type":"Vehicle","speed":{"type":"Property","value":55}}"#;

/// The JSON text `text`, as a value whose objects compare without regard to member order.
fn json(text: &str) -> serde_json::Value {
    serde_json::from_str(text).unwrap()
}

/// The entity `text` reads as in `form`, with the `@context` `context` when one is given.
fn entity(text: &str, form: Lossless, context: Option<&str>) -> Entity<Input> {
    let mut entity = Entity::<Input>::parse(text, form).unwrap();
    entity.context = context.map(inline);
    entity
}

/// Clause 4.5.0: an entity expands to the Fully Qualified Names of its type, its attribute and the core
/// members.
#[tokio::test]
async fn an_entity_expands_its_type_its_attributes_and_the_core_members() {
    let a = active(A).await;
    let expanded = a.expand_document(&entity(VEHICLE, Lossless::Normalized, None)).await.unwrap();
    assert_eq!(
        json(&expanded.render().to_string()),
        json(
            r#"[{
                "@id": "urn:ngsi-ld:Vehicle:A1",
                "@type": ["https://example.org/Vehicle"],
                "https://example.org/speed": [{
                    "@type": ["https://uri.etsi.org/ngsi-ld/Property"],
                    "https://uri.etsi.org/ngsi-ld/hasValue": [{"@value": 55}]
                }]
            }]"#
        )
    );
}

/// Clause 5.5.7: compacted under the context it was expanded under, an entity comes back with that context.
#[tokio::test]
async fn an_expanded_entity_compacts_back_under_the_same_context() {
    let a = active(A).await;
    let expanded = a.expand_document(&entity(VEHICLE, Lossless::Normalized, None)).await.unwrap();
    let compacted: Entity<Input> = a.compact_document(&expanded).await.unwrap();
    assert_eq!(compacted, entity(VEHICLE, Lossless::Normalized, Some(A)));
}

/// Clause 5.5.7: compacted under another context, the same names take that context's spelling.
#[tokio::test]
async fn an_expanded_entity_compacts_under_another_context_with_its_spelling() {
    let a = active(A).await;
    let b = active(B).await;
    let expanded = a.expand_document(&entity(VEHICLE, Lossless::Normalized, None)).await.unwrap();
    let compacted: Entity<Input> = b.compact_document(&expanded).await.unwrap();
    let expected = r#"{"id":"urn:ngsi-ld:Vehicle:A1","type":"Car","velocity":{"type":"Property","value":55}}"#;
    assert_eq!(compacted, entity(expected, Lossless::Normalized, Some(B)));
}

/// Clause 4.5.7: an entity read concise expands exactly as the same entity read normalized; the
/// document's own `@context` is not the one applied.
#[tokio::test]
async fn the_concise_and_normalized_forms_of_an_entity_expand_alike() {
    let core = core().await;
    let concise = entity(include_str!("../../../tests/golden/authored/vehicle-concise.json"), Lossless::Concise, None);
    let normalized = entity(
        include_str!("../../../tests/golden/authored/vehicle-normalized.json"),
        Lossless::Normalized,
        None,
    );
    assert_eq!(core.expand_document(&concise).await.unwrap(), core.expand_document(&normalized).await.unwrap());
}

/// Clauses 4.5.0 and 5.5.7: the keys of a `JsonProperty`'s `json` are never expanded.
#[tokio::test]
async fn the_keys_of_a_json_property_survive_a_change_of_context() {
    let a = active(A).await;
    let b = active(B).await;
    let text = r#"{"id":"urn:ngsi-ld:Vehicle:A1","type":"Vehicle","config":{"type":"JsonProperty","json":{"speed":1}}}"#;
    let expanded = a.expand_document(&entity(text, Lossless::Normalized, None)).await.unwrap();
    let compacted: Entity<Input> = b.compact_document(&expanded).await.unwrap();
    let expected = r#"{"id":"urn:ngsi-ld:Vehicle:A1","type":"Car","config":{"type":"JsonProperty","json":{"speed":1}}}"#;
    assert_eq!(compacted, entity(expected, Lossless::Normalized, Some(B)));
}

/// Clause 4.9: a subscription round-trips; its `q` is a string and comes through as written, and its
/// one-element arrays stay arrays.
#[tokio::test]
async fn a_subscription_round_trips_with_its_query_as_written() {
    let a = active(A).await;
    let text = r#"{"type":"Subscription","entities":[{"type":"Vehicle"}],"watchedAttributes":["speed"],"q":"speed>50","notification":{"endpoint":{"uri":"http://my.endpoint.org/notify"}}}"#;
    let subscription = Subscription::<Input>::parse(text).unwrap();
    let expanded = a.expand_document(&subscription).await.unwrap();
    let compacted: Subscription<Input> = a.compact_document(&expanded).await.unwrap();
    let mut expected = subscription;
    expected.context = Some(inline(A));
    assert_eq!(compacted, expected);
}

/// A shape wraps a bare value into an array, and reaches into objects and the elements of arrays.
#[test]
fn a_shape_restores_the_arrays_compaction_writes_bare() {
    const SHAPE: Shape = Shape::Object(&[("entities", Shape::Array(&Shape::Object(&[("names", ARRAY)]))), ("kept", Shape::Kept)]);
    let mut value: Json = r#"{"entities":{"names":"speed"},"kept":"x","other":"y"}"#.parse().unwrap();
    SHAPE.restore(&mut value);
    assert_eq!(
        json(&value.compact_print().to_string()),
        json(r#"{"entities":[{"names":["speed"]}],"kept":"x","other":"y"}"#)
    );
}

/// JSON-LD 1.1 API, compaction step 12.8.2: a one-element array where the data type allows one value or an
/// array comes back as its element; the two documents are the same JSON-LD.
#[tokio::test]
async fn a_one_element_array_comes_back_as_its_element() {
    let a = active(A).await;
    let arrays = r#"{"id":"urn:ngsi-ld:Vehicle:A1","type":["Vehicle"],"scope":["/Madrid"],"speed":{"type":"Property","value":[7]},"owner":{"type":"Relationship","object":["urn:ngsi-ld:Person:P1"]},"color":{"type":"VocabProperty","vocab":["ex:red"]}}"#;
    let bare = r#"{"id":"urn:ngsi-ld:Vehicle:A1","type":"Vehicle","scope":"/Madrid","speed":{"type":"Property","value":7},"owner":{"type":"Relationship","object":"urn:ngsi-ld:Person:P1"},"color":{"type":"VocabProperty","vocab":"ex:red"}}"#;
    let with_arrays = entity(arrays, Lossless::Normalized, None);
    let expanded = a.expand_document(&with_arrays).await.unwrap();
    let compacted: Entity<Input> = a.compact_document(&expanded).await.unwrap();
    assert_eq!(compacted, entity(bare, Lossless::Normalized, Some(A)));
    assert_ne!(compacted, entity(arrays, Lossless::Normalized, Some(A)));
    assert_eq!(expanded, a.expand_document(&entity(bare, Lossless::Normalized, None)).await.unwrap());
}

/// Clauses 4.9 and 5.5.7: a registration's Context Source Properties expand by name as attributes do,
/// compact under another context with its spelling, and come back as compaction writes their values, a one-element
/// array as its element, as for the value of a Property.
#[tokio::test]
async fn the_properties_of_a_registration_expand_and_compact_as_attributes() {
    let a = active(A).await;
    let b = active(B).await;
    let text = r#"{"type":"ContextSourceRegistration","information":[{"propertyNames":["speed"]}],"endpoint":"http://my.csource.org:1026/","speed":55,"csourceProperty1":"aValue","ex:brand":["Mercedes"],"details":{"note":"x"}}"#;
    let registration = CSourceRegistration::<Input>::parse(text).unwrap();
    let expanded = a.expand_document(&registration).await.unwrap();
    let rendered = json(&expanded.render().to_string());
    let node = &rendered[0];
    assert_eq!(node["https://example.org/speed"], json(r#"[{"@value":55}]"#));
    assert_eq!(node[format!("{DEFAULT_CONTEXT}csourceProperty1").as_str()], json(r#"[{"@value":"aValue"}]"#));
    assert_eq!(node["https://example.org/brand"], json(r#"[{"@value":"Mercedes"}]"#));
    let compacted: CSourceRegistration<Input> = b.compact_document(&expanded).await.unwrap();
    let expected = r#"{"type":"ContextSourceRegistration","information":[{"propertyNames":["velocity"]}],"endpoint":"http://my.csource.org:1026/","velocity":55,"csourceProperty1":"aValue","https://example.org/brand":"Mercedes","details":{"note":"x"}}"#;
    let mut expected = CSourceRegistration::<Input>::parse(expected).unwrap();
    expected.set_context(Seal::new(), Some(inline(B)));
    assert_eq!(compacted, expected);
    let back: CSourceRegistration<Input> = a.compact_document(&expanded).await.unwrap();
    assert_eq!(back.properties().len(), 4);
}
