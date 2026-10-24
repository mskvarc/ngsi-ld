//! The entity type documents of discovery: the entity type list (NGSI-LD clauses 4.5.10 and 5.2.24), an element of
//! the detailed entity type list (clauses 4.5.11 and 5.2.25) and the information about one entity type (clauses
//! 4.5.12 and 5.2.26).

use crate::model::{
    AttributeName,
    Context,
    DocumentError,
    EntityTypeName,
    RawJson,
    Uri,
    discovery::{
        DiscoveryType,
        EntityTypeAttribute,
        members::{expect_type, head, names, names_json, uri},
    },
    keyword::CONTEXT,
    reader::{array, context, count, object, once, parse_json, parsed, required, unexpected, within},
    representation::write::write_context,
};
use jstrict::{Value as Json, object::Entry};

/// The entity types a broker holds instances of (NGSI-LD clauses 4.5.10 and 5.2.24), the answer of Retrieve
/// Available Entity Types.
///
/// ```
/// use ngsi_ld::model::discovery::EntityTypeList;
///
/// let text = r#"{"id":"urn:ngsi-ld:EntityTypeList:1","type":"EntityTypeList","typeList":["Vehicle","OffStreetParking"]}"#;
/// let list = EntityTypeList::parse(text)?;
/// assert_eq!(list.type_list.len(), 2);
/// assert_eq!(list.render().to_string(), text);
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct EntityTypeList {
    /// `id`: a URI identifying the list.
    pub id: Uri,
    /// `typeList`: the entity type names, short where the request's context defines them.
    pub type_list: Vec<EntityTypeName>,
    /// The document's `@context`, when it carries one.
    pub context: Option<Context>,
}

impl EntityTypeList {
    /// The list `id` of `type_list`, without an `@context`.
    #[must_use]
    pub const fn new(id: Uri, type_list: Vec<EntityTypeName>) -> EntityTypeList {
        EntityTypeList { id, type_list, context: None }
    }

    /// Reads the list.
    ///
    /// # Errors
    ///
    /// Returns [`DocumentError`] naming the member at fault.
    pub fn parse(text: &str) -> Result<Self, DocumentError> {
        Self::from_json(parse_json(text)?)
    }

    /// Writes the list in canonical member order: `id`, `type`, `typeList`, `@context`.
    #[must_use]
    pub fn render(&self) -> RawJson {
        let mut object = head(&self.id, DiscoveryType::EntityTypeList);
        object.push("typeList".into(), names_json(&self.type_list));
        write_context(&mut object, self.context.as_ref());
        RawJson::from_json(Json::Object(object))
    }

    /// Reads the list from JSON already parsed.
    pub(crate) fn from_json(json: Json) -> Result<Self, DocumentError> {
        let mut id = None;
        let mut typed = None;
        let mut type_list = None;
        let mut document_context = None;
        for Entry { key, value } in object(json)? {
            let key = key.as_str();
            match key {
                "id" => once(&mut id, key, within(key, uri(value))?)?,
                "type" => once(&mut typed, key, expect_type(value, DiscoveryType::EntityTypeList)?)?,
                "typeList" => once(&mut type_list, key, within(key, names(value, "typeList"))?)?,
                CONTEXT => once(&mut document_context, key, within(key, context(value))?)?,
                other => return Err(unexpected(other)),
            }
        }
        required(typed, "type")?;
        Ok(Self {
            id: required(id, "id")?,
            type_list: required(type_list, "typeList")?,
            context: document_context,
        })
    }
}

/// One entity type with the names of the attributes its instances have (NGSI-LD clauses 4.5.11 and 5.2.25): an
/// element of the answer of Retrieve Details of Available Entity Types.
///
/// Clause 5.2.25 names this data type `EntityType`, its `type` token; the crate keeps that name for the `type`
/// member of an entity.
///
/// ```
/// use ngsi_ld::model::discovery::EntityTypeDetails;
///
/// let text = r#"{"id":"https://example.org/Vehicle","type":"EntityType","attributeNames":["speed"],"typeName":"Vehicle"}"#;
/// let vehicle = EntityTypeDetails::parse(text)?;
/// assert_eq!(vehicle.type_name.as_ref(), "Vehicle");
/// assert_eq!(vehicle.render().to_string(), text);
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct EntityTypeDetails {
    /// `id`: the Fully Qualified Name of the entity type.
    pub id: Uri,
    /// `attributeNames`: the names of the attributes instances of the type have.
    pub attribute_names: Vec<AttributeName>,
    /// `typeName`: the name of the type, short where the request's context defines it.
    pub type_name: EntityTypeName,
    /// The document's `@context`, when it carries one.
    pub context: Option<Context>,
}

