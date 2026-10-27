//! The query string of List @contexts in typed form (NGSI-LD clause 6.29.3.2), as a broker reads the request.

use crate::http::{
    QueryParameter,
    operation::{ContextKindFilter, ParameterError, parameters::Details, received_parameters::ReceivedQuery},
};

/// The parameters of List @contexts (Table 6.29.3.2-1).
const LIST: [QueryParameter; 2] = [QueryParameter::Details, QueryParameter::Kind];

/// The query string of a List @contexts request, read in typed form.
///
/// The request is `GET /jsonldContexts` (NGSI-LD Table 6.29.3.2-1): `details` asks for the metadata of each context
/// instead of its URL, and `kind` limits the list to the contexts of one kind.
///
/// Each parameter is read with its grammar, and a parameter the table does not define is reported.
///
/// The struct is `#[non_exhaustive]`: later versions of the specification add parameters.
///
/// ```
/// use ngsi_ld::http::operation::{ContextKindFilter, Details, ListContextsParameters};
///
/// let received = ListContextsParameters::parse("details=true&kind=Hosted")?;
/// assert_eq!(received.details, Details::Detailed);
/// assert_eq!(received.kind, Some(ContextKindFilter::Hosted));
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ListContextsParameters {
    /// `details`: the URLs, or the metadata.
    pub details: Details,
    /// `kind`: the kind of contexts listed, every kind when absent.
    pub kind: Option<ContextKindFilter>,
}

impl ListContextsParameters {
    /// Reads `query`, the part of the request URI after its `?`.
    ///
    /// # Errors
    ///
    /// Returns [`ParameterError`] naming the parameter at fault: one Table 6.29.3.2-1 does not define, one given
    /// twice, a `details` that is neither `true` nor `false`, and a `kind` other than `Cached`, `Hosted` and
    /// `ImplicitlyCreated`.
    pub fn parse(query: &str) -> Result<ListContextsParameters, ParameterError> {
        let received = ReceivedQuery::read(query, &[&LIST])?;
        Ok(ListContextsParameters {
            details: received.details()?,
            kind: received.token(QueryParameter::Kind)?,
        })
    }
}
