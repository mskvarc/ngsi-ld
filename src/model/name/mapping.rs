//! Rewriting every name a document holds: the walk the `linked_data` module drives to move a document from one
//! `@context` to another (NGSI-LD clause 5.5.7).
//!
//! A [`NameMapping`] says what each name becomes; [`MapNames`] is implemented by every document and every part of
//! one that holds names, beside the type, and rebuilds the value with each name mapped. The walk knows nothing of
//! JSON-LD: the mapping does. A failure is reported at the member that holds the name, as the readers report theirs.
//!
//! The traits are `pub` so the sealed traits of the `linked_data` module can build on them; the module is private
//! to the crate, so no caller can name them.

use crate::{
    model::{
        AttributeName,
        AttributesError,
        Context,
        DocumentError,
        EntityTypeName,
        GeoPropertyOutputMembers,
        JsonPropertyOutputMembers,
        LanguagePropertyOutputMembers,
        ListPropertyOutputMembers,
        PropertyOutputMembers,
        Term,
        VocabTerm,
        evolution::AggregatedAttribute,
    },
    query::type_selection::SelectableTypeError,
    seal::Seal,
};
use indexmap::IndexMap;

/// What each kind of name becomes.
pub trait NameMapping {
    /// Why a name cannot be mapped. Two places refuse a mapped name for what it is rather than for the mapping,
    /// so the error holds their errors: an attribute map that two names collapse into one refuses the second
    /// ([`AttributesError`]), and a type selection refuses an entity type that holds one of its operators
    /// ([`SelectableTypeError`]).
    type Error: From<AttributesError> + From<SelectableTypeError>;

    /// Maps an entity type: the `type` of an entity, the `objectType` of a relationship, an entity type a query
    /// selects.
    ///
    /// # Errors
    ///
    /// Returns the mapping's error when the name has no counterpart.
    fn entity_type(&self, name: &EntityTypeName) -> Result<EntityTypeName, Self::Error>;

    /// Maps an attribute name: a member of an entity, or an attribute a document names.
    ///
    /// # Errors
    ///
    /// Returns the mapping's error when the name has no counterpart.
    fn attribute(&self, name: &AttributeName) -> Result<AttributeName, Self::Error>;

    /// Maps a bare term of the query languages (clause 4.9).
    ///
    /// # Errors
    ///
    /// Returns the mapping's error when the name has no counterpart a bare term can hold.
    fn term(&self, name: &Term) -> Result<Term, Self::Error>;

    /// Maps a vocabulary term: the `vocab` of a `VocabProperty`, a `valueType`.
    ///
    /// # Errors
    ///
    /// Returns the mapping's error when the term has no counterpart.
    fn vocab(&self, name: &VocabTerm) -> Result<VocabTerm, Self::Error>;
}

/// A value whose names a [`NameMapping`] rewrites.
pub trait MapNames: Sized {
    /// Rebuilds the value with every name it holds mapped.
    ///
    /// # Errors
    ///
    /// Returns the mapping's error at the member that holds the name that failed.
    fn map_names<M: NameMapping>(self, mapping: &M) -> Result<Self, DocumentError<M::Error>>;
}

/// A document that carries an `@context` member, which re-contextualisation replaces.
pub trait CarriesContext {
    /// Replaces the `@context`; a document with no `@context` member ignores it.
    fn set_context(&mut self, seal: Seal, context: Option<Context>);
}

