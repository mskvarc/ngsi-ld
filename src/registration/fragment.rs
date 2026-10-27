//! Registration fragments (NGSI-LD clauses 5.4, 5.5.8 and 5.9.3): the changes Update Context Source Registration
//! applies to a registration.

use crate::{
    model::{
        BuildError,
        ContextSourceAlias,
        DatasetId,
        DateTime,
        Duration,
        MemberChange,
        MemberProblem,
        OneOrMany,
        RawJson,
        Scope,
        Tenant,
        UrlDraft,
        Value,
        geometry::Geometry,
    },
    registration::{
        ContextSourceInfo,
        ContextSourceInfoDraft,
        ContextSourcePropertyName,
        ContextSourcePropertyNameDraft,
        Problem,
        RegistrationField,
        RegistrationInfo,
        RegistrationManagementInfo,
        RegistrationMember,
        RegistrationMode,
        SupportedOperation,
        TimeInterval,
        context_source_info,
        property,
        rules::auxiliary_operations,
        writer::fragment_object,
    },
};
use bon::bon;
use indexmap::IndexMap;
use jstrict::{Object, Value as Json};
use url::Url;

/// Whether a registration fragment carries the `type` member.
///
/// Clause 5.4 makes `type` optional "for certain bindings where it can be determined from the operation signature",
/// and the path of Update Context Source Registration names a registration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum RegistrationTypeMember {
    /// The fragment writes `"type": "ContextSourceRegistration"`.
    #[default]
    Written,
    /// The fragment leaves the `type` out.
    Omitted,
}

/// The changes Update Context Source Registration applies to a registration.
///
/// NGSI-LD clauses 5.4 and 5.9.3.
///
/// Each member a fragment gives replaces the registration's member, and a member whose value is the NGSI-LD Null
/// removes it (clause 5.4). A fragment names only the optional members it deletes, through [`RegistrationMember`],
/// so `information` and `endpoint`, which a registration needs, can be replaced but not removed.
///
/// It is written with `"type": "ContextSourceRegistration"`, unless [`RegistrationTypeMember::Omitted`] leaves it
/// out, and without `id`: the binding names the registration,
/// whose `id` cannot change (Table 5.2.9-1). A replaced `information` replaces the whole list. Like an entity
/// fragment, it is only written.
///
/// ```
/// use ngsi_ld::{
///     model::DateTime,
///     registration::{RegistrationFragment, RegistrationMember},
/// };
///
/// let fragment = RegistrationFragment::builder()
///     .endpoint("http://my.csource.org:1027/")
///     .expires_at("2030-01-01T00:00:00Z".parse::<DateTime>()?)
///     .delete(RegistrationMember::Location)
///     .build()?;
/// assert_eq!(
///     fragment.render().to_string(),
///     concat!(
///         r#"{"type":"ContextSourceRegistration","#,
///         r#""endpoint":"http://my.csource.org:1027/","#,
///         r#""location":"urn:ngsi-ld:null","#,
///         r#""expiresAt":"2030-01-01T00:00:00Z"}"#
///     )
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistrationFragment {
    /// Whether the fragment writes `type`.
    pub(crate) type_member: RegistrationTypeMember,
    /// `registrationName`.
    pub(crate) name: Option<MemberChange<String>>,
    /// `description`.
    pub(crate) description: Option<MemberChange<String>>,
    /// The replacement `information`, which can be replaced but not removed.
    pub(crate) information: Option<Vec<RegistrationInfo>>,
    /// The replacement `endpoint`, which can be replaced but not removed.
    pub(crate) endpoint: Option<Url>,
    /// `contextSourceInfo`.
    pub(crate) context_source_info: Option<MemberChange<Vec<ContextSourceInfo>>>,
    /// `contextSourceAlias`.
    pub(crate) context_source_alias: Option<MemberChange<ContextSourceAlias>>,
    /// `mode`.
    pub(crate) mode: Option<MemberChange<RegistrationMode>>,
    /// `operations`.
    pub(crate) operations: Option<MemberChange<Vec<SupportedOperation>>>,
    /// `management`.
    pub(crate) management: Option<MemberChange<RegistrationManagementInfo>>,
    /// `tenant`.
    pub(crate) tenant: Option<MemberChange<Tenant>>,
    /// `scope`.
    pub(crate) scope: Option<MemberChange<OneOrMany<Scope>>>,
    /// `datasetId`.
    pub(crate) dataset_ids: Option<MemberChange<Vec<DatasetId>>>,
    /// `location`.
    pub(crate) location: Option<MemberChange<Geometry>>,
    /// `observationSpace`.
    pub(crate) observation_space: Option<MemberChange<Geometry>>,
    /// `operationSpace`.
    pub(crate) operation_space: Option<MemberChange<Geometry>>,
    /// `observationInterval`.
    pub(crate) observation_interval: Option<MemberChange<TimeInterval>>,
    /// `managementInterval`.
    pub(crate) management_interval: Option<MemberChange<TimeInterval>>,
    /// `expiresAt`.
    pub(crate) expires_at: Option<MemberChange<DateTime>>,
    /// `refreshRate`.
    pub(crate) refresh_rate: Option<MemberChange<Duration>>,
    /// The Context Source Properties set or deleted (clause 4.9), in order.
    pub(crate) properties: IndexMap<ContextSourcePropertyName, MemberChange<Value>>,
}

