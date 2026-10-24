//! The two kinds of query a snapshot holds (NGSI-LD Table 5.2.41-1): queries of entities, without `temporalQ`, in
//! `snapshotQueries`, and queries of the temporal evolution of entities, each with a `temporalQ`, in
//! `snapshotTemporalQueries`.

use crate::{
    model::{
        AttributeName,
        AttributeNameDraft,
        DocumentError,
        Duration,
        NameError,
        draft::Draft,
        evolution::{TemporalAttributeName, TemporalAttributeNameDraft},
        reader::{duration, object, once, required, string, unexpected, within},
    },
    query::temporal_query::{AggregationMethods, TemporalQuery},
    seal::Seal,
    snapshot::{AggregationMember, QueryMember},
    subscription::{
        ReadErrorKind,
        reader::{error, temporal_query},
        writer::push,
    },
};
use jstrict::{Object, Value as Json, object::Entry};
use std::{
    error::Error,
    fmt::{Debug, Display},
    hash::Hash,
    str::FromStr,
};

/// The crate-only side of the query kinds: the sealing supertrait, and the members each kind gathers while it
/// is read.
pub(crate) mod sealed {
    use crate::{
        model::DocumentError,
        query::temporal_query::TemporalQuery,
        seal::Seal,
        snapshot::{AggregationParams, EntityQuery, QueryMember, TemporalEvolutionQuery},
        subscription::ReadErrorKind,
    };
    use jstrict::Value as Json;
    use std::{error::Error, fmt::Debug};

    /// Keeps the kinds of query closed to the two of Table 5.2.41-1.
    pub trait Sealed {}

    impl Sealed for EntityQuery {}
    impl Sealed for TemporalEvolutionQuery {}

    /// The members of a temporal query gathered so far.
    #[derive(Debug, Default)]
    pub struct PendingTemporalMembers {
        /// `temporalQ`, mandatory.
        pub(crate) temporal_q: Option<TemporalQuery>,
        /// `aggrParams`.
        pub(crate) aggr_params: Option<AggregationParams>,
    }

    /// The members only one kind of query has: how they are gathered, read and written.
    pub trait KindMembers: Sized + Debug + Clone + PartialEq + Eq + Send + Sync {
        /// What a builder or a reader gathers before the members are complete.
        type Pending: Default;

        /// Reads `member` into `pending` when the kind owns it, or gives the value back.
        fn read<P: Error>(seal: Seal, pending: &mut Self::Pending, member: QueryMember, json: Json) -> Result<Option<Json>, DocumentError<ReadErrorKind<P>>>;

        /// The members gathered, or `None` when a mandatory one is missing: the `temporalQ` of a temporal query.
        fn complete(seal: Seal, pending: Self::Pending) -> Option<Self>;

        /// `temporalQ`, when the kind has it.
        fn temporal_q(&self, seal: Seal) -> Option<&TemporalQuery>;

        /// `aggrParams`, when the kind has it and it is given.
        fn aggregation(&self, seal: Seal) -> Option<&AggregationParams>;
    }
}

/// A kind of query a snapshot holds: [`EntityQuery`] or [`TemporalEvolutionQuery`].
///
/// The kind is a type parameter of [`QueryDocument`](crate::snapshot::QueryDocument), so a member
/// of the other kind has no setter and no getter: `temporalQ`, and `aggrParams`, which clause 5.2.44 defines for
/// "aggregation methods in temporal requests", exist on [`TemporalEvolutionQuery`] documents only, and only their
/// `attrs` may name `scope`, which the temporal representation reifies (clauses 4.18 and C.5.16.2).
///
/// The trait is sealed: Table 5.2.41-1 defines these two kinds.
pub trait QueryKind: sealed::Sealed + Debug + Clone + Copy + PartialEq + Eq + Hash + Default + Send + Sync {
    /// The members only this kind has.
    type Members: sealed::KindMembers;
    /// What `attrs` names: an attribute, or, in a temporal query, an attribute or the entity's `scope`.
    type AttrsName: Debug + Clone + PartialEq + Eq + Hash + Display + FromStr<Err = NameError> + Send + Sync;
    /// What a builder takes for a name of `attrs`.
    type AttrsDraft: Draft<Value = Self::AttrsName, Error = NameError> + Debug;
}

/// A query of entities (clause 5.7.2), the kind a snapshot holds in `snapshotQueries`: it has no `temporalQ`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct EntityQuery;

impl QueryKind for EntityQuery {
    type AttrsDraft = AttributeNameDraft;
    type AttrsName = AttributeName;
    type Members = ();
}

/// A query of the temporal evolution of entities (clause 5.7.4), the kind a snapshot holds in
/// `snapshotTemporalQueries`: its `temporalQ` is mandatory, and it may carry `aggrParams`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct TemporalEvolutionQuery;

impl QueryKind for TemporalEvolutionQuery {
    type AttrsDraft = TemporalAttributeNameDraft;
    type AttrsName = TemporalAttributeName;
    type Members = TemporalQueryMembers;
}

impl sealed::KindMembers for () {
    type Pending = ();

