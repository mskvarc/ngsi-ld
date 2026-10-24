//! The optional members of a context source registration (NGSI-LD Table 5.2.9-1), named once: a fragment sets or
//! deletes them, and the readers and writers of registrations spell them through [`RegistrationMember`].

/// An optional member of a registration (Table 5.2.9-1), which a fragment may set or delete; they are ordered as
/// the canonical form writes them.
///
/// `id` and `type` cannot be changed, and `information` and `endpoint` can be replaced but not removed, so none of
/// them is a variant.
///
/// ```
/// use ngsi_ld::registration::RegistrationMember;
///
/// assert_eq!(
///     RegistrationMember::ManagementInterval.to_string(),
///     "managementInterval"
/// );
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, strum::Display, strum::EnumString, strum::IntoStaticStr, strum::EnumIter)]
#[strum(serialize_all = "camelCase")]
#[non_exhaustive]
pub enum RegistrationMember {
    /// `registrationName`.
    RegistrationName,
    /// `description`.
    Description,
    /// `contextSourceInfo`.
    ContextSourceInfo,
    /// `contextSourceAlias`.
    ContextSourceAlias,
    /// `mode`.
    Mode,
    /// `operations`.
    Operations,
    /// `management`.
    Management,
    /// `tenant`.
    Tenant,
    /// `scope`.
    Scope,
    /// `datasetId`.
    DatasetId,
    /// `location`.
    Location,
    /// `observationSpace`.
    ObservationSpace,
    /// `operationSpace`.
    OperationSpace,
    /// `observationInterval`.
    ObservationInterval,
    /// `managementInterval`.
    ManagementInterval,
    /// `expiresAt`.
    ExpiresAt,
    /// `refreshRate`.
    RefreshRate,
}

/// A member of a registration only a broker writes (Table 5.2.9-2, and `createdAt` and `modifiedAt` of Table
/// 5.2.2-1), as the readers and writers spell it; they are ordered as the canonical form writes them.
///
/// The enum is `#[non_exhaustive]`: later versions of the specification add members to the table.
///
/// ```
/// use ngsi_ld::registration::RegistrationOutputMember;
///
/// assert_eq!(RegistrationOutputMember::TimesSent.to_string(), "timesSent");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::Display, strum::EnumString, strum::IntoStaticStr, strum::EnumIter)]
#[strum(serialize_all = "camelCase")]
#[non_exhaustive]
pub enum RegistrationOutputMember {
    /// `status`.
    Status,
    /// `timesSent`.
    TimesSent,
    /// `timesFailed`.
    TimesFailed,
    /// `lastSuccess`.
    LastSuccess,
    /// `lastFailure`.
    LastFailure,
    /// `createdAt`.
    CreatedAt,
    /// `modifiedAt`.
    ModifiedAt,
}

/// A mandatory member of a registration (Table 5.2.9-1): those [`RegistrationMember`] leaves out, as the readers
/// and writers spell them. The table closes the set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::EnumString, strum::IntoStaticStr)]
#[strum(serialize_all = "camelCase")]
pub(crate) enum RegistrationMandatoryMember {
    /// `id`.
    Id,
    /// `type`.
    Type,
    /// `information`.
    Information,
    /// `endpoint`.
    Endpoint,
}

/// A member of a registration information element (Table 5.2.10-1), as the readers and writers spell it and as a
/// problem names it.
///
/// The enum is `#[non_exhaustive]`: later versions of the specification add members to the table.
///
/// ```
/// use ngsi_ld::registration::RegistrationInfoMember;
///
/// assert_eq!(
///     RegistrationInfoMember::PropertyNames.to_string(),
///     "propertyNames"
/// );
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::Display, strum::EnumString, strum::IntoStaticStr, strum::EnumIter)]
#[strum(serialize_all = "camelCase")]
#[non_exhaustive]
pub enum RegistrationInfoMember {
    /// `entities`.
    Entities,
    /// `propertyNames`.
    PropertyNames,
    /// `relationshipNames`.
    RelationshipNames,
}

/// A member of an entity information (Table 5.2.8-1), as the readers and writers spell it.
///
/// The enum is `#[non_exhaustive]`: later versions of the specification add members to the table.
///
/// ```
/// use ngsi_ld::registration::EntityInfoMember;
///
/// assert_eq!(EntityInfoMember::IdPattern.to_string(), "idPattern");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::Display, strum::EnumString, strum::IntoStaticStr, strum::EnumIter)]
#[strum(serialize_all = "camelCase")]
#[non_exhaustive]
pub enum EntityInfoMember {
    /// `id`.
    Id,
    /// `idPattern`.
    IdPattern,
    /// `type`.
    Type,
}

/// A member of the management information of a registration (Table 5.2.34-1), as the readers and writers spell it
/// and as a problem names it.
///
/// The enum is `#[non_exhaustive]`: later versions of the specification add members to the table.
///
/// ```
/// use ngsi_ld::registration::ManagementMember;
///
/// assert_eq!(ManagementMember::CacheDuration.to_string(), "cacheDuration");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::Display, strum::EnumString, strum::IntoStaticStr, strum::EnumIter)]
#[strum(serialize_all = "camelCase")]
#[non_exhaustive]
pub enum ManagementMember {
    /// `localOnly`.
    LocalOnly,
    /// `cacheDuration`.
    CacheDuration,
    /// `timeout`.
    Timeout,
    /// `cooldown`.
    Cooldown,
}

/// A member a registration problem names: `information`, `endpoint`, an optional member of the registration, a
/// member of an `information` element, or a member of the management information. It displays as the member's own
/// name.
///
/// The enum is exhaustive: a registration has no other part.
///
/// ```
/// use ngsi_ld::registration::{RegistrationField, RegistrationInfoMember};
///
/// assert_eq!(RegistrationField::Information.to_string(), "information");
/// assert_eq!(
///     RegistrationField::from(RegistrationInfoMember::PropertyNames).to_string(),
///     "propertyNames"
/// );
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, derive_more::Display, derive_more::From)]
pub enum RegistrationField {
    /// `information` (Table 5.2.9-1).
    #[display("information")]
    #[from(skip)]
    Information,
    /// `endpoint` (Table 5.2.9-1).
    #[display("endpoint")]
    #[from(skip)]
    Endpoint,
    /// An optional member of the registration (Table 5.2.9-1).
    Optional(RegistrationMember),
    /// A member of an `information` element (Table 5.2.10-1).
    InformationElement(RegistrationInfoMember),
    /// A member of the management information (Table 5.2.34-1).
    Management(ManagementMember),
}
