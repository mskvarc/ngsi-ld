//! Deliberately invalid requests, for checking that a broker rejects what the specification says it must reject
//! (clause 5.5.4).

use crate::{
    http::{Endpoint, RequestError, operation::Operation},
    model::{Entity, Input, Lossless, RawJson, keyword::CONTEXT, null::NGSI_LD_NULL, representation::write::write_entity},
};
use bytes::Bytes;
use headers::{ContentLength, Header, HeaderMapExt};
use http::{HeaderName, HeaderValue, Method, Request, Uri};
use jstrict::{Object, Print, Value as Json};
use std::{fmt, str};

/// A JSON document that may break any rule: a member removed, any member set to any JSON, an attribute renamed to
/// an invalid name, an NGSI-LD Null where one is not allowed.
///
/// It starts from a valid entity, from any document the crate renders (a subscription, a registration, a snapshot,
/// a fragment, through [`TryFrom<RawJson>`](#impl-TryFrom%3CRawJson%3E-for-UncheckedDocument)), or from nothing.
///
/// ```
/// use iri_rs::iri;
/// use ngsi_ld::{
///     http::UncheckedDocument,
///     model::{Entity, Lossless},
/// };
///
/// let vehicle = Entity::builder()
///     .id(iri!("urn:ngsi-ld:Vehicle:A4567"))
///     .entity_type("Vehicle")
///     .property("speed", 80)
///     .build()?;
/// let broken = UncheckedDocument::from_entity(&vehicle, Lossless::Normalized)
///     .remove_member("type")
///     .rename_member("speed", "1speed");
/// assert_eq!(
///     broken.to_string(),
///     r#"{"id":"urn:ngsi-ld:Vehicle:A4567","1speed":{"type":"Property","value":80}}"#
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UncheckedDocument(Object);

impl UncheckedDocument {
    /// Starts from a valid entity.
    #[must_use]
    pub fn from_entity(entity: &Entity<Input>, form: Lossless) -> UncheckedDocument {
        UncheckedDocument(write_entity(entity, form))
    }

    /// Starts from an empty object.
    #[must_use]
    pub fn empty() -> UncheckedDocument {
        UncheckedDocument(Object::new())
    }

    /// Sets a member to a string, as [`set_member`](Self::set_member) does.
    #[must_use]
    pub fn set_text(self, name: &str, text: &str) -> Self {
        self.set_member(name, &RawJson::from_json(Json::from(text)))
    }

    /// Removes every occurrence of a member.
    #[must_use]
    pub fn remove_member(mut self, name: &str) -> Self {
        self.0.remove(name);
        self
    }

    /// Sets a member to any JSON, replacing it in place if present and appending it otherwise.
    #[must_use]
    pub fn set_member(mut self, name: &str, value: &RawJson) -> Self {
        // The member is set from a borrowed value, and the object owns its members: a stack-safe copy.
        let value = value.to_json();
        match self.0.iter_mut().find(|(key, _)| key.as_str() == name) {
            Some((_, slot)) => *slot = value,
            None => {
                self.0.push(name.into(), value);
            }
        }
        self
    }

    /// Renames a member, keeping its position and value; `to` is not checked.
    #[must_use]
    pub fn rename_member(mut self, from: &str, to: &str) -> Self {
        self.0 = self
            .0
            .into_iter()
            .map(|mut entry| {
                if entry.key.as_str() == from {
                    entry.key = to.into();
                }
                entry
            })
            .collect();
        self
    }

    /// Sets a member to the NGSI-LD Null, `"urn:ngsi-ld:null"` (clause 4.5.0), wherever the caller wants it.
    #[must_use]
    pub fn null_member(self, name: &str) -> Self {
        let null = RawJson::from_json(Json::from(NGSI_LD_NULL.as_str()));
        self.set_member(name, &null)
    }
}

/// Why a rendered document cannot start an [`UncheckedDocument`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
#[error("an unchecked document starts from a JSON object")]
pub struct NotAnObject;

// A rendered document is an object or not; no derive can tell.
impl TryFrom<RawJson> for UncheckedDocument {
    type Error = NotAnObject;

