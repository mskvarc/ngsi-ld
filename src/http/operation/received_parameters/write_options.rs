//! The `options` of the write operations a broker forwards, read in typed form: `noOverwrite` of Append Attributes
//! and Batch Entity Update (Table 6.6.3.1-1, clause 6.16.3.1), and `replace` or `update` of Batch Entity Upsert
//! (clause 6.15.3.1).

use crate::http::{
    QueryParameter,
    operation::{Overwrite, ParameterError, UpsertMode, received_parameters::ReceivedQuery},
};

/// The `options` token of Append Attributes and Batch Entity Update; Table 6.6.3.1-1 and clause 6.16.3.1 define one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::EnumString)]
enum OverwriteToken {
    /// `noOverwrite`.
    #[strum(serialize = "noOverwrite")]
    NoOverwrite,
}

/// The `options` tokens of Batch Entity Upsert (clause 6.15.3.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::EnumString)]
enum UpsertToken {
    /// `replace`, the default mode.
    #[strum(serialize = "replace")]
    Replace,
    /// `update`.
    #[strum(serialize = "update")]
    Update,
}

/// Reads the `options` of Append Attributes or Batch Entity Update: [`Overwrite::Keep`] when they name
/// `noOverwrite`, [`Overwrite::Replace`] when they are absent.
pub(crate) fn overwrite(received: &ReceivedQuery<'_>) -> Result<Overwrite, ParameterError> {
    let tokens = received.tokens::<OverwriteToken>(QueryParameter::Options)?;
    Ok(if tokens.contains(&OverwriteToken::NoOverwrite) {
        Overwrite::Keep
    } else {
        Overwrite::Replace
    })
}

/// Reads the `options` of Batch Entity Upsert: the mode they name, or [`UpsertMode::Replace`], the default, when
/// they name none.
pub(crate) fn upsert_mode(received: &ReceivedQuery<'_>) -> Result<UpsertMode, ParameterError> {
    let tokens = received.tokens::<UpsertToken>(QueryParameter::Options)?;
    match (tokens.contains(&UpsertToken::Replace), tokens.contains(&UpsertToken::Update)) {
        (true, true) => Err(ParameterError::UpsertModeTwice),
        (false, true) => Ok(UpsertMode::Update),
        (true | false, false) => Ok(UpsertMode::Replace),
    }
}
