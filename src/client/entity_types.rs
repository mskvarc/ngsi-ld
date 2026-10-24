//! The entity type discovery operations (NGSI-LD clauses 5.7.5 to 5.7.7), as builders that finish with `send()`.

use crate::{
    client::{Answering, ClientError, OutcomeOnly, Transport, route::Route},
    http::operation::{RetrieveEntityTypeDetails, RetrieveEntityTypeInfo, RetrieveEntityTypes, SummaryDetails},
    model::{
        EntityTypeName,
        Tenant,
        discovery::{EntityTypeDetails, EntityTypeInfo, EntityTypeList},
    },
    query::Locality,
};
use bon::bon;

/// The entity type discovery operations of a [`Client`](crate::client::Client), from
/// [`Client::entity_types`](crate::client::Client::entity_types), named after the operations "Retrieve Available Entity
/// Types" and "Retrieve Available Entity Type Information".
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
/// #     async fn send(
/// #         &self,
/// #         request: Request<Bytes>,
/// #     ) -> Result<Response<Bytes>, Infallible> {
/// #         let body: &'static [u8] = if request.uri().path().ends_with("/types") {
/// #             concat!(
/// #                 r#"{"id":"urn:ngsi-ld:EntityTypeList:1","#,
/// #                 r#""type":"EntityTypeList","typeList":["Vehicle"]}"#
/// #             )
/// #             .as_bytes()
/// #         } else {
/// #             concat!(
/// #                 r#"{"id":"https://example.org/Vehicle","#,
/// #                 r#""type":"EntityTypeInfo","attributeDetails":[],"#,
/// #                 r#""entityCount":2,"typeName":"Vehicle"}"#
/// #             )
/// #             .as_bytes()
/// #         };
/// #         Ok(Response::new(Bytes::from_static(body)))
/// #     }
/// # }
/// use ngsi_ld::client::Client;
///
/// async fn count_vehicles(client: &Client<Offline>) -> anyhow::Result<Option<u64>> {
///     let types = client.entity_types().list().send().await?;
///     let Some(vehicle) = types
///         .type_list
///         .iter()
///         .find(|name| name.as_ref() == "Vehicle")
///     else {
///         return Ok(None);
///     };
///     let info = client.entity_types().retrieve(vehicle).send().await?;
///     Ok(Some(info.entity_count))
/// }
/// ```
#[derive(Debug, Clone, Copy)]
pub struct AvailableEntityTypes<'c, T, M = OutcomeOnly> {
    /// Where the calls go: the client, and the snapshot they are aimed at.
    route: Route<'c, T, M>,
}

impl<'c, T, M> AvailableEntityTypes<'c, T, M> {
    /// The handle sending its calls through `route`.
    pub(crate) const fn new(route: Route<'c, T, M>) -> Self {
        Self { route }
    }
}

#[bon]
impl<T: Transport, M: Answering> AvailableEntityTypes<'_, T, M> {
    /// Retrieve Available Entity Types (clause 6.25.3.1): the names of the entity types the broker holds instances
    /// of, short where the client's context defines them.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the answer is not an entity type list or the call fails.
    #[builder(finish_fn = send)]
    pub async fn list(
        &self,
        #[builder(default)] details: SummaryDetails,
        #[builder(default)] locality: Locality,
        tenant: Option<&Tenant>,
    ) -> Result<M::Answer<EntityTypeList>, ClientError<T::Error>> {
        self.route
            .send(&RetrieveEntityTypes::builder().details(details).locality(locality).build(), tenant)
            .await
    }

    /// Retrieve Details of Available Entity Types (clause 6.25.3.1 with `details=true`): every entity type with the
    /// names of the attributes its instances have.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when an element of the answer is not an entity type or the call fails.
    #[builder(finish_fn = send)]
    pub async fn list_details(
        &self,
        #[builder(default)] locality: Locality,
        tenant: Option<&Tenant>,
    ) -> Result<M::Answer<Vec<EntityTypeDetails>>, ClientError<T::Error>> {
        self.route.send(&RetrieveEntityTypeDetails::builder().locality(locality).build(), tenant).await
    }

    /// Retrieve Available Entity Type Information (clause 6.26.3.1): what the broker holds of `entity_type`, named
    /// by its short name or its Fully Qualified Name.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the broker holds no instance of the type (`ResourceNotFound`), the answer is
    /// not entity type information, or the call fails.
    #[builder(finish_fn = send)]
    pub async fn retrieve(
        &self,
        #[builder(start_fn)] entity_type: &EntityTypeName,
        #[builder(default)] locality: Locality,
        tenant: Option<&Tenant>,
    ) -> Result<M::Answer<EntityTypeInfo>, ClientError<T::Error>> {
        self.route
            .send(&RetrieveEntityTypeInfo::builder().entity_type(entity_type).locality(locality).build(), tenant)
            .await
    }
}
