//! Snapshots whose endpoint is written as the caller gave it.

use crate::{
    model::{Input, UrlDraft},
    query::EntitySelector,
    snapshot::{QueryDocument, Snapshot, SnapshotClone, SnapshotFragment},
};
use pretty_assertions::assert_eq;

/// A snapshot of every vehicle, sent to `endpoint`.
fn snapshot(endpoint: UrlDraft) -> Snapshot<Input> {
    let query = QueryDocument::builder()
        .entities([EntitySelector::new("Vehicle".parse().unwrap())])
        .build()
        .unwrap();
    Snapshot::builder().query(query).endpoint(endpoint).build().unwrap()
}

#[test]
fn a_snapshot_endpoint_given_verbatim_is_written_as_given() {
    let verbatim = snapshot(UrlDraft::verbatim("http://localhost:8080"));
    assert!(verbatim.render().to_string().contains(r#""endpoint":"http://localhost:8080""#));
    assert_eq!(verbatim.endpoint().map(url::Url::as_str), Some("http://localhost:8080/"));
    assert_eq!(verbatim, snapshot("http://localhost:8080".into()));
}

#[test]
fn a_snapshot_clone_and_fragment_endpoint_given_verbatim_are_written_as_given() {
    let clone = SnapshotClone::builder().endpoint(UrlDraft::verbatim("http://localhost:8080")).build().unwrap();
    assert_eq!(clone.render().to_string(), r#"{"type":"Snapshot","endpoint":"http://localhost:8080"}"#);
    let fragment = SnapshotFragment::builder()
        .endpoint(UrlDraft::verbatim("http://localhost:8080"))
        .build()
        .unwrap();
    assert_eq!(fragment.render().to_string(), r#"{"type":"Snapshot","endpoint":"http://localhost:8080"}"#);
}