    /// Starts from a rendered document, such as `registration.render()`, keeping its members in order.
    ///
    /// ```
    /// use ngsi_ld::{http::UncheckedDocument, model::RawJson};
    ///
    /// let registration: RawJson =
    ///     r#"{"type":"ContextSourceRegistration","endpoint":"http://source.test"}"#.parse()?;
    /// let broken = UncheckedDocument::try_from(registration)?.remove_member("endpoint");
    /// assert_eq!(
    ///     broken.to_string(),
    ///     r#"{"type":"ContextSourceRegistration"}"#
    /// );
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    fn try_from(document: RawJson) -> Result<Self, NotAnObject> {
        match document.into_json() {
            Json::Object(object) => Ok(UncheckedDocument(object)),
            Json::Null | Json::Boolean(_) | Json::Number(_) | Json::String(_) | Json::Array(_) => Err(NotAnObject),
        }
    }
}

// The compact JSON text is the text form.
impl fmt::Display for UncheckedDocument {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Clone: the printer works on values, not on bare objects, and the document keeps its members.
        fmt::Display::fmt(&Json::Object(self.0.clone()).compact_print(), f)
    }
}

/// An HTTP request that may break any rule: built from a valid operation through the one pipeline, then changed
/// by name: another method or URI, any header added or removed, any body.
///
/// The client sends it with `Client::send_unchecked`, which decodes the response with the same decoders as every
/// other call, so a rejection comes back as a typed problem.
///
/// A negative test breaks one rule at a time. When the operation's body carried an `@context` (delivery
/// `application/ld+json`, clause 6.3.5), [`document`](Self::document) keeps it in the replacement body, because a
/// broker rejects an `ld+json` body without one and would otherwise reject the missing context instead of the
/// defect under test. [`document_as_is`](Self::document_as_is) sends a document exactly as it is, to test a
/// missing context on purpose.
#[derive(Debug)]
pub struct UncheckedRequest {
    /// The request as it will be sent.
    request: Request<Bytes>,
    /// The `@context` member of the body the operation built, if it had one.
    context: Option<Json>,
}

impl UncheckedRequest {
    /// Builds the request `operation` would send to `endpoint`.
    ///
    /// # Errors
    ///
    /// Returns [`RequestError`] when the operation itself refuses its parameters.
    pub fn from_operation<O: Operation>(operation: &O, endpoint: &Endpoint) -> Result<UncheckedRequest, RequestError> {
        let request = operation.to_request(endpoint)?;
        let context = body_context(request.body());
        Ok(UncheckedRequest { request, context })
    }

    /// Replaces the method.
    #[must_use]
    pub fn method(mut self, method: Method) -> Self {
        *self.request.method_mut() = method;
        self
    }

    /// Replaces the URI.
    #[must_use]
    pub fn uri(mut self, uri: Uri) -> Self {
        *self.request.uri_mut() = uri;
        self
    }

    /// Adds a header value, keeping any the request already has under that name.
    #[must_use]
    pub fn append_header(mut self, name: HeaderName, value: HeaderValue) -> Self {
        self.request.headers_mut().append(name, value);
        self
    }

    /// Sets a typed header, replacing every value the request already has under its name.
    ///
    /// ```
    /// use headers::IfNoneMatch;
    /// use http::header;
    /// use iri_rs::iri;
    /// use ngsi_ld::{
    ///     http::{Endpoint, UncheckedRequest, operation::DeleteEntity},
    ///     model::EntityId,
    /// };
    /// use url::Url;
    ///
    /// let endpoint = Endpoint::builder()
    ///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
    ///     .build()?;
    /// let id = EntityId::from(iri!("urn:ngsi-ld:Vehicle:A4567"));
    /// let request =
    ///     UncheckedRequest::from_operation(&DeleteEntity::builder().id(&id).build(), &endpoint)?
    ///         .typed_header(IfNoneMatch::any());
    /// assert_eq!(request.as_request().headers()[&header::IF_NONE_MATCH], "*");
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    #[must_use]
    pub fn typed_header<H: Header>(mut self, header: H) -> Self {
        self.request.headers_mut().typed_insert(header);
        self
    }

    /// Removes every value of a header.
    #[must_use]
    pub fn remove_header(mut self, name: &HeaderName) -> Self {
        self.request.headers_mut().remove(name);
        self
    }

