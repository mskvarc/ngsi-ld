//! Writing registration documents in the member order `CSourceRegistration::render` documents: a registration,
//! its information, management information and time intervals, and the members a fragment shares with them.

use crate::{
    model::{OneOrMany, Scope, geometry::Geometry},
    registration::{
        CSourceRegistration,
        ContextSourceInfo,
        EntityInfo,
        EntityInfoMember,
        ManagementMember,
        RegistrationDirection,
        RegistrationFragment,
        RegistrationInfo,
        RegistrationInfoMember,
        RegistrationManagementInfo,
        RegistrationMember,
        SupportedOperation,
        TimeInterval,
        member::RegistrationMandatoryMember,
    },
    seal::Seal,
    subscription::{
        DocumentType,
        direction::sealed::{IdMember, OutputBlock},
        endpoint::push_number,
        writer::{dataset_id_array, name_array, printed, push, push_change},
    },
};
use jstrict::{Object, Value as Json};

/// An `EntityInfo` object: `type`, `id`, `idPattern`.
fn entity_info(entity: &EntityInfo) -> Json {
    let mut object = Object::new();
    object.push(
        <&str>::from(EntityInfoMember::Type).into(),
        entity.entity_type.to_json(|name| Json::from(name.as_ref())),
    );
    push(&mut object, EntityInfoMember::Id.into(), entity.id.as_ref().map(|id| Json::from(id.as_str())));
    push(
        &mut object,
        EntityInfoMember::IdPattern.into(),
        entity.id_pattern.as_ref().map(|pattern| Json::from(pattern.as_ref())),
    );
    Json::Object(object)
}

/// A `RegistrationInfo` object: `entities`, `propertyNames`, `relationshipNames`.
fn registration_info(information: &RegistrationInfo) -> Json {
    let mut object = Object::new();
    push(
        &mut object,
        RegistrationInfoMember::Entities.into(),
        (!information.entities.is_empty()).then(|| Json::Array(information.entities.iter().map(entity_info).collect())),
    );
    push(
        &mut object,
        RegistrationInfoMember::PropertyNames.into(),
        name_array(&information.property_names),
    );
    push(
        &mut object,
        RegistrationInfoMember::RelationshipNames.into(),
        name_array(&information.relationship_names),
    );
    Json::Object(object)
}

/// `information`.
pub(crate) fn information(information: &[RegistrationInfo]) -> Json {
    Json::Array(information.iter().map(registration_info).collect())
}

/// `contextSourceInfo`: an array of key-value pairs (clause 5.2.22).
fn context_source_info(pairs: &[ContextSourceInfo]) -> Json {
    Json::Array(pairs.iter().map(ContextSourceInfo::to_json).collect())
}

/// `operations`.
pub(crate) fn operations(operations: &[SupportedOperation]) -> Json {
    Json::Array(operations.iter().map(printed).collect())
}

/// A `RegistrationManagementInfo` object: `localOnly`, `cacheDuration`, `timeout`, `cooldown`.
pub(crate) fn management(management: &RegistrationManagementInfo) -> Json {
    let mut object = Object::new();
    push(&mut object, ManagementMember::LocalOnly.into(), management.locality.flag().map(Json::from));
    push(
        &mut object,
        ManagementMember::CacheDuration.into(),
        management.cache_duration.as_ref().map(printed),
    );
    push_number(&mut object, ManagementMember::Timeout.into(), management.timeout.as_ref());
    push_number(&mut object, ManagementMember::Cooldown.into(), management.cooldown.as_ref());
    Json::Object(object)
}

/// A `TimeInterval` object: `startAt`, `endAt`.
pub(crate) fn time_interval(interval: &TimeInterval) -> Json {
    let mut object = Object::new();
    object.push("startAt".into(), printed(interval.start_at()));
    push(&mut object, "endAt", interval.end_at().map(printed));
    Json::Object(object)
}

/// `scope`: one scope as a string, several as an array, as read.
pub(crate) fn scopes(scopes: &OneOrMany<Scope>) -> Json {
    scopes.to_json(|scope| Json::from(scope.as_ref()))
}

