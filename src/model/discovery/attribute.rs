//! The attribute documents of discovery: the attribute list (NGSI-LD clauses 4.5.13 and 5.2.27) and the three
//! shapes of the `Attribute` data type of clause 5.2.28: an element of `attributeDetails`, an element of the
//! detailed attribute list (clause 4.5.14) and the information about one attribute (clause 4.5.15).

use crate::model::{
    AttributeKind,
    AttributeName,
    Context,
    DocumentError,
    EntityTypeName,
    RawJson,
    Uri,
    discovery::{
        DiscoveryType,
        members::{AttributeMembers, AttributeShape, expect_type, head, names, names_json, uri},
    },
    keyword::CONTEXT,
    reader::{context, object, once, parse_json, required, unexpected, within},
    representation::write::write_context,
};
use jstrict::{Object, Value as Json, object::Entry};

/// The attributes a broker holds instances of (NGSI-LD clauses 4.5.13 and 5.2.27), the answer of Retrieve
/// Available Attributes.
///
/// ```
/// use ngsi_ld::model::discovery::AttributeList;
///
/// let text = r#"{"id":"urn:ngsi-ld:AttributeList:1","type":"AttributeList","attributeList":["speed","brandName"]}"#;
/// let list = AttributeList::parse(text)?;
/// assert_eq!(list.attribute_list.len(), 2);
/// assert_eq!(list.render().to_string(), text);
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct AttributeList {
    /// `id`: a URI identifying the list.
    pub id: Uri,
    /// `attributeList`: the attribute names, short where the request's context defines them.
    pub attribute_list: Vec<AttributeName>,
    /// The document's `@context`, when it carries one.
    pub context: Option<Context>,
}

impl AttributeList {
    /// The list `id` of `attribute_list`, without an `@context`.
    #[must_use]
    pub const fn new(id: Uri, attribute_list: Vec<AttributeName>) -> AttributeList {
        AttributeList {
            id,
            attribute_list,
            context: None,
        }
    }

    /// Reads the list.
    ///
    /// # Errors
    ///
    /// Returns [`DocumentError`] naming the member at fault.
    pub fn parse(text: &str) -> Result<Self, DocumentError> {
        Self::from_json(parse_json(text)?)
    }

    /// Writes the list in canonical member order: `id`, `type`, `attributeList`, `@context`.
    #[must_use]
    pub fn render(&self) -> RawJson {
        let mut object = head(&self.id, DiscoveryType::AttributeList);
        object.push("attributeList".into(), names_json(&self.attribute_list));
        write_context(&mut object, self.context.as_ref());
        RawJson::from_json(Json::Object(object))
    }

    /// Reads the list from JSON already parsed.
    pub(crate) fn from_json(json: Json) -> Result<Self, DocumentError> {
        let mut id = None;
        let mut typed = None;
        let mut attribute_list = None;
        let mut document_context = None;
        for Entry { key, value } in object(json)? {
            let key = key.as_str();
            match key {
                "id" => once(&mut id, key, within(key, uri(value))?)?,
                "type" => once(&mut typed, key, expect_type(value, DiscoveryType::AttributeList)?)?,
                "attributeList" => once(&mut attribute_list, key, within(key, names(value, "attributeList"))?)?,
                CONTEXT => once(&mut document_context, key, within(key, context(value))?)?,
                other => return Err(unexpected(other)),
            }
        }
        required(typed, "type")?;
        Ok(Self {
            id: required(id, "id")?,
            attribute_list: required(attribute_list, "attributeList")?,
            context: document_context,
        })
    }
}

/// One attribute of an entity type, with the kinds its instances have: an element of the `attributeDetails` of
/// [`EntityTypeInfo`](crate::model::discovery::EntityTypeInfo), the `Attribute` data type of clause
/// 5.2.28 restricted by Table 5.2.26-1 to `id`, `type`, `attributeName` and `attributeTypes`.
///
/// ```
/// use iri_rs::iri;
/// use ngsi_ld::model::{AttributeKind, discovery::EntityTypeAttribute};
///
/// let mut speed =
///     EntityTypeAttribute::new(iri!("https://example.org/speed").into(), "speed".parse()?);
/// speed.attribute_types = Some(vec![AttributeKind::Property]);
/// assert_eq!(speed.attribute_name.as_ref(), "speed");
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct EntityTypeAttribute {
    /// `id`: the Fully Qualified Name of the attribute.
    pub id: Uri,
    /// `attributeName`: the name of the attribute, short where the request's context defines it.
    pub attribute_name: AttributeName,
    /// `attributeTypes`: the kinds of the attribute's instances; `None` when the member is absent.
    pub attribute_types: Option<Vec<AttributeKind>>,
}

impl EntityTypeAttribute {
    /// The attribute `id`, named `attribute_name`, with no `attributeTypes`.
    #[must_use]
    pub const fn new(id: Uri, attribute_name: AttributeName) -> EntityTypeAttribute {
        EntityTypeAttribute {
            id,
            attribute_name,
            attribute_types: None,
        }
    }

    /// Reads an element of `attributeDetails`.
    pub(crate) fn from_json(json: Json) -> Result<Self, DocumentError> {
        let members = AttributeMembers::read(json, AttributeShape::EntityTypeElement)?;
        Ok(Self {
            id: members.id,
            attribute_name: members.attribute_name,
            attribute_types: members.attribute_types,
        })
    }

    /// Writes the element in canonical member order: `id`, `type`, `attributeName`, `attributeTypes`.
    pub(crate) fn to_object(&self) -> Object {
        AttributeMembers::write(&self.id, &self.attribute_name, None, self.attribute_types.as_deref(), None, None)
    }
}

