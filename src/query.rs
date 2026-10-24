//! The NGSI-LD query languages a client writes (NGSI-LD clauses 4.9, 4.10, 4.11, 4.15, 4.17, 4.19, 4.21 and 4.23).
//!
//! Each language is a typed syntax tree with a builder, a printer (`Display`) and a parser (`FromStr`):
//!
//! - [`q`]: the NGSI-LD Query Language of clause 4.9, written in the `q` and `csf` parameters;
//! - [`type_selection`]: the Entity Type Selection Language of clause 4.17, written in the `type` parameter;
//! - [`projection`]: the Attribute Projection Language of clause 4.21, written in the `pick` and `omit`
//!   parameters;
//! - [`language_filter`]: the Language Filter of clause 4.15, written in the `lang` parameter, its weights the
//!   [`Quality`] values of IETF RFC 9110 that the content negotiation headers share;
//! - [`ordering`]: the Entity Ordering Language of clause 4.23, written in the `orderBy` parameter, with the
//!   `collation`, `orderFrom` and `orderGeometry` parameters that complete it;
//! - [`geoquery`]: the Geoquery Language of clause 4.10, written in the `georel`, `geometry`, `coordinates` and
//!   `geoproperty` parameters, or as the `GeoQuery` object of clause 5.2.13;
//! - [`scope_query`]: the Scope Query Language of clause 4.19, written in the `scopeQ` parameter;
//! - [`temporal_query`]: the Temporal Query Language of clause 4.11, written in the `timerel`, `timeAt`,
//!   `endTimeAt` and `timeproperty` parameters with `lastN`, `aggrMethods` and `aggrPeriodDuration`, or as the
//!   `TemporalQuery` object of clause 5.2.21.
//!
//! Beside the languages, the module owns what a query and a subscription share: the [`EntitySelector`] of clause
//! 5.2.33 and the [`IdPattern`] its entity ids are matched against, the linked entity retrieval [`Join`] of clause
//! 4.5.23, and the [`Locality`] and [`EntitySplitting`] choices of clauses 5.5.13 and 6.3.18.
//!
//! The printer produces the canonical string and the parser accepts every string the grammar allows, so
//! `parse(print(x)) == x` holds for every tree. The printed string is the raw query text: percent-encoding it
//! into a URI (clause 4.9, IETF RFC 3986) is the job of the HTTP binding, not of the language.
//!
//! # Entity selection
//!
//! [`EntitySelector`], [`EntitySelectorDraft`], [`EntitySelectorError`], [`SelectorType`], [`IdPattern`].
//!
//! # Linked entities and distribution
//!
//! [`Join`], [`Locality`], [`EntitySplitting`], [`EntityMapReturn`].
//!
//! # Weights
//!
//! [`Quality`], [`QualityError`], [`Weighted`].
//!
//! # Errors
//!
//! [`Problem`], [`ParseError`], [`Expected`], [`RegularExpressionError`].
//!
//! # Nested namespaces
//!
//! [`q`], [`type_selection`], [`projection`], [`language_filter`], [`ordering`], [`geoquery`], [`scope_query`],
//! [`temporal_query`].
//!
//! # Builders
//!
//! The builders of these types are in [`builders`].

pub use crate::query::{
    build_error::Problem,
    distribution::{EntityMapReturn, EntitySplitting, Locality},
    entity_selector::{EntitySelector, EntitySelectorDraft, EntitySelectorError, SelectorType},
    id_pattern::IdPattern,
    linked_entities::Join,
    parse_error::{Expected, ParseError},
    quality::{Quality, QualityError, Weighted},
    regular_expression::RegularExpressionError,
};

mod build_error;
pub mod builders;
#[cfg(test)]
mod depth_check;
mod distribution;
pub(crate) mod entity_selector;
pub mod geoquery;
mod grammar;
mod id_pattern;
pub mod language_filter;
mod linked_entities;
pub mod ordering;
mod parse_error;
pub mod projection;
pub mod q;
mod quality;
mod regular_expression;
pub mod scope_query;
pub mod temporal_query;
pub mod type_selection;
