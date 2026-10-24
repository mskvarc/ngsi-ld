//! Register Context Source (NGSI-LD clauses 5.9.2 and 6.8.3.1).

use crate::{
    http::{
        Endpoint,
        RequestError,
        ResourceCreated,
        ResponseContext,
        ResponseError,
        operation::{
            Operation,
            sealed::{BuildsRequest, Sealed},
        },
        path::ResourcePath,
        query_string::QueryString,
        request::{RequestParts, scope::RequestScope},
        response::classify,
    },
    model::Input,
    registration::{CSourceRegistration, RegistrationId},
};
use bytes::Bytes;
use http::{Method, Request, Response};

/// Registers a context source.
///
/// Its request is `POST /csourceRegistrations` (NGSI-LD clause 6.8.3.1). A registration sent without an `id` is given
/// one by the broker (clause 5.9.2.4), which [`ResourceCreated::id`] reads from the `Location` of the answer.
///
/// ```
/// use ngsi_ld::{
///     http::{
///         Endpoint,
///         operation::{Operation, RegisterContextSource},
///     },
///     registration::{CSourceRegistration, RegistrationInfo},
/// };
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
///     .build()?;
/// let vehicles = RegistrationInfo::builder()
///     .entities(["Vehicle"])
///     .property_names(["speed"])
///     .build()?;
/// let registration = CSourceRegistration::builder("http://my.csource.org:1026")
///     .information([vehicles])
///     .build()?;
/// let request = RegisterContextSource {
///     registration: &registration,
/// }
/// .to_request(&endpoint)?;
/// assert_eq!(request.method(), "POST");
/// assert_eq!(
///     request.uri(),
///     "http://broker.test/ngsi-ld/v1/csourceRegistrations"
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegisterContextSource<'a> {
    /// The registration.
    pub registration: &'a CSourceRegistration<Input>,
}

impl Sealed for RegisterContextSource<'_> {}

impl Operation for RegisterContextSource<'_> {
    type Outcome = ResourceCreated<RegistrationId>;

    fn decode(&self, _: &ResponseContext, response: Response<Bytes>) -> Result<ResourceCreated<RegistrationId>, ResponseError> {
        ResourceCreated::decode(&classify(response)?)
    }
}

impl BuildsRequest for RegisterContextSource<'_> {
    fn scoped_request(&self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
        RequestParts::resource(Method::POST, ResourcePath::registrations(), QueryString::default())
            .with_body(self.registration.to_object())
            .into_request(endpoint, scope)
    }
}
