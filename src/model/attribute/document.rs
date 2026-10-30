//! A lone attribute as a JSON-LD document of its own: the body of Replace Attribute or Partial Attribute Update with the
//! `@context` it carries (NGSI-LD clauses 5.6.4, 5.6.19 and 6.3.5).

use crate::model::{
    Attribute,
    AttributeFragment,
    Change,
    Context,
    DocumentError,
    Input,
    InstanceDeletion,
    Lossless,
    RawJson,
    keyword::CONTEXT,
    reader::{context, duplicate, parse_json, within},
    representation::{
        read::{read_attribute_body, read_lone_instance},
        write::{write_attribute_body, write_attribute_object, write_context},
    },
};
use jstrict::{Object, Value as Json};

/// One attribute as a document of its own, with the `@context` it carries.
///
/// Replace Attribute (NGSI-LD clause 5.6.19) and Partial Attribute Update (clause 5.6.4) take "a JSON-LD document
/// representing an NGSI-LD Attribute Fragment". Sent as `application/ld+json`, that document carries its `@context` at
/// the top (clause 6.3.5), where the request pipeline writes the endpoint's. An attribute nested in an entity has no
/// context of its own, so [`Attribute::parse_input`] and [`Change::<AttributeFragment, InstanceDeletion>::parse`] refuse
/// the member; this document reads it, as a broker or a context source receives the body.
///
/// `A` is what the operation takes: an [`Attribute<Input>`] for Replace Attribute, a
/// [`Change<AttributeFragment, InstanceDeletion>`] for Partial Attribute Update. The document is written as the
/// operation writes its body: always an object, with the `@context` last when it has one.
///
/// The struct is exhaustive: the document is the attribute and its context, and nothing else (clause 6.3.5).
///
/// ```
/// use iri_rs::IriRefBuf;
/// use ngsi_ld::model::{Attribute, AttributeDocument, Context, Input, Lossless};
///
/// let body = r#"{"@context":"https://example.org/context.jsonld","type":"Property","value":30}"#;
/// let document = AttributeDocument::<Attribute<Input>>::parse(body, Lossless::Normalized)?;
/// assert_eq!(
///     document
///         .context
///         .as_ref()
///         .and_then(Context::link_target)
///         .map(IriRefBuf::as_str),
///     Some("https://example.org/context.jsonld")
/// );
/// assert_eq!(
///     document.render(Lossless::Normalized).to_string(),
///     r#"{"type":"Property","value":30,"@context":"https://example.org/context.jsonld"}"#
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
///
/// A document is built from its two members:
///
/// ```
/// use iri_rs::IriRefBuf;
/// use ngsi_ld::model::{Attribute, AttributeDocument, Context, Lossless, Property};
///
/// let document = AttributeDocument {
///     attribute: Attribute::from(Property::builder(30).build()?),
///     context: Some(Context::from(IriRefBuf::new(
///         "https://example.org/context.jsonld",
///     )?)),
/// };
/// assert_eq!(
///     document.render(Lossless::Concise).to_string(),
///     r#"{"value":30,"@context":"https://example.org/context.jsonld"}"#
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttributeDocument<A> {
    /// The attribute, as the operation takes it.
    pub attribute: A,
    /// The `@context` the document carried; a body sent as `application/json` has none, its context travels in a
    /// `Link` header (clause 6.3.5).
    pub context: Option<Context>,
}

impl AttributeDocument<Attribute<Input>> {
    /// Reads the body of Replace Attribute (clauses 5.6.19 and 6.7.3.3) in normalized or concise representation,
    /// keeping its `@context`.
    ///
    /// # Errors
    ///
    /// Returns [`DocumentError`] when the `@context` is not a context, or when the rest is not what
    /// [`Attribute::parse_input`] reads.
    pub fn parse(text: &str, form: Lossless) -> Result<Self, DocumentError> {
        let (json, context) = take_context(parse_json(text)?)?;
        Ok(AttributeDocument {
            attribute: read_lone_instance::<Input>(json, form)?,
            context,
        })
    }

    /// Writes the document in normalized or concise representation, as Replace Attribute sends its body.
    #[must_use]
    pub fn render(&self, form: Lossless) -> RawJson {
        document(write_attribute_object(&self.attribute, form), self.context.as_ref())
    }
}

impl AttributeDocument<Change<AttributeFragment, InstanceDeletion>> {
    /// Reads the body of Partial Attribute Update (clauses 5.6.4 and 6.7.3.1) in normalized or concise
    /// representation, keeping its `@context`: an attribute fragment, or the deletion of the instance.
    ///
    /// # Errors
    ///
    /// Returns [`DocumentError`] when the `@context` is not a context, or when the rest is not what
    /// [`Change::<AttributeFragment, InstanceDeletion>::parse`] reads.
    pub fn parse(text: &str, form: Lossless) -> Result<Self, DocumentError> {
        let (json, context) = take_context(parse_json(text)?)?;
        Ok(AttributeDocument {
            attribute: read_attribute_body(json, form)?,
            context,
        })
    }

    /// Writes the document in normalized or concise representation, as Partial Attribute Update sends its body.
    #[must_use]
    pub fn render(&self, form: Lossless) -> RawJson {
        document(write_attribute_body(&self.attribute, form), self.context.as_ref())
    }
}

/// Takes the `@context` out of a document, wherever it is written; a document that is not an object has none.
fn take_context(json: Json) -> Result<(Json, Option<Context>), DocumentError> {
    match json {
        Json::Object(mut object) => {
            let entry = object.remove_unique(CONTEXT).map_err(|_| duplicate(CONTEXT))?;
            let context = entry.map(|entry| within(CONTEXT, context(entry.value))).transpose()?;
            Ok((Json::Object(object), context))
        }
        other @ (Json::Null | Json::Boolean(_) | Json::Number(_) | Json::String(_) | Json::Array(_)) => Ok((other, None)),
    }
}

/// Writes the attribute object with the `@context` last.
fn document(mut object: Object, context: Option<&Context>) -> RawJson {
    write_context(&mut object, context);
    RawJson::from_json(Json::Object(object))
}

#[cfg(test)]
mod tests;
