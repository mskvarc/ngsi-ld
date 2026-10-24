//! An entity whose inline join is 2000 levels deep (clause 4.5.23.2, `joinLevel`), and entities whose Property value
//! (clause 3.1) or `JsonProperty` json (clause 4.5.24) nests 300 000 levels deep: reading, cloning, comparing,
//! debug-printing, writing and dropping them fit the stack of a test thread.

use ngsi_ld::model::{Entity, Input, Lossless, Output, Representation, SimplifiedEntity};

/// How deep the chain of linked entities goes.
const DEPTH: usize = 2000;

/// How many times the payload nests `{"a": [{"b": ...}]}`, each three JSON levels deep.
const PAYLOAD_DEPTH: usize = 100_000;

/// A chain of `depth` entities, each linking the next through `next`, the innermost linking nothing, in normalized
/// form with every linked entity inline.
fn chain(depth: usize) -> String {
    let opening = (0..depth).map(|level| {
        format!(
            r#"{{"id":"urn:ngsi-ld:Node:{level}","type":"Node","next":{{"type":"Relationship","object":"urn:ngsi-ld:Node:{}","entity":"#,
            level + 1
        )
    });
    let innermost = format!(r#"{{"id":"urn:ngsi-ld:Node:{depth}","type":"Node"}}"#);
    let closing = (0..depth).map(|_| "}}".to_owned());
    opening.chain([innermost]).chain(closing).collect()
}

#[test]
fn an_inline_join_2000_levels_deep_reads_clones_compares_writes_and_drops() {
    let text = chain(DEPTH);
    let entity = Entity::<Output>::parse(&text, Lossless::Normalized).unwrap();
    let copy = entity.clone();
    assert_eq!(copy, entity);
    assert_eq!(entity.render(Representation::Normalized).to_string(), text);
    drop(copy);
    drop(entity);
}

#[test]
fn an_inline_join_2000_levels_deep_reads_and_writes_in_concise_form() {
    let text = chain(DEPTH);
    let entity = Entity::<Output>::parse(&text, Lossless::Concise).unwrap();
    let concise = entity.render(Representation::Concise).to_string();
    assert_eq!(Entity::<Output>::parse(&concise, Lossless::Concise).unwrap(), entity);
}

/// An entity in normalized form with one attribute `deep` of `kind`, whose `member` nests `{"a": [{"b": ...}]}`
/// `PAYLOAD_DEPTH` times around the number 1.
fn deep_payload(kind: &str, member: &str) -> String {
    let opening = r#"{"a":[{"b":"#.repeat(PAYLOAD_DEPTH);
    let closing = "}]}".repeat(PAYLOAD_DEPTH);
    format!(r#"{{"id":"urn:ngsi-ld:Deep:1","type":"Deep","deep":{{"type":"{kind}","{member}":{opening}1{closing}}}}}"#)
}

/// Reads `text` as a response and a request, clones, compares, debug-prints and writes it in every representation,
/// reads the concise and simplified forms back, and drops everything.
fn walk_every_way(text: &str) {
    let entity = Entity::<Output>::parse(text, Lossless::Normalized).unwrap();
    let copy = entity.clone();
    assert_eq!(copy, entity);
    assert!(format!("{copy:?}").starts_with("Entity"));
    assert_eq!(entity.render(Representation::Normalized).to_string(), text);
    let concise = entity.render(Representation::Concise).to_string();
    assert_eq!(Entity::<Output>::parse(&concise, Lossless::Concise).unwrap(), entity);
    let simplified = SimplifiedEntity::parse(&entity.render(Representation::Simplified).to_string()).unwrap();
    assert_eq!(simplified.clone(), simplified);
    let request = Entity::<Input>::parse(text, Lossless::Normalized).unwrap();
    assert_eq!(request.render(Representation::Normalized).to_string(), text);
    drop(copy);
    drop(entity);
}

#[test]
fn a_property_value_300_000_levels_deep_reads_clones_compares_writes_and_drops() {
    walk_every_way(&deep_payload("Property", "value"));
}

#[test]
fn a_json_property_300_000_levels_deep_reads_clones_compares_writes_and_drops() {
    walk_every_way(&deep_payload("JsonProperty", "json"));
}

#[test]
fn a_simplified_dataset_nested_200_000_levels_deep_is_refused_without_reading_the_inner_ones() {
    let opening = r#"{"dataset":{"@none":"#.repeat(PAYLOAD_DEPTH);
    let closing = "}}".repeat(PAYLOAD_DEPTH);
    let text = format!(r#"{{"id":"urn:ngsi-ld:Deep:1","type":"Deep","deep":{opening}1{closing}}}"#);
    assert!(SimplifiedEntity::parse(&text).is_err());
}
