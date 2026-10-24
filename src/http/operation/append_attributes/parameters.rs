//! The query string of an Append Attributes request in typed form (NGSI-LD clause 6.6.3.1, Table 6.6.3.1-1), as a
//! context source reads an append a broker forwards to it (clause 5.6.3.4).

use crate::{
    http::{
        QueryParameter,
        operation::{
            Overwrite,
            ParameterError,
            received_parameters::{ReceivedQuery, write_options},
        },
    },
    query::{Locality, type_selection::TypeSelection},
};

/// The parameters Append Attributes accepts: Table 6.6.3.1-1, with `local` (clause 6.3.18).
const ACCEPTED: [QueryParameter; 3] = [QueryParameter::Options, QueryParameter::Type, QueryParameter::Local];

/// The query string of an Append Attributes request, read in typed form.
///
/// The request is `POST /entities/{entityId}/attrs` (NGSI-LD Table 6.6.3.1-1). A context source receives this query
/// string when a broker forwards an append to it (clause 5.6.3.4).
///
/// The struct is `#[non_exhaustive]`: later versions of the specification add parameters.
///
/// ```
/// use ngsi_ld::http::operation::{AppendAttributesParameters, Overwrite};
///
/// let received = AppendAttributesParameters::parse("options=noOverwrite&type=Vehicle")?;
/// assert_eq!(received.overwrite, Overwrite::Keep);
/// assert_eq!(
///     received
///         .entity_type
///         .map(|selection| selection.to_string())
///         .as_deref(),
///     Some("Vehicle")
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct AppendAttributesParameters {
    /// `options=noOverwrite`, or the default replacement of existing attributes.
    pub overwrite: Overwrite,
    /// `type` (clause 4.17).
    pub entity_type: Option<TypeSelection>,
    /// `local` (clause 6.3.18).
    pub locality: Locality,
}

impl AppendAttributesParameters {
    /// Reads `query`, the part of the request URI after its `?`.
    ///
    /// # Errors
    ///
    /// Returns [`ParameterError`] naming the parameter at fault: one Table 6.6.3.1-1 does not define, one given
    /// twice, an `options` token other than `noOverwrite`, or a value that does not follow its grammar.
    pub fn parse(query: &str) -> Result<AppendAttributesParameters, ParameterError> {
        let received = ReceivedQuery::read(query, &[&ACCEPTED])?;
        Ok(AppendAttributesParameters {
            overwrite: write_options::overwrite(&received)?,
            entity_type: received.parsed(QueryParameter::Type, ParameterError::syntax)?,
            locality: received.locality()?,
        })
    }
}
