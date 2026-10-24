//! Uses the HTTP binding without an HTTP library: a `Client` over an in-memory transport of its own, then a Retrieve
//! Entity with no client at all, its request built by the operation and its answers read by it.
//!
//! Run it with `cargo run --features http --example sans_io`.

use bytes::Bytes;
use http::{Method, Request, Response, StatusCode};
use iri_rs::{IriRefBuf, iri};
use ngsi_ld::{
    client::{Client, Transport},
    http::{
        Endpoint,
        ResponseContext,
        answer,
        headers::{Link, Location},
        operation::{DeleteEntity, Operation, RetrieveEntity},
    },
    model::{Entity, EntityId, ErrorType, Lossless, ProblemDetails, RawJson, Representation},
    query::projection::Projection,
};
use std::{
    convert::Infallible,
    future,
    sync::{Mutex, PoisonError},
};
use url::Url;

/// The API root the stand-in and the endpoint answer under.
const ROOT: &str = "http://broker.test/ngsi-ld/v1/";

/// The path of the entities collection under [`ROOT`].
const ENTITIES: &str = "/ngsi-ld/v1/entities";

/// The path of the one entity the stand-in and the exchange hold.
const VEHICLE: &str = "/ngsi-ld/v1/entities/urn:ngsi-ld:Vehicle:A4567";

/// The vehicle the caller's own exchange holds, in concise form.
const CONCISE_VEHICLE: &str = r#"{"id": "urn:ngsi-ld:Vehicle:A4567", "type": "Vehicle", "speed": 80}"#;

/// A 404 `ResourceNotFound` answer.
fn not_found() -> Response<Bytes> {
    let problem = ProblemDetails::builder()
        .error_type(ErrorType::ResourceNotFound)
        .title("Resource not found")
        .detail("no such entity")
        .build();
    answer::problem(StatusCode::NOT_FOUND, problem)
}

/// A transport that answers in memory, as a broker holding one vehicle does, and records every request it is sent.
#[derive(Debug)]
struct StandIn {
    /// Where the stand-in stores the vehicle it creates.
    location: Location,
    /// The vehicle, as the stand-in returns it once created.
    vehicle: RawJson,
    /// The `Link` to the core context its answers carry.
    link: Link,
    /// Every request received, in order.
    received: Mutex<Vec<Request<Bytes>>>,
}

impl StandIn {
    /// Answers `request` as Create Entity, Retrieve Entity and Delete Entity do, and adds it to the record.
    fn answer(&self, request: Request<Bytes>) -> Response<Bytes> {
        let mut received = self.received.lock().unwrap_or_else(PoisonError::into_inner);
        let created = received.iter().any(|earlier| earlier.method() == Method::POST);
        let response = match (request.method(), request.uri().path()) {
            (&Method::POST, ENTITIES) => answer::created(self.location.clone()),
            (&Method::GET, VEHICLE) if created => answer::document(&self.vehicle, &self.link),
            (&Method::DELETE, VEHICLE) => answer::no_content(),
            _ => not_found(),
        };
        received.push(request);
        response
    }
}

impl Transport for StandIn {
    type Error = Infallible;

    // The stand-in answers at once, so its future is ready when made.
    fn send(&self, request: Request<Bytes>) -> impl Future<Output = Result<Response<Bytes>, Infallible>> + Send {
        future::ready(Ok(self.answer(request)))
    }
}

/// The caller's own exchange: whatever moves a request to a broker and its answer back. Here it holds one vehicle.
fn exchange(request: &Request<Bytes>, vehicle: &RawJson, link: &Link) -> Response<Bytes> {
    if request.uri().path() == VEHICLE {
        answer::document(vehicle, link)
    } else {
        not_found()
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let vehicle = Entity::builder()
        .id(iri!("urn:ngsi-ld:Vehicle:A4567"))
        .entity_type("Vehicle")
        .property("brandName", "Mercedes")
        .property("speed", 80)
        .build()?;
    let link = Link::core_context()?;

    let stand_in = StandIn {
        location: Location::new(IriRefBuf::new(VEHICLE.to_owned())?)?,
        vehicle: vehicle.render(Representation::Normalized),
        link: Link::core_context()?,
        received: Mutex::default(),
    };
    let client = Client::builder().base_url(Url::parse(ROOT)?).transport(&stand_in).build()?;
    let created = client.entities().create(&vehicle).send().await?;
    println!("created through the client: {created:?}");
    let retrieved = client.entities().retrieve(&vehicle.id).send().await?.into_body();
    println!("retrieved through the client: {}", retrieved.render(Representation::Concise));
    // Any operation can go through the client, not only those its methods build.
    let deleted = client.send(&DeleteEntity::builder().id(&vehicle.id).build()).await?;
    println!("deleted through the client: {deleted:?}");
    for request in stand_in.received.lock().unwrap_or_else(PoisonError::into_inner).iter() {
        println!("the stand-in received {} {}", request.method(), request.uri());
    }

    let held: RawJson = CONCISE_VEHICLE.parse()?;
    let endpoint = Endpoint::builder().base_url(Url::parse(ROOT)?).build()?;
    let without_brand: Projection = "brandName".parse()?;
    let retrieve = RetrieveEntity::builder().id(&vehicle.id).omit(&without_brand).format(Lossless::Concise).build();
    let request = retrieve.to_request(&endpoint)?;
    println!("built {} {}", request.method(), request.uri());
    let response = exchange(&request, &held, &link);
    let retrieved = retrieve.decode(&ResponseContext::new(&request), response)?.into_body();
    println!("read {}", retrieved.render(Representation::Concise));

    let missing = EntityId::from(iri!("urn:ngsi-ld:Vehicle:B9211"));
    let retrieve_missing = RetrieveEntity { id: &missing, ..retrieve };
    let request = retrieve_missing.to_request(&endpoint)?;
    let response = exchange(&request, &held, &link);
    match retrieve_missing.decode(&ResponseContext::new(&request), response) {
        Ok(found) => println!("read {}", found.into_body().render(Representation::Concise)),
        Err(error) => println!("read the answer for {missing} as: {error}"),
    }
    Ok(())
}
