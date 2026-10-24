//! A [`RemoteContextLoader`] over the client's [`Transport`], for the remote contexts an `@context` references.

use crate::{
    client::{Client, ClientError, Transport},
    http::headers::{Accept, any_content_type::any_media_type},
    linked_data::RemoteContextLoader,
    model::{ContextId, InvalidJson, NgsiLdMediaType, RawJson, keyword::CONTEXT},
};
use bon::bon;
use bytes::Bytes;
use headers::HeaderMapExt;
use http::{Request, StatusCode};
use iri_rs::Iri;
use jstrict::{Object, Value as Json, parse::parse_slice_value};
use mediatype::{MediaType, MediaTypeBuf, names};

/// Where a [`ContextLoader`] fetches a context. The two sources are the only ones the loader has, so the enum is
/// exhaustive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ContextSource {
    /// From the context's own URL, as the JSON-LD 1.1 API `LoadDocumentCallback` does (section 9.4).
    #[default]
    Direct,
    /// From the broker, through Serve @context (clause 6.30.3.1) with the context's URL as its identifier; the
    /// client's tenant and headers apply.
    Broker,
}

/// Whether a [`ContextLoader`] fetching a context [`Direct`](ContextSource::Direct)ly requires a JSON media type.
/// A loader either checks or does not, so the enum is exhaustive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum MediaTypeCheck {
    /// The response must be `application/json`, `application/ld+json` or another `+json` type, as the JSON-LD 1.1
    /// API requires (section 9.4).
    #[default]
    Required,
    /// Any media type is read as JSON: a raw file host may serve a context as `text/plain`.
    Ignored,
}

/// Why a [`ContextLoader`] could not load a context; `E` is the error of the client's transport.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ContextLoadError<E> {
    /// The context's URL is not one an HTTP request can target.
    #[error("the context URL cannot be requested")]
    Request {
        /// Why the request could not be built.
        #[source]
        source: http::Error,
    },
    /// The transport failed.
    #[error("the transport failed")]
    Transport {
        /// The transport's error.
        #[source]
        source: E,
    },
    /// The server answered with another status than 200.
    #[error("the context was answered with status {status}")]
    Status {
        /// The status of the answer.
        status: StatusCode,
    },
    /// The answer is not JSON by its media type; `found` is `None` when it names none, or none that is valid.
    #[error("the context was answered with a media type that is not JSON")]
    MediaType {
        /// The media type of the answer.
        found: Option<MediaTypeBuf>,
    },
    /// The answer is not JSON.
    #[error("the context is not JSON")]
    NotJson {
        /// Why the body is not JSON.
        #[source]
        source: InvalidJson,
    },
    /// The broker refused to serve the context, or the call failed.
    #[error("the broker did not serve the context")]
    Client {
        /// The client's error.
        #[source]
        source: ClientError<E>,
    },
}

/// A [`RemoteContextLoader`] over a [`Client`]'s transport, built with [`Client::context_loader`].
///
/// The loader fetches a context either from its own URL, as the `LoadDocumentCallback` of the JSON-LD 1.1 API
/// (section 9.4) does, or from the broker the client speaks to, through the Serve @context operation (NGSI-LD
/// clause 6.30.3.1), so a context the broker caches or hosts is read where the broker reads it.
///
/// ```
/// # use bytes::Bytes;
/// # use http::{Request, Response, header::CONTENT_TYPE};
/// # use std::convert::Infallible;
/// # struct Offline;
/// # impl ngsi_ld::client::Transport for Offline {
/// #     type Error = Infallible;
/// #     async fn send(&self, _: Request<Bytes>) -> Result<Response<Bytes>, Infallible> {
/// #         let mut response = Response::new(Bytes::from_static(
/// #             br#"{"@context":{"speed":"https://example.org/speed"}}"#,
/// #         ));
/// #         response
/// #             .headers_mut()
/// #             .insert(CONTENT_TYPE, http::HeaderValue::from_static("text/plain"));
/// #         Ok(response)
/// #     }
/// # }
/// use ngsi_ld::{
///     client::{Client, MediaTypeCheck},
///     linked_data::ContextProcessor,
///     model::{AttributeName, Context},
/// };
/// use url::Url;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> anyhow::Result<()> {
/// let client = Client::builder()
///     .base_url(Url::parse("http://localhost:9090/ngsi-ld/v1/")?)
///     .transport(Offline)
///     .build()?;
/// // Raw files served as `text/plain` hold contexts too.
/// let loader = client
///     .context_loader()
///     .media_type(MediaTypeCheck::Ignored)
///     .build();
/// let context = Context::from(iri_rs::IriRefBuf::new(
///     "https://example.org/context.jsonld",
/// )?);
/// let active = ContextProcessor::try_new()?
///     .process(&context, &loader)
///     .await?;
/// let speed: AttributeName = "speed".parse()?;
/// assert_eq!(active.expand(&speed)?.as_str(), "https://example.org/speed");
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Copy)]
pub struct ContextLoader<'c, T> {
    /// The client whose transport, or whose broker, serves the contexts.
    client: &'c Client<T>,
    /// Where contexts are fetched.
    source: ContextSource,
    /// Whether a directly fetched context must be JSON by its media type.
    media_type: MediaTypeCheck,
}

