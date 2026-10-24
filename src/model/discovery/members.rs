//! Reading and writing the members the discovery documents share: their object, their `type` token, lists of
//! names, attribute kinds and counts, and the three shapes of the `Attribute` data type of clause 5.2.28, which
//! differ only in the members each allows.

use crate::model::{
    AttributeKind,
    AttributeName,
    Context,
    DocumentError,
    DocumentErrorKind,
    EntityTypeName,
    NameError,
    Uri,
    discovery::DiscoveryType,
    keyword::CONTEXT,
    reader::{array, context, count, identifier, object, once, parsed, required, string, unexpected, within},
    representation::write::write_context,
};
use jstrict::{Object, Value as Json, object::Entry};
use std::str::FromStr;

/// Checks the `type` member.
pub(crate) fn expect_type(json: Json, expected: DiscoveryType) -> Result<(), DocumentError> {
    let found = string(json, "type")?;
    if found == <&str>::from(expected) {
        Ok(())
    } else {
        Err(DocumentError::top(DocumentErrorKind::DiscoveryType { expected, found }))
    }
}

/// Reads the `id` member: a URI.
pub(crate) fn uri(json: Json) -> Result<Uri, DocumentError> {
    identifier(json, "id")
}

/// Reads an array of names.
pub(crate) fn names<T: FromStr<Err = NameError>>(json: Json, member: &'static str) -> Result<Vec<T>, DocumentError> {
    array(json)?.into_iter().map(|item| parsed(item, member)).collect()
}

/// Reads `attributeTypes`: an array of attribute kinds.
fn kinds(json: Json) -> Result<Vec<AttributeKind>, DocumentError> {
    array(json)?
        .into_iter()
        .map(|item| {
            let found = string(item, "attributeTypes")?;
            found.parse().map_err(|_| DocumentError::top(DocumentErrorKind::UnknownAttributeType { found }))
        })
        .collect()
}

/// Writes names as an array of strings.
pub(crate) fn names_json<T: AsRef<str>>(names: &[T]) -> Json {
    Json::Array(names.iter().map(|name| Json::from(name.as_ref())).collect())
}

/// Starts a document: its `id` and `type`.
pub(crate) fn head(id: &Uri, document: DiscoveryType) -> Object {
    let mut object = Object::new();
    object.push("id".into(), Json::from(id.as_str()));
    object.push("type".into(), Json::from(<&str>::from(document)));
    object
}

/// The three shapes of the `Attribute` data type of clause 5.2.28, which allow different members beside `id`,
/// `type` and `attributeName`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AttributeShape {
    /// An element of `attributeDetails`: `attributeTypes` only (Table 5.2.26-1).
    EntityTypeElement,
    /// An element of the detailed attribute list: `typeNames` and `@context` (clauses 4.5.14 and 5.7.9.4).
    Details,
    /// The attribute information: `attributeCount`, `attributeTypes`, `typeNames` and `@context` (clause 4.5.15).
    Info,
}

impl AttributeShape {
    /// Whether the shape allows `attributeCount`.
    const fn allows_count(self) -> bool {
        matches!(self, Self::Info)
    }

    /// Whether the shape allows `attributeTypes`.
    const fn allows_kinds(self) -> bool {
        matches!(self, Self::EntityTypeElement | Self::Info)
    }

    /// Whether the shape allows `typeNames` and `@context`.
    const fn allows_type_names_and_context(self) -> bool {
        matches!(self, Self::Details | Self::Info)
    }
}

/// The members of an `Attribute` document (Table 5.2.28-1), as far as its shape allows them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AttributeMembers {
    /// `id`.
    pub(crate) id: Uri,
    /// `attributeName`.
    pub(crate) attribute_name: AttributeName,
    /// `attributeCount`.
    pub(crate) attribute_count: Option<u64>,
    /// `attributeTypes`.
    pub(crate) attribute_types: Option<Vec<AttributeKind>>,
    /// `typeNames`.
    pub(crate) type_names: Option<Vec<EntityTypeName>>,
    /// `@context`.
    pub(crate) context: Option<Context>,
}

impl AttributeMembers {
    /// Reads an `Attribute` document of `shape`, refusing the members the shape does not allow.
    pub(crate) fn read(json: Json, shape: AttributeShape) -> Result<Self, DocumentError> {
        let mut id = None;
        let mut typed = None;
        let mut attribute_name = None;
        let mut attribute_count = None;
        let mut attribute_types = None;
        let mut type_names = None;
        let mut document_context = None;
        for Entry { key, value } in object(json)? {
            let key = key.as_str();
            match key {
                "id" => once(&mut id, key, within(key, uri(value))?)?,
                "type" => once(&mut typed, key, expect_type(value, DiscoveryType::Attribute)?)?,
                "attributeName" => once(&mut attribute_name, key, within(key, parsed(value, "attributeName"))?)?,
                "attributeCount" if shape.allows_count() => once(&mut attribute_count, key, within(key, count(value, "attributeCount"))?)?,
                "attributeTypes" if shape.allows_kinds() => once(&mut attribute_types, key, within(key, kinds(value))?)?,
                "typeNames" if shape.allows_type_names_and_context() => once(&mut type_names, key, within(key, names(value, "typeNames"))?)?,
                CONTEXT if shape.allows_type_names_and_context() => once(&mut document_context, key, within(key, context(value))?)?,
                other => return Err(unexpected(other)),
            }
        }
        required(typed, "type")?;
        Ok(Self {
            id: required(id, "id")?,
            attribute_name: required(attribute_name, "attributeName")?,
            attribute_count,
            attribute_types,
            type_names,
            context: document_context,
        })
    }

    /// Writes an `Attribute` document in canonical member order: `id`, `type`, `attributeName`, `attributeCount`,
    /// `attributeTypes`, `typeNames`, `@context`, each optional member when present.
    pub(crate) fn write(
        id: &Uri,
        attribute_name: &AttributeName,
        attribute_count: Option<u64>,
        attribute_types: Option<&[AttributeKind]>,
        type_names: Option<&[EntityTypeName]>,
        document_context: Option<&Context>,
    ) -> Object {
        let mut object = head(id, DiscoveryType::Attribute);
        object.push("attributeName".into(), Json::from(attribute_name.as_ref()));
        if let Some(count) = attribute_count {
            object.push("attributeCount".into(), Json::from(count));
        }
        if let Some(kinds) = attribute_types {
            object.push(
                "attributeTypes".into(),
                Json::Array(kinds.iter().map(|kind| Json::from(kind.as_str())).collect()),
            );
        }
        if let Some(type_names) = type_names {
            object.push("typeNames".into(), names_json(type_names));
        }
        write_context(&mut object, document_context);
        object
    }
}
