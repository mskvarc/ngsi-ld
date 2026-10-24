//! How far an operation or a subscription reaches: the broker's own data or the context sources registered with
//! it (NGSI-LD clauses 5.5.13 and 6.3.18), whether those sources may each hold part of one entity, and whether a
//! distributed query returns the location of the entity map it used (clauses 4.3.6.7 and 4.5.25).

/// Whether an operation or a subscription is limited to the broker's own data.
///
/// It is the `local` parameter (NGSI-LD clauses 5.5.13 and 6.3.18) and the `localOnly` member of a subscription (Table
/// 5.2.12-1).
///
/// ```
/// use ngsi_ld::query::Locality;
///
/// assert_eq!(Locality::default(), Locality::Unspecified);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Locality {
    /// Not stated: the default, which may include registered context sources.
    #[default]
    Unspecified,
    /// `true`: only the broker's own data, no distributed operation.
    Local,
    /// `false`: matching context source registrations are considered.
    Distributed,
}

flag_choice!(Locality, Local, Distributed);

/// Whether the matching context sources may each hold part of one entity.
///
/// It is the `splitEntities` parameter (NGSI-LD clause 6.4.3.2) and the `splitEntities` member of a subscription (Table
/// 5.2.12-1). It decides whether filters are applied before or after the parts are merged.
///
/// ```
/// use ngsi_ld::query::EntitySplitting;
///
/// assert_eq!(EntitySplitting::default(), EntitySplitting::Unspecified);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum EntitySplitting {
    /// Not stated: the broker's configured default.
    #[default]
    Unspecified,
    /// `true`: an entity may be distributed over several brokers and context sources, so filters apply to the
    /// merged entity.
    Split,
    /// `false`: every broker and context source holds whole entities, so filters apply locally.
    Whole,
}

flag_choice!(EntitySplitting, Split, Whole);

/// Whether a query asks for the location of its entity map.
///
/// It is the `entityMap` member of NGSI-LD Table 5.2.23-1 (clause 4.5.25). An explicit `false` is kept apart from an
/// absent member so that it survives a round trip.
///
/// ```
/// use ngsi_ld::query::EntityMapReturn;
///
/// assert_eq!(EntityMapReturn::default(), EntityMapReturn::Unspecified);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum EntityMapReturn {
    /// `entityMap` is absent.
    #[default]
    Unspecified,
    /// `entityMap` is `true`: the location of the entity map is returned.
    Returned,
    /// `entityMap` is `false`.
    NotReturned,
}

flag_choice!(EntityMapReturn, Returned, NotReturned);
