//! The names a context source registration holds: the entity types and attributes of its `information` and the
//! names of its Context Source Properties (NGSI-LD clauses 4.9, 5.2.9, 5.2.10 and 5.5.7).

use crate::{
    model::{
        AttributesError,
        DocumentError,
        name::mapping::{MapNames, NameMapping, member},
    },
    registration::{
        CSourceRegistration,
        ContextSourcePropertyName,
        EntityInfo,
        EntityInfoMember,
        EntityMap,
        RegistrationDirection,
        RegistrationFragment,
        RegistrationInfo,
        RegistrationInfoMember,
        member::RegistrationMandatoryMember,
    },
};
use indexmap::IndexMap;

/// Maps the names of the Context Source Properties of a registration or a fragment, in order, as attribute names
/// (a `csf` targets them as it targets attributes, clause 4.9). The values are kept as written: like the value of a
/// Property, a property value is data, whose keys clause 5.5.7 does not rewrite. An error is placed under the name
/// as written; a name translated onto one another property or a member of the registration data type already has
/// is refused with [`AttributesError::NameTaken`].
fn map_properties<V, M: NameMapping>(
    properties: IndexMap<ContextSourcePropertyName, V>,
    mapping: &M,
) -> Result<IndexMap<ContextSourcePropertyName, V>, DocumentError<M::Error>> {
    let mut mapped = IndexMap::with_capacity(properties.len());
    for (name, value) in properties {
        let within = |error: M::Error| DocumentError::top(error).within(name.as_ref());
        let taken = |target| within(M::Error::from(AttributesError::NameTaken { name: target }));
        let target = mapping.attribute(name.as_attribute()).map_err(within)?;
        let target = ContextSourcePropertyName::from_attribute(target).map_err(taken)?;
        if mapped.contains_key(&target) {
            return Err(taken(target.into_attribute()));
        }
        mapped.insert(target, value);
    }
    Ok(mapped)
}

impl<D: RegistrationDirection> MapNames for CSourceRegistration<D> {
    fn map_names<M: NameMapping>(self, mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        Ok(Self {
            information: member(self.information, RegistrationMandatoryMember::Information.into(), mapping)?,
            properties: map_properties(self.properties, mapping)?,
            ..self
        })
    }
}

impl MapNames for RegistrationFragment {
    fn map_names<M: NameMapping>(self, mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        Ok(Self {
            information: member(self.information, RegistrationMandatoryMember::Information.into(), mapping)?,
            properties: map_properties(self.properties, mapping)?,
            ..self
        })
    }
}

impl MapNames for RegistrationInfo {
    fn map_names<M: NameMapping>(self, mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        Ok(Self {
            entities: member(self.entities, RegistrationInfoMember::Entities.into(), mapping)?,
            property_names: member(self.property_names, RegistrationInfoMember::PropertyNames.into(), mapping)?,
            relationship_names: member(self.relationship_names, RegistrationInfoMember::RelationshipNames.into(), mapping)?,
        })
    }
}

impl MapNames for EntityInfo {
    fn map_names<M: NameMapping>(self, mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        Ok(Self {
            entity_type: member(self.entity_type, EntityInfoMember::Type.into(), mapping)?,
            ..self
        })
    }
}

/// An entity map holds entity identifiers and registration identifiers, never a name (clause 5.2.45).
impl MapNames for EntityMap {
    fn map_names<M: NameMapping>(self, _mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        Ok(self)
    }
}

carries_context!(impl<D: RegistrationDirection> for CSourceRegistration<D> => context);

carries_context!(impl for EntityMap => context);

carries_context!(
    /// A fragment carries no `@context`: its request delivers the endpoint's (clause 6.3.5).
    impl for RegistrationFragment
);
