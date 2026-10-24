//! The `Query` body of the POST binding of Query Entities (NGSI-LD clauses 5.2.23 and 6.23.3.1).

use crate::{
    http::{RequestError, operation::QueryEntities},
    snapshot::query_document::writer::query_object,
};
use jstrict::Object;

impl<F, C> QueryEntities<'_, F, C> {
    /// The `Query` body (Table 5.2.23-1), written by the writer of the `Query` document. Its members carry what the
    /// GET binding sends as `type`, `id`, `idPattern`, `q`, the geoquery parameters (as the `GeoQuery` object of
    /// clause 5.2.13), `scopeQ`, `csf`, `pick`, `omit`, `expandValues`, `jsonKeys`, `datasetId`, `lang`, `join`,
    /// `joinLevel`, `containedBy`, the ordering parameters, `splitEntities`, `entityMap` and `entityMapLifetime`.
    pub(super) fn query_body(&self) -> Result<Object, RequestError> {
        let members = self.selection().query_members()?;
        Ok(query_object(self.assembly().query_members(self.linked_entities().query_members(members))))
    }
}
