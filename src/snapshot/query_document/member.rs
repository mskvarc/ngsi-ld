//! The members of the queries a snapshot holds (NGSI-LD Tables 5.2.23-1 and 5.2.44-1), named once: the readers and
//! writers spell them through [`QueryMember`] and [`AggregationMember`], and a problem names them.

/// A member of a `Query` a snapshot holds.
///
/// NGSI-LD Table 5.2.23-1 defines the members; `temporalQ` and `aggrParams` of a temporal query are included.
///
/// The enum is `#[non_exhaustive]`: later versions of the specification add members to the table.
///
/// ```
/// use ngsi_ld::snapshot::QueryMember;
///
/// assert_eq!(QueryMember::ContainedBy.to_string(), "containedBy");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::Display, strum::EnumString, strum::IntoStaticStr, strum::EnumIter)]
#[strum(serialize_all = "camelCase")]
#[non_exhaustive]
pub enum QueryMember {
    /// `entities`.
    Entities,
    /// `attrs`.
    Attrs,
    /// `q`.
    Q,
    /// `geoQ`.
    GeoQ,
    /// `scopeQ`.
    ScopeQ,
    /// `csf`.
    Csf,
    /// `temporalQ`.
    TemporalQ,
    /// `aggrParams`.
    AggrParams,
    /// `pick`.
    Pick,
    /// `omit`.
    Omit,
    /// `containedBy`.
    ContainedBy,
    /// `datasetId`.
    DatasetId,
    /// `expandValues`.
    ExpandValues,
    /// `entityMap`.
    EntityMap,
    /// `entityMapLifetime`.
    EntityMapLifetime,
    /// `jsonKeys`.
    JsonKeys,
    /// `join`.
    Join,
    /// `joinLevel`.
    JoinLevel,
    /// `lang`.
    Lang,
    /// `ordering`.
    Ordering,
    /// `splitEntities`.
    SplitEntities,
}

/// A member of the `AggregationParams` of a temporal query.
///
/// NGSI-LD Table 5.2.44-1.
///
/// The enum is `#[non_exhaustive]`: later versions of the specification add members to the table.
///
/// ```
/// use ngsi_ld::snapshot::AggregationMember;
///
/// assert_eq!(
///     AggregationMember::AggrPeriodDuration.to_string(),
///     "aggrPeriodDuration"
/// );
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::Display, strum::EnumString, strum::IntoStaticStr, strum::EnumIter)]
#[strum(serialize_all = "camelCase")]
#[non_exhaustive]
pub enum AggregationMember {
    /// `aggrMethods`.
    AggrMethods,
    /// `aggrPeriodDuration`.
    AggrPeriodDuration,
}
