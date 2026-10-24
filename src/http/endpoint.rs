//! The broker an operation is addressed to, and how the client speaks to it.

use crate::{
    http::{
        ContextDelivery,
        EndpointError,
        headers::{Accept, Link, LinkBuildError, LinkValue, NgsiLdEntityMap, NgsiLdSnapshot, NgsiLdTenant, Prefer},
    },
    model::{Context, Tenant, context::core_context_reference},
};
use bon::bon;
use headers::{ContentLength, ContentType, Header, HeaderMapExt};
use http::{
    HeaderMap,
    HeaderName,
    HeaderValue,
    header::{ACCEPT, CONTENT_ENCODING, HOST, LINK, TRANSFER_ENCODING},
};
use iri_rs::{IriBuf, IriRefBuf};
use url::Url;

pub(super) mod error;

/// Where and how a client reaches a broker.
///
/// It holds the API root of a broker (the URL that the resource paths of NGSI-LD clause 6.2 are relative to, such as
/// `http://localhost:9090/ngsi-ld/v1/`), the `@context` the client's documents use, where requests carry that context
/// (clause 6.3.5), and the tenant every request addresses (`NGSILD-Tenant`, clause 6.3.14).
///
/// It also holds what every request states besides: the preferences (`Prefer`, clause 6.3.6), the representations
/// accepted (`Accept`, clause 6.3.4) and any further header, such as `Via` (clause 6.3.18).
///
/// Without a context, requests use the default context: GET and DELETE send no `Link` header, and a request whose
/// body carries its context (see [`ContextDelivery::LdJsonBody`]) names the NGSI-LD core context, since clause
/// 6.3.5 rejects such a body without an `@context`. A context that is not a single reference (an inline definition
/// or an array) cannot travel in a `Link` header, so only requests whose body carries it can use it.
///
/// Without an `Accept`, each operation asks for the representation it reads (`application/json`, or
/// `application/geo+json` for the `GeoJSON` operations).
///
/// ```
/// use iri_rs::IriRefBuf;
/// use ngsi_ld::http::{ContextDelivery, Endpoint};
/// use url::Url;
///
/// let endpoint = Endpoint::builder()
///     .base_url(Url::parse("http://localhost:9090/ngsi-ld/v1")?)
///     .context(IriRefBuf::new("https://example.org/context.jsonld")?)
///     .context_delivery(ContextDelivery::LdJsonBody)
///     .build()?;
/// assert_eq!(
///     endpoint.base_url().as_str(),
///     "http://localhost:9090/ngsi-ld/v1/"
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Endpoint {
    /// The API root, always ending in `/`.
    base_url: Url,
    /// The context of the client's documents.
    context: Option<Context>,
    /// The `Link` header naming the context (clause 6.3.5), rendered once; absent when the context is not a single
    /// reference.
    context_link: Option<Link>,
    /// The `profile` parameter of the context link (clause 6.3.21).
    context_profile: Option<IriBuf>,
    /// Where bodies carry the context.
    context_delivery: ContextDelivery,
    /// The preferences every request states (`Prefer`, clause 6.3.6).
    prefer: Option<Prefer>,
    /// The representations every request accepts, replacing the one the operation asks for (clause 6.3.4).
    accept: Option<Accept>,
    /// Further headers every request carries.
    headers: HeaderMap,
    /// A further `Link` every request carries beside the context's, such as a profile link (clause 6.3.21).
    link: Option<Link>,
    /// The tenant every request addresses; `None` is the default tenant (clause 4.14).
    tenant: Option<Tenant>,
}

