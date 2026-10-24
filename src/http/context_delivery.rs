//! How a request carries its JSON-LD `@context` (NGSI-LD clause 6.3.5).

/// Where a request with a body carries its `@context` (NGSI-LD clause 6.3.5).
///
/// GET and DELETE requests always carry the context in a `Link` header; this choice decides POST, PATCH and PUT,
/// where clause 6.3.5 forbids mixing the two: with `application/json` the context goes in the `Link` header and
/// never in the body, with `application/ld+json` it goes in the body and the `Link` header is never sent. Clause
/// 6.3.4 also lets a PATCH body be `application/merge-patch+json` (IETF RFC 7396), which a broker reads as
/// `application/json`, so its context goes in the `Link` header too. The library defines this choice, so its text
/// form is kebab-case.
///
/// The enum is exhaustive: clause 6.3.4 closes the set of media types a request body is sent as.
///
/// ```
/// use ngsi_ld::http::ContextDelivery;
///
/// assert_eq!(ContextDelivery::default(), ContextDelivery::JsonWithLink);
/// assert_eq!(ContextDelivery::LdJsonBody.to_string(), "ld-json-body");
/// assert_eq!(
///     ContextDelivery::MergePatchWithLink.to_string(),
///     "merge-patch-with-link"
/// );
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, strum::Display, strum::EnumString)]
#[strum(serialize_all = "kebab-case")]
pub enum ContextDelivery {
    /// `Content-Type: application/json`, the context in a `Link` header.
    #[default]
    JsonWithLink,
    /// `Content-Type: application/ld+json`, the context as the body's `@context` member.
    LdJsonBody,
    /// `Content-Type: application/merge-patch+json` for a PATCH body and `application/json` for a POST or PUT body,
    /// which clause 6.3.4 admits for PATCH only; the context in a `Link` header.
    MergePatchWithLink,
}
