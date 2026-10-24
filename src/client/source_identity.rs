//! Retrieve Context Source Identity Information (NGSI-LD clause 5.15.1), as a builder that finishes with `send()`.

use crate::{
    client::{Answering, Client, ClientError, Transport},
    http::operation::RetrieveSourceIdentity,
    model::{ContextSourceIdentity, Tenant},
};
use bon::bon;

#[bon]
impl<T: Transport, M: Answering> Client<T, M> {
    /// Retrieve Context Source Identity Information (clause 6.33.3.1): the identity of the broker, or of the
    /// tenant the call names (clause 5.15.1.1).
    ///
    /// ```
    /// # use bytes::Bytes;
    /// # use http::{Request, Response};
    /// # use std::convert::Infallible;
    /// # struct Offline;
    /// # impl ngsi_ld::client::Transport for Offline {
    /// #     type Error = Infallible;
    /// #     async fn send(&self, _: Request<Bytes>) -> Result<Response<Bytes>, Infallible> {
    /// #         Ok(Response::new(Bytes::from_static(br#"{"id":"urn:ngsi-ld:ContextSource:cs1","type":"ContextSourceIdentity","contextSourceAlias":"broker-1","contextSourceUptime":"P1DT2H","contextSourceTimeAt":"2024-05-01T10:00:00Z"}"#)))
    /// #     }
    /// # }
    /// use ngsi_ld::client::Client;
    ///
    /// async fn alias(client: &Client<Offline>) -> anyhow::Result<String> {
    ///     let identity = client.source_identity().send().await?;
    ///     Ok(identity.alias.to_string())
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the broker cannot supply its identity (501, clause 5.15.1.4), the answer is not
    /// a context source identity, or the call fails.
    #[builder(finish_fn = send)]
    pub async fn source_identity(&self, tenant: Option<&Tenant>) -> Result<M::Answer<ContextSourceIdentity>, ClientError<T::Error>> {
        self.send_on_tenant(&RetrieveSourceIdentity, tenant).await
    }
}
