//! The entities a query names by type, identifier and identifier pattern (NGSI-LD clause 5.2.33): the `type`, `id`
//! and `idPattern` parameters of the GET binding (Tables 6.4.3.2-1, 6.8.3.2-1 and 6.18.3.2-1), or the `entities`
//! member of the `Query` body of the POST binding (clause 5.2.23), which holds any number of entity selectors.

use crate::{
    http::{QueryParameter, operation::ArgumentError, query_string::QueryString},
    model::{EntityId, OneOrMany},
    query::{EntitySelector, IdPattern, Locality, SelectorType, entity_selector::selector_object, type_selection::TypeSelection},
};
use jstrict::{Object, Value as Json};
use std::fmt::{Debug, Display};

/// What the `type` of a query holds, as far as a request needs it: printed, and whether it selects every type.
pub(crate) trait TypeParameter: Debug + Display {
    /// Whether the value is `*`, which implies local scope (Tables 6.4.3.2-1 and 5.2.33-1).
    fn selects_every_type(&self) -> bool;
}

/// A type selection of clause 4.17, the `type` of the operations whose tables do not allow `*`.
impl TypeParameter for TypeSelection {
    fn selects_every_type(&self) -> bool {
        false
    }
}

/// A type selection or `*`, the `type` of the operations whose tables allow it.
impl TypeParameter for SelectorType {
    fn selects_every_type(&self) -> bool {
        self.is_all()
    }
}

/// The entities a query names by type, identifier and identifier pattern: `type`, `id` and `idPattern`, and the
/// further entity selectors only a `Query` body can carry (clause 5.2.33).
#[derive(Debug, Clone, Copy)]
pub(crate) struct EntityTargets<'a> {
    /// `type`.
    entity_type: Option<&'a dyn TypeParameter>,
    /// `id`.
    ids: &'a [EntityId],
    /// `idPattern`.
    id_pattern: Option<&'a IdPattern>,
    /// Further entity selectors, each matched on its own.
    selectors: &'a [EntitySelector],
}

impl<'a> EntityTargets<'a> {
    /// The entities of `entity_type`, among `ids` when there are some, whose id `id_pattern` matches when there is
    /// one.
    pub(crate) const fn parameters<T: TypeParameter>(entity_type: Option<&'a T>, ids: &'a [EntityId], id_pattern: Option<&'a IdPattern>) -> EntityTargets<'a> {
        EntityTargets {
            entity_type: match entity_type {
                Some(entity_type) => Some(entity_type as &dyn TypeParameter),
                None => None,
            },
            ids,
            id_pattern,
            selectors: &[],
        }
    }

    /// These entities and the entities each of `selectors` matches.
    pub(crate) const fn with_selectors(self, selectors: &'a [EntitySelector]) -> Self {
        Self { selectors, ..self }
    }

    /// Whether the parameters name anything.
    const fn has_parameters(&self) -> bool {
        self.entity_type.is_some() || !self.ids.is_empty() || self.id_pattern.is_some()
    }

    /// Whether the entities are restricted to named types: by `type`, or by a selector, which always has one.
    pub(crate) const fn names_type(&self) -> bool {
        self.entity_type.is_some() || !self.selectors.is_empty()
    }

    /// Whether `*` asks for every type, which implies local scope (Tables 6.4.3.2-1 and 5.2.33-1).
    pub(crate) fn selects_every_type(&self) -> bool {
        self.entity_type.is_some_and(TypeParameter::selects_every_type) || self.selectors.iter().any(|selector| selector.selector_type().is_all())
    }

    /// Refuses `*` with `local=false`, which Tables 6.4.3.2-1 and 5.2.33-1 forbid.
    pub(crate) fn check_locality(&self, locality: Locality) -> Result<(), ArgumentError> {
        if locality == Locality::Distributed && self.selects_every_type() {
            return Err(ArgumentError::AllTypesNotLocal);
        }
        Ok(())
    }

    /// Refuses what a query string cannot carry: more than one selection, since `type`, `id` and `idPattern` hold
    /// one (Table 6.4.3.2-1). A single selector given alone is carried as the three parameters.
    pub(crate) const fn check_query_string(&self) -> Result<(), ArgumentError> {
        match (self.has_parameters(), self.selectors) {
            (_, []) | (false, [_]) => Ok(()),
            (true, [_, ..]) | (false, [_, _, ..]) => Err(ArgumentError::SeveralSelectorsInQueryString),
        }
    }

    /// Adds `type`, `id` and `idPattern`: the parameters, or the one selector given alone. More selectors are
    /// refused by [`check_query_string`](Self::check_query_string).
    pub(crate) fn push_query(&self, query: &mut QueryString) {
        if let (false, [selector]) = (self.has_parameters(), self.selectors) {
            query.push_type_selection(QueryParameter::Type, Some(selector.selector_type()));
            query.push_list(QueryParameter::Id, selector.ids().map_or(&[][..], OneOrMany::as_slice));
            query.push_some(QueryParameter::IdPattern, selector.id_pattern());
        } else {
            query.push_type_selection(QueryParameter::Type, self.entity_type);
            query.push_list(QueryParameter::Id, self.ids);
            query.push_some(QueryParameter::IdPattern, self.id_pattern);
        }
    }

    /// The `entities` member of a `Query` body: the selector the parameters make, then the further selectors.
    ///
    /// # Errors
    ///
    /// Returns [`ArgumentError::SelectorWithoutType`] when ids or an id pattern are given without a type, which an
    /// `EntitySelector` (clause 5.2.33) cannot express, and [`ArgumentError::IdsWithIdPatternInSelector`] when both
    /// are given, which the GET binding intersects (clause 5.7.2.4) but a selector would not, `id` taking precedence
    /// there (clause 5.2.33).
    pub(crate) fn entities(&self) -> Result<Option<Json>, ArgumentError> {
        let first = self.parameter_selector()?;
        let selectors: Vec<Json> = first
            .into_iter()
            .chain(self.selectors.iter().map(EntitySelector::to_object))
            .map(Json::Object)
            .collect();
        Ok((!selectors.is_empty()).then(|| Json::Array(selectors.into())))
    }

    /// The `EntitySelector` the parameters make, whose `type` is mandatory (Table 5.2.33-1).
    fn parameter_selector(&self) -> Result<Option<Object>, ArgumentError> {
        if !self.ids.is_empty() && self.id_pattern.is_some() {
            return Err(ArgumentError::IdsWithIdPatternInSelector);
        }
        let Some(entity_type) = self.entity_type else {
            return if self.ids.is_empty() && self.id_pattern.is_none() {
                Ok(None)
            } else {
                Err(ArgumentError::SelectorWithoutType)
            };
        };
        let ids = match self.ids {
            [] => None,
            [id] => Some(Json::from(id.as_str())),
            ids => Some(Json::Array(ids.iter().map(|id| Json::from(id.as_str())).collect())),
        };
        Ok(Some(selector_object(entity_type, ids, self.id_pattern)))
    }
}
