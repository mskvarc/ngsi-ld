//! The forwarded Query Entities, through GET and through POST, and the POST query the source refuses.

use crate::forwarding::{forwarded, query_string};
use anyhow::Context;
use bytes::Bytes;
use headers::HeaderMapExt;
use http::{Method, Response};
use ngsi_ld::{
    http::{
        ResponseContext,
        answer,
        headers::{Link, NgsiLdResultsCount},
        operation::{Operation, QueryEntities, QueryEntitiesParameters, QueryEntitiesPostParameters, QueryMethod, ResultsCount},
    },
    model::{Entity, ErrorType, Input, Lossless, ProblemDetails, Representation},
    query::{EntitySelector, SelectorType, q::Query},
    snapshot::{EntityQuery, QueryDocument},
};
use std::str;

/// The `Query` body of the forwarded POST query.
const QUERY_BODY: &str = r#"{"type": "Query", "entities": [{"type": "Vehicle"}], "q": "speed>50"}"#;

/// The Query Entities the broker forwarded, with the members the source read from it and no others.
fn query_operation<'a>(
    entity_type: Option<&'a SelectorType>,
    q: Option<&'a Query>,
    limit: Option<u32>,
    count: ResultsCount,
    method: QueryMethod,
    format: Lossless,
) -> QueryEntities<'a, Lossless> {
    QueryEntities::builder()
        .maybe_entity_type(entity_type)
        .maybe_q(q)
        .maybe_limit(limit)
        .count(count)
        .method(method)
        .format(format)
        .build()
}

/// Answers a query with the one vehicle that matches it, in `form`, with the total in `NGSILD-Results-Count`.
fn matching_vehicle(vehicle: &Entity<Input>, form: Lossless) -> anyhow::Result<Response<Bytes>> {
    let representation = match form {
        Lossless::Normalized => Representation::Normalized,
        Lossless::Concise => Representation::Concise,
    };
    let mut response = answer::list([vehicle.render(representation)], &Link::try_core_context()?);
    response.headers_mut().typed_insert(NgsiLdResultsCount::new(1));
    Ok(response)
}

/// Reads and answers a forwarded `GET /entities?type=Vehicle&q=speed>50&limit=10&count=true`.
pub(crate) fn get_query(vehicle: &Entity<Input>) -> anyhow::Result<()> {
    let request = forwarded(Method::GET, "/entities?type=Vehicle&q=speed%3E50&limit=10&count=true", None)?;
    let received = QueryEntitiesParameters::parse(query_string(&request))?;
    println!(
        "GET query read: type {:?}, q {:?}, limit {:?}, count {:?}",
        received.selection.entity_type.as_ref().map(ToString::to_string),
        received.selection.q.as_ref().map(ToString::to_string),
        received.limit,
        received.count
    );
    let operation = query_operation(
        received.selection.entity_type.as_ref(),
        received.selection.q.as_ref(),
        received.limit,
        received.count,
        QueryMethod::Get,
        Lossless::Normalized,
    );
    let page = operation.decode(&ResponseContext::new(&request), matching_vehicle(vehicle, Lossless::Normalized)?)?;
    println!(
        "GET query answered: {:?} of {:?}",
        page.items().iter().map(|item| item.id.to_string()).collect::<Vec<_>>(),
        page.total()
    );
    Ok(())
}

/// Reads and answers a forwarded `POST /entityOperations/query?format=concise&limit=10&count=true`, whose
/// selection is in the body.
pub(crate) fn post_query(vehicle: &Entity<Input>) -> anyhow::Result<()> {
    let request = forwarded(Method::POST, "/entityOperations/query?format=concise&limit=10&count=true", Some(QUERY_BODY))?;
    let received = QueryEntitiesPostParameters::parse(query_string(&request))?;
    let body = QueryDocument::<EntityQuery>::parse(str::from_utf8(request.body())?)?;
    println!(
        "POST query read: {} in the body, {:?}, limit {:?}, count {:?}",
        body.render(),
        received.representation,
        received.limit,
        received.count
    );
    let entity_type = body.entities().first().map(EntitySelector::selector_type);
    let operation = query_operation(entity_type, body.q(), received.limit, received.count, QueryMethod::Post, Lossless::Concise);
    let page = operation.decode(&ResponseContext::new(&request), matching_vehicle(vehicle, Lossless::Concise)?)?;
    println!(
        "POST query answered: {:?}",
        page.items().iter().map(|item| item.id.to_string()).collect::<Vec<_>>()
    );
    Ok(())
}

/// Refuses a forwarded POST query whose query string carries `type`, which belongs in the `Query` body.
pub(crate) fn refused_query() -> anyhow::Result<()> {
    let request = forwarded(Method::POST, "/entityOperations/query?type=Vehicle", Some(QUERY_BODY))?;
    // `type` belongs in the `Query` body, not in the query string.
    let Err(error) = QueryEntitiesPostParameters::parse(query_string(&request)) else {
        println!("POST query accepted");
        return Ok(());
    };
    println!("POST query refused: {error}");
    let status = answer::error_status(&ErrorType::BadRequestData).context("BadRequestData has no HTTP status")?;
    let problem = ProblemDetails::builder()
        .error_type(ErrorType::BadRequestData)
        .title("Bad request data")
        .detail(error.to_string())
        .build();
    let response = answer::problem(status, problem);
    let operation = query_operation(None, None, None, ResultsCount::Omit, QueryMethod::Post, Lossless::Normalized);
    let decoded = operation.decode(&ResponseContext::new(&request), response);
    match decoded {
        Ok(page) => println!("the broker reads the refusal as {} entities", page.items().len()),
        Err(error) => println!("the broker reads the refusal as: {error}"),
    }
    Ok(())
}
