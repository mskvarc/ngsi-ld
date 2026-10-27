//! The `EntitySelector` of NGSI-LD clause 5.2.33: which entities a query or a subscription is about, by type and,
//! optionally, by identifier or identifier pattern.

#[cfg(feature = "linked-data")]
use crate::model::name::mapping::{MapNames, NameMapping, member};
use crate::{
    model::{
        BuildError,
        DocumentError,
        EntityId,
        MemberError,
        OneOrMany,
        RawJson,
        keyword_alias::unalias_members,
        reader::{identifier, object, once, parse_json, required, string, unexpected, within},
    },
    query::{
        IdPattern,
        ParseError,
        Problem,
        type_selection::{self, SelectableType, SelectableTypeError, TypeSelection, builders::TypeSelectionBuilder},
    },
};
use jstrict::{Object, Value as Json, object::Entry};
use std::{fmt, str::FromStr};

/// The `type` of an [`EntitySelector`] and of a query.
///
/// It is an entity type selection (NGSI-LD clause 4.17), or `*` for the entities of every type, which implies local
/// scope (Table 5.2.33-1, and the `type` parameter of Tables 6.4.3.2-1, 6.4.3.3-1 and 6.18.3.2-1). An operation on one
/// entity takes a [`TypeSelection`] alone, since its table does not allow `*` (Table 6.5.3.1-1).
///
/// The enum is exhaustive: Table 5.2.33-1 allows a type selection or `*`, and the type selection language of clause
/// 4.17 grows inside [`TypeSelection`], not here.
///
/// ```
/// use ngsi_ld::query::SelectorType;
///
/// let vehicles: SelectorType = "Vehicle".parse()?;
/// assert_eq!(vehicles.to_string(), "Vehicle");
/// assert_eq!("*".parse::<SelectorType>()?, SelectorType::All);
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::From, derive_more::IsVariant)]
pub enum SelectorType {
    /// `*`: every entity type, with local scope implied.
    #[from(skip)]
    All,
    /// The entity types a selection names.
    Selection(TypeSelection),
}

// `*` and a selection print differently, which a derive cannot express.
impl fmt::Display for SelectorType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::All => f.write_str("*"),
            Self::Selection(selection) => selection.fmt(f),
        }
    }
}

// `*` is tried before the type selection grammar, which a derive cannot express.
impl FromStr for SelectorType {
    type Err = ParseError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        if text == "*" { Ok(Self::All) } else { text.parse().map(Self::Selection) }
    }
}

/// The `type` of a query under construction.
///
/// It is every entity type (`*`), or a type selection whose names are checked when it is built (NGSI-LD clause 4.17,
/// Table 6.4.3.2-1).
///
/// A string names one entity type, as [`type_selection::entity_type`] does; `*` is not an entity type name, so
/// every type is asked for with [`SelectorTypeBuilder::All`] or [`SelectorType::All`].
///
/// The enum is exhaustive, as [`SelectorType`] is.
///
/// ```
/// use ngsi_ld::query::{SelectorType, builders::SelectorTypeBuilder, type_selection};
///
/// let vehicles = SelectorTypeBuilder::from("Vehicle").build()?;
/// assert_eq!(vehicles.to_string(), "Vehicle");
/// let either = SelectorTypeBuilder::from(type_selection::entity_type("Car").or("Bus")).build()?;
/// assert_eq!(either.to_string(), "Car|Bus");
/// assert_eq!(SelectorTypeBuilder::All.build()?, SelectorType::All);
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, derive_more::From)]
pub enum SelectorTypeBuilder {
    /// `*`: every entity type.
    #[from(skip)]
    All,
    /// A type selection under construction.
    Selection(TypeSelectionBuilder),
}

impl SelectorTypeBuilder {
    /// Returns the `type`.
    ///
    /// # Errors
    ///
    /// Returns a [`BuildError`] listing every invalid entity type of the selection.
    pub fn build(self) -> Result<SelectorType, BuildError<Problem>> {
        match self {
            Self::All => Ok(SelectorType::All),
            Self::Selection(selection) => selection.build().map(SelectorType::Selection),
        }
    }
}

/// A selection of the one entity type `name`.
impl From<&str> for SelectorTypeBuilder {
    fn from(name: &str) -> Self {
        Self::Selection(type_selection::entity_type(name))
    }
}

/// A selection already built.
impl From<TypeSelection> for SelectorTypeBuilder {
    fn from(selection: TypeSelection) -> Self {
        Self::Selection(selection.into())
    }
}