/// Implements [`CarriesContext`] for a document type, so every document replaces its `@context` the same way.
///
/// `carries_context!(impl<generics> for Document<..> => field)` expands to an impl whose `set_context` stores the
/// context in `self.field`; `carries_context!(impl<generics> for Document<..>)` expands to one whose `set_context`
/// ignores it, for a document with no `@context` member. The generics are optional; attributes written first, such
/// as a doc comment saying why a document carries no `@context` or a `cfg`, are put on the impl.
macro_rules! carries_context {
    ($(#[$meta:meta])* impl $(<$($param:ident $(: $bound:path)?),+>)? for $document:ty => $field:ident) => {
        $(#[$meta])*
        impl$(<$($param $(: $bound)?),+>)? $crate::model::name::mapping::CarriesContext for $document {
            fn set_context(&mut self, _: $crate::seal::Seal, context: Option<$crate::model::Context>) {
                self.$field = context;
            }
        }
    };
    ($(#[$meta:meta])* impl $(<$($param:ident $(: $bound:path)?),+>)? for $document:ty) => {
        $(#[$meta])*
        impl$(<$($param $(: $bound)?),+>)? $crate::model::name::mapping::CarriesContext for $document {
            fn set_context(&mut self, _: $crate::seal::Seal, _context: Option<$crate::model::Context>) {}
        }
    };
}

/// Implements [`MapNames`] for an entity-shaped document: its `type`, mapped as its completeness holds the member
/// (clause 4.21), and its attributes are mapped, and every other member is kept (clause 5.5.7).
///
/// `entity_names!(impl<generics> for Document<..> => C)` expands to an impl that maps `self.entity_type` through
/// `C::map_member` and `self.attributes` through their own [`MapNames`], `C` being the generic parameter bound by
/// `MapCompleteness`. Attributes written first, such as a doc comment, are put on the impl.
macro_rules! entity_names {
    ($(#[$meta:meta])* impl<$($param:ident $(: $bound:ident $(+ $more:ident)*)?),+> for $document:ty => $completeness:ident) => {
        $(#[$meta])*
        impl<$($param $(: $bound $(+ $more)*)?),+> $crate::model::name::mapping::MapNames for $document {
            fn map_names<M: $crate::model::name::mapping::NameMapping>(
                self,
                mapping: &M,
            ) -> Result<Self, $crate::model::DocumentError<M::Error>> {
                Ok(Self {
                    entity_type: $completeness::map_member(self.entity_type, "type", mapping)?,
                    attributes: $crate::model::name::mapping::MapNames::map_names(self.attributes, mapping)?,
                    ..self
                })
            }
        }
    };
}

/// Maps `value`, placing an error under the member `member`.
pub(crate) fn member<T: MapNames, M: NameMapping>(value: T, member: &str, mapping: &M) -> Result<T, DocumentError<M::Error>> {
    value.map_names(mapping).map_err(|error| error.within(member))
}

/// Maps every item of a list that a string member writes, such as the names of a query: the error stays at the
/// member, which has no elements to index.
pub(crate) fn each_in_text<T: MapNames, M: NameMapping>(items: Vec<T>, mapping: &M) -> Result<Vec<T>, DocumentError<M::Error>> {
    items.into_iter().map(|item| item.map_names(mapping)).collect()
}

/// Maps one name with `map`, at the top of the member being mapped.
fn leaf<T, E>(name: &T, map: impl FnOnce(&T) -> Result<T, E>) -> Result<T, DocumentError<E>> {
    map(name).map_err(DocumentError::top)
}

impl MapNames for EntityTypeName {
    fn map_names<M: NameMapping>(self, mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        leaf(&self, |name| mapping.entity_type(name))
    }
}

impl MapNames for AttributeName {
    fn map_names<M: NameMapping>(self, mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        leaf(&self, |name| mapping.attribute(name))
    }
}

impl MapNames for Term {
    fn map_names<M: NameMapping>(self, mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        leaf(&self, |name| mapping.term(name))
    }
}

impl MapNames for VocabTerm {
    fn map_names<M: NameMapping>(self, mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        leaf(&self, |name| mapping.vocab(name))
    }
}

/// The elements of an array, each placed under its index.
impl<T: MapNames> MapNames for Vec<T> {
    fn map_names<M: NameMapping>(self, mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        self.into_iter()
            .enumerate()
            .map(|(index, item)| item.map_names(mapping).map_err(|error| error.at_index(index)))
            .collect()
    }
}

/// Values by attribute name, in document order: each name and value is mapped, an error is placed under the name as
/// written, and two names mapped onto one are refused with [`AttributesError::NameTaken`].
impl<V: MapNames> MapNames for IndexMap<AttributeName, V> {
    fn map_names<M: NameMapping>(self, mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        map_attribute_names(self, mapping, |value| value.map_names(mapping), |_, _| Ok(()))
    }
}

/// Maps a map keyed by attribute name, in document order: each name through `mapping`, each value through
/// `map_value`, and `check` says whether the mapped value may stand under its new name. An error is placed under
/// the name as written, and two names mapped onto one are refused with [`AttributesError::NameTaken`].
pub(crate) fn map_attribute_names<V, W, M: NameMapping>(
    map: IndexMap<AttributeName, V>,
    mapping: &M,
    map_value: impl Fn(V) -> Result<W, DocumentError<M::Error>>,
    check: impl Fn(&AttributeName, &W) -> Result<(), AttributesError>,
) -> Result<IndexMap<AttributeName, W>, DocumentError<M::Error>> {
    let mut mapped = IndexMap::with_capacity(map.len());
    for (name, value) in map {
        let within = |error: DocumentError<M::Error>| error.within(name.as_ref());
        let refused = |source: AttributesError| within(DocumentError::top(M::Error::from(source)));
        let target = mapping.attribute(&name).map_err(|error| within(DocumentError::top(error)))?;
        let value = map_value(value).map_err(within)?;
        if mapped.contains_key(&target) {
            return Err(refused(AttributesError::NameTaken { name: target }));
        }
        check(&target, &value).map_err(refused)?;
        mapped.insert(target, value);
    }
    Ok(mapped)
}

impl<T: MapNames> MapNames for Option<T> {
    fn map_names<M: NameMapping>(self, mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        self.map(|value| value.map_names(mapping)).transpose()
    }
}

impl<T: MapNames> MapNames for Box<T> {
    fn map_names<M: NameMapping>(self, mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        (*self).map_names(mapping).map(Box::new)
    }
}

/// Implements [`MapNames`] for values that hold no names, which map to themselves.
macro_rules! holds_no_names {
    ($($holder:ty),+ $(,)?) => {
        $(
            impl MapNames for $holder {
                fn map_names<M: NameMapping>(self, _mapping: &M) -> Result<Self, DocumentError<M::Error>> {
                    Ok(self)
                }
            }
        )+
    };
}

// The output-only members of a request (`()`), those of a response that hold values, never names (a previous
// value, list, language map or JSON is data, and clause 5.5.7 expands no key inside a `json`), and the aggregated
// values of an attribute, which hold a kind and numbers.
holds_no_names!(
    (),
    PropertyOutputMembers,
    GeoPropertyOutputMembers,
    LanguagePropertyOutputMembers,
    ListPropertyOutputMembers,
    JsonPropertyOutputMembers,
    AggregatedAttribute,
);
