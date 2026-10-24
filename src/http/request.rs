//! The one request pipeline: every operation's request is built here, so every header rule of clause 6.3 is
//! applied in exactly one place.

pub(super) mod error;
pub(crate) mod scope;

use crate::{
    http::{
        ContextDelivery,
        ContextDeliveryError,
        Endpoint,
        QueryParameter,
        RequestError,
        endpoint::profiled_context_link,
        headers::{Accept, ContentType, Link, NgsiLdEntityMap, NgsiLdSnapshot, NgsiLdTenant},
        path::ResourcePath,
        query_string::QueryString,
        request::scope::RequestScope,
    },
    model::{CORE_CONTEXT, Context, EntityId, NgsiLdMediaType, keyword::CONTEXT},
    query::Locality,
};
use bytes::Bytes;
use headers::{ContentLength, Header, HeaderMapExt};
use http::{Method, Request, Uri, header::LINK};
use jstrict::{Object, Print, Value as Json};
use url::Url;

/// Where a request goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Target {
    /// A resource below the API root, with its parameters.
    Resource {
        /// The resource.
        path: ResourcePath,
        /// The parameters.
        query: QueryString,
    },
    /// A URL a broker handed out, such as the next page of a query (clause 6.3.10).
    Url(Url),
}

/// The JSON body of a request, without `@context`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum RequestBody {
    /// One document: an entity, a fragment, an attribute or a query.
    Document(Object),
    /// An array of documents, the body of a batch operation (clauses 6.14 to 6.16 and 6.31). With
    /// `application/ld+json` every element carries the context, since the array has no member of its own to hold
    /// it.
    Documents(Vec<Object>),
    /// An array of entity ids, the body of Batch Entity Delete (clause 6.17), each a JSON string. No element can
    /// hold a context, so the body is always sent as `application/json` with the `Link` header, which clause 6.3.5
    /// allows for every POST.
    EntityIds(Vec<Json>),
    /// A JSON-LD local context, `{"@context": ...}`, the body of Add @context (Table 6.29.3.1-1). Its `@context` is
    /// the context being stored, which clause 6.3.5 refuses in an `application/json` body, so it is always sent as
    /// `application/ld+json`, without the `Link` header and without the client's own context.
    LocalContext(Object),
}

impl RequestBody {
    /// How the body carries the context: as the endpoint asks, except an array of ids, which has no object to
    /// hold one, and a local context, which is its own `@context`.
    const fn delivery(&self, requested: ContextDelivery) -> ContextDelivery {
        match self {
            Self::Document(_) | Self::Documents(_) => requested,
            Self::EntityIds(_) => ContextDelivery::JsonWithLink,
            Self::LocalContext(_) => ContextDelivery::LdJsonBody,
        }
    }

    /// The body as sent, with the context in every document when `delivery` puts it in the body.
    fn into_json(self, endpoint: &Endpoint, delivery: ContextDelivery) -> Json {
        match self {
            Self::Document(body) => with_context(body, endpoint, delivery),
            Self::Documents(bodies) => Json::Array(bodies.into_iter().map(|body| with_context(body, endpoint, delivery)).collect()),
            Self::EntityIds(ids) => Json::Array(ids.into()),
            Self::LocalContext(document) => Json::Object(document),
        }
    }
}

/// What an operation contributes to its request; the pipeline adds everything else.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RequestParts {
    /// The HTTP method.
    pub(crate) method: Method,
    /// Where the request goes.
    pub(crate) target: Target,
    /// The JSON body, without `@context`.
    pub(crate) body: Option<RequestBody>,
    /// The representation the operation reads from a response body, when it reads one.
    pub(crate) accept: Option<NgsiLdMediaType>,
    /// The entity map the broker is asked to use (clauses 6.4.3.2 and 6.5.3.1).
    pub(crate) entity_map: Option<NgsiLdEntityMap>,
}

impl RequestParts {
    /// A request to a resource below the API root.
    pub(crate) const fn resource(method: Method, path: ResourcePath, query: QueryString) -> Self {
        Self {
            method,
            target: Target::Resource { path, query },
            body: None,
            accept: None,
            entity_map: None,
        }
    }

    /// Adds a body that is one document.
    pub(crate) fn with_body(mut self, body: Object) -> Self {
        self.body = Some(RequestBody::Document(body));
        self
    }

    /// Adds a body that is an array of documents.
    pub(crate) fn with_documents(mut self, documents: Vec<Object>) -> Self {
        self.body = Some(RequestBody::Documents(documents));
        self
    }

