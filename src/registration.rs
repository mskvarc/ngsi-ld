//! Context source registrations.
//!
//! They are the documents a context provider registers a context source with, and the notifications a Context Source
//! Registration Subscription receives, independent of any transport (NGSI-LD clauses 5.2.8 to 5.2.11, 5.2.34, 5.3.2,
//! 5.9 and 5.10).
//!
//! `CSourceRegistration<Input>` is what Register Context Source sends, `CSourceRegistration<Output>` what Retrieve
//! and Query Context Source Registrations return, and a [`RegistrationFragment`]
//! what Update Context Source Registration sends. A Context Source Registration Subscription is a
//! [`Subscription`](crate::subscription::Subscription) of kind
//! [`RegistrationSubscription`](crate::subscription::RegistrationSubscription); its notifications are
//! [`CSourceNotification`]s.
//!
//! A registration is read and checked as a subscription is, so this module shares the readers and the errors of
//! [`subscription`](crate::subscription): the dependency runs from registrations to subscriptions, never back.
//!
//! ```
//! use ngsi_ld::{model::Input, registration::CSourceRegistration};
//!
//! let text = concat!(
//!     r#"{"type":"ContextSourceRegistration","#,
//!     r#""information":[{"entities":[{"type":"Vehicle"}],"#,
//!     r#""propertyNames":["speed"]}],"#,
//!     r#""endpoint":"http://my.csource.org:1026/"}"#
//! );
//! let registration = CSourceRegistration::<Input>::parse(text)?;
//! assert_eq!(registration.render().to_string(), text);
//! # Ok::<(), anyhow::Error>(())
//! ```
//!
//! # Registrations
//!
//! [`CSourceRegistration`], [`RegistrationFragment`], [`RegistrationTypeMember`], [`RegistrationId`], [`RegistrationDirection`],
//! [`RegistrationStatus`], [`RegistrationOutputMembers`], [`RegistrationMode`], [`RegistrationManagementInfo`],
//! [`TimeInterval`].
//!
//! # Registered information
//!
//! [`RegistrationInfo`], [`EntityInfo`], [`EntityInfoDraft`], [`ContextSourcePropertyName`],
//! [`ContextSourcePropertyNameDraft`], [`ContextSourcePropertyNameError`].
//!
//! # Operations
//!
//! [`SupportedOperation`], [`DistributedOperation`], [`DistributedOperationIter`], [`OperationGroup`],
//! [`OperationGroupIter`].
//!
//! # Context source information
//!
//! [`ContextSourceInfo`], [`ContextSourceInfoKey`], [`ContextSourceInfoKeyIter`], [`ContextSourceInfoValue`],
//! [`ContextSourceMediaType`], [`OtherContextSourceInfo`], [`ContextSourceInfoDraft`], [`ContextSourceInfoError`].
//!
//! # Notifications
//!
//! [`CSourceNotification`], [`TriggerReason`].
//!
//! # Entity maps
//!
//! [`EntityMap`], [`EntityMapFragment`], [`EntityMapId`], [`EntitySource`].
//!
//! # Members and errors
//!
//! [`RegistrationMember`], [`RegistrationMemberIter`], [`RegistrationOutputMember`], [`RegistrationOutputMemberIter`],
//! [`RegistrationInfoMember`], [`RegistrationInfoMemberIter`], [`EntityInfoMember`], [`EntityInfoMemberIter`],
//! [`ManagementMember`], [`ManagementMemberIter`], [`RegistrationField`], [`Problem`].
//!
//! # Builders
//!
//! The builders of these types are in [`builders`].

