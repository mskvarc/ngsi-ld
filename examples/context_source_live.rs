//! Serves the trucks of a depot as a context source registered with a broker: the broker forwards the queries and
//! retrievals of trucks to it, and merges the depot's trucks with the one it holds itself.
//!
//! Run it with `NGSI_LD_BROKER_URL=http://localhost:9090/ngsi-ld/v1/ NGSI_LD_CALLBACK_HOST=host.docker.internal cargo run --example context_source_live`.

use anyhow::Context;
use axum::{
    Router,
    body::Body,
    extract::{Path, Request, State},
    response::Response,
    routing::get,
};
use http::StatusCode;
use iri_rs::iri;
use ngsi_ld::{
    client::{Client, ReqwestTransport},
    http::{answer, headers::Link, operation::QueryEntitiesParameters},
    model::{Entity, EntityId, ErrorType, Input, ProblemDetails, Representation},
    registration::{CSourceRegistration, RegistrationInfo},
};
use std::{env, slice, sync::Arc};
use tokio::net::TcpListener;
use url::Url;

/// The trucks the depot holds, which the broker does not.
type Depot = Arc<[Entity<Input>]>;

/// Answers a forwarded Query Entities with every truck of the depot.
async fn query(State(depot): State<Depot>, request: Request) -> Response {
    println!("  the depot answers {} {}", request.method(), request.uri());
    if let Err(error) = QueryEntitiesParameters::parse(request.uri().query().unwrap_or_default()) {
        return refusal(StatusCode::BAD_REQUEST, ErrorType::BadRequestData, "Bad request data", &error.to_string());
    }
    match Link::try_core_context() {
        Ok(link) => answer::list(depot.iter().map(|truck| truck.render(Representation::Normalized)), &link).map(Body::from),
        Err(error) => refusal(
            StatusCode::INTERNAL_SERVER_ERROR,
            ErrorType::InternalError,
            "Internal error",
            &error.to_string(),
        ),
    }
}

/// Answers a forwarded Retrieve Entity with the truck it names.
async fn retrieve(State(depot): State<Depot>, Path(id): Path<String>) -> Response {
    println!("  the depot answers GET /ngsi-ld/v1/entities/{id}");
    let Some(truck) = depot.iter().find(|truck| truck.id.as_str() == id) else {
        return refusal(
            StatusCode::NOT_FOUND,
            ErrorType::ResourceNotFound,
            "Resource not found",
            &format!("the depot holds no truck {id}"),
        );
    };
    match Link::try_core_context() {
        Ok(link) => answer::document(&truck.render(Representation::Normalized), &link).map(Body::from),
        Err(error) => refusal(
            StatusCode::INTERNAL_SERVER_ERROR,
            ErrorType::InternalError,
            "Internal error",
            &error.to_string(),
        ),
    }
}

/// A problem answer of `error_type`, with `title` and `detail`.
fn refusal(status: StatusCode, error_type: ErrorType, title: &str, detail: &str) -> Response {
    let problem = ProblemDetails::builder().error_type(error_type).title(title).detail(detail).build();
    answer::problem(status, problem).map(Body::from)
}

/// Prints the trucks a query of the broker returns.
async fn list_trucks(client: &Client<ReqwestTransport>) -> anyhow::Result<()> {
    let page = client.entities().query().entity_type("Truck").send().await?;
    for truck in page.items() {
        println!("  {}", truck.render(Representation::Concise));
    }
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let base_url: Url = env::var("NGSI_LD_BROKER_URL")
        .context("set NGSI_LD_BROKER_URL to a broker's API root")?
        .parse()?;
    let host = env::var("NGSI_LD_CALLBACK_HOST").context("set NGSI_LD_CALLBACK_HOST to the host under which the broker reaches this machine")?;
    let client = Client::builder().base_url(base_url).transport(ReqwestTransport::try_default()?).build()?;

    let depot: Depot = Arc::new([
        Entity::builder()
            .id(iri!("urn:ngsi-ld:Truck:depot-2"))
            .entity_type("Truck")
            .property("load", 12)
            .property("status", "loading")
            .build()?,
        Entity::builder()
            .id(iri!("urn:ngsi-ld:Truck:depot-3"))
            .entity_type("Truck")
            .property("load", 0)
            .property("status", "idle")
            .build()?,
    ]);
    let listener = TcpListener::bind("0.0.0.0:0").await?;
    let endpoint = format!("http://{host}:{}", listener.local_addr()?.port());
    let router = Router::new()
        .route("/ngsi-ld/v1/entities", get(query))
        .route("/ngsi-ld/v1/entities/{id}", get(retrieve))
        .with_state(depot);
    tokio::spawn(async move { axum::serve(listener, router).await });

    let on_the_road = Entity::builder()
        .id(iri!("urn:ngsi-ld:Truck:road-1"))
        .entity_type("Truck")
        .property("load", 20)
        .property("status", "driving")
        .build()?;
    client.entities().batch_delete(slice::from_ref(&on_the_road.id)).send().await?;
    client.entities().create(&on_the_road).send().await?;
    println!("the broker holds {}", on_the_road.id);

    let source = CSourceRegistration::builder(endpoint.as_str())
        .registration_name("depot trucks")
        .information([RegistrationInfo::builder().entities(["Truck"]).build()?])
        .build()?;
    let created = client.registrations().register(&source).send().await?;
    let id = created.id().context("the broker named no registration in the Location of its answer")?;
    println!("registered the depot at {endpoint} as {id}");

    println!("the trucks the broker returns:");
    list_trucks(&client).await?;

    let depot_truck = EntityId::from(iri!("urn:ngsi-ld:Truck:depot-2"));
    println!("one truck of the depot, retrieved through the broker:");
    let truck = client.entities().retrieve(&depot_truck).send().await?.into_body();
    println!("  {}", truck.render(Representation::Concise));

    client.registrations().delete(&id).send().await?;
    println!("deleted {id}, so the broker returns only its own:");
    list_trucks(&client).await?;

    client.entities().batch_delete(slice::from_ref(&on_the_road.id)).send().await?;
    Ok(())
}
