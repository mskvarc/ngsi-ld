//! The query string of a Delete Attribute request in typed form (NGSI-LD clause 6.7.3.2, Table 6.7.3.2-1), as a
//! context source reads a deletion a broker forwards to it (clause 5.6.5.4).

use crate::{
    http::{
        QueryParameter,
        operation::{InstanceSelection, ParameterError, received_parameters::ReceivedQuery},
    },
    model::Uri,
    query::{Locality, type_selection::TypeSelection},
};

/// The parameters Delete Attribute accepts: Table 6.7.3.2-1, with `local` (clause 6.3.18).
const ACCEPTED: [QueryParameter; 4] = [
    QueryParameter::DatasetId,
    QueryParameter::DeleteAll,
    QueryParameter::Type,
    QueryParameter::Local,
];

/// The `deleteAll` flag of Delete Attribute.
///
/// NGSI-LD Table 6.7.3.2-1.
///
/// The enum is exhaustive: the flag is given as `true`, given as `false`, or absent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum DeleteAll {
    /// No `deleteAll`.
    #[default]
    Unspecified,
    /// `deleteAll=true`: every instance is deleted.
    Every,
    /// `deleteAll=false`: only the instance `datasetId` names, or the default instance, is deleted.
    Selected,
}

/// The query string of a Delete Attribute request, read in typed form.
///
/// The request is `DELETE /entities/{entityId}/attrs/{attrId}` (NGSI-LD Table 6.7.3.2-1).
/// [`instances`](Self::instances) says which instances it removes.
///
/// The struct is `#[non_exhaustive]`: later versions of the specification add parameters.
///
/// ```
/// use ngsi_ld::http::operation::{DeleteAttributeParameters, InstanceSelection};
///
/// assert_eq!(
///     DeleteAttributeParameters::parse("deleteAll=true")?.instances(),
///     InstanceSelection::All
/// );
/// assert_eq!(
///     DeleteAttributeParameters::parse("")?.instances(),
///     InstanceSelection::Default
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct DeleteAttributeParameters {
    /// `datasetId`: the instance to delete.
    pub dataset_id: Option<Uri>,
    /// `deleteAll`.
    pub delete_all: DeleteAll,
    /// `type` (clause 4.17).
    pub entity_type: Option<TypeSelection>,
    /// `local` (clause 6.3.18).
    pub locality: Locality,
}

impl DeleteAttributeParameters {
    /// Reads `query`, the part of the request URI after its `?`.
    ///
    /// # Errors
    ///
    /// Returns [`ParameterError`] naming the parameter at fault: one Table 6.7.3.2-1 does not define, one given
    /// twice, a `datasetId` that is not a URI, or a `deleteAll` or `local` that is not a boolean.
    pub fn parse(query: &str) -> Result<DeleteAttributeParameters, ParameterError> {
        let received = ReceivedQuery::read(query, &[&ACCEPTED])?;
        Ok(DeleteAttributeParameters {
            dataset_id: received.parsed(QueryParameter::DatasetId, ParameterError::uri)?,
            delete_all: match received.flag(QueryParameter::DeleteAll)? {
                Some(true) => DeleteAll::Every,
                Some(false) => DeleteAll::Selected,
                None => DeleteAll::Unspecified,
            },
            entity_type: received.parsed(QueryParameter::Type, ParameterError::syntax)?,
            locality: received.locality()?,
        })
    }

    /// The instances the request removes (Table 6.7.3.2-1): every instance with `deleteAll=true`, otherwise the
    /// instance `datasetId` names, or the default instance when neither is given.
    #[must_use]
    pub const fn instances(&self) -> InstanceSelection<'_> {
        match (self.delete_all, &self.dataset_id) {
            (DeleteAll::Every, _) => InstanceSelection::All,
            (DeleteAll::Unspecified | DeleteAll::Selected, Some(dataset_id)) => InstanceSelection::Dataset(dataset_id),
            (DeleteAll::Unspecified | DeleteAll::Selected, None) => InstanceSelection::Default,
        }
    }
}
