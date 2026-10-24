//! The builder of a snapshot a client creates (NGSI-LD clauses 5.2.41 and 5.16.1).

use crate::{
    model::{BuildError, Context, Duration, Input, KeyValuePair, UrlDraft},
    snapshot::{EntityQuery, Problem, QueryDocument, Snapshot, SnapshotId, TemporalEvolutionQuery, reader::creation_problems, settings::Settings},
};
use bon::bon;

#[bon]
impl Snapshot<Input> {
    /// Starts the builder of a snapshot to create (clause 5.16.1). It needs `snapshotQueries` or
    /// `snapshotTemporalQueries`, set with `query`, `queries`, `temporal_query` or `temporal_queries`; every other
    /// member is optional, and every problem is reported by `build()`.
    ///
    /// # Errors
    ///
    /// `build()` returns [`BuildError`] listing a snapshot with neither list of queries, a priority outside 1 to
    /// 10, and an endpoint that is not a URL (Table 5.2.41-1).
    #[builder]
    pub fn new(
        #[builder(field)] queries: Option<Vec<QueryDocument<EntityQuery>>>,
        #[builder(field)] temporal_queries: Option<Vec<QueryDocument<TemporalEvolutionQuery>>>,
        #[builder(into)] id: Option<SnapshotId>,
        lifetime: Option<Duration>,
        priority: Option<u8>,
        #[builder(into)] endpoint: Option<UrlDraft>,
        #[builder(with = |pairs: impl IntoIterator<Item = KeyValuePair>| pairs.into_iter().collect())] receiver_info: Option<Vec<KeyValuePair>>,
        context: Option<Context>,
    ) -> Result<Self, BuildError<Problem>> {
        let mut problems = Vec::new();
        let snapshot = Self {
            id,
            queries,
            temporal_queries,
            settings: Settings::from_builder(lifetime, priority, endpoint, receiver_info, &mut problems),
            output: (),
            context,
        };
        problems.extend(creation_problems(&snapshot));
        BuildError::finish(problems, snapshot)
    }
}

impl<S: snapshot_builder::State> SnapshotBuilder<S> {
    /// Adds a query of entities (`snapshotQueries`); the queries keep the order they are added in, which the
    /// details of their execution follow (Table 5.2.41-2).
    pub fn query(mut self, query: QueryDocument<EntityQuery>) -> Self {
        self.queries.get_or_insert_default().push(query);
        self
    }

    /// Adds queries of entities. The snapshot holds `snapshotQueries` even when `queries` is empty, which Table
    /// 5.2.41-1 allows.
    pub fn queries(mut self, queries: impl IntoIterator<Item = QueryDocument<EntityQuery>>) -> Self {
        self.queries.get_or_insert_default().extend(queries);
        self
    }

    /// Adds a query of the temporal evolution of entities (`snapshotTemporalQueries`).
    pub fn temporal_query(mut self, query: QueryDocument<TemporalEvolutionQuery>) -> Self {
        self.temporal_queries.get_or_insert_default().push(query);
        self
    }

    /// Adds queries of the temporal evolution of entities. The snapshot holds `snapshotTemporalQueries` even when
    /// `queries` is empty.
    pub fn temporal_queries(mut self, queries: impl IntoIterator<Item = QueryDocument<TemporalEvolutionQuery>>) -> Self {
        self.temporal_queries.get_or_insert_default().extend(queries);
        self
    }
}
