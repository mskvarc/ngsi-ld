use crate::{
    linked_data::{
        CompactionError,
        ExpansionError,
        NameMappingError,
        TranslationError,
        fixtures::{A, B, active, core},
    },
    model::{AttributesError, DocumentError, Entity, Input, Lossless, Output, discovery::EntityTypeInfo, evolution::TemporalEntity},
    registration::{CSourceRegistration, RegistrationFragment},
    snapshot::{EntityQuery, QueryDocument, Snapshot},
    subscription::{Notification, Subscription},
};
use pretty_assertions::assert_eq;

/// `text` with the `@context` `context` added as its last member.
fn with_context(text: &str, context: &str) -> String {
    format!("{},\"@context\":{context}}}", text.strip_suffix('}').unwrap())
}

/// The path of `error`, written `a.b.0`.
fn path<K>(error: &DocumentError<K>) -> String {
    error.path.to_string()
}

/// A subscription under `A` naming `Vehicle` and `speed` in every member that holds a name.
const SUBSCRIPTION: &str = r#"{"type":"Subscription","entities":[{"type":"Vehicle"}],"watchedAttributes":["speed"],"q":"speed>50","geoQ":{"georel":"near;maxDistance==2000","geometry":"Point","coordinates":[100,-1],"geoproperty":"speed"},"notification":{"attributes":["speed"],"endpoint":{"uri":"http://my.endpoint.org/notify"}}}"#;

/// Clause 5.5.7: every name of an entity is translated; a name the target has no term for takes its Fully
/// Qualified Name.
#[tokio::test]
async fn an_entity_takes_the_spelling_of_the_target_context_and_its_context() {
    let a = active(A).await;
    let b = active(B).await;
    let text = r#"{"id":"urn:ngsi-ld:Vehicle:A1","type":"Vehicle","speed":{"type":"Property","value":55,"ex:accuracy":{"type":"Property","value":0.9}},"towedBy":{"type":"Relationship","object":"urn:ngsi-ld:Vehicle:B2","objectType":"Vehicle"},"color":{"type":"VocabProperty","vocab":"ex:red"}}"#;
    let entity = Entity::<Input>::parse(text, Lossless::Normalized).unwrap();
    let expected = r#"{"id":"urn:ngsi-ld:Vehicle:A1","type":"Car","velocity":{"type":"Property","value":55,"https://example.org/accuracy":{"type":"Property","value":0.9}},"towedBy":{"type":"Relationship","object":"urn:ngsi-ld:Vehicle:B2","objectType":"Car"},"color":{"type":"VocabProperty","vocab":"https://example.org/red"}}"#;
    let expected = Entity::<Input>::parse(&with_context(expected, B), Lossless::Normalized).unwrap();
    assert_eq!(b.recontextualize(entity, &a).unwrap(), expected);
}

/// Clauses 4.9 and 5.2.12: a subscription's names, its query-language members included.
#[tokio::test]
async fn a_subscription_translates_its_selectors_attributes_and_queries() {
    let a = active(A).await;
    let b = active(B).await;
    let subscription = Subscription::<Input>::parse(SUBSCRIPTION).unwrap();
    let expected = SUBSCRIPTION.replace("Vehicle", "Car").replace("speed", "velocity");
    let expected = Subscription::<Input>::parse(&with_context(&expected, B)).unwrap();
    let translated = b.recontextualize(subscription, &a).unwrap();
    assert_eq!(translated.q.as_ref().map(ToString::to_string).as_deref(), Some("velocity>50"));
    assert_eq!(translated, expected);
    let picked =
        r#"{"type":"Subscription","entities":[{"type":"Vehicle"}],"notification":{"pick":["speed"],"endpoint":{"uri":"http://my.endpoint.org/notify"}}}"#;
    let translated = b.recontextualize(Subscription::<Input>::parse(picked).unwrap(), &a).unwrap();
    let expected = with_context(&picked.replace("Vehicle", "Car").replace("speed", "velocity"), B);
    assert_eq!(translated, Subscription::<Input>::parse(&expected).unwrap());
    // The core context has no term for `https://example.org/speed`, and a projection holds bare terms only.
    let core = core().await;
    let error = core.recontextualize(Subscription::<Input>::parse(picked).unwrap(), &a).unwrap_err();
    assert_eq!(path(&error), "notification.pick");
}

/// Clause 5.2.10: the entity types and attribute names of a registration's `information`.
#[tokio::test]
async fn a_registration_translates_its_information() {
    let a = active(A).await;
    let b = active(B).await;
    let text = r#"{"type":"ContextSourceRegistration","information":[{"entities":[{"type":"Vehicle"}],"propertyNames":["speed"]}],"endpoint":"http://my.csource.org:1026"}"#;
    let registration = CSourceRegistration::<Input>::parse(text).unwrap();
    let expected = with_context(&text.replace("Vehicle", "Car").replace("speed", "velocity"), B);
    assert_eq!(
        b.recontextualize(registration, &a).unwrap(),
        CSourceRegistration::<Input>::parse(&expected).unwrap()
    );
}