impl EntityTypeDetails {
    /// The type `id`, named `type_name`, whose instances have `attribute_names`, without an `@context`.
    #[must_use]
    pub const fn new(id: Uri, type_name: EntityTypeName, attribute_names: Vec<AttributeName>) -> EntityTypeDetails {
        EntityTypeDetails {
            id,
            attribute_names,
            type_name,
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

    /// Writes the document in canonical member order: `id`, `type`, `attributeNames`, `typeName`, `@context`.
    #[must_use]
    pub fn render(&self) -> RawJson {
        let mut object = head(&self.id, DiscoveryType::EntityType);
        object.push("attributeNames".into(), names_json(&self.attribute_names));
        object.push("typeName".into(), Json::from(self.type_name.as_ref()));
        write_context(&mut object, self.context.as_ref());
        RawJson::from_json(Json::Object(object))
    }

    /// Reads the document from JSON already parsed, such as an element of the detailed list.
    pub(crate) fn from_json(json: Json) -> Result<Self, DocumentError> {
        let mut id = None;
        let mut typed = None;
        let mut attribute_names = None;
        let mut type_name = None;
        let mut document_context = None;
        for Entry { key, value } in object(json)? {
            let key = key.as_str();
            match key {
                "id" => once(&mut id, key, within(key, uri(value))?)?,
                "type" => once(&mut typed, key, expect_type(value, DiscoveryType::EntityType)?)?,
                "attributeNames" => once(&mut attribute_names, key, within(key, names(value, "attributeNames"))?)?,
                "typeName" => once(&mut type_name, key, within(key, parsed(value, "typeName"))?)?,
                CONTEXT => once(&mut document_context, key, within(key, context(value))?)?,
                other => return Err(unexpected(other)),
            }
        }
        required(typed, "type")?;
        Ok(Self {
            id: required(id, "id")?,
            attribute_names: required(attribute_names, "attributeNames")?,
            type_name: required(type_name, "typeName")?,
            context: document_context,
        })
    }
}

/// What a broker holds of one entity type (NGSI-LD clauses 4.5.12 and 5.2.26): the answer of Retrieve Available
/// Entity Type Information.
///
/// Clause 4.5.12 lists only `id`, `type` and `typeName` as mandatory; Table 5.2.26-1, which clause 5.7.7.5 names
/// as the output of the operation, makes `attributeDetails` and `entityCount` mandatory too, so they are required.
///
/// ```
/// use ngsi_ld::model::discovery::EntityTypeInfo;
///
/// let text = concat!(
///     r#"{"id":"https://example.org/Vehicle","type":"EntityTypeInfo","attributeDetails":[{"id":"https://example.org/speed","#,
///     r#""type":"Attribute","attributeName":"speed","attributeTypes":["Property"]}],"entityCount":2,"typeName":"Vehicle"}"#
/// );
/// let vehicle = EntityTypeInfo::parse(text)?;
/// assert_eq!(vehicle.entity_count, 2);
/// assert_eq!(vehicle.attribute_details[0].attribute_name.as_ref(), "speed");
/// assert_eq!(vehicle.render().to_string(), text);
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct EntityTypeInfo {
    /// `id`: the Fully Qualified Name of the entity type.
    pub id: Uri,
    /// `attributeDetails`: the attributes instances of the type have, with their kinds.
    pub attribute_details: Vec<EntityTypeAttribute>,
    /// `entityCount`: how many instances of the type there are.
    pub entity_count: u64,
    /// `typeName`: the name of the type, short where the request's context defines it.
    pub type_name: EntityTypeName,
    /// The document's `@context`, when it carries one.
    pub context: Option<Context>,
}

impl EntityTypeInfo {
    /// The type `id`, named `type_name`, with `entity_count` instances whose attributes are `attribute_details`,
    /// without an `@context`.
    #[must_use]
    pub const fn new(id: Uri, type_name: EntityTypeName, entity_count: u64, attribute_details: Vec<EntityTypeAttribute>) -> EntityTypeInfo {
        EntityTypeInfo {
            id,
            attribute_details,
            entity_count,
            type_name,
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

    /// Writes the document in canonical member order: `id`, `type`, `attributeDetails`, `entityCount`,
    /// `typeName`, `@context`.
    #[must_use]
    pub fn render(&self) -> RawJson {
        let mut object = head(&self.id, DiscoveryType::EntityTypeInfo);
        object.push(
            "attributeDetails".into(),
            Json::Array(self.attribute_details.iter().map(|attribute| Json::Object(attribute.to_object())).collect()),
        );
        object.push("entityCount".into(), Json::from(self.entity_count));
        object.push("typeName".into(), Json::from(self.type_name.as_ref()));
        write_context(&mut object, self.context.as_ref());
        RawJson::from_json(Json::Object(object))
    }

    /// Reads the document from JSON already parsed.
    pub(crate) fn from_json(json: Json) -> Result<Self, DocumentError> {
        let mut id = None;
        let mut typed = None;
        let mut attribute_details = None;
        let mut entity_count = None;
        let mut type_name = None;
        let mut document_context = None;
        for Entry { key, value } in object(json)? {
            let key = key.as_str();
            match key {
                "id" => once(&mut id, key, within(key, uri(value))?)?,
                "type" => once(&mut typed, key, expect_type(value, DiscoveryType::EntityTypeInfo)?)?,
                "attributeDetails" => once(&mut attribute_details, key, within(key, details(value))?)?,
                "entityCount" => once(&mut entity_count, key, within(key, count(value, "entityCount"))?)?,
                "typeName" => once(&mut type_name, key, within(key, parsed(value, "typeName"))?)?,
                CONTEXT => once(&mut document_context, key, within(key, context(value))?)?,
                other => return Err(unexpected(other)),
            }
        }
        required(typed, "type")?;
        Ok(Self {
            id: required(id, "id")?,
            attribute_details: required(attribute_details, "attributeDetails")?,
            entity_count: required(entity_count, "entityCount")?,
            type_name: required(type_name, "typeName")?,
            context: document_context,
        })
    }
}

/// Reads `attributeDetails`, naming the index of an element at fault.
fn details(json: Json) -> Result<Vec<EntityTypeAttribute>, DocumentError> {
    array(json)?
        .into_iter()
        .enumerate()
        .map(|(index, item)| EntityTypeAttribute::from_json(item).map_err(|error| error.at_index(index)))
        .collect()
}
