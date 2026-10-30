//! The temporal operations over the client (clauses 6.18 to 6.22 and 6.3.10).

use crate::{
    fixtures::client,
    links::{context_link, link_field},
};
use http::header;
use iri_rs::{IriRefBuf, iri};
use ngsi_ld::{
    client::{ClientError, ReqwestError},
    http::{
        Coverage,
        RequestError,
        TemporalUpsertOutcome,
        headers::{ContentRange, LinkValue, RelationType},
        operation::{ArgumentError, InstanceSelection, QueryMethod, TemporalSelection},
    },
    model::{
        APPLICATION_JSON,
        Attribute,
        AttributeName,
        DateTime,
        EntityId,
        Input,
        InstanceId,
        OneOrMany,
        Property,
        evolution::{TemporalAttributes, TemporalEntity},
    },
    query::{
        EntitySelector,
        projection::{self, EntityMember},
        temporal_query::{self, TemporalQuery},
    },
};
use pretty_assertions::assert_eq;
use serde_json::{Value, json};
use std::num::NonZeroU64;
use wiremock::{
    Mock,
    MockServer,
    ResponseTemplate,
    matchers::{any, body_json, header, method, path, query_param, query_param_is_missing},
};

/// `B`'s path below the API root.
const B_PATH: &str = "/ngsi-ld/v1/temporal/entities/urn:ngsi-ld:Vehicle:B9211";

/// `B`.
fn b() -> EntityId {
    iri!("urn:ngsi-ld:Vehicle:B9211").into()
}

/// A time.
fn time(text: &str) -> DateTime {
    text.parse().unwrap()
}

/// `evolution`, as JSON.
fn evolution_json() -> Value {
    json!({
        "id": "urn:ngsi-ld:Vehicle:B9211",
        "type": "Vehicle",
        "speed": [
            {"type": "Property", "value": 120, "observedAt": "2018-08-01T12:03:00Z"},
            {"type": "Property", "value": 80, "observedAt": "2018-08-01T12:05:00Z"}
        ]
    })
}

/// `evolution`.
fn evolution() -> TemporalEntity<Input> {
    TemporalEntity::builder()
        .id(b())
        .entity_type("Vehicle")
        .property("speed", Property::builder(120).observed_at(time("2018-08-01T12:03:00Z")))
        .property("speed", Property::builder(80).observed_at(time("2018-08-01T12:05:00Z")))
        .build()
        .unwrap()
}

/// A temporal entity with the id `id` and one `speed` instance.
fn retrieved(id: &str) -> Value {
    json!({"id": id, "type": "Vehicle", "speed": [{"type": "Property", "value": 80, "observedAt": "2018-08-01T12:05:00Z"}]})
}

/// The request error of a refused call.
fn refusal<T>(result: Result<T, ClientError<ReqwestError>>) -> RequestError {
    match result {
        Err(ClientError::Request(error)) => error,
        Err(other) => panic!("expected a refusal, got {other:?}"),
        Ok(_) => panic!("expected a refusal"),
    }
}

#[tokio::test]
async fn create_or_update_sends_the_evolution_and_reads_created() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/ngsi-ld/v1/temporal/entities"))
        .and(header(&header::CONTENT_TYPE, "application/json"))
        .and(header(&header::LINK, context_link().to_string()))
        .and(body_json(evolution_json()))
        .respond_with(ResponseTemplate::new(201).insert_header(&header::LOCATION, B_PATH))
        .expect(1)
        .mount(&server)
        .await;
    let outcome = client(&server.uri()).temporal().create_or_update(&evolution()).send().await.unwrap();
    assert!(matches!(outcome, TemporalUpsertOutcome::Created { .. }));
}