    /// Adds a body that is an array of entity ids.
    pub(crate) fn with_entity_ids(mut self, ids: &[EntityId]) -> Self {
        self.body = Some(RequestBody::EntityIds(ids.iter().map(|id| Json::from(id.as_str())).collect()));
        self
    }

    /// Adds a body that is a JSON-LD local context.
    pub(crate) fn with_local_context(mut self, document: Object) -> Self {
        self.body = Some(RequestBody::LocalContext(document));
        self
    }

    /// Names the entity map the broker is asked to use.
    pub(crate) fn with_entity_map(mut self, map: Option<&NgsiLdEntityMap>) -> Self {
        // The parts own their headers; the map is a URI reference and its rendered field value.
        self.entity_map = map.cloned();
        self
    }

    /// Asks for a response body in `media_type`.
    pub(crate) const fn accepting(mut self, media_type: NgsiLdMediaType) -> Self {
        self.accept = Some(media_type);
        self
    }

    /// Builds the HTTP request: the URI from the API root and the target, with the `local` of `scope` (clause
    /// 6.3.18), the `@context` delivered as clause 6.3.5 requires, `Accept` (the endpoint's, or the operation's own),
    /// `NGSILD-Tenant` (clause 6.3.14) and `NGSILD-Snapshot` (clause 6.3.22) of `scope`, `NGSILD-EntityMap` (clause
    /// 6.4.3.2), the endpoint's `Prefer` (clause 6.3.6), `Content-Length` (clause 6.3.4) and the endpoint's further
    /// headers.
    pub(crate) fn into_request(self, endpoint: &Endpoint, scope: RequestScope<'_>) -> Result<Request<Bytes>, RequestError> {
        let uri = request_uri(self.target, endpoint, scope.locality())?;
        let delivery = self
            .body
            .as_ref()
            .map_or(endpoint.context_delivery(), |body| body.delivery(endpoint.context_delivery()));
        let link = context_link(endpoint, self.body.as_ref(), delivery)?;
        if let Some(body) = &self.body {
            check_body_context(endpoint, body, delivery)?;
        }
        let body = self.body.map(|body| body.into_json(endpoint, delivery));
        let has_body = body.is_some();
        let bytes = body.map_or_else(Bytes::new, |body| Bytes::from(body.compact_print().to_string()));
        let length = bytes.len() as u64;
        // Clause 6.3.4 admits `application/merge-patch+json` for PATCH alone.
        let merge_patch = delivery == ContextDelivery::MergePatchWithLink && self.method == Method::PATCH;
        let mut request = Request::new(bytes);
        *request.method_mut() = self.method;
        *request.uri_mut() = uri;
        let headers = request.headers_mut();
        match (endpoint.accept(), self.accept) {
            // The header owns its rendered value, and the endpoint keeps its own for the next request.
            (Some(accept), _) => headers.typed_insert(accept.clone()),
            (None, Some(media_type)) => headers.typed_insert(Accept::new(media_type)),
            (None, None) => {}
        }
        if has_body {
            headers.typed_insert(match delivery {
                ContextDelivery::MergePatchWithLink if merge_patch => ContentType::MergePatch,
                ContextDelivery::JsonWithLink | ContextDelivery::MergePatchWithLink => ContentType::NgsiLd(NgsiLdMediaType::Json),
                ContextDelivery::LdJsonBody => ContentType::NgsiLd(NgsiLdMediaType::JsonLd),
            });
            headers.typed_insert(ContentLength(length));
        }
        if let Some(link) = link {
            headers.typed_insert(link);
        }
        if let Some(map) = self.entity_map {
            headers.typed_insert(map);
        }
        if let Some(prefer) = endpoint.prefer() {
            // The header owns its rendered value; cloning it is a reference count.
            headers.typed_insert(prefer.clone());
        }
        if let Some(tenant) = scope.tenant() {
            // The header owns the tenant it renders.
            headers.typed_insert(NgsiLdTenant::try_new(tenant.clone()).map_err(|source| RequestError::Tenant { source })?);
        }
        if let Some(snapshot) = scope.snapshot() {
            // The header owns the snapshot id it renders.
            headers.typed_insert(NgsiLdSnapshot::try_new(snapshot.clone()).map_err(|source| RequestError::Snapshot { source })?);
        }
        if let Some(link) = endpoint.link() {
            let mut lines = Vec::new();
            link.encode(&mut lines);
            // A further link travels as its own field line, so it never merges with the context's (RFC 9110 section
            // 5.3 lets a list header be split over lines).
            for line in lines {
                headers.append(LINK, line);
            }
        }
        for (name, value) in endpoint.headers() {
            // The request owns its headers; the endpoint keeps its own for the next request.
            headers.append(name.clone(), value.clone());
        }
        Ok(request)
    }
}

