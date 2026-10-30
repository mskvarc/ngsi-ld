//! Reading a registration fragment (NGSI-LD clauses 5.4 and 5.5.8): the members it sets, and the ones it deletes
//! with the NGSI-LD Null.

use crate::{
    model::{
        BuildError,
        DocumentError,
        MemberChange,
        Value,
        keyword::CONTEXT,
        keyword_alias::unalias_members,
        member_change::read_change,
        reader::{
            context,
            context_source_alias,
            date_time,
            duplicate,
            duration,
            geometry,
            identifier,
            object,
            once,
            scopes,
            token,
            unexpected,
            value,
            within,
            written_url,
        },
    },
    registration::{
        ContextSourcePropertyName,
        Problem,
        RegistrationField,
        RegistrationFragment,
        RegistrationId,
        RegistrationMember,
        RegistrationTypeMember,
        member::RegistrationMandatoryMember,
        reader::{context_source_info, information::information, management, operations, tenant, text, time_interval},
        rules::{FragmentMembers, fragment_member_problems},
    },
    subscription::{
        DocumentType,
        ReadErrorKind,
        reader::{dataset_id_list, error, expect_type},
    },
};
use indexmap::IndexMap;
use jstrict::{Value as Json, object::Entry};

/// Reads a registration fragment. `type` may be left out where the operation names the target (clause 5.4); an `id`
/// is checked and not kept, as is the `@context`, which belongs to the binding (clause 6.3.5). `information` and
/// `endpoint` are replaced, never removed, so the Null is not one of their values.
pub(crate) fn read_registration_fragment(json: Json) -> Result<RegistrationFragment, DocumentError<ReadErrorKind<Problem>>> {
    let mut fragment = RegistrationFragment {
        type_member: RegistrationTypeMember::Omitted,
        ..RegistrationFragment::default()
    };
    let mut id: Option<RegistrationId> = None;
    let mut fragment_context = None;
    for Entry { key, value } in unalias_members(object(json)?)? {
        let key = key.as_str();
        if let Ok(member) = key.parse::<RegistrationMember>() {
            read_optional(&mut fragment, member, value)?;
        } else if let Ok(mandatory) = key.parse::<RegistrationMandatoryMember>() {
            let key = mandatory.into();
            match mandatory {
                RegistrationMandatoryMember::Id => once(&mut id, key, within(key, identifier(value, key))?)?,
                RegistrationMandatoryMember::Type => {
                    within(key, expect_type(value, DocumentType::ContextSourceRegistration))?;
                    if fragment.type_member == RegistrationTypeMember::Written {
                        return Err(duplicate(key));
                    }
                    fragment.type_member = RegistrationTypeMember::Written;
                }
                RegistrationMandatoryMember::Information => once(
                    &mut fragment.information,
                    key,
                    within(key, replacement(value, RegistrationField::Information, information))?,
                )?,
                RegistrationMandatoryMember::Endpoint => once(
                    &mut fragment.endpoint,
                    key,
                    within(key, replacement(value, RegistrationField::Endpoint, |value| written_url(value, key)))?,
                )?,
            }
        } else if key == CONTEXT {
            once(&mut fragment_context, key, within(key, context(value))?)?;
        } else {
            property_change(&mut fragment.properties, key, value)?;
        }
    }
    let problems = fragment_member_problems(&FragmentMembers {
        information: fragment.information.as_deref(),
        mode: set(fragment.mode.as_ref()).copied(),
        operations: set(fragment.operations.as_ref()).map(Vec::as_slice),
        texts: [
            (RegistrationMember::RegistrationName, set(fragment.name.as_ref()).map(String::as_str)),
            (RegistrationMember::Description, set(fragment.description.as_ref()).map(String::as_str)),
        ],
    });
    BuildError::finish(problems, fragment).map_err(|source| error(ReadErrorKind::Invalid { source }))
}