/// Writes a registration in canonical member order, without its `@context`: the body of a request, whose context
/// the request pipeline delivers (clause 6.3.5).
pub(crate) fn registration_object<D: RegistrationDirection>(registration: &CSourceRegistration<D>) -> Object {
    let mut object = Object::new();
    push(
        &mut object,
        RegistrationMandatoryMember::Id.into(),
        registration.id.get(Seal::new()).map(|id| Json::from(id.as_str())),
    );
    object.push(
        <&str>::from(RegistrationMandatoryMember::Type).into(),
        Json::from(<&str>::from(DocumentType::ContextSourceRegistration)),
    );
    push(
        &mut object,
        RegistrationMember::RegistrationName.into(),
        registration.name.as_deref().map(Json::from),
    );
    push(
        &mut object,
        RegistrationMember::Description.into(),
        registration.description.as_deref().map(Json::from),
    );
    object.push(
        <&str>::from(RegistrationMandatoryMember::Information).into(),
        information(&registration.information),
    );
    object.push(
        <&str>::from(RegistrationMandatoryMember::Endpoint).into(),
        Json::from(registration.endpoint.as_str()),
    );
    push(
        &mut object,
        RegistrationMember::ContextSourceInfo.into(),
        registration.context_source_info.as_deref().map(context_source_info),
    );
    push(
        &mut object,
        RegistrationMember::ContextSourceAlias.into(),
        registration.context_source_alias.as_ref().map(|alias| Json::from(alias.as_ref())),
    );
    push(
        &mut object,
        RegistrationMember::Mode.into(),
        registration.mode.map(|mode| Json::from(<&str>::from(mode))),
    );
    push(
        &mut object,
        RegistrationMember::Operations.into(),
        registration.operations.as_deref().map(operations),
    );
    push(
        &mut object,
        RegistrationMember::Management.into(),
        registration.management.as_ref().map(management),
    );
    push(
        &mut object,
        RegistrationMember::Tenant.into(),
        registration.tenant.as_ref().map(|tenant| Json::from(tenant.as_ref())),
    );
    push(&mut object, RegistrationMember::Scope.into(), registration.scope.as_ref().map(scopes));
    push(
        &mut object,
        RegistrationMember::DatasetId.into(),
        registration.dataset_ids.as_deref().map(dataset_id_array),
    );
    push(
        &mut object,
        RegistrationMember::Location.into(),
        registration.location.as_ref().map(Geometry::to_json),
    );
    push(
        &mut object,
        RegistrationMember::ObservationSpace.into(),
        registration.observation_space.as_ref().map(Geometry::to_json),
    );
    push(
        &mut object,
        RegistrationMember::OperationSpace.into(),
        registration.operation_space.as_ref().map(Geometry::to_json),
    );
    push(
        &mut object,
        RegistrationMember::ObservationInterval.into(),
        registration.observation_interval.as_ref().map(time_interval),
    );
    push(
        &mut object,
        RegistrationMember::ManagementInterval.into(),
        registration.management_interval.as_ref().map(time_interval),
    );
    push(&mut object, RegistrationMember::ExpiresAt.into(), registration.expires_at.as_ref().map(printed));
    push(
        &mut object,
        RegistrationMember::RefreshRate.into(),
        registration.refresh_rate.as_ref().map(printed),
    );
    for (name, value) in &registration.properties {
        object.push(name.as_ref().into(), value.to_json());
    }
    registration.output.write(Seal::new(), &mut object);
    object
}

/// Writes a registration fragment: `type`, then the members it sets or deletes in canonical order, `information`
/// and `endpoint` among them, without an `@context`.
pub(crate) fn fragment_object(fragment: &RegistrationFragment) -> Object {
    let mut object = Object::new();
    object.push(
        <&str>::from(RegistrationMandatoryMember::Type).into(),
        Json::from(<&str>::from(DocumentType::ContextSourceRegistration)),
    );
    push_change(&mut object, RegistrationMember::RegistrationName.into(), fragment.name.as_ref(), |name| {
        Some(Json::from(name.as_str()))
    });
    push_change(&mut object, RegistrationMember::Description.into(), fragment.description.as_ref(), |text| {
        Some(Json::from(text.as_str()))
    });
    if let Some(replacement) = &fragment.information {
        object.push(<&str>::from(RegistrationMandatoryMember::Information).into(), information(replacement));
    }
    if let Some(endpoint) = &fragment.endpoint {
        object.push(<&str>::from(RegistrationMandatoryMember::Endpoint).into(), Json::from(endpoint.as_str()));
    }
    push_change(
        &mut object,
        RegistrationMember::ContextSourceInfo.into(),
        fragment.context_source_info.as_ref(),
        |pairs| Some(context_source_info(pairs)),
    );
    push_change(
        &mut object,
        RegistrationMember::ContextSourceAlias.into(),
        fragment.context_source_alias.as_ref(),
        |alias| Some(Json::from(alias.as_ref())),
    );
    push_change(&mut object, RegistrationMember::Mode.into(), fragment.mode.as_ref(), |mode| {
        Some(Json::from(<&str>::from(*mode)))
    });
    push_change(&mut object, RegistrationMember::Operations.into(), fragment.operations.as_ref(), |chosen| {
        Some(operations(chosen))
    });
    push_change(&mut object, RegistrationMember::Management.into(), fragment.management.as_ref(), |info| {
        Some(management(info))
    });
    push_change(&mut object, RegistrationMember::Tenant.into(), fragment.tenant.as_ref(), |tenant| {
        Some(Json::from(tenant.as_ref()))
    });
    push_change(&mut object, RegistrationMember::Scope.into(), fragment.scope.as_ref(), |scope| {
        Some(scopes(scope))
    });
    // An empty `datasetId` is written, as the list of no dataset.
    push_change(&mut object, RegistrationMember::DatasetId.into(), fragment.dataset_ids.as_ref(), |ids| {
        Some(dataset_id_array(ids))
    });
    push_change(&mut object, RegistrationMember::Location.into(), fragment.location.as_ref(), |geometry| {
        Some(geometry.to_json())
    });
    push_change(
        &mut object,
        RegistrationMember::ObservationSpace.into(),
        fragment.observation_space.as_ref(),
        |geometry| Some(geometry.to_json()),
    );
    push_change(
        &mut object,
        RegistrationMember::OperationSpace.into(),
        fragment.operation_space.as_ref(),
        |geometry| Some(geometry.to_json()),
    );
    push_change(
        &mut object,
        RegistrationMember::ObservationInterval.into(),
        fragment.observation_interval.as_ref(),
        |interval| Some(time_interval(interval)),
    );
    push_change(
        &mut object,
        RegistrationMember::ManagementInterval.into(),
        fragment.management_interval.as_ref(),
        |interval| Some(time_interval(interval)),
    );
    push_change(&mut object, RegistrationMember::ExpiresAt.into(), fragment.expires_at.as_ref(), |time| {
        Some(printed(time))
    });
    push_change(&mut object, RegistrationMember::RefreshRate.into(), fragment.refresh_rate.as_ref(), |rate| {
        Some(printed(rate))
    });
    for (name, change) in &fragment.properties {
        push(&mut object, name.as_ref(), change.write(|value| Some(value.to_json())));
    }
    object
}