/// The `Link` header naming the client's context, when the request carries the context there: a request without
/// a body, or whose body is `application/json` (clause 6.3.5).
///
/// A context that is not a single reference cannot be named there, which refuses the request; an array of entity
/// ids is the exception, since it holds no term the context would expand, so it is sent without the context.
///
/// The body of Add @context sends neither the client's context nor a profile stated for it: the body is the context
/// being stored, so no `Link` header may travel beside it (clause 6.3.5), and it holds no NGSI-LD datatype the
/// profile of clause 6.3.21 would let a broker fall back on.
fn context_link(endpoint: &Endpoint, body: Option<&RequestBody>, delivery: ContextDelivery) -> Result<Option<Link>, ContextDeliveryError> {
    if let Some(RequestBody::LocalContext(_)) = body {
        return Ok(None);
    }
    if body.is_some() && delivery == ContextDelivery::LdJsonBody {
        // The context travels in the body, so no `Link` carries a profile stated for it.
        return match endpoint.context_profile() {
            Some(_) => Err(ContextDeliveryError::ProfileWithoutLink),
            None => Ok(None),
        };
    }
    if let Some(profile) = endpoint.context_profile() {
        return profiled_context_link(endpoint.context(), profile)
            .map(Some)
            .map_err(|source| ContextDeliveryError::Link { source });
    }
    match (endpoint.context(), endpoint.context_link(), body) {
        // The header owns its rendered value; cloning it is a reference count.
        (_, Some(link), _) => Ok(Some(link.clone())),
        (None, None, _) | (Some(_), None, Some(RequestBody::EntityIds(_))) => Ok(None),
        (Some(_), None, _) => Err(ContextDeliveryError::NotLinkable),
    }
}

/// Refuses a body that would carry a relative context reference: the client's context goes into a document under
/// `application/ld+json` (clause 6.3.5), and a JSON-LD processor at the broker has no base to resolve it against,
/// unlike the target of a `Link` header (IETF RFC 8288 section 3.2).
fn check_body_context(endpoint: &Endpoint, body: &RequestBody, delivery: ContextDelivery) -> Result<(), ContextDeliveryError> {
    let carried = match body {
        RequestBody::Document(_) | RequestBody::Documents(_) => delivery == ContextDelivery::LdJsonBody,
        RequestBody::EntityIds(_) | RequestBody::LocalContext(_) => false,
    };
    match endpoint
        .context()
        .into_iter()
        .flat_map(Context::references)
        .find(|reference| !reference.is_absolute())
    {
        // The error owns the reference it names.
        Some(reference) if carried => Err(ContextDeliveryError::RelativeInBody { reference: reference.clone() }),
        Some(_) | None => Ok(()),
    }
}

/// The absolute request URI, with the `local` parameter of clause 6.3.18 that `locality` adds after the target's
/// own parameters; [`Locality::Unspecified`] adds none.
fn request_uri(target: Target, endpoint: &Endpoint, locality: Locality) -> Result<Uri, RequestError> {
    let text = match target {
        Target::Resource { path, mut query } => {
            query.push_some(QueryParameter::Local, locality.flag());
            if query.is_empty() {
                format!("{}{path}", endpoint.base_url())
            } else {
                format!("{}{path}?{query}", endpoint.base_url())
            }
        }
        Target::Url(mut url) => {
            if let Some(flag) = locality.flag() {
                url.query_pairs_mut().append_pair(QueryParameter::Local.into(), &flag.to_string());
            }
            String::from(url)
        }
    };
    Uri::try_from(text).map_err(|source| RequestError::Uri { source })
}

/// A document as sent: with `application/ld+json` the context goes into the document (clause 6.3.5), the client's
/// own or the core context, since such a body without one is rejected.
fn with_context(mut body: Object, endpoint: &Endpoint, delivery: ContextDelivery) -> Json {
    if delivery == ContextDelivery::LdJsonBody {
        let context = endpoint.context().map_or_else(|| Json::from(CORE_CONTEXT.as_str()), Context::to_json);
        body.push(CONTEXT.into(), context);
    }
    Json::Object(body)
}
