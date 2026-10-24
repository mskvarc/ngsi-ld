//! The names of the URL parameters of the HTTP binding, the one spelling the crate writes into a request and reads
//! from a received one.

/// A URL parameter of the HTTP binding, spelled as the specification writes it: every parameter of the tables of
/// clause 6 (Tables 6.4.3.2-1, 6.4.3.3-1, 6.5.3.1-1 to 6.5.3.4-1, 6.6.3.1-1, 6.6.3.2-1, 6.7.3.1-1 to 6.7.3.3-1,
/// 6.8.3.2-1, 6.10.3.2-1, 6.12.3.2-1, 6.18.3.2-1, 6.19.3.1-1, 6.21.3.1-1, 6.25.3.1-1, 6.27.3.1-1, 6.29.3.2-1,
/// 6.30.3.1-1, 6.30.3.2-1 and 6.36.3.2-1), and those clause 6.3 adds to them: `format` and `options` of clauses
/// 6.3.7, 6.3.11 and 6.3.12, `limit` and `offset` of clause 6.3.10, `count` of clause 6.3.13, `geometryProperty` of
/// clause 6.3.15 and `local` of clause 6.3.18.
///
/// The enum is `#[non_exhaustive]`: later versions of the specification add parameters.
///
/// ```
/// use ngsi_ld::http::QueryParameter;
///
/// assert_eq!(QueryParameter::IdPattern.to_string(), "idPattern");
/// assert_eq!("scopeQ".parse::<QueryParameter>()?, QueryParameter::ScopeQ);
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::Display, strum::EnumString, strum::IntoStaticStr)]
#[strum(serialize_all = "camelCase")]
#[non_exhaustive]
pub enum QueryParameter {
    /// `type`.
    Type,
    /// `id`.
    Id,
    /// `idPattern`.
    IdPattern,
    /// `attrs`.
    Attrs,
    /// `q`.
    Q,
    /// `csf`.
    Csf,
    /// `scopeQ`.
    ScopeQ,
    /// `georel`.
    Georel,
    /// `geometry`.
    Geometry,
    /// `coordinates`.
    Coordinates,
    /// `geoproperty`.
    Geoproperty,
    /// `geometryProperty`.
    GeometryProperty,
    /// `pick`.
    Pick,
    /// `omit`.
    Omit,
    /// `lang`.
    Lang,
    /// `format`.
    Format,
    /// `options`.
    Options,
    /// `limit`.
    Limit,
    /// `offset`.
    Offset,
    /// `count`.
    Count,
    /// `local`.
    Local,
    /// `join`.
    Join,
    /// `joinLevel`.
    JoinLevel,
    /// `containedBy`.
    ContainedBy,
    /// `orderBy`.
    OrderBy,
    /// `collation`.
    Collation,
    /// `orderFrom`.
    OrderFrom,
    /// `orderGeometry`.
    OrderGeometry,
    /// `datasetId`.
    DatasetId,
    /// `entityMap`.
    EntityMap,
    /// `entityMapLifetime`.
    EntityMapLifetime,
    /// `expandValues`.
    ExpandValues,
    /// `jsonKeys`.
    JsonKeys,
    /// `splitEntities`.
    SplitEntities,
    /// `timerel`.
    Timerel,
    /// `timeAt`.
    TimeAt,
    /// `endTimeAt`.
    EndTimeAt,
    /// `timeproperty`.
    Timeproperty,
    /// `lastN`.
    LastN,
    /// `aggrMethods`.
    AggrMethods,
    /// `aggrPeriodDuration`.
    AggrPeriodDuration,
    /// `drop`, of Purge Entities.
    Drop,
    /// `keep`, of Purge Entities.
    Keep,
    /// `deleteAll`, of Delete Attribute and Delete Attribute from Temporal Evolution.
    DeleteAll,
    /// `observedAt`, of Merge Entity.
    ObservedAt,
    /// `details`, of Retrieve Available Entity Types, Retrieve Available Attributes, List @contexts and Serve
    /// @context.
    Details,
    /// `kind`, of List @contexts.
    Kind,
    /// `reload`, of Delete and Reload @context.
    Reload,
}
