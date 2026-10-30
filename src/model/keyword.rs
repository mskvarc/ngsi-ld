//! The JSON-LD keywords the documents of NGSI-LD carry (NGSI-LD clause 4.4, W3C JSON-LD 1.1 section 1.7): the
//! `@context` member, the `@none` key of a language map, a dataset id or an entity map, and the `@type` and
//! `@value` members of a JSON-LD value object, and the `@json` datatype of a JSON literal.

/// `@context`, the member that carries the context of a document (clause 4.4).
pub(crate) const CONTEXT: &str = "@context";

/// `@none`: the default language of a language map (clause 4.6.5), the default instance of an attribute (clause
/// 4.5.5), the entity a broker holds itself in an entity map (Table 5.2.39-2), and no join (clause 4.5.23.2).
pub(crate) const NONE: &str = "@none";

/// `@type`, the datatype member of a JSON-LD value object (clause 4.6.3).
pub(crate) const TYPE: &str = "@type";

/// `@json`, the datatype of a JSON-LD JSON literal, which is not expanded and so can hold a JSON `null` (clause 4.4).
pub(crate) const JSON: &str = "@json";

/// `@value`, the lexical form member of a JSON-LD value object (clause 4.6.3).
pub(crate) const VALUE: &str = "@value";
