//! Update Context Source Registration (NGSI-LD clauses 5.9.3 and 6.9.3.2).

use crate::{
    http::{
        Endpoint,
        RequestError,
        ResponseContext,
        ResponseError,
        operation::{
            Operation,
            no_content::decode_no_content,
            sealed::{BuildsRequest, Sealed},
        },
        path::ResourcePath,
        query_string::QueryString,
        request::{RequestParts, scope::RequestScope},
    },
    registration::{RegistrationFragment, RegistrationId},
};
use bytes::Bytes;
use http::{Method, Request, Response};

/// Applies a fragment to a registration.
///
/// Its request is `PATCH /csourceRegistrations/{registrationId}` (NGSI-LD clause 6.9.3.2).
///
/// The body is the fragment alone, with its `type` and without an `id`: clause 5.9.3.3 takes the target from the
/// path and the changes from a Context Source Registration Fragment (clause 5.4), and Table 6.9.3.2-1, unlike
/// Table 6.11.3.2-1 for subscriptions, does not ask the fragment to repeat the `id`.
///
/// ```
/// use iri_rs::iri;
/// use ngsi_ld::{
///     http::{
///         Endpoint,
///         operation::{Operation, UpdateRegistration},
///     },
///     model::DateTime,
///     registration::{RegistrationFragment, RegistrationId},
/// };
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let id = RegistrationId::from(iri!("urn:ngsi-ld:ContextSourceRegistration:csr1a3456"));
/// let postpone = RegistrationFragment::builder()
///     .expires_at("2030-01-01T00:00:00Z".parse::<DateTime>()?)
///     .build()?;
/// let request = UpdateRegistration {
///     id: &id,
///     fragment: &postpone,
/// }
/// .to_request(&endpoint)?;
/// assert_eq!(request.method(), "PATCH");
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
pub struct UpdateRegistration<'a> {
    /// The registration.
    pub id: &'a RegistrationId,
    /// The changes.
    pub fragment: &'a RegistrationFragment,
}

impl Sealed for UpdateRegistration<'_> {}

impl Operation for UpdateRegistration<'_> {
    type Outcome = ();

    fn decode(&self, _: &ResponseContext, response: Response<Bytes>) -> Result<(), ResponseError> {
        decode_no_content(response)
    }
}

impl BuildsRequest for UpdateRegistration<'_> {
    fn scoped_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
        RequestParts::resource(Method::PATCH, ResourcePath::registration(self.id), QueryString::default())
            .with_body(self.fragment.to_object())
            .into_request(endpoint, scope)
    }
}