#[tokio::test]
async fn query_temporal_evolution_walks_every_page() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/ngsi-ld/v1/temporal/entities"))
        .and(query_param("offset", "1"))
        .and(header(&header::ACCEPT, "application/json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([retrieved("urn:ngsi-ld:Vehicle:A4567")])))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/ngsi-ld/v1/temporal/entities"))
        .and(query_param("limit", "1"))
        .and(query_param("timerel", "between"))
        .and(header(&header::ACCEPT, "application/json"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header(
                    &header::LINK,
                    &link_field([LinkValue::builder(IriRefBuf::new("?type=Vehicle&limit=1&offset=1").unwrap())
                        .rel(RelationType::Next)
                        .media_type(APPLICATION_JSON)
                        .build()]),
                )
                .set_body_json(json!([retrieved("urn:ngsi-ld:Vehicle:B9211")])),
        )
        .expect(1)
        .mount(&server)
        .await;
    let client = client(&server.uri());
    let window = temporal_query::between(time("2018-08-01T12:03:00Z"), time("2018-08-01T12:07:00Z"));
    let first = client.temporal().query(window).entity_type("Vehicle").limit(1).send().await.unwrap();
    let every = client.pages(first).try_collect().await.unwrap();
    let ids: Vec<String> = every.iter().map(|entity| entity.id.to_string()).collect();
    assert_eq!(ids, ["urn:ngsi-ld:Vehicle:B9211", "urn:ngsi-ld:Vehicle:A4567"]);
}

#[tokio::test]
async fn a_temporal_query_by_post_sends_several_selectors_in_its_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/ngsi-ld/v1/temporal/entityOperations/query"))
        .and(body_json(json!({
            "type": "Query",
            "entities": [{"type": "Vehicle", "id": ["urn:ngsi-ld:Vehicle:B9211"]}, {"type": "Car"}],
            "temporalQ": {"timerel": "between", "timeAt": "2018-08-01T12:03:00Z", "endTimeAt": "2018-08-01T12:07:00Z"}
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([retrieved("urn:ngsi-ld:Vehicle:B9211")])))
        .expect(1)
        .mount(&server)
        .await;
    let client = client(&server.uri());
    let window = temporal_query::between(time("2018-08-01T12:03:00Z"), time("2018-08-01T12:07:00Z"));
    let selectors = [
        EntitySelector::new("Vehicle".parse().unwrap()).with_ids(OneOrMany::try_from(vec![b()]).unwrap()),
        EntitySelector::new("Car".parse().unwrap()),
    ];
    let page = client
        .temporal()
        .query(window.clone())
        .selectors(selectors.clone())
        .method(QueryMethod::Post)
        .send()
        .await
        .unwrap();
    assert_eq!(page.items().len(), 1);
    let error = refusal(client.temporal().query(window).selectors(selectors).send().await);
    assert!(matches!(error, RequestError::Argument(ArgumentError::SeveralSelectorsInQueryString)));
}

#[tokio::test]
async fn a_projected_temporal_query_walks_every_page_of_entities_without_id() {
    let server = MockServer::start().await;
    let speed_only = |value: u32| json!({"speed": [{"type": "Property", "value": value, "observedAt": "2018-08-01T12:05:00Z"}]});
    Mock::given(method("GET"))
        .and(path("/ngsi-ld/v1/temporal/entities"))
        .and(query_param("offset", "1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([speed_only(40)])))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/ngsi-ld/v1/temporal/entities"))
        .and(query_param("limit", "1"))
        .and(query_param("pick", "speed"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header(
                    &header::LINK,
                    &link_field([LinkValue::builder(IriRefBuf::new("?type=Vehicle&limit=1&offset=1&pick=speed").unwrap())
                        .rel(RelationType::Next)
                        .media_type(APPLICATION_JSON)
                        .build()]),
                )
                .set_body_json(json!([speed_only(80)])),
        )
        .expect(1)
        .mount(&server)
        .await;
    let client = client(&server.uri());
    let window = temporal_query::between(time("2018-08-01T12:03:00Z"), time("2018-08-01T12:07:00Z"));
    let first = client
        .temporal()
        .query_projected(window)
        .entity_type("Vehicle")
        .pick(projection::attributes(["speed"]))
        .limit(1)
        .send()
        .await
        .unwrap();
    let every = client.pages(first).try_collect().await.unwrap();
    assert_eq!(every.len(), 2);
    assert!(every.iter().all(|entity| entity.id.is_none() && entity.entity_type.is_none()));
}

#[tokio::test]
async fn a_projected_temporal_retrieve_reads_the_entity_without_id() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(B_PATH))
        .and(query_param("omit", "id,type"))
        .and(query_param("format", "temporalValues"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"speed": {"type": "Property", "values": [[80, "2018-08-01T12:05:00Z"]]}})))
        .expect(1)
        .mount(&server)
        .await;
    let retrieved = client(&server.uri())
        .temporal()
        .retrieve_projected_values(&b())
        .omit(projection::member(EntityMember::Id).member(EntityMember::Type))
        .send()
        .await
        .unwrap();
    assert_eq!((&retrieved.body().id, &retrieved.body().entity_type), (&None, &None));
}