#[bon]
impl Endpoint {
    /// Checks and stores an API root. A missing trailing `/` is added, so `http://host/ngsi-ld/v1` and
    /// `http://host/ngsi-ld/v1/` name the same root.
    ///
    /// # Errors
    ///
    /// Returns [`EndpointError`] when the URL cannot be an API root, the context holds a relative reference and is not
    /// a single reference ([`EndpointError::RelativeContextReference`]), or the context reference cannot be carried
    /// in a `Link` header.
    #[builder]
    pub fn new(
        mut base_url: Url,
        #[builder(into)] context: Option<Context>,
        #[builder(default)] context_delivery: ContextDelivery,
        prefer: Option<Prefer>,
        accept: Option<Accept>,
        tenant: Option<Tenant>,
    ) -> Result<Endpoint, EndpointError> {
        if base_url.cannot_be_a_base() {
            return Err(EndpointError::NotABase);
        }
        if base_url.query().is_some() || base_url.fragment().is_some() {
            return Err(EndpointError::QueryOrFragment);
        }
        if !base_url.path().ends_with('/') {
            let path = format!("{}/", base_url.path());
            base_url.set_path(&path);
        }
        Ok(Endpoint {
            base_url,
            context_link: context_link(context.as_ref())?,
            context_profile: None,
            context,
            context_delivery,
            prefer,
            accept,
            headers: HeaderMap::new(),
            link: None,
            tenant,
        })
    }
}

impl Endpoint {
    /// The API root, ending in `/`.
    #[must_use]
    pub const fn base_url(&self) -> &Url {
        &self.base_url
    }

    /// The context of the client's documents, if not the default one.
    #[must_use]
    pub const fn context(&self) -> Option<&Context> {
        self.context.as_ref()
    }

    /// The `Link` header naming the context, when it is a single reference.
    pub(crate) const fn context_link(&self) -> Option<&Link> {
        self.context_link.as_ref()
    }

    /// Where request bodies carry the context.
    #[must_use]
    pub const fn context_delivery(&self) -> ContextDelivery {
        self.context_delivery
    }

    /// The preferences every request states, such as the version of the specification the payloads are wanted in.
    #[must_use]
    pub const fn prefer(&self) -> Option<&Prefer> {
        self.prefer.as_ref()
    }

    /// The representations every request accepts, when they replace the one each operation asks for.
    #[must_use]
    pub const fn accept(&self) -> Option<&Accept> {
        self.accept.as_ref()
    }

    /// The further headers every request carries.
    #[must_use]
    pub const fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    /// The further `Link` every request carries beside the context's.
    #[must_use]
    pub const fn link(&self) -> Option<&Link> {
        self.link.as_ref()
    }

    /// The tenant every request addresses, `None` for the default tenant.
    #[must_use]
    pub const fn tenant(&self) -> Option<&Tenant> {
        self.tenant.as_ref()
    }

    /// The same endpoint addressing `tenant` (clause 4.14).
    ///
    /// ```
    /// use headers::Header;
    /// use iri_rs::iri;
    /// use ngsi_ld::{
    ///     http::{
    ///         Endpoint,
    ///         headers::NgsiLdTenant,
    ///         operation::{DeleteEntity, Operation},
    ///     },
    ///     model::EntityId,
    /// };
    /// use url::Url;
    ///
    /// let madrid = Endpoint::builder()
    ///     .base_url(Url::parse("http://broker.test/ngsi-ld/v1/")?)
    ///     .build()?
    ///     .with_tenant("city-of-madrid".parse()?);
    /// let id = EntityId::from(iri!("urn:ngsi-ld:Vehicle:A4567"));
    /// let request = DeleteEntity::builder()
    ///     .id(&id)
    ///     .build()
    ///     .to_request(&madrid)?;
    /// assert_eq!(request.headers()[NgsiLdTenant::name()], "city-of-madrid");
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    #[must_use]
    pub fn with_tenant(mut self, tenant: Tenant) -> Endpoint {
        self.tenant = Some(tenant);
        self
    }

    /// The same endpoint addressing the default tenant (clause 4.14).
    #[must_use]
    pub fn without_tenant(mut self) -> Endpoint {
        self.tenant = None;
        self
    }

    /// The same endpoint without a further `Link`.
    #[must_use]
    pub fn without_link(mut self) -> Endpoint {
        self.link = None;
        self
    }

    /// The same endpoint with another context.
    ///
    /// # Errors
    ///
    /// Returns [`EndpointError::RelativeContextReference`] when the context holds a relative reference and is not a
    /// single reference, [`EndpointError::ProfileWithoutContextLink`] when the endpoint states a context profile and
    /// the context is not a single reference, and [`EndpointError::Context`] when the context reference cannot be
    /// carried in a `Link` header.
    pub fn with_context(mut self, context: Context) -> Result<Endpoint, EndpointError> {
        check_profile(Some(&context), self.context_profile.as_ref())?;
        self.context_link = context_link(Some(&context))?;
        self.context = Some(context);
        Ok(self)
    }

