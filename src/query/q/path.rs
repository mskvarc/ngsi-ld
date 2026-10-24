//! Attribute paths of the NGSI-LD Query Language (the `Attribute` production of NGSI-LD clause 4.9).

#[cfg(feature = "linked-data")]
use crate::model::{
    DocumentError,
    name::mapping::{MapNames, NameMapping, each_in_text},
};
use crate::{model::Term, query::grammar::write_separated};
use std::fmt;

/// Why a string is not a valid member of a member path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
#[non_exhaustive]
pub enum MemberError {
    /// The string is empty.
    #[error("a member name cannot be empty")]
    Empty,
    /// A character is neither a letter, a number, `_` nor `-`, or a `*` is not the whole member.
    #[error("`{found}` at byte {position} is not allowed in a member name")]
    InvalidCharacter {
        /// The offending character.
        found: char,
        /// Its byte offset in the member.
        position: usize,
    },
}

/// Whether `c` can appear in a member name other than `*`.
pub(crate) fn is_member_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '-'
}

/// Validates a member name: `*`, or a non-empty run of letters, numbers, `_` and `-`.
fn validate_member(value: &str) -> Result<(), MemberError> {
    if value.is_empty() {
        return Err(MemberError::Empty);
    }
    if value == "*" {
        return Ok(());
    }
    value
        .char_indices()
        .find(|&(_, found)| !is_member_char(found))
        .map_or(Ok(()), |(position, found)| Err(MemberError::InvalidCharacter { found, position }))
}

/// One member of the trailing member path of an attribute path, which addresses a member of a compound value
/// (NGSI-LD clause 4.9, example 9).
///
/// Clause 4.9 evaluates the member path as an ECMA 262 `MemberExpression`. Its ABNF allows only `AttrName`s
/// there, but its prose writes `color[*]` (any language of a `LanguageProperty`) and Annex C.5.13 writes
/// `marque[en-GB]` (one language), so a member is `*` or any non-empty run of letters, numbers, `_` and `-`.
///
/// ```
/// use ngsi_ld::query::q::MemberName;
///
/// let member: MemberName = "en-GB".parse()?;
/// assert_eq!(member.to_string(), "en-GB");
/// assert!("*".parse::<MemberName>().is_ok());
/// assert!("city.name".parse::<MemberName>().is_err());
/// # Ok::<(), anyhow::Error>(())
/// ```
#[nutype::nutype(
    validate(with = validate_member, error = MemberError),
    derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Display, AsRef, FromStr, TryFrom)
)]
pub struct MemberName(String);

/// An attribute path.
///
/// It is the `Attribute` production of NGSI-LD clause 4.9.
///
/// The enum is exhaustive: clause 4.9 addresses an attribute either directly or through a relationship.
///
/// ```
/// use ngsi_ld::query::q;
///
/// let query = q::attr("sensor").linked("humidity").eq(40).build()?;
/// assert_eq!(query.to_string(), "sensor{humidity}==40");
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display, derive_more::From)]
pub enum AttributePath {
    /// An attribute of the matching entity (the `ValuePath` production).
    #[display("{_0}")]
    Value(ValuePath),
    /// An attribute of the entity a relationship of the matching entity targets (the `LinkedEntityRelation`
    /// production, clause 4.5.23).
    #[display("{_0}")]
    Linked(LinkedPath),
}

/// A dotted path of attribute names, optionally followed by a member path.
///
/// The member path leads into the compound value of the last attribute: `sensor.rawdata[airquality.particulate]` (the
/// `ValuePath` production of NGSI-LD clause 4.9).
///
/// ```
/// use ngsi_ld::query::q::ValuePath;
///
/// let path = ValuePath::new("sensor".parse()?).with_sub_attribute("rawdata".parse()?);
/// assert_eq!(path.to_string(), "sensor.rawdata");
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ValuePath {
    /// The dotted attribute names; never empty.
    attributes: Vec<Term>,
    /// The member path; empty when there is none.
    members: Vec<MemberName>,
}

impl ValuePath {
    /// A path to the attribute `attribute`.
    #[must_use]
    pub fn new(attribute: Term) -> ValuePath {
        Self {
            attributes: vec![attribute],
            members: Vec::new(),
        }
    }

    /// Appends a sub-attribute (a Property or Relationship of the attribute addressed so far).
    #[must_use]
    pub fn with_sub_attribute(mut self, attribute: Term) -> Self {
        self.attributes.push(attribute);
        self
    }

    /// Appends a member to the member path into the compound value of the attribute.
    #[must_use]
    pub fn with_member(mut self, member: MemberName) -> Self {
        self.members.push(member);
        self
    }

    /// Returns the dotted attribute names; there is at least one.
    #[must_use]
    pub fn attributes(&self) -> &[Term] {
        &self.attributes
    }

    /// Returns the member path; empty when there is none.
    #[must_use]
    pub fn members(&self) -> &[MemberName] {
        &self.members
    }

    /// Returns the attribute name when the path is a single name without a member path, and the path otherwise.
    pub(crate) fn into_single_name(self) -> Result<Term, Self> {
        let Self { attributes, members } = self;
        if !members.is_empty() {
            return Err(Self { attributes, members });
        }
        <[Term; 1]>::try_from(attributes)
            .map(|[name]| name)
            .map_err(|attributes| Self { attributes, members })
    }
}

// The printer joins the names and writes the brackets only when there is a member path, which a derive cannot
// express.
impl fmt::Display for ValuePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_separated(f, &self.attributes, '.')?;
        if self.members.is_empty() {
            return Ok(());
        }
        f.write_str("[")?;
        write_separated(f, &self.members, '.')?;
        f.write_str("]")
    }
}