#[tokio::test]
async fn a_partial_retrieve_reads_its_content_range() {
    let server = MockServer::start().await;
    let values = json!({"id": "urn:ngsi-ld:Vehicle:B9211", "type": "Vehicle", "speed": {"type": "Property", "values": [[80, "2018-08-01T12:05:00Z"]]}});
    Mock::given(method("GET"))
        .and(path(B_PATH))
        .and(query_param("lastN", "2"))
        .and(query_param("format", "temporalValues"))
        .respond_with(
            ResponseTemplate::new(206)
                .insert_header(&header::CONTENT_RANGE, "DateTime 2018-08-01T12:07:00Z-2018-08-01T12:05:00Z/2")
                .set_body_json(values),
        )
        .expect(1)
        .mount(&server)
        .await;
    let retrieved = client(&server.uri()).temporal().retrieve_values(&b()).last_n(2).send().await.unwrap();
    assert!(matches!(retrieved.coverage(), Coverage::Partial(_)));
    assert_eq!(retrieved.coverage().content_range().map(ContentRange::end), Some(&time("2018-08-01T12:05:00Z")));
    assert_eq!(
        retrieved.coverage().following_query(TemporalSelection::Unbounded {
            time_property: None,
            last_n: NonZeroU64::new(2),
            aggregation_methods: None,
            aggregation_period: None,
        }),
        Some(TemporalQuery::before(time("2018-08-01T12:05:00Z")).with_last_n(NonZeroU64::new(2).unwrap()))
    );
}

#[tokio::test]
async fn retrieve_refuses_without_a_request() {
    let server = MockServer::start().await;
    Mock::given(any()).respond_with(ResponseTemplate::new(500)).expect(0).mount(&server).await;
    let client = client(&server.uri());
    let temporal = client.temporal();
    let beside = temporal
        .retrieve(&b())
        .temporal_q(temporal_query::after(time("2018-08-01T12:03:00Z")))
        .last_n(2)
        .send()
        .await;
    assert!(matches!(
        refusal(beside),
        RequestError::Argument(ArgumentError::InstanceOptionsBesideTemporalQuery)
    ));
    assert!(matches!(
        refusal(temporal.retrieve(&b()).last_n(0).send().await),
        RequestError::Argument(ArgumentError::ZeroLastN)
    ));
    assert!(matches!(
        refusal(temporal.retrieve_aggregated(&b()).send().await),
        RequestError::Argument(ArgumentError::AggregationWithoutMethods)
    ));
}