    /// The same endpoint with the default context.
    #[must_use]
    pub fn without_context(mut self) -> Endpoint {
        self.context = None;
        self.context_link = None;
        self
    }

    /// The same endpoint delivering the context of request bodies as `delivery` says.
    #[must_use]
    pub const fn with_context_delivery(mut self, delivery: ContextDelivery) -> Endpoint {
        self.context_delivery = delivery;
        self
    }

    /// The same endpoint stating `prefer` on every request, or no preference when it is `None`.
    #[must_use]
    pub fn with_prefer(mut self, prefer: Option<Prefer>) -> Endpoint {
        self.prefer = prefer;
        self
    }

    /// The same endpoint accepting `accept` on every request, or the representation each operation asks for when
    /// it is `None`.
    ///
    /// The override reaches every operation, the `GeoJSON` ones and the requests for following pages included; an
    /// operation still reads its answer as the representation it defines, so an answer in another media type than
    /// the operation's own (for instance `application/json` to a `GeoJSON` query) fails to decode, as a negative test
    /// expects.
    #[must_use]
    pub fn with_accept(mut self, accept: Option<Accept>) -> Endpoint {
        self.accept = accept;
        self
    }

    /// The same endpoint sending `header` on every request, replacing an earlier value of the same header.
    ///
    /// ```
    /// use headers::IfNoneMatch;
    /// use http::header;
    /// use ngsi_ld::http::Endpoint;
    /// use url::Url;
    ///
    /// let endpoint = Endpoint::builder()
    ///     .base_url(Url::parse("http://localhost:9090/ngsi-ld/v1/")?)
    ///     .build()?
    ///     .with_header(IfNoneMatch::any())?;
    /// assert_eq!(endpoint.headers()[&header::IF_NONE_MATCH], "*");
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`EndpointError::ReservedHeader`] for a header the request pipeline sets itself.
    pub fn with_header<H: Header>(mut self, header: H) -> Result<Endpoint, EndpointError> {
        if is_reserved(H::name()) {
            // The error owns the name; a header name is a static or shared value, so cloning it is cheap.
            return Err(EndpointError::ReservedHeader { name: H::name().clone() });
        }
        self.headers.typed_insert(header);
        Ok(self)
    }

    /// The same endpoint sending the field `name: value` on every request, replacing an earlier value of the same
    /// header: a header no typed [`Header`] models, such as one a context source expects to have forwarded from the
    /// original request (clause 4.3.6.5).
    ///
    /// ```
    /// use http::{HeaderName, HeaderValue};
    /// use ngsi_ld::http::Endpoint;
    /// use url::Url;
    ///
    /// // A header of the client's own, which no NGSI-LD rule names.
    /// const USER: HeaderName = HeaderName::from_static("x-user");
    /// let endpoint = Endpoint::builder()
    ///     .base_url(Url::parse("http://localhost:9090/ngsi-ld/v1/")?)
    ///     .build()?
    ///     .with_header_field(USER, HeaderValue::from_static("fleet-operator"))?;
    /// assert_eq!(endpoint.headers()[&USER], "fleet-operator");
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`EndpointError::ReservedHeader`] for a header the request pipeline sets itself.
    pub fn with_header_field(mut self, name: HeaderName, value: HeaderValue) -> Result<Endpoint, EndpointError> {
        if is_reserved(&name) {
            return Err(EndpointError::ReservedHeader { name });
        }
        self.headers.insert(name, value);
        Ok(self)
    }
}

impl Endpoint {
    /// The same endpoint naming `profile` in the `profile` parameter of the link to its context, such as
    /// `https://uri.etsi.org/ngsi-ld/profile/version` (clause 6.3.21). Without a context of its own, the profile rides
    /// on a link to the core context. A request whose body carries the context (`application/ld+json`) carries no
    /// `Link` header, so it is refused with `ContextDeliveryError::ProfileWithoutLink` rather than sent without the
    /// profile. Add @context sends no context of the client's, so it is sent without a profile.
    ///
    /// # Errors
    ///
    /// Returns [`EndpointError::ProfileWithoutContextLink`] when the context is not a single reference.
    pub fn with_context_profile(mut self, profile: IriBuf) -> Result<Endpoint, EndpointError> {
        check_profile(self.context.as_ref(), Some(&profile))?;
        self.context_profile = Some(profile);
        Ok(self)
    }