pub use crate::registration::{
    build_error::Problem,
    context_source_info::{
        ContextSourceInfo,
        ContextSourceInfoDraft,
        ContextSourceInfoError,
        ContextSourceInfoKey,
        ContextSourceInfoKeyIter,
        ContextSourceInfoValue,
        ContextSourceMediaType,
        OtherContextSourceInfo,
    },
    direction::{RegistrationDirection, RegistrationOutputMembers, RegistrationStatus},
    entity_map::{EntityMap, EntityMapId, EntitySource, fragment::EntityMapFragment},
    fragment::{RegistrationFragment, RegistrationTypeMember},
    identifier::RegistrationId,
    information::{EntityInfo, EntityInfoDraft, RegistrationInfo},
    management::RegistrationManagementInfo,
    member::{
        EntityInfoMember,
        EntityInfoMemberIter,
        ManagementMember,
        ManagementMemberIter,
        RegistrationField,
        RegistrationInfoMember,
        RegistrationInfoMemberIter,
        RegistrationMember,
        RegistrationMemberIter,
        RegistrationOutputMember,
        RegistrationOutputMemberIter,
    },
    mode::RegistrationMode,
    notification::{CSourceNotification, TriggerReason},
    operation::{DistributedOperation, DistributedOperationIter, OperationGroup, OperationGroupIter, SupportedOperation},
    property::{ContextSourcePropertyName, ContextSourcePropertyNameDraft, ContextSourcePropertyNameError},
    time_interval::TimeInterval,
};
use crate::{
    model::{
        Context,
        ContextSourceAlias,
        DatasetId,
        DateTime,
        DocumentError,
        Duration,
        Input,
        OneOrMany,
        Output,
        RawJson,
        Scope,
        Tenant,
        Value,
        geometry::Geometry,
        keyword::CONTEXT,
    },
    registration::{reader::read_registration, writer::registration_object},
    subscription::ReadErrorKind,
};
use indexmap::IndexMap;
use jstrict::Value as Json;
use url::Url;

mod build_error;
mod builder;
pub mod builders;
mod context_source_info;
mod direction;
mod entity_map;
mod fragment;
mod identifier;
mod information;
mod management;
mod member;
mod mode;
#[cfg(feature = "linked-data")]
mod names;
mod notification;
mod operation;
mod property;
mod reader;
mod rules;
mod time_interval;
mod writer;

/// A context source registration.
///
/// It holds the endpoint of a context source and the entities and attributes it may provide information about (NGSI-LD
/// clause 5.2.9).
///
/// `D` is the direction: [`Input`] for what Register Context Source sends, with an optional `id`; [`Output`] for
/// what Retrieve and Query Context Source Registrations return, with an `id` and the members only a broker writes.
/// Build one with [`CSourceRegistration::builder`]; read one with [`CSourceRegistration::parse`].
///
/// ```
/// use ngsi_ld::registration::{CSourceRegistration, RegistrationInfo, RegistrationMode};
///
/// let registration = CSourceRegistration::builder("http://my.csource.org:1026")
///     .information([RegistrationInfo::builder()
///         .entities(["Vehicle"])
///         .property_names(["speed"])
///         .build()?])
///     .mode(RegistrationMode::Inclusive)
///     .build()?;
/// assert_eq!(
///     registration.render().to_string(),
///     concat!(
///         r#"{"type":"ContextSourceRegistration","#,
///         r#""information":[{"entities":[{"type":"Vehicle"}],"#,
///         r#""propertyNames":["speed"]}],"#,
///         r#""endpoint":"http://my.csource.org:1026/","#,
///         r#""mode":"inclusive"}"#
///     )
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CSourceRegistration<D: RegistrationDirection> {
    /// `id`.
    pub(crate) id: D::Id,
    /// `registrationName`.
    pub(crate) name: Option<String>,
    /// `description`.
    pub(crate) description: Option<String>,
    /// `information`; never empty.
    pub(crate) information: Vec<RegistrationInfo>,
    /// `endpoint`.
    pub(crate) endpoint: Url,
    /// `contextSourceInfo`; an empty array is kept, as no rule of Table 5.2.9-1 refuses it.
    pub(crate) context_source_info: Option<Vec<ContextSourceInfo>>,
    /// `contextSourceAlias`.
    pub(crate) context_source_alias: Option<ContextSourceAlias>,
    /// `mode`; `None` means `inclusive`.
    pub(crate) mode: Option<RegistrationMode>,
    /// `operations`; `None` means `federationOps`.
    pub(crate) operations: Option<Vec<SupportedOperation>>,
    /// `management`.
    pub(crate) management: Option<RegistrationManagementInfo>,
    /// `tenant`.
    pub(crate) tenant: Option<Tenant>,
    /// `scope`.
    pub(crate) scope: Option<OneOrMany<Scope>>,
    /// `datasetId`; an empty array is kept, as no rule of Table 5.2.9-1 refuses it.
    pub(crate) dataset_ids: Option<Vec<DatasetId>>,
    /// `location`.
    pub(crate) location: Option<Geometry>,
    /// `observationSpace`.
    pub(crate) observation_space: Option<Geometry>,
    /// `operationSpace`.
    pub(crate) operation_space: Option<Geometry>,
    /// `observationInterval`.
    pub(crate) observation_interval: Option<TimeInterval>,
    /// `managementInterval`.
    pub(crate) management_interval: Option<TimeInterval>,
    /// `expiresAt`.
    pub(crate) expires_at: Option<DateTime>,
    /// `refreshRate`.
    pub(crate) refresh_rate: Option<Duration>,
    /// The Context Source Properties (clause 4.9), in order.
    pub(crate) properties: IndexMap<ContextSourcePropertyName, Value>,
    /// The members only a broker writes; `()` in a request.
    pub(crate) output: D::RegistrationOutput,
    /// The `@context` the document carried (clause 5.2.3).
    pub(crate) context: Option<Context>,
}

