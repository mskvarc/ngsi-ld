<h1 align="center">NGSI-LD</h1>

<p align="center">
  <strong>A typed, ergonomic <a href="https://ngsi-ld.org/">NGSI-LD</a> library for Rust.</strong><br>
  Built on ETSI GS CIM 009 v1.9.1, and works with any conformant broker.
</p>

<p align="center">
  <a href="#overview">Overview</a> ·
  <a href="#getting-started">Getting Started</a> ·
  <a href="#features">Features</a> ·
  <a href="#broker-compatibility">Broker Compatibility</a> ·
  <a href="#licence">Licence</a>
</p>

<p align="center">
  <a href="https://crates.io/crates/ngsi-ld"><img alt="crates.io" src="https://img.shields.io/crates/v/ngsi-ld"></a>
  <a href="https://docs.rs/ngsi-ld"><img alt="docs.rs" src="https://img.shields.io/docsrs/ngsi-ld"></a>
  <a href="#licence"><img alt="licence: MIT OR Apache-2.0" src="https://img.shields.io/badge/licence-MIT%20OR%20Apache--2.0-blue"></a>
  <img alt="msrv: 1.96" src="https://img.shields.io/badge/msrv-1.96-orange">
  <img alt="ngsi-ld v1.9.1" src="https://img.shields.io/badge/ngsi--ld-v1.9.1-8a2be2">
</p>

<p align="center">
  <a href="https://github.com/mskvarc/ngsi-ld/actions/workflows/tests.yaml"><img alt="tests" src="https://github.com/mskvarc/ngsi-ld/actions/workflows/tests.yaml/badge.svg"></a>
  <a href="https://github.com/mskvarc/ngsi-ld/actions/workflows/clippy.yaml"><img alt="clippy" src="https://github.com/mskvarc/ngsi-ld/actions/workflows/clippy.yaml/badge.svg"></a>
  <a href="https://github.com/mskvarc/ngsi-ld/actions/workflows/deny.yaml"><img alt="deny" src="https://github.com/mskvarc/ngsi-ld/actions/workflows/deny.yaml/badge.svg"></a>
</p>

---

## Overview

NGSI-LD describes the world as **entities**, like vehicles, parking sites or sensors, with their properties and the relationships between them. A context broker stores them, answers queries and notifies subscribers when they change. This crate lets Rust code do all of that against any conformant broker. It also works for a context source answering the requests a broker forwards.

The types follow the specification. Builders don't compile without their required members, a request can't carry members only a broker writes, and queries are typed syntax trees instead of strings.

## Getting Started

```toml
[dependencies]
anyhow = "1.0.104"
iri-rs = { version = "3.4.1", features = ["static"] }
ngsi-ld = "1.0.0"
tokio = { version = "1.54.1", features = ["macros", "rt-multi-thread"] }
url = "2.5.8"
```

Create an entity, then look for fast vehicles of other brands:

```rust
use iri_rs::iri;
use ngsi_ld::{
    client::{Client, ReqwestTransport},
    model::Entity,
    query::q,
};
use url::Url;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = Client::builder()
        .base_url(Url::parse("http://localhost:9090/ngsi-ld/v1/")?)
        .transport(ReqwestTransport::try_default()?)
        .build()?;

    let vehicle = Entity::builder()
        .id(iri!("urn:ngsi-ld:Vehicle:A4567"))
        .entity_type("Vehicle")
        .property("brandName", "Mercedes")
        .property("speed", 80)
        .build()?;
    client.entities().create(&vehicle).send().await?;

    let page = client
        .entities()
        .query()
        .entity_type("Vehicle")
        .q(q::attr("speed").gt(50).and(q::attr("brandName").ne("Mercedes")))
        .limit(20)
        .send()
        .await?;
    println!("{} fast vehicles of other brands", page.items().len());
    Ok(())
}
```

More complete programs are in [examples](examples/), and the [live testing guide](docs/live-testing.md) shows how to run the ones that need a broker.

## Features

The defaults give you the client over `reqwest`, with TLS through rustls. For native TLS, set `default-features = false` and enable `reqwest-native-tls`, plus `reqwest-http2` and `reqwest-system-proxy` if you want them. With no features at all you get the data model and the query languages, and no HTTP client or async runtime.

| Feature | Default | Enables |
|---|---|---|
| `http` | yes | The HTTP binding and the client, with no transport |
| `reqwest` | yes | A transport over `reqwest`, with no TLS |
| `reqwest-rustls` | yes | TLS through rustls |
| `reqwest-http2` | yes | HTTP/2 |
| `reqwest-system-proxy` | yes | The system proxy settings |
| `reqwest-native-tls` | no | TLS through the platform's native library |
| `linked-data` | no | JSON-LD processing of the `@context` |
| `chrono` | no | Conversions to and from `chrono` types |
| `geo-types` | no | Conversions to and from `geo-types` geometries |
| `geojson` | no | Conversions to and from `geojson` geometries |

## Broker Compatibility

The crate implements NGSI-LD as the specification defines it and expects the broker to do the same. Brokers often deviate, mostly in the less used parts of the API, and the crate doesn't work around any of them. An operation that runs into a deviation fails with an error. Only conformant brokers are supported for now. A way to tell the client which broker it talks to, so it can handle that broker's known quirks, is planned.

## Documentation

The API reference is on [docs.rs](https://docs.rs/ngsi-ld). The [architecture overview](docs/architecture.md) explains how the crate is built, and the [live testing guide](docs/live-testing.md) covers running it against a real broker.

## Community

Report bugs and suggest features in [issues](https://github.com/mskvarc/ngsi-ld/issues), and ask questions in [discussions](https://github.com/mskvarc/ngsi-ld/discussions). Everyone taking part follows the [Code of Conduct](CODE_OF_CONDUCT.md).

## Licence

Licensed under either of the [Apache License, Version 2.0](LICENSE-APACHE-2.0.md) or the [MIT licence](LICENSE-MIT.md), at your option.

A few files keep their own terms. The [HTTP header](src/http/headers/) files derived from [hyperium/headers](https://github.com/hyperium/headers) keep its MIT notice. The [bundled JSON-LD contexts](src/linked_data/core_context/), the [Annex C fixtures](tests/golden/etsi/) and their [corrected copies](tests/golden/etsi-corrected/) are under ETSI's BSD-3-Clause licence, with the W3C Software and Document License for one of the contexts. The [authored fixtures](tests/golden/authored/) are under MIT No Attribution.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