/// Reads the change to an optional member into `fragment`.
fn read_optional(fragment: &mut RegistrationFragment, member: RegistrationMember, value: Json) -> Result<(), DocumentError<ReadErrorKind<Problem>>> {
    let key = member.into();
    match member {
        RegistrationMember::RegistrationName => once(&mut fragment.name, key, within(key, read_change(value, |value| text(value, key)))?),
        RegistrationMember::Description => once(&mut fragment.description, key, within(key, read_change(value, |value| text(value, key)))?),
        RegistrationMember::ContextSourceInfo => once(
            &mut fragment.context_source_info,
            key,
            within(key, read_change(value, |value| context_source_info(value, key)))?,
        ),
        RegistrationMember::ContextSourceAlias => once(&mut fragment.context_source_alias, key, within(key, read_change(value, context_source_alias))?),
        RegistrationMember::Mode => once(&mut fragment.mode, key, within(key, read_change(value, |value| token(value, key)))?),
        RegistrationMember::Operations => once(&mut fragment.operations, key, within(key, read_change(value, |value| operations(value, key)))?),
        RegistrationMember::Management => once(&mut fragment.management, key, within(key, read_change(value, management))?),
        RegistrationMember::Tenant => once(&mut fragment.tenant, key, within(key, read_change(value, |value| tenant(value, key)))?),
        RegistrationMember::Scope => once(&mut fragment.scope, key, within(key, read_change(value, scopes))?),
        RegistrationMember::DatasetId => once(
            &mut fragment.dataset_ids,
            key,
            within(key, read_change(value, |value| dataset_id_list(value, key)))?,
        ),
        RegistrationMember::Location => once(&mut fragment.location, key, within(key, read_change(value, |value| geometry(&value)))?),
        RegistrationMember::ObservationSpace => once(&mut fragment.observation_space, key, within(key, read_change(value, |value| geometry(&value)))?),
        RegistrationMember::OperationSpace => once(&mut fragment.operation_space, key, within(key, read_change(value, |value| geometry(&value)))?),
        RegistrationMember::ObservationInterval => once(&mut fragment.observation_interval, key, within(key, read_change(value, time_interval))?),
        RegistrationMember::ManagementInterval => once(&mut fragment.management_interval, key, within(key, read_change(value, time_interval))?),
        RegistrationMember::ExpiresAt => once(&mut fragment.expires_at, key, within(key, read_change(value, |value| date_time(value, key)))?),
        RegistrationMember::RefreshRate => once(&mut fragment.refresh_rate, key, within(key, read_change(value, |value| duration(value, key)))?),
    }
}

/// Reads the replacement of a member a registration needs: the NGSI-LD Null would delete it, which a fragment cannot do.
fn replacement<T>(
    json: Json,
    member: RegistrationField,
    read: impl FnOnce(Json) -> Result<T, DocumentError<ReadErrorKind<Problem>>>,
) -> Result<T, DocumentError<ReadErrorKind<Problem>>> {
    match read_change(json, read)? {
        MemberChange::Set(value) => Ok(value),
        MemberChange::Delete => Err(error(ReadErrorKind::Invalid {
            source: BuildError::single(Problem::MandatoryMemberDeleted { member }),
        })),
    }
}

/// The value a fragment sets, `None` when it leaves the member as it is or deletes it.
const fn set<T>(change: Option<&MemberChange<T>>) -> Option<&T> {
    match change {
        Some(MemberChange::Set(value)) => Some(value),
        Some(MemberChange::Delete) | None => None,
    }
}

/// Reads the member `key`, which no table defines, as a change to a Context Source Property (clause 4.9); a name that
/// cannot name one is a member the fragment does not define.
fn property_change(
    properties: &mut IndexMap<ContextSourcePropertyName, MemberChange<Value>>,
    key: &str,
    json: Json,
) -> Result<(), DocumentError<ReadErrorKind<Problem>>> {
    let name = key.parse::<ContextSourcePropertyName>().map_err(|_| unexpected(key))?;
    let change = within(key, read_change(json, value))?;
    if properties.insert(name, change).is_some() {
        return Err(duplicate(key));
    }
    Ok(())
}