#[bon]
impl RegistrationFragment {
    /// Starts a builder. Each setter replaces a member; `delete` removes one, and may be called for several.
    /// `property` sets a Context Source Property (clause 4.9) and `delete_property` removes one, each once per
    /// property.
    ///
    /// # Errors
    ///
    /// `build()` returns [`BuildError`] listing an endpoint that is not a URL, an empty `information` or text, a
    /// `contextSourceInfo` value its key does not allow, a member both set and deleted, a Context Source Property
    /// name that is invalid or given twice, and the rules of the `auxiliary` and `exclusive` modes when the
    /// fragment sets the mode together with `operations` or `information`; whether the result is valid against the
    /// target is the broker's to check.
    #[builder]
    pub fn new(
        #[builder(field)] deletions: Vec<RegistrationMember>,
        #[builder(field)] properties: Vec<(ContextSourcePropertyNameDraft, MemberChange<Value>)>,
        #[builder(default)] type_member: RegistrationTypeMember,
        #[builder(into)] registration_name: Option<String>,
        #[builder(into)] description: Option<String>,
        #[builder(with = |information: impl IntoIterator<Item = RegistrationInfo>| information.into_iter().collect())] information: Option<
            Vec<RegistrationInfo>,
        >,
        #[builder(into)] endpoint: Option<UrlDraft>,
        #[builder(with = |pairs: impl IntoIterator<Item = impl Into<ContextSourceInfoDraft>>| pairs.into_iter().map(Into::into).collect())]
        context_source_info: Option<Vec<ContextSourceInfoDraft>>,
        context_source_alias: Option<ContextSourceAlias>,
        mode: Option<RegistrationMode>,
        #[builder(with = |operations: impl IntoIterator<Item = impl Into<SupportedOperation>>| operations.into_iter().map(Into::into).collect())]
        operations: Option<Vec<SupportedOperation>>,
        management: Option<RegistrationManagementInfo>,
        tenant: Option<Tenant>,
        #[builder(into)] scope: Option<OneOrMany<Scope>>,
        #[builder(with = |ids: impl IntoIterator<Item = DatasetId>| ids.into_iter().collect())] dataset_ids: Option<Vec<DatasetId>>,
        #[builder(into)] location: Option<Geometry>,
        #[builder(into)] observation_space: Option<Geometry>,
        #[builder(into)] operation_space: Option<Geometry>,
        observation_interval: Option<TimeInterval>,
        management_interval: Option<TimeInterval>,
        expires_at: Option<DateTime>,
        refresh_rate: Option<Duration>,
    ) -> Result<Self, BuildError<Problem>> {
        let mut problems = Vec::new();
        for (member, text) in [
            (RegistrationMember::RegistrationName, registration_name.as_deref()),
            (RegistrationMember::Description, description.as_deref()),
        ] {
            if text.is_some_and(str::is_empty) {
                problems.push(Problem::from(MemberProblem::Empty {
                    member: RegistrationField::from(member),
                }));
            }
        }
        if let Some(information) = &information {
            if information.is_empty() {
                problems.push(Problem::from(MemberProblem::Empty {
                    member: RegistrationField::Information,
                }));
            }
            if mode == Some(RegistrationMode::Exclusive) && !information.iter().all(RegistrationInfo::names_entity_ids_and_attributes) {
                problems.push(Problem::ExclusiveWithoutEntityIdAndAttributes);
            }
        }
        if let (Some(RegistrationMode::Auxiliary), Some(operations)) = (mode, &operations)
            && !auxiliary_operations(operations)
        {
            problems.push(Problem::AuxiliaryOperations);
        }
        let context_source_info = context_source_info.map(|pairs| context_source_info::resolve(pairs, &mut problems));
        let properties = property::resolve(properties, &mut problems);
        let endpoint = endpoint.and_then(|url| url.resolve(RegistrationField::Endpoint, &mut problems));
        let mut fragment = Self {
            type_member,
            name: registration_name.map(MemberChange::Set),
            description: description.map(MemberChange::Set),
            information,
            endpoint,
            context_source_info: context_source_info.map(MemberChange::Set),
            context_source_alias: context_source_alias.map(MemberChange::Set),
            mode: mode.map(MemberChange::Set),
            operations: operations.map(MemberChange::Set),
            management: management.map(MemberChange::Set),
            tenant: tenant.map(MemberChange::Set),
            scope: scope.map(MemberChange::Set),
            dataset_ids: dataset_ids.map(MemberChange::Set),
            location: location.map(MemberChange::Set),
            observation_space: observation_space.map(MemberChange::Set),
            operation_space: operation_space.map(MemberChange::Set),
            observation_interval: observation_interval.map(MemberChange::Set),
            management_interval: management_interval.map(MemberChange::Set),
            expires_at: expires_at.map(MemberChange::Set),
            refresh_rate: refresh_rate.map(MemberChange::Set),
            properties,
        };
        for member in deletions {
            fragment.delete(member, &mut problems);
        }
        BuildError::finish(problems, fragment)
    }
}

