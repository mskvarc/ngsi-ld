//! Projected temporal entities (clauses 4.21, 6.18.3.2, 6.19.3.1 and 6.24.3.1): a `pick` or `omit` that removes `id`
//! or `type` is sent with a projected request, and its answer is read in each temporal representation.

use crate::{
    fixtures::{context, endpoint, response},
    temporal_fixtures::{b, query_string, window},
};
use ngsi_ld::{
    http::operation::{Aggregated, Operation, QueryMethod, QueryTemporalEvolution, RetrieveTemporalEvolution, SimplifiedTemporal, TemporalSelection},
    model::Lossless,
    query::{SelectorType, projection::Projection, temporal_query::AggregationMethods},
};
use pretty_assertions::assert_eq;
use serde_json::{Value, json};

/// A temporal entity of `speed` alone, in normalized temporal representation.
const NORMALIZED: &str = r#"{"speed":[{"type":"Property","value":120,"observedAt":"2018-08-01T12:03:00Z"}]}"#;

/// A temporal entity of `speed` alone, in simplified temporal representation.
const VALUES: &str = r#"{"speed":{"type":"Property","values":[[120,"2018-08-01T12:03:00Z"]]}}"#;

/// A temporal entity of `speed` alone, in aggregated temporal representation.
const AGGREGATED: &str = r#"{"speed":{"type":"Property","max":[[120,"2018-08-01T12:03:00Z","2018-08-01T12:07:00Z"]]}}"#;

/// The projection that keeps `speed` and drops `id` and `type`.
fn speed_only() -> Projection {
    "speed".parse().unwrap()
}

/// `Vehicle`.
fn vehicle_type() -> SelectorType {
    "Vehicle".parse().unwrap()
}

#[test]
fn a_projected_temporal_query_reads_each_representation_without_id_and_type() {
    let entity_type = vehicle_type();
    let pick = speed_only();
    let window = window();
    let normalized = QueryTemporalEvolution {
        pick: Some(&pick),
        ..QueryTemporalEvolution::projected_builder()
            .entity_type(&entity_type)
            .temporal_q(&window)
            .format(Lossless::Normalized)
            .build()
    };
    assert!(query_string(&normalized).is_some_and(|query| query.contains("pick=speed")));
    let page = normalized.decode(&context(), response(200, &[], &format!("[{NORMALIZED}]"))).unwrap();
    assert_eq!(page.items().len(), 1);
    assert_eq!((&page.items()[0].id, &page.items()[0].entity_type), (&None, &None));
    assert_eq!(page.items()[0].render(Lossless::Normalized).to_string(), NORMALIZED);
    let values = QueryTemporalEvolution {
        pick: Some(&pick),
        ..QueryTemporalEvolution::projected_builder()
            .entity_type(&entity_type)
            .temporal_q(&window)
            .format(SimplifiedTemporal)
            .build()
    };
    let page = values.decode(&context(), response(200, &[], &format!("[{VALUES}]"))).unwrap();
    assert_eq!((&page.items()[0].id, &page.items()[0].entity_type), (&None, &None));
    assert_eq!(page.items()[0].render().to_string(), VALUES);
    let aggregating = window.clone().with_aggregation_methods("max".parse::<AggregationMethods>().unwrap());
    let aggregated = QueryTemporalEvolution {
        pick: Some(&pick),
        ..QueryTemporalEvolution::projected_builder()
            .entity_type(&entity_type)
            .temporal_q(&aggregating)
            .format(Aggregated)
            .build()
    };
    assert!(query_string(&aggregated).is_some_and(|query| query.contains("pick=speed")));
    let page = aggregated.decode(&context(), response(200, &[], &format!("[{AGGREGATED}]"))).unwrap();
    assert_eq!((&page.items()[0].id, &page.items()[0].entity_type), (&None, &None));
    assert_eq!(page.items()[0].render().to_string(), AGGREGATED);
}

#[test]
fn a_projected_temporal_query_by_post_sends_the_projection_in_the_body() {
    let entity_type = vehicle_type();
    let omit: Projection = "id,type".parse().unwrap();
    let window = window();
    let query = QueryTemporalEvolution {
        method: QueryMethod::Post,
        omit: Some(&omit),
        ..QueryTemporalEvolution::projected_builder()
            .entity_type(&entity_type)
            .temporal_q(&window)
            .format(Lossless::Normalized)
            .build()
    };
    let request = query.to_request(&endpoint()).unwrap();
    let body: Value = serde_json::from_slice(request.body()).unwrap();
    assert_eq!(body["omit"], json!(["id", "type"]));
    let page = query.decode(&context(), response(200, &[], &format!("[{NORMALIZED}]"))).unwrap();
    assert_eq!(page.items()[0].id, None);
}

#[test]
fn a_projected_temporal_retrieve_reads_the_simplified_and_aggregated_forms_without_id_and_type() {
    let id = b();
    let omit: Projection = "id,type".parse().unwrap();
    let window = window();
    let values = RetrieveTemporalEvolution {
        omit: Some(&omit),
        ..RetrieveTemporalEvolution::projected_builder()
            .id(&id)
            .selection(TemporalSelection::Query(&window))
            .format(SimplifiedTemporal)
            .build()
    };
    assert!(query_string(&values).is_some_and(|query| query.contains("omit=id,type")));
    let retrieved = values.decode(&context(), response(200, &[], VALUES)).unwrap();
    assert_eq!((&retrieved.body().id, &retrieved.body().entity_type), (&None, &None));
    assert_eq!(retrieved.body().render().to_string(), VALUES);
    let aggregating = window.clone().with_aggregation_methods("max".parse::<AggregationMethods>().unwrap());
    let aggregated = RetrieveTemporalEvolution {
        omit: Some(&omit),
        ..RetrieveTemporalEvolution::projected_builder()
            .id(&id)
            .selection(TemporalSelection::Query(&aggregating))
            .format(Aggregated)
            .build()
    };
    let retrieved = aggregated.decode(&context(), response(200, &[], AGGREGATED)).unwrap();
    assert_eq!((&retrieved.body().id, &retrieved.body().entity_type), (&None, &None));
    assert_eq!(retrieved.body().render().to_string(), AGGREGATED);
}
