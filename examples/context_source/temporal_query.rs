//! The forwarded Query Temporal Evolution through POST, whose selection and temporal query are in the body.

use crate::forwarding::{forwarded, query_string};
use headers::HeaderMapExt;
use http::Method;
use ngsi_ld::{
    http::{
        ResponseContext,
        answer,
        headers::{Link, NgsiLdResultsCount},
        operation::{Operation, QueryMethod, QueryTemporalEvolution, QueryTemporalEvolutionPostParameters},
    },
    model::{
        AttributeName,
        DateTime,
        Entity,
        Input,
        Lossless,
        Property,
        evolution::{Evolution, TemporalEntity},
    },
    query::EntitySelector,
    snapshot::{QueryDocument, TemporalEvolutionQuery},
};
use std::str;

/// The `Query` body of the forwarded temporal query, with its `temporalQ`.
const TEMPORAL_BODY: &str =
    r#"{"type": "Query", "entities": [{"type": "Vehicle"}], "temporalQ": {"timerel": "after", "timeAt": "2024-05-01T10:00:00Z", "lastN": 2}}"#;

/// Reads and answers a forwarded `POST /temporal/entityOperations/query?limit=5&count=true`, whose selection and
/// temporal query are in the body.
pub(crate) fn temporal_query(vehicle: &Entity<Input>) -> anyhow::Result<()> {
    let request = forwarded(Method::POST, "/temporal/entityOperations/query?limit=5&count=true", Some(TEMPORAL_BODY))?;
    let received = QueryTemporalEvolutionPostParameters::parse(query_string(&request))?;
    let body = QueryDocument::<TemporalEvolutionQuery>::parse(str::from_utf8(request.body())?)?;
    let temporal_q = body.temporal_q();
    println!(
        "temporal query read: {} in the body, limit {:?}, count {:?}",
        body.render(),
        received.limit,
        received.count
    );
    let evolution = TemporalEntity::builder()
        .id(vehicle.id.clone())
        .entity_type("Vehicle")
        .property("speed", Property::builder(85).observed_at("2024-05-01T10:05:00Z".parse::<DateTime>()?))
        .property("speed", Property::builder(95).observed_at("2024-05-01T10:10:00Z".parse::<DateTime>()?))
        .build()?;
    let mut response = answer::list([evolution.render(Lossless::Normalized)], &Link::core_context()?);
    response.headers_mut().typed_insert(NgsiLdResultsCount::new(1));
    let operation = QueryTemporalEvolution::builder()
        .maybe_entity_type(body.entities().first().map(EntitySelector::selector_type))
        .maybe_q(body.q())
        .temporal_q(temporal_q)
        .system_attributes(received.system_attributes)
        .maybe_limit(received.limit)
        .maybe_offset(received.offset)
        .count(received.count)
        .locality(received.locality)
        .method(QueryMethod::Post)
        .format(Lossless::Normalized)
        .spelling(received.spelling)
        .build();
    let page = operation.decode(&ResponseContext::new(&request), response)?;
    let speed: AttributeName = "speed".parse()?;
    for item in page.items() {
        println!(
            "temporal query answered: {} speed readings of {}",
            item.attribute(&speed).map_or(0, Evolution::len),
            item.id
        );
    }
    Ok(())
}