member_slots!(
    impl for RegistrationFragment;
    member: RegistrationMember, field: RegistrationField, problem: Problem;
    slots: RegistrationName => name,
    Description => description,
    ContextSourceInfo => context_source_info,
    ContextSourceAlias => context_source_alias,
    Mode => mode,
    Operations => operations,
    Management => management,
    Tenant => tenant,
    Scope => scope,
    DatasetId => dataset_ids,
    Location => location,
    ObservationSpace => observation_space,
    OperationSpace => operation_space,
    ObservationInterval => observation_interval,
    ManagementInterval => management_interval,
    ExpiresAt => expires_at,
    RefreshRate => refresh_rate;
);

impl<S: registration_fragment_builder::State> RegistrationFragmentBuilder<S> {
    /// Deletes `member` from the registration: the fragment writes the NGSI-LD Null for it (clause 5.4).
    pub fn delete(mut self, member: RegistrationMember) -> Self {
        self.deletions.push(member);
        self
    }

    /// Sets the Context Source Property `name` (clause 4.9) to `value`: a
    /// [`ContextSourcePropertyName`], or text read when the fragment is built.
    pub fn property(mut self, name: impl Into<ContextSourcePropertyNameDraft>, value: impl Into<Value>) -> Self {
        self.properties.push((name.into(), MemberChange::Set(value.into())));
        self
    }

    /// Deletes the Context Source Property `name` (clause 4.9): the fragment writes the NGSI-LD Null for it (clause
    /// 5.4).
    pub fn delete_property(mut self, name: impl Into<ContextSourcePropertyNameDraft>) -> Self {
        self.properties.push((name.into(), MemberChange::Delete));
        self
    }
}

impl RegistrationFragment {
    /// Returns the change to the name (`registrationName`).
    #[must_use]
    pub fn name(&self) -> Option<MemberChange<&str>> {
        self.name.as_ref().map(|change| change.as_ref().map(String::as_str))
    }

    /// Returns the change to the description (`description`).
    #[must_use]
    pub fn description(&self) -> Option<MemberChange<&str>> {
        self.description.as_ref().map(|change| change.as_ref().map(String::as_str))
    }

    /// Returns the replacement `information`, when the fragment replaces it.
    #[must_use]
    pub fn information(&self) -> Option<&[RegistrationInfo]> {
        self.information.as_deref()
    }

    /// Returns the replacement `endpoint`, when the fragment replaces it.
    #[must_use]
    pub const fn endpoint(&self) -> Option<&Url> {
        self.endpoint.as_ref()
    }

