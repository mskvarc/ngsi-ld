//! Reading registration documents: a registration (Table 5.2.9-1), with the members only a broker writes when the
//! direction has them (Table 5.2.9-2), its management information (Table 5.2.34-1) and time intervals (Table
//! 5.2.11-1).

use crate::{
    model::{
        BuildError,
        DocumentError,
        Tenant,
        Value,
        keyword::CONTEXT,
        reader::{
            boolean,
            context,
            context_source_alias,
            date_time,
            duplicate,
            duration,
            empty,
            geometry,
            identifier,
            object,
            once,
            parse_json,
            required,
            scopes,
            string,
            token,
            unexpected,
            url,
            value,
            within,
        },
    },
    query::Locality,
    registration::{
        CSourceRegistration,
        ContextSourceInfo,
        ContextSourcePropertyName,
        ManagementMember,
        Problem,
        RegistrationDirection,
        RegistrationManagementInfo,
        RegistrationMember,
        SupportedOperation,
        TimeInterval,
        member::RegistrationMandatoryMember,
        reader::information::information,
        rules,
    },
    seal::Seal,
    subscription::{
        DocumentType,
        ReadErrorKind,
        direction::sealed::{IdMember, OutputBlock},
        reader::{Emptiness, dataset_id_list, error, expect_type, list, notification_params::key_value_pairs, positive},
    },
};
use indexmap::IndexMap;
use jstrict::{Object, Value as Json, object::Entry};
use std::error::Error;

mod information;

/// Reads a text the specification forbids to be empty.
fn text<P: Error>(json: Json, member: &'static str) -> Result<String, DocumentError<ReadErrorKind<P>>> {
    let text = string(json, member)?;
    if text.is_empty() { Err(empty()) } else { Ok(text) }
}

/// Reads `contextSourceInfo`: an array of key-value pairs, each value one its key allows (clause 4.3.6.6).
fn context_source_info(json: Json, _member: &'static str) -> Result<Vec<ContextSourceInfo>, DocumentError<ReadErrorKind<Problem>>> {
    key_value_pairs(json)?
        .into_iter()
        .enumerate()
        .map(|(index, pair)| {
            ContextSourceInfo::parse(pair.key(), pair.value()).map_err(|source| {
                error(ReadErrorKind::Invalid {
                    source: BuildError::single(Problem::ContextSourceInfo { index, source }),
                })
            })
        })
        .collect()
}

/// Reads the member `key`, which no table defines, as a Context Source Property (clause 4.9) into `properties`;
/// a name that cannot name one is a member the document does not define.
fn property<P: Error>(properties: &mut IndexMap<ContextSourcePropertyName, Value>, key: &str, json: Json) -> Result<(), DocumentError<ReadErrorKind<P>>> {
    let name = key.parse::<ContextSourcePropertyName>().map_err(|_| unexpected(key))?;
    let value = within(key, value(json))?;
    if properties.insert(name, value).is_some() {
        return Err(duplicate(key));
    }
    Ok(())
}

/// Reads `tenant`, written as the member `member`.
fn tenant<P: Error>(json: Json, member: &'static str) -> Result<Tenant, DocumentError<ReadErrorKind<P>>> {
    string(json, member)?.parse().map_err(|source| error(ReadErrorKind::Tenant { source }))
}

/// Reads `operations`, written as the member `member`: an array of operation and group names (Table 4.20-1,
/// Table 4.20-2).
fn operations<P: Error>(json: Json, member: &'static str) -> Result<Vec<SupportedOperation>, DocumentError<ReadErrorKind<P>>> {
    list(json, Emptiness::Allowed)?.into_iter().map(|item| token(item, member)).collect()
}

/// Reads a `TimeInterval` object (Table 5.2.11-1).
fn time_interval<P: Error>(json: Json) -> Result<TimeInterval, DocumentError<ReadErrorKind<P>>> {
    let mut start_at = None;
    let mut end_at = None;
    for Entry { key, value } in object(json)? {
        let key = key.as_str();
        match key {
            "startAt" => once(&mut start_at, key, within(key, date_time(value, "startAt"))?)?,
            "endAt" => once(&mut end_at, key, within(key, date_time(value, "endAt"))?)?,
            other => return Err(unexpected(other)),
        }
    }
    let start = required(start_at, "startAt")?;
    Ok(end_at.map_or(TimeInterval::starting_at(start), |end| TimeInterval::between(start, end)))
}

/// Reads a `RegistrationManagementInfo` object (Table 5.2.34-1).
fn management<P: Error>(json: Json) -> Result<RegistrationManagementInfo, DocumentError<ReadErrorKind<P>>> {
    let mut locality = None;
    let mut cache_duration = None;
    let mut timeout = None;
    let mut cooldown = None;
    for Entry { key, value } in object(json)? {
        let Ok(member) = key.as_str().parse::<ManagementMember>() else {
            return Err(unexpected(key.as_str()));
        };
        let key = member.into();
        match member {
            ManagementMember::LocalOnly => once(&mut locality, key, within(key, boolean(&value, key)).map(Locality::from_flag)?)?,
            ManagementMember::CacheDuration => once(&mut cache_duration, key, within(key, duration(value, key))?)?,
            ManagementMember::Timeout => once(&mut timeout, key, within(key, positive(value, key))?)?,
            ManagementMember::Cooldown => once(&mut cooldown, key, within(key, positive(value, key))?)?,
        }
    }
    Ok(RegistrationManagementInfo {
        locality: locality.unwrap_or_default(),
        cache_duration,
        timeout,
        cooldown,
    })
}

