//! The forwarded operations on one entity: a Retrieve Entity and an Append Attributes with `noOverwrite`.

use crate::forwarding::{forwarded, query_string};
use http::Method;
use ngsi_ld::{
    http::{
        NotUpdatedDetails,
        ResponseContext,
        UpdateResult,
        answer,
        headers::Link,
        operation::{AppendAttributes, AppendAttributesParameters, Operation, RetrieveEntity, RetrieveEntityParameters},
    },
    model::{AttributeName, Entity, EntityFragment, EntityId, Input, Lossless, Projected, Representation},
};
use std::str;

/// The fragment of the forwarded Append Attributes: one attribute the source holds, and one it does not.
const APPEND_BODY: &str = r#"{"speed": {"type": "Property", "value": 90}, "color": {"type": "Property", "value": "white"}}"#;

/// Reads and answers a forwarded `GET /entities/{entityId}?omit=brandName&format=concise`.
pub(crate) fn retrieval(vehicle: &Entity<Input>) -> anyhow::Result<()> {
    let request = forwarded(Method::GET, "/entities/urn:ngsi-ld:Vehicle:A4567?omit=brandName&format=concise", None)?;
    let id: EntityId = request.uri().path().trim_start_matches("/ngsi-ld/v1/entities/").parse()?;
    let received = RetrieveEntityParameters::parse(query_string(&request))?;
    println!(
        "retrieval read: {id}, omit {:?}, {:?}",
        received.omit.as_ref().map(ToString::to_string),
        received.representation
    );
    // The answer leaves out the brandName the request omits.
    let brand_name: AttributeName = "brandName".parse()?;
    let mut kept = vehicle.clone();
    kept.attributes.remove(&brand_name);
    let operation = RetrieveEntity::builder()
        .id(&id)
        .maybe_omit(received.omit.as_ref())
        .system_attributes(received.system_attributes)
        .spelling(received.spelling)
        .format(Lossless::Concise)
        .build();
    let response = answer::document(&kept.render(Representation::Concise), &Link::try_core_context()?);
    let retrieved = operation.decode(&ResponseContext::new(&request), response)?.into_body();
    println!("retrieval answered: {}", retrieved.render(Representation::Concise));
    Ok(())
}

/// Reads and answers a forwarded `POST /entities/{entityId}/attrs?options=noOverwrite`: the attribute the source
/// does not hold is appended, and the one it holds is kept and reported as not updated.
pub(crate) fn append(vehicle: &Entity<Input>) -> anyhow::Result<()> {
    let request = forwarded(Method::POST, "/entities/urn:ngsi-ld:Vehicle:A4567/attrs?options=noOverwrite", Some(APPEND_BODY))?;
    let received = AppendAttributesParameters::parse(query_string(&request))?;
    let fragment = EntityFragment::<Projected>::parse(str::from_utf8(request.body())?, Lossless::Normalized)?;
    println!("append read: {} attributes, {:?}", fragment.attributes.len(), received.overwrite);
    let (held, new): (Vec<&AttributeName>, Vec<&AttributeName>) = fragment.attributes.names().partition(|name| vehicle.attribute(name).is_some());
    let result = UpdateResult {
        updated: new.into_iter().cloned().collect(),
        not_updated: held
            .into_iter()
            .map(|name| NotUpdatedDetails {
                attribute_name: name.clone(),
                reason: "the attribute exists and noOverwrite keeps it".to_owned(),
                registration_id: None,
            })
            .collect(),
    };
    let response = answer::update_result(&result);
    let operation = AppendAttributes::builder()
        .id(&vehicle.id)
        .fragment(&fragment)
        .overwrite(received.overwrite)
        .maybe_entity_type(received.entity_type.as_ref())
        .build();
    let outcome = operation.decode(&ResponseContext::new(&request), response)?;
    println!("append answered: {outcome:?}");
    Ok(())
}