    /// Returns the change to the context source information (`contextSourceInfo`).
    #[must_use]
    pub fn context_source_info(&self) -> Option<MemberChange<&[ContextSourceInfo]>> {
        self.context_source_info.as_ref().map(|change| change.as_ref().map(Vec::as_slice))
    }

    /// Returns the change to the context source alias (`contextSourceAlias`).
    #[must_use]
    pub fn context_source_alias(&self) -> Option<MemberChange<&ContextSourceAlias>> {
        self.context_source_alias.as_ref().map(MemberChange::as_ref)
    }

    /// Returns the change to the mode (`mode`).
    #[must_use]
    pub const fn mode(&self) -> Option<MemberChange<RegistrationMode>> {
        self.mode
    }

    /// Returns the change to the supported operations (`operations`).
    #[must_use]
    pub fn operations(&self) -> Option<MemberChange<&[SupportedOperation]>> {
        self.operations.as_ref().map(|change| change.as_ref().map(Vec::as_slice))
    }

    /// Returns the change to the management information (`management`).
    #[must_use]
    pub fn management(&self) -> Option<MemberChange<&RegistrationManagementInfo>> {
        self.management.as_ref().map(MemberChange::as_ref)
    }

    /// Returns the change to the tenant (`tenant`).
    #[must_use]
    pub fn tenant(&self) -> Option<MemberChange<&Tenant>> {
        self.tenant.as_ref().map(MemberChange::as_ref)
    }

    /// Returns the change to the scope (`scope`).
    #[must_use]
    pub fn scope(&self) -> Option<MemberChange<&OneOrMany<Scope>>> {
        self.scope.as_ref().map(MemberChange::as_ref)
    }

    /// Returns the change to the dataset ids (`datasetId`).
    #[must_use]
    pub fn dataset_ids(&self) -> Option<MemberChange<&[DatasetId]>> {
        self.dataset_ids.as_ref().map(|change| change.as_ref().map(Vec::as_slice))
    }

    /// Returns the change to the location (`location`).
    #[must_use]
    pub fn location(&self) -> Option<MemberChange<&Geometry>> {
        self.location.as_ref().map(MemberChange::as_ref)
    }

    /// Returns the change to the observation space (`observationSpace`).
    #[must_use]
    pub fn observation_space(&self) -> Option<MemberChange<&Geometry>> {
        self.observation_space.as_ref().map(MemberChange::as_ref)
    }

    /// Returns the change to the operation space (`operationSpace`).
    #[must_use]
    pub fn operation_space(&self) -> Option<MemberChange<&Geometry>> {
        self.operation_space.as_ref().map(MemberChange::as_ref)
    }

    /// Returns the change to the observation interval (`observationInterval`).
    #[must_use]
    pub fn observation_interval(&self) -> Option<MemberChange<&TimeInterval>> {
        self.observation_interval.as_ref().map(MemberChange::as_ref)
    }

    /// Returns the change to the management interval (`managementInterval`).
    #[must_use]
    pub fn management_interval(&self) -> Option<MemberChange<&TimeInterval>> {
        self.management_interval.as_ref().map(MemberChange::as_ref)
    }

    /// Returns the change to when the registration expires (`expiresAt`).
    #[must_use]
    pub fn expires_at(&self) -> Option<MemberChange<&DateTime>> {
        self.expires_at.as_ref().map(MemberChange::as_ref)
    }

    /// Returns the change to the refresh rate (`refreshRate`).
    #[must_use]
    pub fn refresh_rate(&self) -> Option<MemberChange<&Duration>> {
        self.refresh_rate.as_ref().map(MemberChange::as_ref)
    }

    /// Returns the changes to the Context Source Properties (clause 4.9), in order.
    #[must_use]
    pub const fn properties(&self) -> &IndexMap<ContextSourcePropertyName, MemberChange<Value>> {
        &self.properties
    }

    /// Returns whether the fragment writes `type`.
    #[must_use]
    pub const fn type_member(&self) -> RegistrationTypeMember {
        self.type_member
    }

    /// Writes the fragment: `type`, then the members in canonical order, `information` and `endpoint` among them.
    #[must_use]
    pub fn render(&self) -> RawJson {
        RawJson::from_json(Json::Object(self.to_object()))
    }

    /// Writes the fragment without an `@context`: the body of a request, whose context the request pipeline
    /// delivers (clause 6.3.5).
    pub(crate) fn to_object(&self) -> Object {
        fragment_object(self)
    }
}