#[tokio::test]
async fn the_operations_that_change_an_evolution_reach_their_resources() {
    let server = MockServer::start().await;
    let instance_path = format!("{B_PATH}/attrs/speed/urn:ngsi-ld:attribute:instance:1");
    for (verb, resource) in [
        ("POST", format!("{B_PATH}/attrs")),
        ("DELETE", format!("{B_PATH}/attrs/speed")),
        ("PATCH", instance_path.clone()),
        ("DELETE", instance_path.clone()),
        ("DELETE", B_PATH.to_owned()),
    ] {
        Mock::given(method(verb))
            .and(path(resource.as_str()))
            .respond_with(ResponseTemplate::new(204))
            .expect(1)
            .mount(&server)
            .await;
    }
    let client = client(&server.uri());
    let temporal = client.temporal();
    let id = b();
    let name: AttributeName = "speed".parse().unwrap();
    let instance_id: InstanceId = "urn:ngsi-ld:attribute:instance:1".parse().unwrap();
    let mut attributes = TemporalAttributes::<Input>::new();
    attributes
        .insert(
            name.clone(),
            Property::builder(70).observed_at(time("2018-08-01T12:07:00Z")).build().unwrap().into(),
        )
        .unwrap();
    let instance: Attribute<Input> = Property::builder(90).build().unwrap().into();
    temporal.add_attributes(&id, &attributes).send().await.unwrap();
    temporal.delete_attribute(&id, &name).instances(InstanceSelection::All).send().await.unwrap();
    temporal.modify_instance(&id, &name, &instance_id, &instance).send().await.unwrap();
    temporal.delete_instance(&id, &name, &instance_id).send().await.unwrap();
    temporal.delete(&id).send().await.unwrap();
}

#[tokio::test]
async fn a_whole_range_retrieve_asks_for_the_rest_and_joins_the_answers() {
    let server = MockServer::start().await;
    let first = json!({"id": "urn:ngsi-ld:Vehicle:B9211", "type": "Vehicle", "speed": [
        {"type": "Property", "value": 120, "observedAt": "2018-08-01T12:03:00Z", "instanceId": "urn:ngsi-ld:i:1"},
        {"type": "Property", "value": 80, "observedAt": "2018-08-01T12:05:00Z", "instanceId": "urn:ngsi-ld:i:2"}
    ]});
    let rest = json!({"id": "urn:ngsi-ld:Vehicle:B9211", "type": "Vehicle", "speed": [
        {"type": "Property", "value": 80, "observedAt": "2018-08-01T12:05:00Z", "instanceId": "urn:ngsi-ld:i:2"},
        {"type": "Property", "value": 100, "observedAt": "2018-08-01T12:07:00Z", "instanceId": "urn:ngsi-ld:i:3"}
    ]});
    Mock::given(method("GET"))
        .and(path(B_PATH))
        .and(query_param("timeAt", "2018-08-01T12:03:00Z"))
        .and(query_param("endTimeAt", "2018-08-01T12:07:00Z"))
        .respond_with(
            ResponseTemplate::new(206)
                .insert_header(&header::CONTENT_RANGE, "DateTime 2018-08-01T12:03:00Z-2018-08-01T12:05:00Z/*")
                .set_body_json(first),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(B_PATH))
        .and(query_param("timerel", "between"))
        .and(query_param("timeAt", "2018-08-01T12:05:00Z"))
        .and(query_param("endTimeAt", "2018-08-01T12:07:00Z"))
        .respond_with(ResponseTemplate::new(200).set_body_json(rest))
        .expect(1)
        .mount(&server)
        .await;
    let window = temporal_query::between(time("2018-08-01T12:03:00Z"), time("2018-08-01T12:07:00Z"));
    let whole = client(&server.uri())
        .temporal()
        .retrieve_whole_range(&b())
        .temporal_q(window)
        .send()
        .await
        .unwrap();
    assert!(whole.coverage().is_whole());
    let speed: AttributeName = "speed".parse().unwrap();
    let values: Vec<String> = whole
        .body()
        .attribute(&speed)
        .unwrap()
        .iter()
        .map(|instance| {
            let Attribute::Property(property) = instance else {
                panic!("unexpected {instance:?}");
            };
            property.value().to_string()
        })
        .collect();
    assert_eq!(values, ["120", "80", "100"]);
}

