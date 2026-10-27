//! The query string of the discovery requests in typed form (NGSI-LD Tables 6.25.3.1-1 and 6.27.3.1-1), as a broker
//! reads a request for the entity types and attributes it holds.

use crate::{
    http::{
        QueryParameter,
        operation::{ParameterError, parameters::Details, received_parameters::ReceivedQuery},
    },
    query::Locality,
};

/// The parameters of a discovery request: Tables 6.25.3.1-1 and 6.27.3.1-1 list `details`, and clause 6.3.18 adds
/// `local`.
const DISCOVERY: [QueryParameter; 2] = [QueryParameter::Details, QueryParameter::Local];

/// The query string of a request for the entity types or attributes a broker holds, read in typed form.
///
/// The requests are `GET /types`, `GET /types/{type}`, `GET /attributes` and `GET /attributes/{attrId}` (NGSI-LD
/// clauses 6.25 to 6.28), which take the same parameters: `details` asks for the detailed representation (Tables
/// 6.25.3.1-1 and 6.27.3.1-1), and `local` limits the answer to the broker's own data (clause 6.3.18).
///
/// Each parameter is read with its grammar, and a parameter the tables do not define is reported.
///
/// The struct is `#[non_exhaustive]`: later versions of the specification add parameters.
///
/// ```
/// use ngsi_ld::{
///     http::operation::{Details, DiscoveryParameters},
///     query::Locality,
/// };
///
/// let received = DiscoveryParameters::parse("details=true&local=true")?;
/// assert_eq!(received.details, Details::Detailed);
/// assert_eq!(received.locality, Locality::Local);
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct DiscoveryParameters {
    /// `details`: the summary, or the detailed representation.
    pub details: Details,
    /// `local` (clause 6.3.18).
    pub locality: Locality,
}

impl DiscoveryParameters {
    /// Reads `query`, the part of the request URI after its `?`.
    ///
    /// # Errors
    ///
    /// Returns [`ParameterError`] naming the parameter at fault: one the discovery tables do not define, one given
    /// twice, or a boolean that is neither `true` nor `false`.
    pub fn parse(query: &str) -> Result<DiscoveryParameters, ParameterError> {
        let received = ReceivedQuery::read(query, &[&DISCOVERY])?;
        Ok(DiscoveryParameters {
            details: received.details()?,
            locality: received.locality()?,
        })
    }
}
