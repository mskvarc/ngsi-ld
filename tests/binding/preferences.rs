//! The `Prefer` header of clause 6.3.6 on every request of an endpoint that states preferences.

use crate::{
    fixtures::{header_value, vehicle_id},
    retrieve_entity,
    temporal_fixtures::b,
};
use headers::Header;
use ngsi_ld::{
    http::{
        Endpoint,
        headers::{GeoJsonBody, Prefer, SpecVersion},
        operation::{DeleteTemporalEvolution, Operation},
    },
    model::Lossless,
};
use pretty_assertions::assert_eq;
use url::Url;

/// The test endpoint, preferring payloads of version 1.9 and `GeoJSON` bodies without `@context`.
fn preferring() -> Endpoint {
    let prefer = Prefer::builder()
        .version(SpecVersion::new(1, 9))
        .geo_json_body(GeoJsonBody::Json)
        .build()
        .unwrap();
    Endpoint::builder()
        .base_url(Url::parse("http://broker.test/ngsi-ld/v1/").unwrap())
        .prefer(prefer)
        .build()
        .unwrap()
}

#[test]
fn every_request_states_the_endpoint_preferences() {
    let id = vehicle_id();
    let retrieve = retrieve_entity(&id, Lossless::Normalized).to_request(&preferring()).unwrap();
    let forget_id = b();
    let forget = DeleteTemporalEvolution { id: &forget_id }.to_request(&preferring()).unwrap();
    for request in [&retrieve, &forget] {
        assert_eq!(header_value(request, Prefer::name()), Some("ngsi-ld=1.9, body=json"));
    }
    let plain = retrieve_entity(&id, Lossless::Normalized)
        .to_request(
            &Endpoint::builder()
                .base_url(Url::parse("http://broker.test/ngsi-ld/v1/").unwrap())
                .build()
                .unwrap(),
        )
        .unwrap();
    assert_eq!(header_value(&plain, Prefer::name()), None);
}