/// Reads the text of a registration in direction `D`.
pub(crate) fn read_registration<D: RegistrationDirection>(text: &str) -> Result<CSourceRegistration<D>, DocumentError<ReadErrorKind<Problem>>> {
    read_registration_json(parse_json(text)?)
}

/// Reads a registration in direction `D` from JSON already parsed, such as an element of an array, and checks the
/// rules its members relate by.
pub(crate) fn read_registration_json<D: RegistrationDirection>(json: Json) -> Result<CSourceRegistration<D>, DocumentError<ReadErrorKind<Problem>>> {
    let registration = read_registration_object::<D>(object(json)?)?;
    BuildError::finish(rules::registration_problems(&registration), ()).map_err(|source| error(ReadErrorKind::Invalid { source }))?;
    Ok(registration)
}

/// Reads the members of a registration object.
fn read_registration_object<D: RegistrationDirection>(object: Object) -> Result<CSourceRegistration<D>, DocumentError<ReadErrorKind<Problem>>> {
    let mut id = None;
    let mut typed = None;
    let mut name = None;
    let mut description = None;
    let mut information_list = None;
    let mut endpoint = None;
    let mut source_info = None;
    let mut alias = None;
    let mut mode = None;
    let mut operation_list = None;
    let mut management_info = None;
    let mut tenant_name = None;
    let mut scope = None;
    let mut dataset_ids = None;
    let mut location = None;
    let mut observation_space = None;
    let mut operation_space = None;
    let mut observation_interval = None;
    let mut management_interval = None;
    let mut expires_at = None;
    let mut refresh_rate = None;
    let mut registration_context = None;
    let mut properties = IndexMap::new();
    let mut output = D::RegistrationOutput::default();
    for Entry { key, value } in object {
        let key = key.as_str();
        let Ok(member) = key.parse::<RegistrationMember>() else {
            if let Ok(mandatory) = key.parse::<RegistrationMandatoryMember>() {
                let key = mandatory.into();
                match mandatory {
                    RegistrationMandatoryMember::Id => once(&mut id, key, within(key, identifier(value, key))?)?,
                    RegistrationMandatoryMember::Type => once(&mut typed, key, within(key, expect_type(value, DocumentType::ContextSourceRegistration))?)?,
                    RegistrationMandatoryMember::Information => once(&mut information_list, key, within(key, information(value))?)?,
                    RegistrationMandatoryMember::Endpoint => once(&mut endpoint, key, within(key, url(value, key))?)?,
                }
            } else if key == CONTEXT {
                once(&mut registration_context, key, within(key, context(value))?)?;
            } else if let Some(value) = output.read(Seal::new(), key, value)? {
                property(&mut properties, key, value)?;
            }
            continue;
        };
        let key = member.into();
        match member {
            RegistrationMember::RegistrationName => once(&mut name, key, within(key, text(value, key))?)?,
            RegistrationMember::Description => once(&mut description, key, within(key, text(value, key))?)?,
            RegistrationMember::ContextSourceInfo => once(&mut source_info, key, within(key, context_source_info(value, key))?)?,
            RegistrationMember::ContextSourceAlias => once(&mut alias, key, within(key, context_source_alias(value))?)?,
            RegistrationMember::Mode => once(&mut mode, key, within(key, token(value, key))?)?,
            RegistrationMember::Operations => once(&mut operation_list, key, within(key, operations(value, key))?)?,
            RegistrationMember::Management => once(&mut management_info, key, within(key, management(value))?)?,
            RegistrationMember::Tenant => once(&mut tenant_name, key, within(key, tenant(value, key))?)?,
            RegistrationMember::Scope => once(&mut scope, key, within(key, scopes(value))?)?,
            RegistrationMember::DatasetId => once(&mut dataset_ids, key, within(key, dataset_id_list(value, key))?)?,
            RegistrationMember::Location => once(&mut location, key, within(key, geometry(&value))?)?,
            RegistrationMember::ObservationSpace => once(&mut observation_space, key, within(key, geometry(&value))?)?,
            RegistrationMember::OperationSpace => once(&mut operation_space, key, within(key, geometry(&value))?)?,
            RegistrationMember::ObservationInterval => once(&mut observation_interval, key, within(key, time_interval(value))?)?,
            RegistrationMember::ManagementInterval => once(&mut management_interval, key, within(key, time_interval(value))?)?,
            RegistrationMember::ExpiresAt => once(&mut expires_at, key, within(key, date_time(value, key))?)?,
            RegistrationMember::RefreshRate => once(&mut refresh_rate, key, within(key, duration(value, key))?)?,
        }
    }
    required(typed, RegistrationMandatoryMember::Type.into())?;
    Ok(CSourceRegistration {
        id: D::Id::from_read(Seal::new(), id)?,
        name,
        description,
        information: required(information_list, RegistrationMandatoryMember::Information.into())?,
        endpoint: required(endpoint, RegistrationMandatoryMember::Endpoint.into())?,
        context_source_info: source_info,
        context_source_alias: alias,
        mode,
        operations: operation_list,
        management: management_info,
        tenant: tenant_name,
        scope,
        dataset_ids,
        location,
        observation_space,
        operation_space,
        observation_interval,
        management_interval,
        expires_at,
        refresh_rate,
        properties,
        output,
        context: registration_context,
    })
}