/// Clauses 5.2.23 and 5.2.41: a query document, and a snapshot through its queries.
#[tokio::test]
async fn a_query_document_and_a_snapshot_translate_their_queries() {
    let a = active(A).await;
    let b = active(B).await;
    let query = r#"{"type":"Query","entities":[{"type":"Vehicle"}],"q":"speed>50","pick":["speed"]}"#;
    let translated = b.recontextualize(QueryDocument::<EntityQuery>::parse(query).unwrap(), &a).unwrap();
    assert_eq!(
        translated,
        QueryDocument::<EntityQuery>::parse(&query.replace("Vehicle", "Car").replace("speed", "velocity")).unwrap()
    );
    let snapshot = format!(r#"{{"type":"Snapshot","snapshotQueries":[{query}]}}"#);
    let translated = b.recontextualize(Snapshot::<Input>::parse(&snapshot).unwrap(), &a).unwrap();
    let expected = with_context(&snapshot.replace("Vehicle", "Car").replace("speed", "velocity"), B);
    assert_eq!(translated, Snapshot::<Input>::parse(&expected).unwrap());
}

/// Clause 5.2.30: the names of a discovery document are translated, its `id`, an IRI, is kept.
#[tokio::test]
async fn an_entity_type_information_translates_its_names_and_keeps_its_ids() {
    let a = active(A).await;
    let b = active(B).await;
    let text = r#"{"id":"https://example.org/Vehicle","type":"EntityTypeInfo","attributeDetails":[{"id":"https://example.org/speed","type":"Attribute","attributeName":"speed","attributeTypes":["Property"]}],"entityCount":3,"typeName":"Vehicle"}"#;
    let expected = with_context(
        r#"{"id":"https://example.org/Vehicle","type":"EntityTypeInfo","attributeDetails":[{"id":"https://example.org/speed","type":"Attribute","attributeName":"velocity","attributeTypes":["Property"]}],"entityCount":3,"typeName":"Car"}"#,
        B,
    );
    assert_eq!(
        b.recontextualize(EntityTypeInfo::parse(text).unwrap(), &a).unwrap(),
        EntityTypeInfo::parse(&expected).unwrap()
    );
}

/// Clause 4.9: a name the target has no term for cannot be written in a query, and the error names the
/// member.
#[tokio::test]
async fn a_query_term_with_no_counterpart_is_refused_at_its_member() {
    let a = active(A).await;
    let core = core().await;
    let error = core.recontextualize(Subscription::<Input>::parse(SUBSCRIPTION).unwrap(), &a).unwrap_err();
    assert_eq!(path(&error), "q");
    assert!(matches!(
        error.kind,
        NameMappingError::Term(translation) if matches!(*translation, TranslationError::Compaction(CompactionError::NotExpressible { .. }))
    ));
}

/// Clause 5.3.1: the entities of a notification are translated, and an error names the entity and the
/// attribute.
#[tokio::test]
async fn a_notification_translates_its_entities_and_names_the_failing_one() {
    let a = active(A).await;
    let b = active(B).await;
    let text = r#"{"id":"urn:ngsi-ld:Notification:1","type":"Notification","subscriptionId":"urn:ngsi-ld:Subscription:1","notifiedAt":"2024-01-01T00:00:00Z","data":[{"id":"urn:ngsi-ld:Vehicle:A1","type":"Vehicle"},{"id":"urn:ngsi-ld:Vehicle:A2","type":"Vehicle","speed":{"type":"Property","value":55}}]}"#;
    let notification = Notification::<Vec<Entity<Output>>>::parse(text, Lossless::Normalized).unwrap();
    let expected = with_context(&text.replace("\"Vehicle\"", "\"Car\"").replace("speed", "velocity"), B);
    let expected = Notification::<Vec<Entity<Output>>>::parse(&expected, Lossless::Normalized).unwrap();
    assert_eq!(b.recontextualize(notification.clone(), &a).unwrap(), expected);
    // Under a context that aliases `speed` to a keyword, the attribute names no Fully Qualified Name.
    let aliased = active(r#"{"Vehicle":"https://example.org/Vehicle","speed":"@id"}"#).await;
    let error = b.recontextualize(notification, &aliased).unwrap_err();
    assert_eq!(path(&error), "data.1.speed");
    assert!(matches!(
        error.kind,
        NameMappingError::Attribute(translation) if matches!(*translation, TranslationError::Expansion(ExpansionError::Keyword { .. }))
    ));
}

/// A context spelling one Fully Qualified Name both `speed` and `velocity`.
const TWO_SPELLINGS: &str = r#"{"Vehicle":"https://example.org/Vehicle","speed":"https://example.org/speed","velocity":"https://example.org/speed"}"#;

/// Clause 4.5.1: two attributes whose names translate to one term are refused at the second.
#[tokio::test]
async fn two_attributes_translated_onto_one_name_are_refused() {
    let from = active(TWO_SPELLINGS).await;
    let b = active(B).await;
    let text = r#"{"id":"urn:ngsi-ld:Vehicle:A1","type":"Vehicle","speed":{"type":"Property","value":55},"velocity":{"type":"Property","value":56}}"#;
    let error = b
        .recontextualize(Entity::<Input>::parse(text, Lossless::Normalized).unwrap(), &from)
        .unwrap_err();
    assert_eq!(path(&error), "velocity");
    assert!(matches!(error.kind, NameMappingError::Attributes(AttributesError::NameTaken { name }) if name.as_ref() == "velocity"));
}

/// Clause 4.5.1: two temporal evolutions whose names translate to one term are refused at the second.
#[tokio::test]
async fn two_temporal_evolutions_translated_onto_one_name_are_refused() {
    let from = active(TWO_SPELLINGS).await;
    let b = active(B).await;
    let text = r#"{"id":"urn:ngsi-ld:Vehicle:A1","type":"Vehicle","speed":[{"type":"Property","value":55,"observedAt":"2024-01-01T00:00:00Z"}],"velocity":[{"type":"Property","value":56,"observedAt":"2024-01-01T00:00:00Z"}]}"#;
    let error = b
        .recontextualize(TemporalEntity::<Input>::parse(text, Lossless::Normalized).unwrap(), &from)
        .unwrap_err();
    assert_eq!(path(&error), "velocity");
    assert!(matches!(error.kind, NameMappingError::Attributes(AttributesError::NameTaken { name }) if name.as_ref() == "velocity"));
}

/// A registration under `A` whose Context Source Properties are `speed`, `csourceProperty1` and `ex:brand`; the
/// value of `speed` holds a key `speed`, which is data.
const PROPERTIES: &str = r#"{"type":"ContextSourceRegistration","information":[{"propertyNames":["speed"]}],"endpoint":"http://my.csource.org:1026/","speed":{"speed":1},"csourceProperty1":"aValue","ex:brand":["Mercedes"]}"#;

/// Clauses 4.9 and 5.5.7: the names of a registration's Context Source Properties are translated as
/// attribute names, in order, and their values are kept as written.
#[tokio::test]
async fn a_registration_translates_the_names_of_its_properties_and_keeps_their_values() {
    let a = active(A).await;
    let b = active(B).await;
    let registration = CSourceRegistration::<Input>::parse(PROPERTIES).unwrap();
    let translated = b.recontextualize(registration, &a).unwrap();
    let expected = r#"{"type":"ContextSourceRegistration","information":[{"propertyNames":["velocity"]}],"endpoint":"http://my.csource.org:1026/","velocity":{"speed":1},"csourceProperty1":"aValue","https://example.org/brand":["Mercedes"]}"#;
    assert_eq!(translated, CSourceRegistration::<Input>::parse(&with_context(expected, B)).unwrap());
    let names: Vec<&str> = translated.properties().keys().map(AsRef::as_ref).collect();
    assert_eq!(names, ["velocity", "csourceProperty1", "https://example.org/brand"]);
}

/// Clauses 4.9 and 5.2.9: a property whose name translates onto another property's, or onto a member of
/// the registration data type, is refused at its name.
#[tokio::test]
async fn a_property_translated_onto_a_taken_name_is_refused() {
    let from = active(TWO_SPELLINGS).await;
    let b = active(B).await;
    let two =
        r#"{"type":"ContextSourceRegistration","information":[{"propertyNames":["speed"]}],"endpoint":"http://my.csource.org:1026/","speed":1,"velocity":2}"#;
    let error = b.recontextualize(CSourceRegistration::<Input>::parse(two).unwrap(), &from).unwrap_err();
    assert_eq!(path(&error), "velocity");
    assert!(matches!(error.kind, NameMappingError::Attributes(AttributesError::NameTaken { name }) if name.as_ref() == "velocity"));
    let aliased = active(r#"{"origin":"https://uri.etsi.org/ngsi-ld/endpoint"}"#).await;
    let core = core().await;
    let member = r#"{"type":"ContextSourceRegistration","information":[{"propertyNames":["speed"]}],"endpoint":"http://my.csource.org:1026/","origin":"x"}"#;
    let error = core
        .recontextualize(CSourceRegistration::<Input>::parse(member).unwrap(), &aliased)
        .unwrap_err();
    assert_eq!(path(&error), "origin");
    assert!(matches!(error.kind, NameMappingError::Attributes(AttributesError::NameTaken { name }) if name.as_ref() == "endpoint"));
}

/// Clauses 4.9 and 5.4: a fragment translates the properties it sets and deletes.
#[tokio::test]
async fn a_fragment_translates_the_properties_it_sets_and_deletes() {
    let a = active(A).await;
    let b = active(B).await;
    let fragment = RegistrationFragment::builder()
        .property("speed", 3)
        .delete_property("csourceProperty1")
        .build()
        .unwrap();
    let translated = b.recontextualize(fragment, &a).unwrap();
    assert_eq!(
        translated.render().to_string(),
        r#"{"type":"ContextSourceRegistration","velocity":3,"csourceProperty1":"urn:ngsi-ld:null"}"#
    );
}
