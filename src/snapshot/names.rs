//! The names a snapshot holds: those of its queries (NGSI-LD clauses 5.2.23, 5.2.41 and 5.5.7).

use crate::{
    model::{
        DocumentError,
        name::mapping::{MapNames, NameMapping, member},
    },
    snapshot::{QueryDocument, QueryKind, QueryMember, Snapshot, SnapshotDirection, SnapshotNotification, TemporalQueryMembers},
};

impl<D: SnapshotDirection> MapNames for Snapshot<D> {
    fn map_names<M: NameMapping>(self, mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        Ok(Self {
            queries: member(self.queries, "snapshotQueries", mapping)?,
            temporal_queries: member(self.temporal_queries, "snapshotTemporalQueries", mapping)?,
            ..self
        })
    }
}

impl<K: QueryKind> MapNames for QueryDocument<K>
where
    K::Members: MapNames,
    K::AttrsName: MapNames,
{
    fn map_names<M: NameMapping>(self, mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        Ok(Self {
            entities: member(self.entities, QueryMember::Entities.into(), mapping)?,
            q: member(self.q, QueryMember::Q.into(), mapping)?,
            geo_q: member(self.geo_q, QueryMember::GeoQ.into(), mapping)?,
            kind_members: self.kind_members.map_names(mapping)?,
            attrs: member(self.attrs, QueryMember::Attrs.into(), mapping)?,
            pick: member(self.pick, QueryMember::Pick.into(), mapping)?,
            omit: member(self.omit, QueryMember::Omit.into(), mapping)?,
            csf: member(self.csf, QueryMember::Csf.into(), mapping)?,
            expand_values: member(self.expand_values, QueryMember::ExpandValues.into(), mapping)?,
            json_keys: member(self.json_keys, QueryMember::JsonKeys.into(), mapping)?,
            ordering: member(self.ordering, QueryMember::Ordering.into(), mapping)?,
            ..self
        })
    }
}

/// A `temporalQ` names a system temporal property and `aggrParams` aggregation methods, never an attribute
/// (clauses 4.11 and 5.2.44).
impl MapNames for TemporalQueryMembers {
    fn map_names<M: NameMapping>(self, _mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        Ok(self)
    }
}

/// A snapshot notification reports results, never a name (clause 5.3.4).
impl MapNames for SnapshotNotification {
    fn map_names<M: NameMapping>(self, _mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        Ok(self)
    }
}

carries_context!(impl<D: SnapshotDirection> for Snapshot<D> => context);

carries_context!(
    /// A query document carries no `@context` of its own.
    impl<K: QueryKind> for QueryDocument<K>
);