    /// The same endpoint naming no profile in the link to its context.
    #[must_use]
    pub fn without_context_profile(mut self) -> Endpoint {
        self.context_profile = None;
        self
    }

    /// The `profile` parameter of the link to the context.
    #[must_use]
    pub const fn context_profile(&self) -> Option<&IriBuf> {
        self.context_profile.as_ref()
    }

    /// The same endpoint sending `link` on every request, as its own `Link` field line beside the one naming the
    /// context, replacing an earlier further `Link`: for instance the profile link of clause 6.3.21,
    /// `<https://uri.etsi.org/ngsi-ld/profile/version>`, through which a broker may accept the payload of another
    /// version.
    ///
    /// # Errors
    ///
    /// Returns [`EndpointError::ContextLink`] when a link-value of `link` names a JSON-LD context.
    pub fn with_link(mut self, link: Link) -> Result<Endpoint, EndpointError> {
        if link.values().iter().any(LinkValue::is_context_link) {
            return Err(EndpointError::ContextLink);
        }
        self.link = Some(link);
        Ok(self)
    }
}

/// The `Link` header for `context`, when it is a single reference.
///
/// A single reference may be relative: the broker resolves a `Link` target against the request URI (IETF RFC 8288
/// section 3.2), so `</ngsi-ld/v1/jsonldContexts/fleet>` names a context the broker stores. Any other context
/// travels whole in a body, where nothing resolves a relative reference for the broker, so every reference it holds
/// must be absolute.
fn context_link(context: Option<&Context>) -> Result<Option<Link>, EndpointError> {
    if let Some(reference) = context
        .filter(|context| context.link_target().is_none())
        .into_iter()
        .flat_map(Context::references)
        .find(|reference| !reference.is_absolute())
    {
        // The error owns the reference it names.
        return Err(EndpointError::RelativeContextReference { reference: reference.clone() });
    }
    context
        .and_then(Context::link_target)
        // The header owns its target, and the endpoint keeps the context it renders.
        .map(|target| Link::try_json_ld_context(target.clone()))
        .transpose()
        .map_err(EndpointError::Context)
}

/// The link to the context with `profile` as its `profile` parameter (clause 6.3.21): to the endpoint's context when
/// it is a single reference, or to the core context, which every context includes (clause 4.4), when it has none.
pub(crate) fn profiled_context_link(context: Option<&Context>, profile: &IriBuf) -> Result<Link, LinkBuildError> {
    // The header owns its target, and the endpoint keeps the context it renders.
    let target = context.and_then(Context::link_target).map_or_else(core_context_reference, IriRefBuf::clone);
    // The header owns its profile, and the endpoint keeps the one it states.
    Link::try_new(LinkValue::profiled_json_ld_context(target, profile.clone()))
}

/// Refuses `profile` beside `context` when the context is not a single reference, which no `Link` header carries.
const fn check_profile(context: Option<&Context>, profile: Option<&IriBuf>) -> Result<(), EndpointError> {
    match (context, profile) {
        (Some(context), Some(_)) if context.link_target().is_none() => Err(EndpointError::ProfileWithoutContextLink),
        (Some(_) | None, Some(_) | None) => Ok(()),
    }
}

/// Whether the request pipeline sets the header `name` itself.
fn is_reserved(name: &HeaderName) -> bool {
    [
        &ACCEPT,
        ContentType::name(),
        ContentLength::name(),
        &CONTENT_ENCODING,
        &TRANSFER_ENCODING,
        &HOST,
        &LINK,
        Prefer::name(),
        NgsiLdTenant::name(),
        NgsiLdEntityMap::name(),
        NgsiLdSnapshot::name(),
    ]
    .contains(&name)
}

