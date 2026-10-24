use crate::{
    linked_data::{
        CompactionError,
        ExpansionError,
        TranslationError,
        fixtures::{A, B, DEFAULT_CONTEXT, active, core},
    },
    model::{AttributeName, EntityTypeName, Term, Uri, VocabTerm},
};
use pretty_assertions::assert_eq;

/// The URI `text`.
fn uri(text: &str) -> Uri {
    text.parse().unwrap()
}

/// Clause 5.5.7: compaction picks the context's term, and falls back to the Fully Qualified Name.
#[tokio::test]
async fn compaction_picks_the_term_and_falls_back_to_the_fully_qualified_name() {
    let a = active(A).await;
    let core = core().await;
    let vehicle = uri("https://example.org/Vehicle");
    assert_eq!(a.compact::<EntityTypeName>(&vehicle).unwrap().to_string(), "Vehicle");
    assert_eq!(core.compact::<EntityTypeName>(&vehicle).unwrap().to_string(), "https://example.org/Vehicle");
    let speed = uri(&format!("{DEFAULT_CONTEXT}speed"));
    assert_eq!(core.compact::<AttributeName>(&speed).unwrap().to_string(), "speed");
}

/// Clause 5.5.7: two spellings of one Fully Qualified Name, and the translation between them.
#[tokio::test]
async fn two_contexts_spell_one_name_differently_and_translate_into_each_other() {
    let a = active(A).await;
    let b = active(B).await;
    let speed: AttributeName = "speed".parse().unwrap();
    let velocity: AttributeName = "velocity".parse().unwrap();
    assert_eq!(a.expand(&speed).unwrap(), b.expand(&velocity).unwrap());
    assert_eq!(a.translate(&speed, &b).unwrap(), velocity);
    let vehicle: EntityTypeName = "Vehicle".parse().unwrap();
    assert_eq!(a.translate(&vehicle, &b).unwrap().to_string(), "Car");
}

/// Clauses 4.9 and 5.5.7: a query-language term holds neither an IRI nor a prefix.
#[tokio::test]
async fn a_term_with_no_unprefixed_counterpart_is_not_expressible() {
    let a = active(A).await;
    let core = core().await;
    let speed: Term = "speed".parse().unwrap();
    let error = a.translate(&speed, &core).unwrap_err();
    assert!(matches!(error, TranslationError::Compaction(CompactionError::NotExpressible { iri }) if iri == uri("https://example.org/speed")));
    // `https://example.org/color` compacts under `A` to `ex:color` through the prefix, which a term cannot hold.
    let color = uri("https://example.org/color");
    assert!(matches!(a.compact::<Term>(&color), Err(CompactionError::NotExpressible { .. })));
    assert_eq!(a.compact::<AttributeName>(&color).unwrap().to_string(), "ex:color");
}

/// Clause 5.5.7: a vocabulary term expands and compacts back through the prefix.
#[tokio::test]
async fn a_vocabulary_term_round_trips_through_its_prefix() {
    let a = active(A).await;
    let red: VocabTerm = "ex:red".parse().unwrap();
    let expanded = a.expand(&red).unwrap();
    assert_eq!(expanded, uri("https://example.org/red"));
    assert_eq!(a.compact::<VocabTerm>(&expanded).unwrap(), red);
}

/// A name that is a keyword alias does not expand to a Fully Qualified Name.
#[tokio::test]
async fn a_keyword_alias_does_not_expand() {
    let aliased = active(r#"{"kind":"@type"}"#).await;
    let kind: AttributeName = "kind".parse().unwrap();
    assert_eq!(aliased.expand(&kind), Err(ExpansionError::Keyword { term: kind }));
}