#[tokio::test]
async fn a_whole_range_retrieve_with_last_n_keeps_the_last_n_instances_of_each_attribute() {
    let server = MockServer::start().await;
    let instance = |value: u32, at: &str| json!({"type": "Property", "value": value, "observedAt": at, "instanceId": format!("urn:ngsi-ld:i:{at}")});
    let first = json!({"id": "urn:ngsi-ld:Vehicle:B9211", "type": "Vehicle",
        "speed": [instance(80, "2018-08-01T12:07:00Z")],
        "rpm": [instance(3000, "2018-08-01T12:09:00Z"), instance(2900, "2018-08-01T12:08:00Z")]
    });
    let rest = json!({"id": "urn:ngsi-ld:Vehicle:B9211", "type": "Vehicle",
        "speed": [instance(100, "2018-08-01T12:05:00Z"), instance(120, "2018-08-01T12:03:00Z")],
        "rpm": [instance(2800, "2018-08-01T12:06:00Z"), instance(2700, "2018-08-01T12:04:00Z")]
    });
    Mock::given(method("GET"))
        .and(path(B_PATH))
        .and(query_param("lastN", "2"))
        .and(query_param_is_missing("timerel"))
        .respond_with(
            ResponseTemplate::new(206)
                .insert_header(&header::CONTENT_RANGE, "DateTime 2018-08-01T12:09:00Z-2018-08-01T12:07:00Z/2")
                .set_body_json(first),
        )
        .expect(1)
        .mount(&server)
        .await;
    // The rest asks for `lastN` again: no one query can ask each attribute for the instances it still lacks.
    Mock::given(method("GET"))
        .and(path(B_PATH))
        .and(query_param("timerel", "before"))
        .and(query_param("timeAt", "2018-08-01T12:07:00Z"))
        .and(query_param("lastN", "2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(rest))
        .expect(1)
        .mount(&server)
        .await;
    let whole = client(&server.uri()).temporal().retrieve_whole_range(&b()).last_n(2).send().await.unwrap();
    assert!(whole.coverage().is_whole());
    let observed = |name: &str| -> Vec<String> {
        whole
            .body()
            .attribute(&name.parse().unwrap())
            .unwrap()
            .iter()
            .map(|instance| instance.meta().observed_at.unwrap().to_string())
            .collect()
    };
    assert_eq!(observed("speed"), ["2018-08-01T12:07:00Z", "2018-08-01T12:05:00Z"]);
    assert_eq!(observed("rpm"), ["2018-08-01T12:09:00Z", "2018-08-01T12:08:00Z"]);
}

#[tokio::test]
async fn a_whole_range_retrieve_stops_when_the_broker_cycles_back_to_a_rest_already_asked_for() {
    let server = MockServer::start().await;
    let answer = |at: &str| {
        json!({"id": "urn:ngsi-ld:Vehicle:B9211", "type": "Vehicle", "speed": [
            {"type": "Property", "value": 80, "observedAt": at, "instanceId": format!("urn:ngsi-ld:i:{at}")}
        ]})
    };
    let partial = |range: &str, at: &str| {
        ResponseTemplate::new(206)
            .insert_header(&header::CONTENT_RANGE, format!("DateTime {range}/*").as_str())
            .set_body_json(answer(at))
    };
    for (time_at, range, at) in [
        ("2018-08-01T12:03:00Z", "2018-08-01T12:03:00Z-2018-08-01T12:05:00Z", "2018-08-01T12:04:00Z"),
        ("2018-08-01T12:05:00Z", "2018-08-01T12:05:00Z-2018-08-01T12:06:00Z", "2018-08-01T12:05:30Z"),
        // The rest after 12:06 is answered with the range the first answer covered, so the following rest is the
        // one already asked for at 12:05.
        ("2018-08-01T12:06:00Z", "2018-08-01T12:03:00Z-2018-08-01T12:05:00Z", "2018-08-01T12:06:30Z"),
    ] {
        Mock::given(method("GET"))
            .and(path(B_PATH))
            .and(query_param("timeAt", time_at))
            .and(query_param("endTimeAt", "2018-08-01T12:07:00Z"))
            .respond_with(partial(range, at))
            .expect(1)
            .mount(&server)
            .await;
    }
    let window = temporal_query::between(time("2018-08-01T12:03:00Z"), time("2018-08-01T12:07:00Z"));
    let whole = client(&server.uri())
        .temporal()
        .retrieve_whole_range(&b())
        .temporal_q(window)
        .send()
        .await
        .unwrap();
    assert!(whole.coverage().is_partial());
}