#[bon]
impl<T: Transport> Client<T> {
    /// Starts a [`ContextLoader`] over this client: `source` says where contexts are fetched, from their own URL
    /// by default, and `media_type` whether a directly fetched one must be JSON by its media type, which it must by
    /// default. The builder finishes with `build()`.
    #[builder(const, finish_fn = build)]
    pub const fn context_loader(
        &self,
        #[builder(default = ContextSource::Direct)] source: ContextSource,
        #[builder(default = MediaTypeCheck::Required)] media_type: MediaTypeCheck,
    ) -> ContextLoader<'_, T> {
        ContextLoader {
            client: self,
            source,
            media_type,
        }
    }
}

/// Whether `media_type` is JSON for the JSON-LD 1.1 API: `application/json`, or any type with the `+json` suffix
/// (section 9.4).
fn is_json(media_type: &MediaType<'_>) -> bool {
    // `Name` compares case-insensitively and cannot appear in a pattern, hence the comparisons.
    (media_type.ty == names::APPLICATION && media_type.subty == names::JSON && media_type.suffix.is_none()) || media_type.suffix == Some(names::JSON)
}

impl<T: Transport> ContextLoader<'_, T> {
    /// Fetches the context from its own URL.
    async fn direct(&self, url: Iri<&str>) -> Result<RawJson, ContextLoadError<T::Error>> {
        // The two media types of the `LoadDocumentCallback`, in its order of preference.
        let mut request = Request::get(url.as_str())
            .body(Bytes::new())
            .map_err(|source| ContextLoadError::Request { source })?;
        request
            .headers_mut()
            .typed_insert(Accept::in_order(NgsiLdMediaType::JsonLd, [NgsiLdMediaType::Json]));
        let response = self
            .client
            .transport
            .send(request)
            .await
            .map_err(|source| ContextLoadError::Transport { source })?;
        if response.status() != StatusCode::OK {
            return Err(ContextLoadError::Status { status: response.status() });
        }
        match (self.media_type, any_media_type(response.headers())) {
            (MediaTypeCheck::Required, Some(media_type)) if is_json(&media_type.to_ref()) => {}
            (MediaTypeCheck::Required, found) => return Err(ContextLoadError::MediaType { found }),
            (MediaTypeCheck::Ignored, _) => {}
        }
        let document = parse_slice_value(response.body()).map_err(|error| ContextLoadError::NotJson {
            source: InvalidJson::of(&error),
        })?;
        Ok(RawJson::from_json(document))
    }

    /// Fetches the context from the broker, which answers its `@context` value.
    async fn broker(&self, url: Iri<&str>) -> Result<RawJson, ContextLoadError<T::Error>> {
        let context = self
            .client
            .contexts()
            .serve(&ContextId::from(url))
            .send()
            .await
            .map_err(|source| ContextLoadError::Client { source })?;
        let mut document = Object::new();
        document.push(CONTEXT.into(), context.to_json());
        Ok(RawJson::from_json(Json::Object(document)))
    }
}

impl<T: Transport> RemoteContextLoader for ContextLoader<'_, T> {
    type Error = ContextLoadError<T::Error>;

    async fn load(&self, url: Iri<&str>) -> Result<RawJson, Self::Error> {
        match self.source {
            ContextSource::Direct => self.direct(url).await,
            ContextSource::Broker => self.broker(url).await,
        }
    }
}