/// A relationship an attribute path follows into the entity it targets.
///
/// It carries an optional hint of the types of that entity. It is one `AttrName{` step of the `LinkedEntityRelation`
/// and `LinkedEntityPath` productions of NGSI-LD clause 4.9.
///
/// The hint lets a broker narrow its lookup of the targeted entity (clause 4.9, example 14).
///
/// ```
/// use ngsi_ld::query::q::Link;
///
/// let link = Link::new("sensor".parse()?).with_entity_types(["Device".parse()?]);
/// assert_eq!(link.relationship().as_ref(), "sensor");
/// assert_eq!(link.entity_types().len(), 1);
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Link {
    /// The relationship followed.
    relationship: Term,
    /// The hinted types of the targeted entity; empty when there is no hint.
    entity_types: Vec<Term>,
}

impl Link {
    /// Follows `relationship`, without a hint of the types of the entity it targets.
    #[must_use]
    pub const fn new(relationship: Term) -> Link {
        Self {
            relationship,
            entity_types: Vec::new(),
        }
    }

    /// Adds entity types to the hint of the types of the targeted entity.
    #[must_use]
    pub fn with_entity_types(mut self, entity_types: impl IntoIterator<Item = Term>) -> Self {
        self.entity_types.extend(entity_types);
        self
    }

    /// Returns the relationship.
    #[must_use]
    pub const fn relationship(&self) -> &Term {
        &self.relationship
    }

    /// Returns the hinted entity types; empty when there is no hint.
    #[must_use]
    pub fn entity_types(&self) -> &[Term] {
        &self.entity_types
    }
}

/// A path through one or more relationships into the entity the last one targets.
///
/// It is the `LinkedEntityRelation` production of NGSI-LD clause 4.9: `sensor{Device:humidity}`, or
/// `building{owner{name}}` through two.
///
/// The links are held as a list, outermost first, rather than as a path nested inside a path: clause 4.9 does not
/// bound how many relationships a path follows, and a list is copied, compared, printed and dropped without
/// recursing once per link.
///
/// ```
/// use ngsi_ld::query::q::{AttributePath, Link, LinkedPath, ValuePath};
///
/// let humidity = AttributePath::from(ValuePath::new("humidity".parse()?));
/// let path = LinkedPath::new(
///     Link::new("sensor".parse()?).with_entity_types(["Device".parse()?]),
///     humidity,
/// );
/// assert_eq!(path.to_string(), "sensor{Device:humidity}");
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LinkedPath {
    /// The relationships followed, outermost first; never empty.
    links: Vec<Link>,
    /// The path inside the entity the last relationship targets.
    target: ValuePath,
}

impl LinkedPath {
    /// A path through `link` to `target` in the entity it targets.
    #[must_use]
    pub fn new(link: Link, target: AttributePath) -> LinkedPath {
        match target {
            AttributePath::Value(target) => Self { links: vec![link], target },
            AttributePath::Linked(Self { links, target }) => Self {
                links: [link].into_iter().chain(links).collect(),
                target,
            },
        }
    }

    /// Returns the relationships followed, outermost first; there is at least one.
    #[must_use]
    pub fn links(&self) -> &[Link] {
        &self.links
    }

    /// Returns the path inside the entity the last relationship targets.
    #[must_use]
    pub const fn target(&self) -> &ValuePath {
        &self.target
    }
}

impl AttributePath {
    /// The path through `links`, outermost first, to `target`; a value path when there is no link.
    pub(crate) fn through(links: Vec<Link>, target: ValuePath) -> AttributePath {
        if links.is_empty() {
            Self::Value(target)
        } else {
            Self::Linked(LinkedPath { links, target })
        }
    }
}

// The printer opens a brace per link, writes each type hint only when there is one, and closes the braces after the
// target, which a derive cannot express.
impl fmt::Display for LinkedPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.links.iter().try_for_each(|link| {
            write!(f, "{}{{", link.relationship)?;
            if link.entity_types.is_empty() {
                return Ok(());
            }
            write_separated(f, &link.entity_types, ',')?;
            f.write_str(":")
        })?;
        self.target.fmt(f)?;
        self.links.iter().try_for_each(|_| f.write_str("}"))
    }
}

#[cfg(feature = "linked-data")]
impl MapNames for AttributePath {
    fn map_names<M: NameMapping>(self, mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        match self {
            Self::Value(path) => path.map_names(mapping).map(Self::Value),
            Self::Linked(path) => path.map_names(mapping).map(Self::Linked),
        }
    }
}

/// The member path addresses keys inside a compound value, which JSON-LD does not expand, so it is kept.
#[cfg(feature = "linked-data")]
impl MapNames for ValuePath {
    fn map_names<M: NameMapping>(self, mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        Ok(Self {
            attributes: each_in_text(self.attributes, mapping)?,
            ..self
        })
    }
}

/// The hinted entity types are bare terms of the language, read like the attribute names (clause 4.9).
#[cfg(feature = "linked-data")]
impl MapNames for Link {
    fn map_names<M: NameMapping>(self, mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        Ok(Self {
            relationship: self.relationship.map_names(mapping)?,
            entity_types: each_in_text(self.entity_types, mapping)?,
        })
    }
}

#[cfg(feature = "linked-data")]
impl MapNames for LinkedPath {
    fn map_names<M: NameMapping>(self, mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        Ok(Self {
            links: each_in_text(self.links, mapping)?,
            target: self.target.map_names(mapping)?,
        })
    }
}