/// One attribute with the entity types whose instances have it (NGSI-LD clause 4.5.14): an element of the answer
/// of Retrieve Details of Available Attributes, the `Attribute` data type of clause 5.2.28 restricted by clause
/// 5.7.9.4 to `id`, `type`, `attributeName` and `typeNames`.
///
/// ```
/// use ngsi_ld::model::discovery::AttributeDetails;
///
/// let text = r#"{"id":"https://example.org/speed","type":"Attribute","attributeName":"speed","typeNames":["Vehicle"]}"#;
/// let speed = AttributeDetails::parse(text)?;
/// assert_eq!(speed.type_names.as_ref().map(Vec::len), Some(1));
/// assert_eq!(speed.render().to_string(), text);
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct AttributeDetails {
    /// `id`: the Fully Qualified Name of the attribute.
    pub id: Uri,
    /// `attributeName`: the name of the attribute, short where the request's context defines it.
    pub attribute_name: AttributeName,
    /// `typeNames`: the entity types whose instances have the attribute; `None` when the member is absent.
    pub type_names: Option<Vec<EntityTypeName>>,
    /// The document's `@context`, when it carries one.
    pub context: Option<Context>,
}

impl AttributeDetails {
    /// The attribute `id`, named `attribute_name`, with no `typeNames` and no `@context`.
    #[must_use]
    pub const fn new(id: Uri, attribute_name: AttributeName) -> AttributeDetails {
        AttributeDetails {
            id,
            attribute_name,
            type_names: None,
            context: None,
        }
    }

    /// Reads the document.
    ///
    /// # Errors
    ///
    /// Returns [`DocumentError`] naming the member at fault.
    pub fn parse(text: &str) -> Result<Self, DocumentError> {
        Self::from_json(parse_json(text)?)
    }

    /// Writes the document in canonical member order: `id`, `type`, `attributeName`, `typeNames`, `@context`.
    #[must_use]
    pub fn render(&self) -> RawJson {
        RawJson::from_json(Json::Object(AttributeMembers::write(
            &self.id,
            &self.attribute_name,
            None,
            None,
            self.type_names.as_deref(),
            self.context.as_ref(),
        )))
    }

    /// Reads the document from JSON already parsed, such as an element of the detailed list.
    pub(crate) fn from_json(json: Json) -> Result<Self, DocumentError> {
        let members = AttributeMembers::read(json, AttributeShape::Details)?;
        Ok(Self {
            id: members.id,
            attribute_name: members.attribute_name,
            type_names: members.type_names,
            context: members.context,
        })
    }
}

/// What a broker holds of one attribute (NGSI-LD clauses 4.5.15 and 5.2.28): the answer of Retrieve Available
/// Attribute Information.
///
/// ```
/// use ngsi_ld::model::{AttributeKind, discovery::AttributeInfo};
///
/// let text = r#"{"id":"https://example.org/speed","type":"Attribute","attributeName":"speed","attributeCount":3,"attributeTypes":["Property"],"typeNames":["Vehicle"]}"#;
/// let speed = AttributeInfo::parse(text)?;
/// assert_eq!(speed.attribute_count, Some(3));
/// assert_eq!(speed.attribute_types, Some(vec![AttributeKind::Property]));
/// assert_eq!(speed.render().to_string(), text);
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct AttributeInfo {
    /// `id`: the Fully Qualified Name of the attribute.
    pub id: Uri,
    /// `attributeName`: the name of the attribute, short where the request's context defines it.
    pub attribute_name: AttributeName,
    /// `attributeCount`: how many instances of the attribute there are.
    pub attribute_count: Option<u64>,
    /// `attributeTypes`: the kinds of the attribute's instances.
    pub attribute_types: Option<Vec<AttributeKind>>,
    /// `typeNames`: the entity types whose instances have the attribute.
    pub type_names: Option<Vec<EntityTypeName>>,
    /// The document's `@context`, when it carries one.
    pub context: Option<Context>,
}

impl AttributeInfo {
    /// The attribute `id`, named `attribute_name`, with none of the optional members.
    #[must_use]
    pub const fn new(id: Uri, attribute_name: AttributeName) -> AttributeInfo {
        AttributeInfo {
            id,
            attribute_name,
            attribute_count: None,
            attribute_types: None,
            type_names: None,
            context: None,
        }
    }

    /// Reads the document.
    ///
    /// # Errors
    ///
    /// Returns [`DocumentError`] naming the member at fault.
    pub fn parse(text: &str) -> Result<Self, DocumentError> {
        Self::from_json(parse_json(text)?)
    }

    /// Writes the document in canonical member order: `id`, `type`, `attributeName`, `attributeCount`,
    /// `attributeTypes`, `typeNames`, `@context`.
    #[must_use]
    pub fn render(&self) -> RawJson {
        RawJson::from_json(Json::Object(AttributeMembers::write(
            &self.id,
            &self.attribute_name,
            self.attribute_count,
            self.attribute_types.as_deref(),
            self.type_names.as_deref(),
            self.context.as_ref(),
        )))
    }

    /// Reads the document from JSON already parsed.
    pub(crate) fn from_json(json: Json) -> Result<Self, DocumentError> {
        let members = AttributeMembers::read(json, AttributeShape::Info)?;
        Ok(Self {
            id: members.id,
            attribute_name: members.attribute_name,
            attribute_count: members.attribute_count,
            attribute_types: members.attribute_types,
            type_names: members.type_names,
            context: members.context,
        })
    }
}
