//! Create or Update Temporal Evolution with the evolution in simplified temporal representation (clauses 5.2.20
//! and 6.18.3.1).

use crate::{
    fixtures::endpoint_with,
    temporal_fixtures::{T1, T2, b, body_text, time},
};
use http::header;
use ngsi_ld::{
    http::{
        ContextDelivery,
        operation::{CreateOrUpdateSimplifiedTemporalEvolution, Operation},
    },
    model::{
        Value,
        evolution::{SimplifiedTemporalInput, TemporalValue, TemporalValues},
    },
};
use pretty_assertions::assert_eq;

#[test]
fn a_simplified_temporal_evolution_is_posted_as_written() {
    let speed = TemporalValues::Property(vec![
        TemporalValue {
            value: Value::from(120),
            time: time(T1),
        },
        TemporalValue {
            value: Value::from(80),
            time: time(T2),
        },
    ]);
    let entity = SimplifiedTemporalInput::builder()
        .id(b())
        .entity_type("Vehicle")
        .values("speed", speed)
        .build()
        .unwrap();
    let create = CreateOrUpdateSimplifiedTemporalEvolution { entity: &entity };
    let linked = create.to_request(&endpoint_with(ContextDelivery::JsonWithLink)).unwrap();
    assert_eq!(linked.uri().path(), "/ngsi-ld/v1/temporal/entities");
    assert_eq!(linked.headers()[&header::CONTENT_TYPE], "application/json");
    assert_eq!(
        body_text(&linked),
        r#"{"id":"urn:ngsi-ld:Vehicle:B9211","type":"Vehicle","speed":{"type":"Property","values":[[120,"2018-08-01T12:03:00Z"],[80,"2018-08-01T12:05:00Z"]]}}"#
    );
    let embedded = create.to_request(&endpoint_with(ContextDelivery::LdJsonBody)).unwrap();
    assert!(body_text(&embedded).ends_with(r#""@context":"https://example.org/context.jsonld"}"#));
}