/// A `type` already built.
impl From<SelectorType> for SelectorTypeBuilder {
    fn from(selector_type: SelectorType) -> Self {
        match selector_type {
            SelectorType::All => Self::All,
            SelectorType::Selection(selection) => selection.into(),
        }
    }
}

impl TryFrom<SelectorTypeBuilder> for SelectorType {
    type Error = BuildError<Problem>;

    fn try_from(builder: SelectorTypeBuilder) -> Result<Self, Self::Error> {
        builder.build()
    }
}

draft!(
    /// An entity selector given to a builder.
    ///
    /// It is an [`EntitySelector`], or text naming one entity type, validated when the builder finishes, so every
    /// invalid type is reported together. A list may mix both, each converted with `.into()`; a selector by identifier,
    /// by pattern or of several types is an [`EntitySelector`].
    ///
    /// ```
    /// use iri_rs::iri;
    /// use ngsi_ld::{
    ///     model::EntityId,
    ///     query::{EntitySelector, EntitySelectorDraft},
    /// };
    ///
    /// let one_car =
    ///     EntitySelector::new("Car".parse()?).with_ids(EntityId::from(iri!("urn:ngsi-ld:Car:1")));
    /// let selectors: [EntitySelectorDraft; 2] = [one_car.into(), "Vehicle".into()];
    /// # let _ = selectors;
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    EntitySelectorDraft(EntitySelector, SelectableTypeError),
    parse = |text: &str| text.parse().map(EntitySelector::of_type)
);

/// Why JSON data is not an `EntitySelector`.
///
/// NGSI-LD clause 5.2.33 defines the selector. A reader reports the error as a
/// [`DocumentError<EntitySelectorError>`](DocumentError), whose `path` names the member at fault.
// `ParseError` carries no `Clone` or equality, so neither can the error that chains it.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum EntitySelectorError {
    /// A problem every reader can meet: the text is not JSON, the data is not an object, a member is missing,
    /// repeated, unknown or of the wrong JSON type, or an `id` is not a URI.
    #[error(transparent)]
    Member(#[from] MemberError),
    /// `type` is not a type selection (clause 4.17) or `*`.
    #[error("invalid entity type selection")]
    Type {
        /// Why the selection is invalid.
        #[source]
        source: ParseError,
    },
    /// `id` is an empty array.
    #[error("an entity selector `id` array cannot be empty")]
    EmptyIds,
    /// `idPattern` is not a regular expression.
    #[error("invalid id pattern")]
    IdPattern {
        /// Why the pattern is invalid.
        #[source]
        source: <IdPattern as FromStr>::Err,
    },
}

/// Selects entities by type and, optionally, by identifier or identifier pattern.
///
/// NGSI-LD clause 5.2.33 defines it; `id` takes precedence over `idPattern`.
///
/// ```
/// use iri_rs::iri;
/// use ngsi_ld::{model::EntityId, query::EntitySelector};
///
/// let selector = EntitySelector::new("Vehicle".parse()?)
///     .with_ids(EntityId::from(iri!("urn:ngsi-ld:Vehicle:A4567")));
/// assert_eq!(
///     selector.render().to_string(),
///     r#"{"type":"Vehicle","id":"urn:ngsi-ld:Vehicle:A4567"}"#
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EntitySelector {
    /// `type`.
    selector_type: SelectorType,
    /// `id`: one identifier or several, written as read.
    ids: Option<OneOrMany<EntityId>>,
    /// `idPattern`.
    id_pattern: Option<IdPattern>,
}

impl EntitySelector {
    /// Selects the entities of `selector_type`.
    #[must_use]
    pub const fn new(selector_type: SelectorType) -> EntitySelector {
        EntitySelector {
            selector_type,
            ids: None,
            id_pattern: None,
        }
    }

    /// Selects the entities of the one entity type `selected`.
    fn of_type(selected: SelectableType) -> EntitySelector {
        Self::new(SelectorType::Selection(TypeSelection::single(selected)))
    }

    /// Selects the entities of every type (`*`), which implies local scope.
    #[must_use]
    pub const fn all() -> EntitySelector {
        Self::new(SelectorType::All)
    }

    /// Restricts the selection to one identifier, or to several.
    #[must_use]
    pub fn with_ids(self, ids: impl Into<OneOrMany<EntityId>>) -> Self {
        Self { ids: Some(ids.into()), ..self }
    }

    /// Restricts the selection to the identifiers `pattern` matches.
    #[must_use]
    pub fn with_id_pattern(self, pattern: IdPattern) -> Self {
        Self {
            id_pattern: Some(pattern),
            ..self
        }
    }

    /// Returns the selected types (`type`).
    #[must_use]
    pub const fn selector_type(&self) -> &SelectorType {
        &self.selector_type
    }

