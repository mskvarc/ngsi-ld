//! What a request is aimed at besides the operation's own parameters.
//!
//! The type is public in a module private to the crate: the sealed trait that builds requests takes it, and that
//! trait is reachable from the public `Operation`, while no caller outside the crate can name or build a scope.

use crate::{model::Tenant, query::Locality, snapshot::SnapshotId};

/// What a request is aimed at besides the operation's own parameters: the tenant (`NGSILD-Tenant`, clause 6.3.14),
/// the snapshot (`NGSILD-Snapshot`, clauses 5.5.15 and 6.3.22) and, for an operation whose own table does not list
/// it, the `local` parameter of clause 6.3.18. The pipeline applies all three, so a wrapper of an operation widens
/// the scope instead of changing the request the operation built.
#[derive(Debug, Clone, Copy, Default)]
pub struct RequestScope<'a> {
    /// The tenant, `None` for the default tenant.
    tenant: Option<&'a Tenant>,
    /// The snapshot the request is aimed at.
    snapshot: Option<&'a SnapshotId>,
    /// The `local` parameter; [`Locality::Unspecified`] adds none.
    locality: Locality,
}

impl<'a> RequestScope<'a> {
    /// A request on `tenant`, or on the default tenant.
    pub(crate) const fn on_tenant(tenant: Option<&'a Tenant>) -> RequestScope<'a> {
        RequestScope {
            tenant,
            snapshot: None,
            locality: Locality::Unspecified,
        }
    }

    /// The scope aimed at `snapshot` instead, when there is one.
    pub(crate) const fn on_snapshot(self, snapshot: Option<&'a SnapshotId>) -> RequestScope<'a> {
        match snapshot {
            Some(snapshot) => RequestScope {
                snapshot: Some(snapshot),
                ..self
            },
            None => self,
        }
    }

    /// The scope limited as `locality` says.
    pub(crate) const fn limited(self, locality: Locality) -> RequestScope<'a> {
        RequestScope { locality, ..self }
    }

    /// The tenant, `None` for the default tenant.
    pub(crate) const fn tenant(self) -> Option<&'a Tenant> {
        self.tenant
    }

    /// The snapshot the request is aimed at.
    pub(crate) const fn snapshot(self) -> Option<&'a SnapshotId> {
        self.snapshot
    }

    /// The `local` parameter the pipeline adds.
    pub(crate) const fn locality(self) -> Locality {
        self.locality
    }
}
