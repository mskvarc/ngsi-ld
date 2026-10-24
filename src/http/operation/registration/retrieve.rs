//! Retrieve Context Source Registration (NGSI-LD clauses 5.10.1 and 6.9.3.1).

use crate::{
    http::{
        Endpoint,
        RequestError,
        ResponseContext,
        ResponseError,
        operation::{
            Operation,
            SystemAttributes,
            registration::READER,
            sealed::{BuildsRequest, Sealed},
        },
        path::ResourcePath,
        query_string::QueryString,
        request::{RequestParts, scope::RequestScope},
    },
    model::{NgsiLdMediaType, Output},
    registration::{CSourceRegistration, RegistrationId},
};
use bytes::Bytes;
use http::{Method, Request, Response};

/// `GET /csourceRegistrations/{registrationId}`: reads a registration (NGSI-LD clause 6.9.3.1).
///
/// The registration is read strictly: a member Tables 5.2.9-1 and 5.2.9-2 do not define fails the response.
///
/// ```
/// use iri_rs::iri;
/// use ngsi_ld::{http::{Endpoint, operation::{Operation, SystemAttributes, RetrieveRegistration}}, registration::RegistrationId};
/// use url::Url;
///
/// let endpoint = Endpoint::builder().base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?).build()?;
/// let id = RegistrationId::from(iri!("urn:ngsi-ld:ContextSourceRegistration:csr1a3456"));
/// let request = RetrieveRegistration::builder().id(&id).system_attributes(SystemAttributes::Include).build().to_request(&endpoint)?;
/// assert_eq!(request.method(), "GET");
/// assert_eq!(request.uri(), "http://broker.test/ngsi-ld/v1/csourceRegistrations/urn:ngsi-ld:ContextSourceRegistration:csr1a3456?options=sysAttrs");
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, bon::Builder)]
pub struct RetrieveRegistration<'a> {
    /// The registration.
    pub id: &'a RegistrationId,
    /// Whether `createdAt` and `modifiedAt` are included (clause 6.3.11 extends `options=sysAttrs` to
    /// `/csourceRegistrations/`).
    #[builder(default)]
    pub system_attributes: SystemAttributes,
}

impl Sealed for RetrieveRegistration<'_> {}

impl Operation for RetrieveRegistration<'_> {
    type Outcome = CSourceRegistration<Output>;

    fn decode(&self, _: &ResponseContext, response: Response<Bytes>) -> Result<CSourceRegistration<Output>, ResponseError> {
        READER.decode_one(response)
    }
}

impl BuildsRequest for RetrieveRegistration<'_> {
    fn scoped_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
        let mut query = QueryString::default();
        self.system_attributes.push_query(&mut query);
        RequestParts::resource(Method::GET, ResourcePath::registration(self.id), query)
            .accepting(NgsiLdMediaType::Json)
            .into_request(endpoint, scope)
    }
}
