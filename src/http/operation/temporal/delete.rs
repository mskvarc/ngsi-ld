//! Delete Temporal Evolution of an Entity (NGSI-LD clauses 5.6.16 and 6.19.3.2).

use crate::{http::path::ResourcePath, model::EntityId};

/// Deletes the temporal evolution of an entity.
///
/// Its request is `DELETE /temporal/entities/{entityId}` (NGSI-LD clause 6.19.3.2).
///
/// ```
/// use iri_rs::iri;
/// use ngsi_ld::{
///     http::{
///         Endpoint,
///         operation::{DeleteTemporalEvolution, Operation},
///     },
///     model::EntityId,
/// };
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let id = EntityId::from(iri!("urn:ngsi-ld:Vehicle:B9211"));
/// let request = DeleteTemporalEvolution { id: &id }.to_request(&endpoint)?;
/// assert_eq!(request.method(), "DELETE");
/// assert_eq!(
///     request.uri(),
///     "http://broker.test/ngsi-ld/v1/temporal/entities/urn:ngsi-ld:Vehicle:B9211"
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeleteTemporalEvolution<'a> {
    /// The entity.
    pub id: &'a EntityId,
}

delete_resource!(DeleteTemporalEvolution => ResourcePath::temporal_entity);