#[cfg(test)]
mod tests {
    use crate::{
        http::{
            Endpoint,
            EndpointError,
            endpoint::{is_reserved, profiled_context_link},
            headers::{Accept, Link, LinkValue, NgsiLdEntityMap, NgsiLdSnapshot, NgsiLdTenant, Prefer, RelationType},
        },
        model::{APPLICATION_LD_JSON, CORE_CONTEXT, Context, ContextEntry, NgsiLdMediaType},
    };
    use headers::{ContentLength, ContentType, Header, IfNoneMatch};
    use http::{HeaderName, HeaderValue, header};
    use iri_rs::{IriBuf, IriRefBuf};
    use pretty_assertions::assert_eq;
    use url::Url;

    fn endpoint() -> Endpoint {
        Endpoint::builder()
            .base_url(Url::parse("http://broker.test/ngsi-ld/v1/").unwrap())
            .build()
            .unwrap()
    }

    #[test]
    fn a_root_without_a_trailing_slash_gets_one() {
        let endpoint = Endpoint::builder()
            .base_url(Url::parse("http://broker.test/ngsi-ld/v1").unwrap())
            .build()
            .unwrap();
        assert_eq!(endpoint.base_url().as_str(), "http://broker.test/ngsi-ld/v1/");
    }

    #[test]
    fn a_root_with_a_query_or_fragment_is_rejected() {
        for url in ["http://broker.test/ngsi-ld/v1/?x=1", "http://broker.test/ngsi-ld/v1/#x"] {
            assert!(matches!(
                Endpoint::builder().base_url(Url::parse(url).unwrap()).build(),
                Err(EndpointError::QueryOrFragment)
            ));
        }
    }

    #[test]
    fn a_url_that_cannot_be_a_base_is_rejected() {
        assert!(matches!(
            Endpoint::builder().base_url(Url::parse("mailto:broker@example.org").unwrap()).build(),
            Err(EndpointError::NotABase)
        ));
    }

    #[test]
    fn a_single_reference_context_gets_a_link_and_any_other_context_none() {
        let single = endpoint()
            .with_context(Context::from(IriRefBuf::new("https://example.org/a.jsonld").unwrap()))
            .unwrap();
        let expected = Link::try_json_ld_context(IriRefBuf::new("https://example.org/a.jsonld").unwrap()).unwrap();
        assert_eq!(single.context_link(), Some(&expected));
        let array = single
            .with_context(Context::from(vec![ContextEntry::from(IriRefBuf::new("https://example.org/a.jsonld").unwrap())]))
            .unwrap();
        assert_eq!(array.context_link(), None);
        assert_eq!(array.without_context().context(), None);
    }

    #[test]
    fn a_single_relative_context_reference_travels_in_the_link_header() {
        for reference in ["/ngsi-ld/v1/jsonldContexts/fleet", "contexts/a.jsonld", "../b.jsonld"] {
            let endpoint = endpoint().with_context(Context::from(IriRefBuf::new(reference).unwrap())).unwrap();
            let expected = Link::try_json_ld_context(IriRefBuf::new(reference).unwrap()).unwrap();
            assert_eq!(endpoint.context_link(), Some(&expected));
        }
    }

    #[test]
    fn a_relative_context_reference_is_refused_in_a_context_that_travels_in_a_body() {
        for context in [
            Context::from(vec![ContextEntry::from(IriRefBuf::new("contexts/a.jsonld").unwrap())]),
            Context::from(vec![
                ContextEntry::from(IriRefBuf::new("https://example.org/a.jsonld").unwrap()),
                ContextEntry::from(IriRefBuf::new("../b.jsonld").unwrap()),
            ]),
        ] {
            assert!(matches!(endpoint().with_context(context), Err(EndpointError::RelativeContextReference { .. })));
        }
    }

    #[test]
    fn an_extra_header_is_kept_and_a_pipeline_header_is_refused() {
        let endpoint = endpoint().with_header(IfNoneMatch::any()).unwrap();
        assert_eq!(endpoint.headers()[&header::IF_NONE_MATCH], "*");
        assert!(matches!(
            endpoint.with_header(ContentType::json()),
            Err(EndpointError::ReservedHeader { name }) if name == header::CONTENT_TYPE
        ));
    }

