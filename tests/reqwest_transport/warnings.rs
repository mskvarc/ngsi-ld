//! The `NGSILD-Warning` header of clause 6.3.17, read by the client from the wire.

use crate::fixtures::client;
use headers::Header;
use ngsi_ld::http::headers::{NgsiLdWarning, NgsiLdWarnings, WarningCode};
use pretty_assertions::assert_eq;
use wiremock::{
    Mock,
    MockServer,
    ResponseTemplate,
    matchers::{method, path, query_param},
};

#[tokio::test]
async fn a_query_answered_from_a_cache_reports_the_stale_warning() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/ngsi-ld/v1/entities"))
        .and(query_param("type", "Vehicle"))
        .respond_with(
            ResponseTemplate::new(200)
                .append_header(NgsiLdWarnings::name(), "110 - Response is Stale")
                .set_body_string("[]"),
        )
        .expect(1)
        .mount(&server)
        .await;
    let page = client(&server.uri()).entities().query().entity_type("Vehicle").send().await.unwrap();
    let codes: Vec<WarningCode> = page
        .warnings()
        .valid()
        .iter()
        .flat_map(|warnings| warnings.iter())
        .map(NgsiLdWarning::code)
        .collect();
    assert_eq!(codes, [WarningCode::ResponseIsStale]);
}
