//! The documents of entity type and attribute discovery (NGSI-LD clauses 4.5.10 to 4.5.15 and 5.2.24 to 5.2.28):
//! what a broker answers when asked which entity types and attributes it holds.
//!
//! Each representation is its own type. Clause 5.2.25 calls the element of the detailed entity type list
//! `EntityType`, and clause 5.2.28 calls three shapes with different members `Attribute`; the crate already names
//! the `type` member of an entity and an attribute so, and each shape allows its own members, so the types are named
//! after the representations: [`EntityTypeList`],
//! [`EntityTypeDetails`], [`EntityTypeInfo`] and its
//! [`EntityTypeAttribute`]s, [`AttributeList`],
//! [`AttributeDetails`] and [`AttributeInfo`]. Their `type`
//! member keeps the specification's token, [`DiscoveryType`].
//!
//! Only a broker writes these documents, so there is no request direction. They are read strictly: a member the
//! clauses do not define for the shape is refused.
//!
//! # Entity types
//!
//! [`EntityTypeList`], [`EntityTypeDetails`], [`EntityTypeInfo`], [`EntityTypeAttribute`].
//!
//! # Attributes
//!
//! [`AttributeList`], [`AttributeDetails`], [`AttributeInfo`].
//!
//! # Document types
//!
//! [`DiscoveryType`].

pub use crate::model::discovery::{
    attribute::{AttributeDetails, AttributeInfo, AttributeList, EntityTypeAttribute},
    entity_type::{EntityTypeDetails, EntityTypeInfo, EntityTypeList},
};

mod attribute;
mod entity_type;
mod members;
#[cfg(feature = "linked-data")]
mod names;

/// The `type` of a discovery document (clauses 4.5.10 to 4.5.15), spelled as the specification writes it.
///
/// The enum is exhaustive: clauses 4.5.10 to 4.5.15 define these five types only.
///
/// ```
/// use ngsi_ld::model::discovery::DiscoveryType;
///
/// assert_eq!(DiscoveryType::EntityTypeInfo.to_string(), "EntityTypeInfo");
/// assert_eq!("Attribute".parse(), Ok(DiscoveryType::Attribute));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::Display, strum::EnumString, strum::IntoStaticStr)]
pub enum DiscoveryType {
    /// `EntityTypeList`: the entity type list (clauses 4.5.10 and 5.2.24).
    EntityTypeList,
    /// `EntityType`: an element of the detailed entity type list (clauses 4.5.11 and 5.2.25).
    EntityType,
    /// `EntityTypeInfo`: the information about one entity type (clauses 4.5.12 and 5.2.26).
    EntityTypeInfo,
    /// `AttributeList`: the attribute list (clauses 4.5.13 and 5.2.27).
    AttributeList,
    /// `Attribute`: an element of the detailed attribute list, of `attributeDetails`, or the information about one
    /// attribute (clauses 4.5.14, 4.5.15 and 5.2.28).
    Attribute,
}

#[cfg(test)]
mod tests;
