//! The operations on the `@context`s a broker stores and serves (NGSI-LD clause 5.13), as builders that finish
//! with `send()`.

use crate::{
    client::{Answering, Client, ClientError, OutcomeOnly, Transport},
    http::{
        ResourceCreated,
        operation::{
            AddContext,
            ContextDeletion,
            ContextKindFilter,
            DeleteContext,
            ListContextDetails,
            ListContexts,
            ServeContext,
            ServeContextDetails,
            SummaryDetails,
        },
    },
    model::{Context, ContextId, ContextMetadata, ExtraMemberName, Tenant, Value},
};
use bon::bon;
use indexmap::IndexMap;
use url::Url;

/// The stored `@context` operations of a [`Client`], from [`Client::contexts`], named after clause 5.13, "Storing,
/// Managing and Serving @contexts".
///
/// Each operation is a builder that finishes with `send()`.
///
/// ```
/// # use bytes::Bytes;
/// # use http::{Method, Request, Response, StatusCode, header::LOCATION};
/// # use std::convert::Infallible;
/// # struct Offline;
/// # impl ngsi_ld::client::Transport for Offline {
/// #     type Error = Infallible;
/// #     async fn send(
/// #         &self,
/// #         request: Request<Bytes>,
/// #     ) -> Result<Response<Bytes>, Infallible> {
/// #         let mut response = Response::new(request.body().clone());
/// #         if request.method() == Method::POST {
/// #             *response.status_mut() = StatusCode::CREATED;
/// #             response.headers_mut().insert(
/// #                 LOCATION,
/// #                 http::HeaderValue::from_static(
/// #                     "/ngsi-ld/v1/jsonldContexts/urn:ngsi-ld:context:c1",
/// #                 ),
/// #             );
/// #         } else if request.method() == Method::DELETE {
/// #             *response.status_mut() = StatusCode::NO_CONTENT;
/// #         } else {
/// #             *response.body_mut() = Bytes::from_static(
/// #                 br#"{"@context":{"speed":"https://example.org/speed"}}"#,
/// #             );
/// #         }
/// #         Ok(response)
/// #     }
/// # }
/// use anyhow::Context as _;
/// use ngsi_ld::{
///     client::Client,
///     model::{Context, ContextEntry, InlineContext},
/// };
///
/// async fn host_context(client: &Client<Offline>) -> anyhow::Result<()> {
///     let context = Context::Single(ContextEntry::Inline(
///         r#"{"speed":"https://example.org/speed"}"#.parse::<InlineContext>()?,
///     ));
///     let stored = client.contexts().add(&context).send().await?;
///     let id = stored.id().context("the broker named no context")?;
///     assert_eq!(client.contexts().serve(&id).send().await?, context);
///     client.contexts().delete(&id).send().await?;
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone, Copy)]
pub struct StoredContexts<'c, T, M = OutcomeOnly> {
    /// The client the calls go through.
    client: &'c Client<T, M>,
}

impl<'c, T, M> StoredContexts<'c, T, M> {
    /// The handle of `client`.
    pub(crate) const fn new(client: &'c Client<T, M>) -> Self {
        Self { client }
    }
}

#[bon]
impl<T: Transport, M: Answering> StoredContexts<'_, T, M> {
    /// Add @context (clause 6.29.3.1): stores `context` at the broker, which serves it as a `Hosted` context;
    /// [`ResourceCreated::id`] reads the identifier the broker gave it. `extra_members` are sent beside `@context`,
    /// in order, and discarded by the broker (clause 5.13.2.3); the map holds each name once.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the broker refuses the context (`BadRequestData`) or the call fails.
    #[builder(finish_fn = send)]
    pub async fn add(
        &self,
        #[builder(start_fn)] context: &Context,
        extra_members: Option<&IndexMap<ExtraMemberName, Value>>,
        tenant: Option<&Tenant>,
    ) -> Result<M::Answer<ResourceCreated<ContextId>>, ClientError<T::Error>> {
        let add = AddContext::builder().context(context).maybe_extra_members(extra_members).build();
        self.client.send_on_tenant(&add, tenant).await
    }

    /// List @contexts (clause 6.29.3.2): the URLs of the stored contexts, of `kind` only when it is given;
    /// `details` states `details=false` when it is [`SummaryDetails::Explicit`].
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the answer is not a list of URLs or the call fails.
    #[builder(finish_fn = send)]
    pub async fn list(
        &self,
        kind: Option<ContextKindFilter>,
        #[builder(default)] details: SummaryDetails,
        tenant: Option<&Tenant>,
    ) -> Result<M::Answer<Vec<Url>>, ClientError<T::Error>> {
        self.client
            .send_on_tenant(&ListContexts::builder().maybe_kind(kind).details(details).build(), tenant)
            .await
    }

    /// List @contexts with `details=true` (clause 6.29.3.2): the metadata of the stored contexts, of `kind` only
    /// when it is given.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when an element of the answer is not context metadata or the call fails.
    #[builder(finish_fn = send)]
    pub async fn list_details(
        &self,
        kind: Option<ContextKindFilter>,
        tenant: Option<&Tenant>,
    ) -> Result<M::Answer<Vec<ContextMetadata>>, ClientError<T::Error>> {
        self.client
            .send_on_tenant(&ListContextDetails::builder().maybe_kind(kind).build(), tenant)
            .await
    }

    /// Serve @context (clause 6.30.3.1): the stored context `id`; `details` states `details=false` when it is
    /// [`SummaryDetails::Explicit`].
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the context is unknown (`ResourceNotFound`) or `Cached`
    /// (`OperationNotSupported`), the answer is not a local context document, or the call fails.
    #[builder(finish_fn = send)]
    pub async fn serve(
        &self,
        #[builder(start_fn)] id: &ContextId,
        #[builder(default)] details: SummaryDetails,
        tenant: Option<&Tenant>,
    ) -> Result<M::Answer<Context>, ClientError<T::Error>> {
        self.client
            .send_on_tenant(&ServeContext::builder().id(id).details(details).build(), tenant)
            .await
    }

    /// Serve @context with `details=true` (clause 6.30.3.1): the metadata of the stored context `id`.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the context is unknown (`ResourceNotFound`), the answer is not context metadata,
    /// or the call fails.
    #[builder(finish_fn = send)]
    pub async fn serve_details(
        &self,
        #[builder(start_fn)] id: &ContextId,
        tenant: Option<&Tenant>,
    ) -> Result<M::Answer<ContextMetadata>, ClientError<T::Error>> {
        self.client.send_on_tenant(&ServeContextDetails { id }, tenant).await
    }

    /// Delete and Reload @context (clause 6.30.3.2): removes the stored context `id`, or does what `deletion`
    /// says: stating `reload=false` ([`ContextDeletion::RemoveExplicitly`]), or downloading a `Cached` context again
    /// and replacing the stored copy ([`ContextDeletion::Reload`]).
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the context is unknown (`ResourceNotFound`), a reload fails, or the call fails.
    #[builder(finish_fn = send)]
    pub async fn delete(
        &self,
        #[builder(start_fn)] id: &ContextId,
        #[builder(default)] deletion: ContextDeletion,
        tenant: Option<&Tenant>,
    ) -> Result<M::Answer<()>, ClientError<T::Error>> {
        self.client
            .send_on_tenant(&DeleteContext::builder().id(id).deletion(deletion).build(), tenant)
            .await
    }
}
