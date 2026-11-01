//! The query string of an unchecked request: a parameter set to text as written, removed, or one the specification
//! does not define added.

use crate::http::{QueryParameter, RequestError, UncheckedRequest, query_string::QueryString};
use http::Uri;

impl UncheckedRequest {
    /// Sets the parameter `name` to `text` as written, which need not be a value the parameter accepts: an invalid
    /// `q`, `scopeQ` or projection, an id that is not a URI, a negative `limit`, a `format` the specification does
    /// not define. The parameter keeps its place in the query string when the operation sent it, and is added last
    /// otherwise; every further occurrence is removed.
    ///
    /// The text is escaped as a typed value of the parameter is, so the broker decodes exactly `text` and the request
    /// differs from a valid one only in the defect under test.
    ///
    /// ```
    /// use ngsi_ld::{
    ///     http::{Endpoint, QueryParameter, UncheckedRequest, operation::QueryEntities},
    ///     model::Lossless,
    ///     query::SelectorType,
    /// };
    /// use url::Url;
    ///
    /// let endpoint = Endpoint::builder()
    ///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
    ///     .build()?;
    /// let buildings: SelectorType = "Building".parse()?;
    /// let query = QueryEntities::builder()
    ///     .entity_type(&buildings)
    ///     .format(Lossless::Normalized)
    ///     .build();
    /// let request = UncheckedRequest::try_from_operation(&query, &endpoint)?
    ///     .query_parameter(QueryParameter::ScopeQ, "/Madrid/#/Gardens")?;
    /// assert_eq!(
    ///     request.as_request().uri(),
    ///     "http://broker.test/ngsi-ld/v1/entities?type=Building&scopeQ=/Madrid/%23/Gardens"
    /// );
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`RequestError::Uri`] when the request URI with the parameter is not a valid URI.
    pub fn query_parameter(self, name: QueryParameter, text: &str) -> Result<UncheckedRequest, RequestError> {
        let pair = format!("{name}={}", QueryString::escaped_as(name, text));
        self.with_query(<&'static str>::from(name), Some(&pair))
    }

    /// Removes every occurrence of the parameter `name`, such as the `type` that bounds a query, to send a request
    /// the operation itself refuses to build.
    ///
    /// ```
    /// use ngsi_ld::{
    ///     http::{Endpoint, QueryParameter, UncheckedRequest, operation::QueryEntities},
    ///     model::{EntityId, Lossless},
    ///     query::SelectorType,
    /// };
    /// use url::Url;
    ///
    /// let endpoint = Endpoint::builder()
    ///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
    ///     .build()?;
    /// let buildings: SelectorType = "Building".parse()?;
    /// let ids: [EntityId; 1] = ["urn:ngsi-ld:Building:1".parse()?];
    /// let query = QueryEntities::builder()
    ///     .entity_type(&buildings)
    ///     .ids(&ids)
    ///     .format(Lossless::Normalized)
    ///     .build();
    /// let unbounded = UncheckedRequest::try_from_operation(&query, &endpoint)?
    ///     .remove_query_parameter(QueryParameter::Type)?;
    /// assert_eq!(
    ///     unbounded.as_request().uri(),
    ///     "http://broker.test/ngsi-ld/v1/entities?id=urn%3Angsi-ld%3ABuilding%3A1"
    /// );
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`RequestError::Uri`] when the request URI without the parameter is not a valid URI.
    pub fn remove_query_parameter(self, name: QueryParameter) -> Result<UncheckedRequest, RequestError> {
        self.with_query(<&'static str>::from(name), None)
    }

    /// Adds a parameter the specification does not define, `name=text`, both escaped as an opaque value is; it is
    /// placed as [`query_parameter`](Self::query_parameter) places one. A broker answers such a parameter with an
    /// `InvalidRequest`.
    ///
    /// # Errors
    ///
    /// Returns [`RequestError::DefinedQueryParameter`] when `name` is the name of a [`QueryParameter`], which
    /// [`query_parameter`](Self::query_parameter) sets, and [`RequestError::Uri`] when the request URI with the
    /// parameter is not a valid URI.
    pub fn undefined_query_parameter(self, name: &str, text: &str) -> Result<UncheckedRequest, RequestError> {
        if let Ok(parameter) = name.parse::<QueryParameter>() {
            return Err(RequestError::DefinedQueryParameter { parameter });
        }
        let name = QueryString::escaped_opaque(name);
        let pair = format!("{name}={}", QueryString::escaped_opaque(text));
        self.with_query(&name, Some(&pair))
    }

    /// The request with every occurrence of the parameter `name` replaced by `pair` in place of the first, or added
    /// last when there is none; with no `pair`, every occurrence removed.
    fn with_query(mut self, name: &str, pair: Option<&str>) -> Result<UncheckedRequest, RequestError> {
        let uri = with_parameter(self.request.uri(), name, pair);
        *self.request.uri_mut() = Uri::try_from(uri).map_err(|source| RequestError::Uri { source })?;
        Ok(self)
    }
}

/// The text of `uri` with the parameter `name` (as written in the query string) replaced: `pair` in place of its
/// first occurrence and every other occurrence removed, `pair` added last when the query has none, and every
/// occurrence removed when there is no `pair`. The `?` is left out when no parameter remains.
fn with_parameter(uri: &Uri, name: &str, pair: Option<&str>) -> String {
    let text = uri.to_string();
    let (base, query) = text.split_once('?').map_or((text.as_str(), None), |(base, query)| (base, Some(query)));
    let mut pairs: Vec<&str> = Vec::new();
    let mut placed = false;
    for existing in query.into_iter().flat_map(|query| query.split('&')) {
        let existing_name = existing.split_once('=').map_or(existing, |(key, _)| key);
        if existing_name != name {
            pairs.push(existing);
        } else if let Some(pair) = pair
            && !placed
        {
            pairs.push(pair);
            placed = true;
        }
    }
    if let Some(pair) = pair
        && !placed
    {
        pairs.push(pair);
    }
    if pairs.is_empty() {
        base.to_owned()
    } else {
        format!("{base}?{}", pairs.join("&"))
    }
}
