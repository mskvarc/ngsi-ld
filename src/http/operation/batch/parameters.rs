//! The query strings of Batch Entity Upsert and Update in typed form (NGSI-LD clauses 6.15.3.1 and 6.16.3.1), as a
//! context source reads a batch a broker forwards to it (clauses 5.6.8.4 and 5.6.9.4).

use crate::{
    http::{
        QueryParameter,
        operation::{
            Overwrite,
            ParameterError,
            UpsertMode,
            received_parameters::{ReceivedQuery, write_options},
        },
    },
    query::Locality,
};

/// The parameters Batch Entity Upsert and Update accept: `options` (clauses 6.15.3.1 and 6.16.3.1) and `local`
/// (clause 6.3.18).
const ACCEPTED: [QueryParameter; 2] = [QueryParameter::Options, QueryParameter::Local];

/// The query string of a Batch Entity Upsert request, read in typed form.
///
/// The request is `POST /entityOperations/upsert` (NGSI-LD clause 6.15.3.1).
///
/// The struct is `#[non_exhaustive]`: later versions of the specification add parameters.
///
/// ```
/// use ngsi_ld::http::operation::{BatchUpsertParameters, UpsertMode};
///
/// assert_eq!(
///     BatchUpsertParameters::parse("options=update")?.mode,
///     UpsertMode::Update
/// );
/// assert_eq!(
///     BatchUpsertParameters::parse("")?.mode,
///     UpsertMode::Unspecified
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct BatchUpsertParameters {
    /// `options=update` or `options=replace`; without `options`, the broker's default, which replaces.
    pub mode: UpsertMode,
    /// `local` (clause 6.3.18).
    pub locality: Locality,
}

impl BatchUpsertParameters {
    /// Reads `query`, the part of the request URI after its `?`.
    ///
    /// # Errors
    ///
    /// Returns [`ParameterError`] naming the parameter at fault: one clause 6.15.3.1 does not define, one given
    /// twice, an `options` token other than `replace` and `update`, both of them
    /// ([`ParameterError::UpsertModeTwice`]), or a `local` that is not a boolean.
    pub fn parse(query: &str) -> Result<BatchUpsertParameters, ParameterError> {
        let received = ReceivedQuery::read(query, &[&ACCEPTED])?;
        Ok(BatchUpsertParameters {
            mode: write_options::upsert_mode(&received)?,
            locality: received.locality()?,
        })
    }
}

/// The query string of a Batch Entity Update request, read in typed form.
///
/// The request is `POST /entityOperations/update` (NGSI-LD clause 6.16.3.1).
///
/// The struct is `#[non_exhaustive]`: later versions of the specification add parameters.
///
/// ```
/// use ngsi_ld::http::operation::{BatchUpdateParameters, Overwrite};
///
/// assert_eq!(
///     BatchUpdateParameters::parse("options=noOverwrite")?.overwrite,
///     Overwrite::Keep
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct BatchUpdateParameters {
    /// `options=noOverwrite`, or the default replacement of existing attributes.
    pub overwrite: Overwrite,
    /// `local` (clause 6.3.18).
    pub locality: Locality,
}

impl BatchUpdateParameters {
    /// Reads `query`, the part of the request URI after its `?`.
    ///
    /// # Errors
    ///
    /// Returns [`ParameterError`] naming the parameter at fault: one clause 6.16.3.1 does not define, one given
    /// twice, an `options` token other than `noOverwrite`, or a `local` that is not a boolean.
    pub fn parse(query: &str) -> Result<BatchUpdateParameters, ParameterError> {
        let received = ReceivedQuery::read(query, &[&ACCEPTED])?;
        Ok(BatchUpdateParameters {
            overwrite: write_options::overwrite(&received)?,
            locality: received.locality()?,
        })
    }
}