impl<D: RegistrationDirection> CSourceRegistration<D> {
    /// Returns the name (`registrationName`).
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Returns the description (`description`).
    #[must_use]
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Returns what the context source may provide (`information`); never empty.
    #[must_use]
    pub fn information(&self) -> &[RegistrationInfo] {
        &self.information
    }

    /// Returns the endpoint through which the context source exposes its NGSI-LD interface (`endpoint`).
    #[must_use]
    pub const fn endpoint(&self) -> &Url {
        &self.endpoint
    }

    /// Returns the pairs a broker conveys when it contacts the context source (`contextSourceInfo`, clause
    /// 4.3.6.5).
    #[must_use]
    pub fn context_source_info(&self) -> Option<&[ContextSourceInfo]> {
        self.context_source_info.as_deref()
    }

    /// Returns the identifier of the context source used to detect loops (`contextSourceAlias`).
    #[must_use]
    pub const fn context_source_alias(&self) -> Option<&ContextSourceAlias> {
        self.context_source_alias.as_ref()
    }

    /// Returns the mode of distributed operation (`mode`); `None` means [`RegistrationMode::Inclusive`].
    #[must_use]
    pub const fn mode(&self) -> Option<RegistrationMode> {
        self.mode
    }

    /// Returns the operations the context source accepts (`operations`); `None` means the group `federationOps`
    /// (clause 4.20).
    #[must_use]
    pub fn operations(&self) -> Option<&[SupportedOperation]> {
        self.operations.as_deref()
    }

    /// Returns how a broker should send it distributed operations (`management`).
    #[must_use]
    pub const fn management(&self) -> Option<&RegistrationManagementInfo> {
        self.management.as_ref()
    }

    /// Returns the tenant every request to the context source names (`tenant`); `None` means the default tenant.
    #[must_use]
    pub const fn tenant(&self) -> Option<&Tenant> {
        self.tenant.as_ref()
    }

    /// Returns the scopes of the entities the context source has (`scope`).
    #[must_use]
    pub const fn scope(&self) -> Option<&OneOrMany<Scope>> {
        self.scope.as_ref()
    }

    /// Returns the dataset ids of the attribute instances the context source can provide (`datasetId`).
    #[must_use]
    pub fn dataset_ids(&self) -> Option<&[DatasetId]> {
        self.dataset_ids.as_deref()
    }

    /// Returns the location the context source may provide information for (`location`).
    #[must_use]
    pub const fn location(&self) -> Option<&Geometry> {
        self.location.as_ref()
    }

    /// Returns the area including the observation spaces of the entities (`observationSpace`).
    #[must_use]
    pub const fn observation_space(&self) -> Option<&Geometry> {
        self.observation_space.as_ref()
    }

    /// Returns the area including the operation spaces of the entities (`operationSpace`).
    #[must_use]
    pub const fn operation_space(&self) -> Option<&Geometry> {
        self.operation_space.as_ref()
    }

    /// Returns the interval of `observedAt` the context source provides temporal information for
    /// (`observationInterval`).
    #[must_use]
    pub const fn observation_interval(&self) -> Option<&TimeInterval> {
        self.observation_interval.as_ref()
    }

    /// Returns the interval of `createdAt`, `modifiedAt` and `deletedAt` the context source provides temporal
    /// information for (`managementInterval`).
    #[must_use]
    pub const fn management_interval(&self) -> Option<&TimeInterval> {
        self.management_interval.as_ref()
    }

    /// Returns when the registration expires (`expiresAt`).
    #[must_use]
    pub const fn expires_at(&self) -> Option<&DateTime> {
        self.expires_at.as_ref()
    }

    /// Returns the likely time between two updates at the endpoint (`refreshRate`).
    #[must_use]
    pub const fn refresh_rate(&self) -> Option<&Duration> {
        self.refresh_rate.as_ref()
    }

