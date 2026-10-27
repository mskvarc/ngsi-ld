//! Reading what a context source may provide: the `RegistrationInfo` elements of `information` (Table 5.2.10-1)
//! and their `EntityInfo` entities (Table 5.2.8-1).

use crate::{
    model::{
        BuildError,
        DocumentError,
        keyword_alias::unalias_members,
        reader::{entity_types, identifier, object, once, required, string, unexpected, within},
    },
    query::IdPattern,
    registration::{EntityInfo, EntityInfoMember, Problem, RegistrationInfo, RegistrationInfoMember},
    subscription::{
        ReadErrorKind,
        reader::{Emptiness, error, list, names},
    },
};
use jstrict::{Value as Json, object::Entry};
use std::error::Error;

/// Reads `information`: a non-empty array of `RegistrationInfo` objects, an error placed under the element's index.
pub(super) fn information(json: Json) -> Result<Vec<RegistrationInfo>, DocumentError<ReadErrorKind<Problem>>> {
    list(json, Emptiness::Refused)?
        .into_iter()
        .enumerate()
        .map(|(index, item)| registration_info(item).map_err(|error| error.at_index(index)))
        .collect()
}

/// Reads one `RegistrationInfo` object, which names entities, Properties or Relationships, or several of them.
fn registration_info(json: Json) -> Result<RegistrationInfo, DocumentError<ReadErrorKind<Problem>>> {
    let mut entities = None;
    let mut property_names = None;
    let mut relationship_names = None;
    for Entry { key, value } in object(json)? {
        let Ok(member) = key.as_str().parse::<RegistrationInfoMember>() else {
            return Err(unexpected(key.as_str()));
        };
        let key = member.into();
        match member {
            RegistrationInfoMember::Entities => once(&mut entities, key, within(key, entity_infos(value))?)?,
            RegistrationInfoMember::PropertyNames => once(&mut property_names, key, within(key, names(value, key, Emptiness::Refused))?)?,
            RegistrationInfoMember::RelationshipNames => once(&mut relationship_names, key, within(key, names(value, key, Emptiness::Refused))?)?,
        }
    }
    let information = RegistrationInfo {
        entities: entities.unwrap_or_default(),
        property_names: property_names.unwrap_or_default(),
        relationship_names: relationship_names.unwrap_or_default(),
    };
    if information.is_empty() {
        return Err(error(ReadErrorKind::Invalid {
            source: BuildError::single(Problem::EmptyRegistrationInfo),
        }));
    }
    Ok(information)
}

/// Reads `entities`: a non-empty array of `EntityInfo` objects, an error placed under the element's index.
fn entity_infos<P: Error>(json: Json) -> Result<Vec<EntityInfo>, DocumentError<ReadErrorKind<P>>> {
    list(json, Emptiness::Refused)?
        .into_iter()
        .enumerate()
        .map(|(index, item)| entity_info(item).map_err(|error| error.at_index(index)))
        .collect()
}

/// Reads one `EntityInfo` object: `type` is required, `id` and `idPattern` optional.
fn entity_info<P: Error>(json: Json) -> Result<EntityInfo, DocumentError<ReadErrorKind<P>>> {
    let mut entity_type = None;
    let mut id = None;
    let mut id_pattern = None;
    for Entry { key, value } in unalias_members(object(json)?)? {
        let Ok(member) = key.as_str().parse::<EntityInfoMember>() else {
            return Err(unexpected(key.as_str()));
        };
        let key = member.into();
        match member {
            EntityInfoMember::Type => once(&mut entity_type, key, within(key, entity_types(value))?)?,
            EntityInfoMember::Id => once(&mut id, key, within(key, identifier(value, key))?)?,
            EntityInfoMember::IdPattern => once(&mut id_pattern, key, within(key, pattern(value))?)?,
        }
    }
    Ok(EntityInfo {
        entity_type: required(entity_type, EntityInfoMember::Type.into())?,
        id,
        id_pattern,
    })
}

/// Reads `idPattern`.
fn pattern<P: Error>(json: Json) -> Result<IdPattern, DocumentError<ReadErrorKind<P>>> {
    string(json, EntityInfoMember::IdPattern.into())?
        .parse()
        .map_err(|source| error(ReadErrorKind::IdPattern { source }))
}
