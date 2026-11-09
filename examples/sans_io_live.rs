//! Drives the HTTP binding over `ureq`, a blocking HTTP client, with no async runtime: creates two vehicles, reads
//! one back, queries the fast one with its total count, reads the problem a missing entity is answered with, and
//! deletes both.
//!
//! Run it with `NGSI_LD_BROKER_URL=http://localhost:9090/ngsi-ld/v1/ cargo run --features http --example sans_io_live`.

use anyhow::Context;
use bytes::Bytes;
use http::{Request, Response};
use iri_rs::iri;
use ngsi_ld::{
    http::{
        Endpoint,
        ResponseContext,
        ResponseError,
        operation::{BatchDelete, CreateEntity, Operation, QueryEntities, ResultsCount, RetrieveEntity},
    },
    model::{Entity, EntityId, Lossless, Representation},
    query::{SelectorType, q::Query},
};
use std::env;
use ureq::Agent;
use url::Url;

/// Sends `request` with `agent` and returns the broker's answer, whatever its status.
fn exchange(agent: &Agent, request: Request<Bytes>) -> anyhow::Result<Response<Bytes>> {
    let response = agent.run(request.map(|body| body.to_vec()))?;
    let (head, mut body) = response.into_parts();
    Ok(Response::from_parts(head, Bytes::from(body.read_to_vec()?)))
}

/// Performs `operation` on the broker at `endpoint`: the operation builds the request and reads the answer, `ureq`
/// carries them.
fn perform<O: Operation>(agent: &Agent, endpoint: &Endpoint, operation: &O) -> anyhow::Result<O::Outcome> {
    let request = operation.to_request(endpoint)?;
    println!("{} {}", request.method(), request.uri());
    let context = ResponseContext::new(&request);
    let response = exchange(agent, request)?;
    Ok(operation.decode(&context, response)?)
}

fn main() -> anyhow::Result<()> {
    let base_url: Url = env::var("NGSI_LD_BROKER_URL")
        .context("set NGSI_LD_BROKER_URL to a broker's API root")?
        .parse()?;
    let endpoint = Endpoint::builder().base_url(base_url).build()?;
    // An error status is an answer the operation reads, not a failure of the exchange.
    let agent: Agent = Agent::config_builder().http_status_as_error(false).build().into();

    let fleet = [
        Entity::builder()
            .id(iri!("urn:ngsi-ld:Vehicle:Y1001"))
            .entity_type("Vehicle")
            .property("speed", 35)
            .build()?,
        Entity::builder()
            .id(iri!("urn:ngsi-ld:Vehicle:Y1002"))
            .entity_type("Vehicle")
            .property("speed", 70)
            .build()?,
    ];
    let ids = fleet.each_ref().map(|vehicle| vehicle.id.clone());
    perform(&agent, &endpoint, &BatchDelete { ids: &ids })?;

    for vehicle in &fleet {
        let created = perform(&agent, &endpoint, &CreateEntity::builder().entity(vehicle).build())?;
        println!("  {created:?}");
    }

    let retrieve = RetrieveEntity::builder().id(&ids[0]).format(Lossless::Concise).build();
    let retrieved = perform(&agent, &endpoint, &retrieve)?.into_body();
    println!("  {}", retrieved.render(Representation::Concise));

    let vehicles: SelectorType = "Vehicle".parse()?;
    let fast: Query = "speed>50".parse()?;
    let query = QueryEntities::builder()
        .entity_type(&vehicles)
        .q(&fast)
        .count(ResultsCount::Include)
        .format(Lossless::Concise)
        .build();
    let page = perform(&agent, &endpoint, &query)?;
    println!("  {} of {:?} vehicles match", page.items().len(), page.total());
    for vehicle in page.items() {
        println!("  {}", vehicle.render(Representation::Concise));
    }

    let missing = EntityId::from(iri!("urn:ngsi-ld:Vehicle:Y9999"));
    match perform(&agent, &endpoint, &RetrieveEntity { id: &missing, ..retrieve }) {
        Ok(found) => println!("  {}", found.into_body().render(Representation::Concise)),
        Err(error) => {
            let Some(ResponseError::Problem { status, details, .. }) = error.downcast_ref() else {
                return Err(error);
            };
            println!("  {status} {:?}: {:?}", details.error_type, details.detail);
        }
    }

    let deleted = perform(&agent, &endpoint, &BatchDelete { ids: &ids })?;
    println!("  {deleted:?}");
    Ok(())
}
