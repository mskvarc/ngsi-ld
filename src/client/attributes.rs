//! The attribute discovery operations (NGSI-LD clauses 5.7.8 to 5.7.10), as builders that finish with `send()`.

use crate::{
    client::{Answering, ClientError, OutcomeOnly, Transport, route::Route},
    http::operation::{RetrieveAttributeDetails, RetrieveAttributeInfo, RetrieveAttributes, SummaryDetails},
    model::{
        AttributeName,
        Tenant,
        discovery::{AttributeDetails, AttributeInfo, AttributeList},
    },
    query::Locality,
};
use bon::bon;

/// The attribute discovery operations of a [`Client`](crate::client::Client).
///
/// [`Client::attributes`](crate::client::Client::attributes) returns them. They are named after the operations
/// "Retrieve Available Attributes" and "Retrieve Available Attribute Information".
///
/// Each operation is a builder that finishes with `send()`.
///
/// ```
/// # use bytes::Bytes;
/// # use http::{Request, Response};
/// # use std::convert::Infallible;
/// # struct Offline;
/// # impl ngsi_ld::client::Transport for Offline {
/// #     type Error = Infallible;
/// #     async fn send(&self, _: Request<Bytes>) -> Result<Response<Bytes>, Infallible> {
/// #         Ok(Response::new(Bytes::from_static(
/// #             concat!(
/// #                 r#"{"id":"https://example.org/speed","type":"Attribute","#,
/// #                 r#""attributeName":"speed","typeNames":["Vehicle"]}"#
/// #             )
/// #             .as_bytes(),
/// #         )))
/// #     }
/// # }
/// use ngsi_ld::{
///     client::Client,
///     model::{AttributeName, EntityTypeName},
/// };
///
/// async fn types_with_speed(client: &Client<Offline>) -> anyhow::Result<Vec<EntityTypeName>> {
///     let speed: AttributeName = "speed".parse()?;
///     let info = client.attributes().retrieve(&speed).send().await?;
///     Ok(info.type_names.unwrap_or_default())
/// }
/// ```
#[derive(Debug, Clone, Copy)]
pub struct AvailableAttributes<'c, T, M = OutcomeOnly> {
    /// Where the calls go: the client, and the snapshot they are aimed at.
    route: Route<'c, T, M>,
}

impl<'c, T, M> AvailableAttributes<'c, T, M> {
    /// The handle sending its calls through `route`.
    pub(crate) const fn new(route: Route<'c, T, M>) -> Self {
        Self { route }
    }
}

#[bon]
impl<T: Transport, M: Answering> AvailableAttributes<'_, T, M> {
    /// Retrieve Available Attributes (clause 6.27.3.1): the names of the attributes the broker holds instances of,
    /// short where the client's context defines them.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the answer is not an attribute list or the call fails.
    #[builder(finish_fn = send)]
    pub async fn list(
        &self,
        #[builder(default)] details: SummaryDetails,
        #[builder(default)] locality: Locality,
        tenant: Option<&Tenant>,
    ) -> Result<M::Answer<AttributeList>, ClientError<T::Error>> {
        self.route
            .send(&RetrieveAttributes::builder().details(details).locality(locality).build(), tenant)
            .await
    }

    /// Retrieve Details of Available Attributes (clause 6.27.3.1 with `details=true`): every attribute with the
    /// entity types whose instances have it.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when an element of the answer is not an attribute or the call fails.
    #[builder(finish_fn = send)]
    pub async fn list_details(
        &self,
        #[builder(default)] locality: Locality,
        tenant: Option<&Tenant>,
    ) -> Result<M::Answer<Vec<AttributeDetails>>, ClientError<T::Error>> {
        self.route.send(&RetrieveAttributeDetails::builder().locality(locality).build(), tenant).await
    }

    /// Retrieve Available Attribute Information (clause 6.28.3.1): what the broker holds of `attribute`, named by
    /// its short name or its Fully Qualified Name.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the broker holds no instance of the attribute (`ResourceNotFound`), the answer
    /// is not attribute information, or the call fails.
    #[builder(finish_fn = send)]
    pub async fn retrieve(
        &self,
        #[builder(start_fn)] attribute: &AttributeName,
        #[builder(default)] locality: Locality,
        tenant: Option<&Tenant>,
    ) -> Result<M::Answer<AttributeInfo>, ClientError<T::Error>> {
        self.route
            .send(&RetrieveAttributeInfo::builder().attribute(attribute).locality(locality).build(), tenant)
            .await
    }
}
