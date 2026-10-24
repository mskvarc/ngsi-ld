//! Acts as a context source a broker forwards requests to: reads each forwarded request, answers it, and reads the
//! answer back with the operation the broker sent, as the broker does. No network is used.
//!
//! Run it with `cargo run --features http --example context_source`.

mod entity_operations;
mod entity_queries;
mod forwarding;
mod temporal_query;

use iri_rs::iri;
use ngsi_ld::model::Entity;

fn main() -> anyhow::Result<()> {
    let vehicle = Entity::builder()
        .id(iri!("urn:ngsi-ld:Vehicle:A4567"))
        .entity_type("Vehicle")
        .property("brandName", "Mercedes")
        .property("speed", 80)
        .build()?;
    entity_queries::get_query(&vehicle)?;
    entity_queries::post_query(&vehicle)?;
    entity_queries::refused_query()?;
    entity_operations::retrieval(&vehicle)?;
    temporal_query::temporal_query(&vehicle)?;
    entity_operations::append(&vehicle)?;
    Ok(())
}
