//! Delete Context Source Registration (NGSI-LD clauses 5.9.4 and 6.9.3.3).

use crate::{http::path::ResourcePath, registration::RegistrationId};

/// `DELETE /csourceRegistrations/{registrationId}`: deletes a registration (NGSI-LD clause 6.9.3.3).
///
/// ```
/// use iri_rs::iri;
/// use ngsi_ld::{
///     http::{
///         Endpoint,
///         operation::{DeleteRegistration, Operation},
///     },
///     registration::RegistrationId,
/// };
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let id = RegistrationId::from(iri!("urn:ngsi-ld:ContextSourceRegistration:csr1a3456"));
/// let request = DeleteRegistration { id: &id }.to_request(&endpoint)?;
/// assert_eq!(request.method(), "DELETE");
/// assert_eq!(
///     request.uri(),
///     concat!(
///         "http://broker.test/ngsi-ld/v1/csourceRegistrations",
///         "/urn:ngsi-ld:ContextSourceRegistration:csr1a3456"
///     )
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeleteRegistration<'a> {
    /// The registration.
    pub id: &'a RegistrationId,
}

delete_resource!(DeleteRegistration => ResourcePath::registration);
