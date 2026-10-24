//! Delete `EntityMap` (NGSI-LD clauses 5.14.3 and 6.32.3.3).

use crate::{http::path::ResourcePath, registration::EntityMapId};

/// `DELETE /entityMaps/{entityMapId}`: removes an entity map from the broker (NGSI-LD clause 6.32.3.3).
///
/// ```
/// use iri_rs::iri;
/// use ngsi_ld::{
///     http::{
///         Endpoint,
///         operation::{DeleteEntityMap, Operation},
///     },
///     registration::EntityMapId,
/// };
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let id = EntityMapId::from(iri!("urn:ngsi-ld:EntityMap:1"));
/// let request = DeleteEntityMap { id: &id }.to_request(&endpoint)?;
/// assert_eq!(request.method(), "DELETE");
/// assert_eq!(
///     request.uri(),
///     "http://broker.test/ngsi-ld/v1/entityMaps/urn:ngsi-ld:EntityMap:1"
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeleteEntityMap<'a> {
    /// The entity map.
    pub id: &'a EntityMapId,
}

delete_resource!(DeleteEntityMap => ResourcePath::entity_map);
