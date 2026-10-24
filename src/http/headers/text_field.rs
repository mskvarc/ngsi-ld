//! A header field whose value is one typed value written as text: a URI reference, such as `Location` (RFC 9110
//! section 10.2.2) and `NGSILD-EntityMap` (NGSI-LD clause 6.4.3.2), a tenant (`NGSILD-Tenant`, clause 6.3.14) or a
//! snapshot id (`NGSILD-Snapshot`, clause 6.3.22).

use crate::{
    http::headers::{
        InvalidFieldValue,
        field_value::{decode_singleton, encode_text},
    },
    model::Tenant,
    snapshot::SnapshotId,
};
use http::HeaderValue;
use iri_rs::IriRefBuf;
use std::iter;

/// A typed value a header field carries as its text.
pub(crate) trait FieldText: Sized {
    /// The value as the field writes it.
    fn field_text(&self) -> &str;

    /// Reads the value from the text of the field, `None` when the text is not one.
    fn from_field_text(text: &str) -> Option<Self>;
}

impl FieldText for IriRefBuf {
    fn field_text(&self) -> &str {
        self.as_str()
    }

    fn from_field_text(text: &str) -> Option<IriRefBuf> {
        IriRefBuf::new(text.to_owned()).ok()
    }
}

impl FieldText for Tenant {
    fn field_text(&self) -> &str {
        self.as_ref()
    }

    fn from_field_text(text: &str) -> Option<Tenant> {
        text.parse().ok()
    }
}

impl FieldText for SnapshotId {
    fn field_text(&self) -> &str {
        self.as_str()
    }

    fn from_field_text(text: &str) -> Option<SnapshotId> {
        text.parse().ok()
    }
}

/// One typed value and its rendering as a field value.
///
/// The value is rendered once, on construction, and kept: a value holding non-ASCII characters, such as an IRI,
/// cannot be carried verbatim, and [`headers::Header::encode`] cannot fail. A relative reference is kept as
/// written.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct TextField<T> {
    /// The value.
    value: T,
    /// The value rendered as a field value.
    encoded: HeaderValue,
}

impl<T: FieldText> TextField<T> {
    /// The field carrying `value`.
    pub(crate) fn new(value: T) -> Result<TextField<T>, InvalidFieldValue> {
        let encoded = encode_text(value.field_text())?;
        Ok(TextField { value, encoded })
    }

    /// The value.
    pub(crate) const fn value(&self) -> &T {
        &self.value
    }

    /// Takes the value.
    pub(crate) fn into_value(self) -> T {
        self.value
    }

    /// Reads the one value of the field, without surrounding whitespace.
    pub(crate) fn decode<'i, I>(values: &mut I) -> Result<TextField<T>, headers::Error>
    where
        I: Iterator<Item = &'i HeaderValue>,
    {
        let value = T::from_field_text(decode_singleton(values)?).ok_or_else(headers::Error::invalid)?;
        TextField::new(value).map_err(|_| headers::Error::invalid())
    }

    /// Writes the rendered value.
    pub(crate) fn encode<E: Extend<HeaderValue>>(&self, values: &mut E) {
        // `encode` borrows the header; cloning a `HeaderValue` only bumps the reference count of its shared bytes.
        values.extend(iter::once(self.encoded.clone()));
    }
}
