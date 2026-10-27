//! Reading the `format` and `options` parameters of a received request (Tables 6.3.7-1 and 6.3.12-1, clause
//! 6.3.11).

use crate::{
    http::{
        QueryParameter,
        operation::{
            NormalizedSpelling,
            ParameterError,
            RepresentationParameter,
            RepresentationSpelling,
            RepresentationToken,
            SystemAttributes,
            received_parameters::ReceivedQuery,
            representation_spelling::RepresentationFamily,
        },
    },
    model::SimplifiedSpelling,
};
use std::str::FromStr;

/// A keyword of the `options` parameter of a request that asks for a representation: `sysAttrs` (clause 6.3.11) or
/// a token of Tables 6.3.7-1 and 6.3.12-1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OptionsKeyword {
    /// `sysAttrs`.
    SystemAttributes,
    /// A representation token.
    Representation(RepresentationToken),
}

// The keyword is `sysAttrs` or one of another enum's tokens, which no derive reads.
impl FromStr for OptionsKeyword {
    type Err = strum::ParseError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        if SystemAttributes::Include.parameter() == Some(text) {
            Ok(Self::SystemAttributes)
        } else {
            text.parse().map(Self::Representation)
        }
    }
}

/// What the `format` and `options` parameters of a received request ask for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ReceivedRepresentation<R> {
    /// The representation, normalized when no token names one, the one in `format` when both parameters do.
    pub(crate) representation: R,
    /// How the tokens were written.
    pub(crate) spelling: RepresentationSpelling,
    /// Whether `options` holds `sysAttrs`.
    pub(crate) system_attributes: SystemAttributes,
}

/// Reads a token of `parameter`, which must name a representation of the family `R`.
fn token<R: RepresentationFamily>(parameter: QueryParameter, text: &str) -> Result<(RepresentationToken, R), ParameterError> {
    text.parse::<RepresentationToken>()
        .ok()
        .and_then(|token| R::from_token(token).map(|representation| (token, representation)))
        .ok_or_else(|| ParameterError::UnknownToken {
            parameter,
            token: text.to_owned(),
        })
}

/// Reads `format` and `options` for a representation of the family `R`. When both name a representation, `format`
/// takes precedence (Table 6.3.12-1) and the `options` keyword is kept in the spelling.
///
/// # Errors
///
/// Returns [`ParameterError::UnknownToken`] for a token the table of `R` and clause 6.3.11 do not define, and
/// [`ParameterError::RepresentationTwice`] when `options` names two representations, which Table 6.3.12-1 forbids.
pub(crate) fn read_representation<R: RepresentationFamily>(received: &ReceivedQuery<'_>) -> Result<ReceivedRepresentation<R>, ParameterError> {
    let format = received
        .text(QueryParameter::Format)?
        .map(|text| token::<R>(QueryParameter::Format, &text))
        .transpose()?;
    let mut keyword: Option<(RepresentationToken, R)> = None;
    let mut system_attributes = SystemAttributes::Omit;
    for option in received.tokens::<OptionsKeyword>(QueryParameter::Options)? {
        match option {
            OptionsKeyword::SystemAttributes => system_attributes = SystemAttributes::Include,
            OptionsKeyword::Representation(token) => {
                let representation = R::from_token(token).ok_or_else(|| ParameterError::UnknownToken {
                    parameter: QueryParameter::Options,
                    token: token.to_string(),
                })?;
                if keyword.replace((token, representation)).is_some() {
                    return Err(ParameterError::RepresentationTwice);
                }
            }
        }
    }
    let (parameter, named) = match (format, keyword) {
        (Some(format), Some((keyword, _))) => (RepresentationParameter::FormatOverOptions(keyword), Some(format)),
        (Some(format), None) => (RepresentationParameter::Format, Some(format)),
        (None, Some(keyword)) => (RepresentationParameter::Options, Some(keyword)),
        (None, None) => (RepresentationParameter::Format, None),
    };
    let token = named.map(|(token, _)| token);
    let stated = !matches!(parameter, RepresentationParameter::FormatOverOptions(_));
    let spelling = RepresentationSpelling {
        parameter,
        normalized: if stated && token == Some(RepresentationToken::Normalized) {
            NormalizedSpelling::Explicit
        } else {
            NormalizedSpelling::Implicit
        },
        simplified: if token == Some(RepresentationToken::KeyValues) {
            SimplifiedSpelling::KeyValues
        } else {
            SimplifiedSpelling::Simplified
        },
    };
    Ok(ReceivedRepresentation {
        representation: named.map_or_else(R::default, |(_, representation)| representation),
        spelling,
        system_attributes,
    })
}