    /// Replaces the body with arbitrary bytes, and `Content-Length` with their length unless the header was
    /// removed on purpose.
    #[must_use]
    pub fn body(mut self, body: Bytes) -> Self {
        if self.request.headers().contains_key(ContentLength::name()) {
            self.request.headers_mut().typed_insert(ContentLength(body.len() as u64));
        }
        *self.request.body_mut() = body;
        self
    }

    /// Replaces the body with a document, as [`body`](Self::body) does, keeping the `@context` the operation's body
    /// carried unless the document sets its own.
    #[must_use]
    pub fn document(self, document: &UncheckedDocument) -> Self {
        let text = self.with_context(document).compact_print().to_string();
        self.body(Bytes::from(text))
    }

    /// Replaces the body with an array of documents, the body of a batch operation, as [`body`](Self::body) does:
    /// each document keeps the `@context` the operation's body carried unless it sets its own, so one broken
    /// element tests the rejection of that element rather than a missing context.
    ///
    /// ```
    /// use bytes::Bytes;
    /// use iri_rs::iri;
    /// use ngsi_ld::{
    ///     http::{Endpoint, UncheckedDocument, UncheckedRequest, operation::BatchCreate},
    ///     model::{Entity, Lossless},
    /// };
    /// use url::Url;
    ///
    /// let vehicle = Entity::builder()
    ///     .id(iri!("urn:ngsi-ld:Vehicle:A4567"))
    ///     .entity_type("Vehicle")
    ///     .build()?;
    /// let endpoint = Endpoint::builder()
    ///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
    ///     .build()?;
    /// let entities = [vehicle];
    /// let create = BatchCreate::builder().entities(&entities).build();
    /// let untyped =
    ///     UncheckedDocument::from_entity(&entities[0], Lossless::Normalized).remove_member("type");
    /// let request = UncheckedRequest::from_operation(&create, &endpoint)?.documents(&[untyped]);
    /// assert_eq!(
    ///     request.as_request().body(),
    ///     &Bytes::from_static(br#"[{"id":"urn:ngsi-ld:Vehicle:A4567"}]"#)
    /// );
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    #[must_use]
    pub fn documents(self, documents: &[UncheckedDocument]) -> Self {
        let array = Json::Array(documents.iter().map(|document| self.with_context(document)).collect());
        self.body(Bytes::from(array.compact_print().to_string()))
    }

    /// Replaces the body with a document exactly as it is, as [`body`](Self::body) does: an `@context` the
    /// operation's body carried is not added, so a document without one tests a missing context.
    #[must_use]
    pub fn document_as_is(self, document: &UncheckedDocument) -> Self {
        self.body(Bytes::from(document.to_string()))
    }

    /// The document with the `@context` the operation's body carried, unless it sets its own.
    fn with_context(&self, document: &UncheckedDocument) -> Json {
        // The document is borrowed and the request owns its body, so the body is a copy.
        let mut object = document.0.clone();
        if let Some(context) = &self.context
            && !object.contains_key(CONTEXT)
        {
            // The request keeps its context for a later replacement.
            object.push(CONTEXT.into(), context.clone());
        }
        Json::Object(object)
    }

    /// The request as it will be sent.
    #[must_use]
    pub const fn as_request(&self) -> &Request<Bytes> {
        &self.request
    }

    /// Takes the request.
    #[must_use]
    pub fn into_request(self) -> Request<Bytes> {
        self.request
    }
}

/// The `@context` member of a JSON object body, or of the first element of an array body (a batch, whose every
/// element carries the same context), if it has exactly one.
fn body_context(body: &Bytes) -> Option<Json> {
    // The body was written by the pipeline, so it is a JSON object, an array or empty; anything else has no context
    // to keep.
    let json: Json = str::from_utf8(body).ok()?.parse().ok()?;
    let document = match &json {
        Json::Array(elements) => elements.first()?,
        Json::Null | Json::Boolean(_) | Json::Number(_) | Json::String(_) | Json::Object(_) => &json,
    };
    document.as_object()?.get_unique(CONTEXT).ok().flatten().cloned()
}