    /// Returns the selected identifiers (`id`).
    #[must_use]
    pub const fn ids(&self) -> Option<&OneOrMany<EntityId>> {
        self.ids.as_ref()
    }

    /// Returns the identifier pattern (`idPattern`).
    #[must_use]
    pub const fn id_pattern(&self) -> Option<&IdPattern> {
        self.id_pattern.as_ref()
    }

    /// Reads the text of an `EntitySelector` object.
    ///
    /// # Errors
    ///
    /// Returns [`DocumentError<EntitySelectorError>`](DocumentError) naming the member at fault and what is
    /// missing or invalid.
    pub fn parse(text: &str) -> Result<Self, DocumentError<EntitySelectorError>> {
        Self::from_json(parse_json(text)?)
    }

    /// Writes the `EntitySelector` object in canonical form: `type`, `id`, `idPattern`.
    #[must_use]
    pub fn render(&self) -> RawJson {
        RawJson::from_json(Json::Object(self.to_object()))
    }

    /// Reads an `EntitySelector` object (Table 5.2.33-1).
    pub(crate) fn from_json(json: Json) -> Result<Self, DocumentError<EntitySelectorError>> {
        let mut selector_type = None;
        let mut ids = None;
        let mut id_pattern = None;
        for Entry { key, value } in unalias_members(object(json)?)? {
            let key = key.as_str();
            match key {
                "type" => once(&mut selector_type, key, within(key, read_type(value))?)?,
                "id" => once(&mut ids, key, within(key, read_ids(value))?)?,
                "idPattern" => once(&mut id_pattern, key, within(key, read_pattern(value))?)?,
                other => return Err(unexpected(other)),
            }
        }
        Ok(Self {
            selector_type: required(selector_type, "type")?,
            ids,
            id_pattern,
        })
    }

    /// Writes the `EntitySelector` object.
    pub(crate) fn to_object(&self) -> Object {
        selector_object(
            &self.selector_type,
            self.ids.as_ref().map(|ids| ids.to_json(|id| Json::from(id.as_str()))),
            self.id_pattern.as_ref(),
        )
    }
}

/// Writes an `EntitySelector` object in canonical order, `type`, `id`, `idPattern`, from its parts: the one writer
/// a subscription and the `Query` body of clause 5.2.23 share.
pub(crate) fn selector_object(selector_type: &dyn fmt::Display, ids: Option<Json>, id_pattern: Option<&IdPattern>) -> Object {
    let mut object = Object::new();
    object.push("type".into(), Json::from(selector_type.to_string()));
    if let Some(ids) = ids {
        object.push("id".into(), ids);
    }
    if let Some(pattern) = id_pattern {
        object.push("idPattern".into(), Json::from(pattern.as_ref()));
    }
    object
}

/// Reads `type`.
fn read_type(json: Json) -> Result<SelectorType, DocumentError<EntitySelectorError>> {
    string(json, "type")?
        .parse()
        .map_err(|source| DocumentError::top(EntitySelectorError::Type { source }))
}

/// Reads `id`: one identifier as a string, or several as an array.
fn read_ids(json: Json) -> Result<OneOrMany<EntityId>, DocumentError<EntitySelectorError>> {
    match json {
        Json::Array(items) => {
            let ids = items
                .into_iter()
                .enumerate()
                .map(|(index, item)| identifier(item, "id").map_err(|error| error.at_index(index)))
                .collect::<Result<Vec<_>, _>>()?;
            OneOrMany::try_from(ids).map_err(|_| DocumentError::top(EntitySelectorError::EmptyIds))
        }
        single @ (Json::Null | Json::Boolean(_) | Json::Number(_) | Json::String(_) | Json::Object(_)) => identifier(single, "id").map(OneOrMany::one),
    }
}

/// Reads `idPattern`.
fn read_pattern(json: Json) -> Result<IdPattern, DocumentError<EntitySelectorError>> {
    string(json, "idPattern")?
        .parse()
        .map_err(|source| DocumentError::top(EntitySelectorError::IdPattern { source }))
}

#[cfg(feature = "linked-data")]
impl MapNames for EntitySelector {
    fn map_names<M: NameMapping>(self, mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        let selector_type = match self.selector_type {
            SelectorType::All => SelectorType::All,
            SelectorType::Selection(selection) => SelectorType::Selection(member(selection, "type", mapping)?),
        };
        Ok(Self { selector_type, ..self })
    }
}

#[cfg(feature = "linked-data")]
carries_context!(
    /// An entity selector carries no `@context`.
    impl for EntitySelector
);

#[cfg(test)]
mod tests;