    fn read<P: Error>(_: Seal, (): &mut (), _: QueryMember, json: Json) -> Result<Option<Json>, DocumentError<ReadErrorKind<P>>> {
        Ok(Some(json))
    }

    fn complete(_: Seal, (): ()) -> Option<Self> {
        Some(())
    }

    fn temporal_q(&self, _: Seal) -> Option<&TemporalQuery> {
        None
    }

    fn aggregation(&self, _: Seal) -> Option<&AggregationParams> {
        None
    }
}

/// The aggregation of a temporal query (the `AggregationParams` of clause 5.2.44): the methods applied, and the
/// duration of each period they are applied over.
///
/// ```
/// use ngsi_ld::{model::evolution::AggregationMethod, snapshot::AggregationParams};
///
/// let params = AggregationParams::new("min,max".parse()?).with_period("PT1H".parse()?);
/// assert_eq!(
///     params.methods().as_slice(),
///     [AggregationMethod::Min, AggregationMethod::Max]
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AggregationParams {
    /// `aggrMethods`.
    methods: AggregationMethods,
    /// `aggrPeriodDuration`; absent, the period spans the whole time range of the temporal query.
    period: Option<Duration>,
}

impl AggregationParams {
    /// The aggregation applying `methods` over the whole time range of the temporal query.
    #[must_use]
    pub const fn new(methods: AggregationMethods) -> AggregationParams {
        AggregationParams { methods, period: None }
    }

    /// The same aggregation, applied over periods of `period` (`aggrPeriodDuration`).
    #[must_use]
    pub fn with_period(self, period: Duration) -> Self {
        Self { period: Some(period), ..self }
    }

    /// Returns the methods (`aggrMethods`).
    #[must_use]
    pub const fn methods(&self) -> &AggregationMethods {
        &self.methods
    }

    /// Returns the period (`aggrPeriodDuration`).
    #[must_use]
    pub const fn period(&self) -> Option<&Duration> {
        self.period.as_ref()
    }

    /// Reads an `AggregationParams` object (Table 5.2.44-1).
    fn from_json<P: Error>(json: Json) -> Result<Self, DocumentError<ReadErrorKind<P>>> {
        let mut methods = None;
        let mut period = None;
        for Entry { key, value } in object(json)? {
            let Ok(member) = key.as_str().parse::<AggregationMember>() else {
                return Err(unexpected(key.as_str()));
            };
            let key = member.into();
            match member {
                AggregationMember::AggrMethods => {
                    let methods_read = string(value, key)?
                        .parse()
                        .map_err(|source| error(ReadErrorKind::Aggregation { source }).within(key))?;
                    once(&mut methods, key, methods_read)?;
                }
                AggregationMember::AggrPeriodDuration => once(&mut period, key, within(key, duration(value, key))?)?,
            }
        }
        Ok(Self {
            methods: required(methods, AggregationMember::AggrMethods.into())?,
            period,
        })
    }

    /// Writes the `AggregationParams` object: `aggrMethods` as one comma-separated string, the type Table
    /// 5.2.44-1 gives it, then `aggrPeriodDuration`.
    pub(crate) fn to_json(&self) -> Json {
        let mut object = Object::new();
        object.push(<&str>::from(AggregationMember::AggrMethods).into(), Json::from(self.methods.to_string()));
        push(
            &mut object,
            AggregationMember::AggrPeriodDuration.into(),
            self.period.as_ref().map(|period| Json::from(period.to_string())),
        );
        Json::Object(object)
    }
}

/// The members of a query of the temporal evolution of entities: its `temporalQ` (clause 5.2.21) and its
/// `aggrParams` (clause 5.2.44).
///
/// The type exists only on [`TemporalEvolutionQuery`] documents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemporalQueryMembers {
    /// `temporalQ`.
    pub(crate) temporal_q: TemporalQuery,
    /// `aggrParams`.
    pub(crate) aggr_params: Option<AggregationParams>,
}

impl sealed::KindMembers for TemporalQueryMembers {
    type Pending = sealed::PendingTemporalMembers;

    fn read<P: Error>(
        _: Seal,
        pending: &mut sealed::PendingTemporalMembers,
        member: QueryMember,
        json: Json,
    ) -> Result<Option<Json>, DocumentError<ReadErrorKind<P>>> {
        let key = member.into();
        if member == QueryMember::TemporalQ {
            once(&mut pending.temporal_q, key, within(key, temporal_query(json))?)?;
        } else if member == QueryMember::AggrParams {
            once(&mut pending.aggr_params, key, within(key, AggregationParams::from_json(json))?)?;
        } else {
            return Ok(Some(json));
        }
        Ok(None)
    }

    fn complete(_: Seal, pending: sealed::PendingTemporalMembers) -> Option<Self> {
        Some(Self {
            temporal_q: pending.temporal_q?,
            aggr_params: pending.aggr_params,
        })
    }

    fn temporal_q(&self, _: Seal) -> Option<&TemporalQuery> {
        Some(&self.temporal_q)
    }

    fn aggregation(&self, _: Seal) -> Option<&AggregationParams> {
        self.aggr_params.as_ref()
    }
}