    /// Returns the Context Source Properties (clause 4.9): the members beyond Tables 5.2.9-1 and 5.2.9-2 that
    /// describe the context source and that a context source filter (`csf`) targets, in the order the document
    /// holds them.
    #[must_use]
    pub const fn properties(&self) -> &IndexMap<ContextSourcePropertyName, Value> {
        &self.properties
    }

    /// Returns the `@context` the document carried.
    #[must_use]
    pub const fn context(&self) -> Option<&Context> {
        self.context.as_ref()
    }

    /// Writes the registration with its members in the order Annex C.3 writes them: `id`, `type`, `registrationName`,
    /// `description`, `information`, `endpoint`, `contextSourceInfo`, `contextSourceAlias`, `mode`, `operations`,
    /// `management`, `tenant`, `scope`, `datasetId`, `location`, `observationSpace`, `operationSpace`,
    /// `observationInterval`, `managementInterval`, `expiresAt`, `refreshRate`, `status`, `timesSent`, `timesFailed`,
    /// `lastSuccess`, `lastFailure`, `createdAt`, `modifiedAt`, and its `@context` last when it has one.
    #[must_use]
    pub fn render(&self) -> RawJson {
        let mut object = registration_object(self);
        if let Some(context) = &self.context {
            object.push(CONTEXT.into(), context.to_json());
        }
        RawJson::from_json(Json::Object(object))
    }

    /// Reads a registration in direction `D`, checking the rules of Tables 5.2.8-1 to 5.2.11-1 and 5.2.34-1 and of
    /// clause 4.3.6, and keeping its `@context`. A member the tables do not define is a Context Source Property
    /// (clause 4.9), read with its value in order.
    ///
    /// # Errors
    ///
    /// Returns [`DocumentError`] naming the member at fault, including a member that is neither defined by the
    /// tables nor a valid Context Source Property name, an output-only member in an [`Input`] document, a property
    /// value holding `null`, and the broken rules.
    pub fn parse(text: &str) -> Result<Self, DocumentError<ReadErrorKind<Problem>>> {
        read_registration(text)
    }

    /// Reads a registration from JSON already parsed, such as an element of an array.
    pub(crate) fn from_json(json: Json) -> Result<Self, DocumentError<ReadErrorKind<Problem>>> {
        reader::read_registration_json(json)
    }

    /// Writes the registration without its `@context`: the body of a request, whose context the request pipeline
    /// delivers (clause 6.3.5).
    #[cfg(feature = "http")]
    pub(crate) fn to_object(&self) -> jstrict::Object {
        registration_object(self)
    }
}

impl CSourceRegistration<Input> {
    /// Returns the identifier the registrant chose (`id`); `None` lets the broker generate one (clause 5.9.2.4).
    #[must_use]
    pub const fn id(&self) -> Option<&RegistrationId> {
        self.id.as_ref()
    }
}

impl CSourceRegistration<Output> {
    /// Returns the identifier (`id`).
    #[must_use]
    pub const fn id(&self) -> &RegistrationId {
        &self.id
    }

    /// Returns the members only a broker writes: `status`, `timesSent`, `timesFailed`, `lastSuccess`,
    /// `lastFailure`, `createdAt`, `modifiedAt`.
    #[must_use]
    pub const fn output(&self) -> &RegistrationOutputMembers {
        &self.output
    }

    /// Converts a retrieved registration into one a client can send again, keeping its `id` and `@context` and
    /// dropping the members only a broker writes (Tables 5.2.9-2 and 5.2.2-1).
    #[must_use]
    pub fn into_input(self) -> CSourceRegistration<Input> {
        CSourceRegistration {
            id: Some(self.id),
            name: self.name,
            description: self.description,
            information: self.information,
            endpoint: self.endpoint,
            context_source_info: self.context_source_info,
            context_source_alias: self.context_source_alias,
            mode: self.mode,
            operations: self.operations,
            management: self.management,
            tenant: self.tenant,
            scope: self.scope,
            dataset_ids: self.dataset_ids,
            location: self.location,
            observation_space: self.observation_space,
            operation_space: self.operation_space,
            observation_interval: self.observation_interval,
            management_interval: self.management_interval,
            expires_at: self.expires_at,
            refresh_rate: self.refresh_rate,
            properties: self.properties,
            output: (),
            context: self.context,
        }
    }
}

#[cfg(test)]
mod tests;
