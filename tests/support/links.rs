//! `Link` header fields built from typed link-values, shared by the test crates that write or expect one.

use iri_rs::{IriBuf, IriRefBuf};
use ngsi_ld::{
    http::headers::{Link, LinkValue, RelationType},
    model::APPLICATION_LD_JSON,
};

/// The context every test endpoint names.
pub(crate) const CONTEXT: &str = "https://example.org/context.jsonld";

/// The `Link` naming the test context (clause 6.3.5).
pub(crate) fn context_link() -> Link {
    Link::json_ld_context(IriRefBuf::new(CONTEXT.to_owned()).unwrap()).unwrap()
}

/// The link-value naming `context` with `profile` as its `profile` parameter (clause 6.3.21).
pub(crate) fn profiled_context_value(context: &str, profile: &str) -> LinkValue {
    LinkValue::builder(IriRefBuf::new(context.to_owned()).unwrap())
        .rel(RelationType::JsonLdContext)
        .media_type(APPLICATION_LD_JSON)
        .profile(IriBuf::new(profile.to_owned()).unwrap())
        .build()
}

/// A link-value to `target` with the relation `rel`, such as the next page of a query (clause 6.3.10).
pub(crate) fn link_value(target: &str, rel: RelationType) -> LinkValue {
    LinkValue::builder(IriRefBuf::new(target.to_owned()).unwrap()).rel(rel).build()
}

/// The `Link` field value carrying `values`, in order, as a broker writes it.
pub(crate) fn link_field(values: impl IntoIterator<Item = LinkValue>) -> String {
    Link::try_from_values(values.into_iter().collect()).unwrap().to_string()
}