    #[test]
    fn a_header_field_is_kept_replacing_an_earlier_value_and_a_pipeline_header_is_refused() {
        let user = HeaderName::from_static("x-user");
        let endpoint = endpoint()
            .with_header_field(user.clone(), HeaderValue::from_static("alice"))
            .unwrap()
            .with_header_field(user.clone(), HeaderValue::from_static("bob"))
            .unwrap();
        assert_eq!(endpoint.headers().get_all(&user).iter().collect::<Vec<_>>(), [HeaderValue::from_static("bob")]);
        for name in [&header::CONTENT_TYPE, &header::LINK, NgsiLdTenant::name(), &header::ACCEPT] {
            assert!(matches!(
                // The endpoint takes the name it refuses; the loop keeps comparing against its own.
                endpoint.clone().with_header_field(name.clone(), HeaderValue::from_static("x")),
                Err(EndpointError::ReservedHeader { name: refused }) if refused == name
            ));
        }
    }

    #[test]
    fn every_header_the_pipeline_sets_is_refused_as_an_extra_header() {
        let endpoint = endpoint();
        let reserved = [
            &header::ACCEPT,
            &header::CONTENT_TYPE,
            &header::CONTENT_LENGTH,
            &header::CONTENT_ENCODING,
            &header::TRANSFER_ENCODING,
            &header::HOST,
            &header::LINK,
            Prefer::name(),
            NgsiLdTenant::name(),
            NgsiLdEntityMap::name(),
            NgsiLdSnapshot::name(),
        ];
        for name in reserved {
            assert!(is_reserved(name), "{name}");
        }
        assert!(!is_reserved(&header::VIA));
        assert!(!is_reserved(&header::IF_NONE_MATCH));
        assert!(matches!(endpoint.with_header(ContentLength(1)), Err(EndpointError::ReservedHeader { .. })));
    }

    #[test]
    fn a_context_profile_rides_on_the_context_link_or_on_the_core_one_and_needs_a_single_reference() {
        let profile = IriBuf::new("https://uri.etsi.org/ngsi-ld/profile/version").unwrap();
        let context = Context::from(IriRefBuf::new("https://example.org/context.jsonld").unwrap());
        let endpoint = endpoint().with_context(context).unwrap().with_context_profile(profile.clone()).unwrap();
        let profiled = |target: &str| {
            LinkValue::builder(IriRefBuf::new(target).unwrap())
                .rel(RelationType::JsonLdContext)
                .media_type(APPLICATION_LD_JSON)
                // Each expected link-value owns its profile.
                .profile(profile.clone())
                .build()
        };
        assert_eq!(
            profiled_context_link(endpoint.context(), &profile).unwrap().values(),
            [profiled("https://example.org/context.jsonld")]
        );
        assert_eq!(profiled_context_link(None, &profile).unwrap().values(), [profiled(CORE_CONTEXT.as_str())]);
        let array = Context::from(vec![ContextEntry::from(IriRefBuf::new("https://example.org/a.jsonld").unwrap())]);
        assert!(matches!(
            endpoint.clone().with_context(array.clone()),
            Err(EndpointError::ProfileWithoutContextLink)
        ));
        assert!(matches!(
            Endpoint::builder()
                .base_url(Url::parse("http://broker.test/ngsi-ld/v1/").unwrap())
                .context(array)
                .build()
                .unwrap()
                .with_context_profile(profile),
            Err(EndpointError::ProfileWithoutContextLink)
        ));
        assert_eq!(endpoint.without_context_profile().context_profile(), None);
    }

    #[test]
    fn a_further_link_is_kept_unless_it_names_a_context() {
        let profile = LinkValue::builder(IriRefBuf::new("https://uri.etsi.org/ngsi-ld/profile/version").unwrap())
            .rel(RelationType::Profile)
            .build();
        let endpoint = endpoint().with_link(Link::try_new(profile).unwrap()).unwrap();
        assert!(endpoint.link().is_some());
        let context = Link::try_json_ld_context(IriRefBuf::new("https://example.org/context.jsonld").unwrap()).unwrap();
        assert!(matches!(endpoint.with_link(context), Err(EndpointError::ContextLink)));
    }

    #[test]
    fn an_accept_override_is_kept_and_can_be_cleared() {
        let endpoint = endpoint().with_accept(Some(Accept::new(NgsiLdMediaType::JsonLd)));
        assert_eq!(endpoint.accept(), Some(&Accept::new(NgsiLdMediaType::JsonLd)));
        assert_eq!(endpoint.with_accept(None).accept(), None);
    }
}
