//! Query parameters sent as written through the unchecked path, over the client.

use crate::fixtures::client;
use ngsi_ld::{
    client::ClientError,
    http::{QueryParameter, ResponseError, operation::QueryEntities},
    model::{ErrorType, Lossless},
    query::SelectorType,
};
use serde_json::json;
use wiremock::{
    Mock,
    MockServer,
    ResponseTemplate,
    matchers::{method, path, query_param},
};

#[tokio::test]
async fn an_invalid_scope_query_reaches_the_broker_as_written_and_its_rejection_decodes() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/ngsi-ld/v1/entities"))
        .and(query_param("type", "Building"))
        .and(query_param("scopeQ", "/Madrid/#/Gardens"))
        .respond_with(
            ResponseTemplate::new(400).set_body_json(json!({"type": "https://uri.etsi.org/ngsi-ld/errors/BadRequestData", "title": "Invalid scopeQ"})),
        )
        .expect(1)
        .mount(&server)
        .await;
    let client = client(&server.uri());
    let buildings: SelectorType = "Building".parse().unwrap();
    let query = QueryEntities::builder().entity_type(&buildings).format(Lossless::Normalized).build();
    let request = client
        .unchecked(&query)
        .unwrap()
        .query_parameter(QueryParameter::ScopeQ, "/Madrid/#/Gardens")
        .unwrap();
    let error = client.send_unchecked(request).await.unwrap_err();
    assert!(
        matches!(&error, ClientError::Response(ResponseError::Problem { details, .. }) if details.error_type == Some(ErrorType::BadRequestData)),
        "{error:?}"
    );
}

#[tokio::test]
async fn an_undefined_parameter_reaches_the_broker_and_its_invalid_request_decodes() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/ngsi-ld/v1/entities"))
        .and(query_param("additionalParam", "type=Building"))
        .respond_with(
            ResponseTemplate::new(400).set_body_json(json!({"type": "https://uri.etsi.org/ngsi-ld/errors/InvalidRequest", "title": "Unknown parameter"})),
        )
        .expect(1)
        .mount(&server)
        .await;
    let client = client(&server.uri());
    let buildings: SelectorType = "Building".parse().unwrap();
    let query = QueryEntities::builder().entity_type(&buildings).format(Lossless::Normalized).build();
    let request = client
        .unchecked(&query)
        .unwrap()
        .undefined_query_parameter("additionalParam", "type=Building")
        .unwrap();
    let error = client.send_unchecked(request).await.unwrap_err();
    assert!(
        matches!(&error, ClientError::Response(ResponseError::Problem { details, .. }) if details.error_type == Some(ErrorType::InvalidRequest)),
        "{error:?}"
    );
}
