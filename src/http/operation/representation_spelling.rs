//! How a request writes the representation it asks for: the `format` parameter, the deprecated `options` parameter,
//! or both, for the entities of Retrieve and Query Entities (NGSI-LD clause 6.3.7, Table 6.3.7-1) and the temporal
//! entities of Retrieve and Query Temporal Evolution (clause 6.3.12, Table 6.3.12-1).

use crate::{
    http::{
        QueryParameter,
        operation::{ArgumentError, SystemAttributes},
        query_string::QueryString,
    },
    model::{Representation, SimplifiedSpelling},
};

pub(crate) mod received;

/// A representation token, as written in `format` or `options`.
///
/// NGSI-LD Tables 6.3.7-1 and 6.3.12-1 define the tokens.
///
/// The enum is exhaustive: the two tables close the set.
///
/// ```
/// use ngsi_ld::http::operation::RepresentationToken;
///
/// assert_eq!(RepresentationToken::KeyValues.to_string(), "keyValues");
/// assert_eq!(
///     "aggregatedValues".parse::<RepresentationToken>()?,
///     RepresentationToken::AggregatedValues
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::Display, strum::EnumString, strum::IntoStaticStr)]
#[strum(serialize_all = "camelCase")]
pub enum RepresentationToken {
    /// `normalized` (Table 6.3.7-1).
    Normalized,
    /// `concise` (Table 6.3.7-1).
    Concise,
    /// `simplified` (Table 6.3.7-1).
    Simplified,
    /// `keyValues`, the synonym of `simplified` (Table 6.3.7-1).
    KeyValues,
    /// `temporalValues`, the simplified temporal representation (Table 6.3.12-1).
    TemporalValues,
    /// `aggregatedValues`, the aggregated temporal representation (Table 6.3.12-1).
    AggregatedValues,
}

/// A family of representations a request asks for with the tokens of one table: the entities of Table 6.3.7-1, or
/// the temporal entities of Table 6.3.12-1.
pub(crate) trait RepresentationFamily: Copy + Default {
    /// The representation `token` names, `None` when the table of the family has no such token.
    fn from_token(token: RepresentationToken) -> Option<Self>;

    /// The token the table of the family writes for this representation, `None` when it writes none.
    fn table_token(self) -> Option<RepresentationToken>;
}

/// The representations of entities (Table 6.3.7-1), which names all three.
impl RepresentationFamily for Representation {
    fn table_token(self) -> Option<RepresentationToken> {
        Some(match self {
            Self::Normalized => RepresentationToken::Normalized,
            Self::Concise => RepresentationToken::Concise,
            Self::Simplified => RepresentationToken::Simplified,
        })
    }

    fn from_token(token: RepresentationToken) -> Option<Self> {
        match token {
            RepresentationToken::Normalized => Some(Self::Normalized),
            RepresentationToken::Concise => Some(Self::Concise),
            RepresentationToken::Simplified | RepresentationToken::KeyValues => Some(Self::Simplified),
            RepresentationToken::TemporalValues | RepresentationToken::AggregatedValues => None,
        }
    }
}

/// The parameter, or parameters, that carry the representation.
///
/// NGSI-LD Tables 6.3.7-1 and 6.3.12-1.
///
/// The enum is exhaustive: the representation is in `format`, in `options`, or in both.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum RepresentationParameter {
    /// `format`.
    #[default]
    Format,
    /// `options`, deprecated by both tables, which a request also uses for `sysAttrs` (clause 6.3.11).
    Options,
    /// `format`, stated for every representation, normalized included, and the deprecated `options` keyword given
    /// here beside it. Table 6.3.12-1 makes `format` take precedence over `options`, so the keyword never changes the
    /// representation answered, which is the one the request reads.
    FormatOverOptions(RepresentationToken),
}

/// How a request asking for the normalized representation, the default, writes it.
///
/// The enum is exhaustive: the default is either written or not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum NormalizedSpelling {
    /// No token: the broker's default representation is normalized.
    #[default]
    Implicit,
    /// The `normalized` token.
    Explicit,
}

/// How a request writes the representation it asks for.
///
/// The default writes the token in `format` (NGSI-LD clauses 6.3.7 and 6.3.12, Tables 6.3.7-1 and 6.3.12-1), and
/// nothing for normalized; `options=sysAttrs` (clause 6.3.11) is written after any keyword, as one comma separated
/// list.
///
/// The representation itself is the request's format type, which decides how the answer is read; the spelling only
/// decides how it is written. Every spelling asks for the same representation, and a context source may support
/// only some of them, so a client chooses.
///
/// Each choice applies only to the representation it names: a request for concise entities writes `concise`
/// whatever [`SimplifiedSpelling`] says, and a request read back keeps the default for the choices its
/// representation does not use.
///
/// ```
/// use ngsi_ld::http::operation::{RepresentationSpelling, RepresentationToken};
///
/// let deprecated = RepresentationSpelling::default().in_options().key_values();
/// assert_ne!(deprecated, RepresentationSpelling::default());
/// let both = RepresentationSpelling::default()
///     .format_over_options(RepresentationToken::AggregatedValues);
/// assert_ne!(both, deprecated);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct RepresentationSpelling {
    /// The parameter, or parameters, that carry the token.
    pub parameter: RepresentationParameter,
    /// Whether the normalized representation is written in `format` or `options` alone.
    pub normalized: NormalizedSpelling,
    /// How the simplified representation of entities is written.
    pub simplified: SimplifiedSpelling,
}

impl RepresentationSpelling {
    /// Writes the token in `options` instead of `format`.
    #[must_use]
    pub const fn in_options(self) -> Self {
        Self {
            parameter: RepresentationParameter::Options,
            ..self
        }
    }

    /// Writes the token in `format`, stated for every representation, and `keyword` in `options` beside it, which
    /// `format` takes precedence over.
    #[must_use]
    pub const fn format_over_options(self, keyword: RepresentationToken) -> Self {
        Self {
            parameter: RepresentationParameter::FormatOverOptions(keyword),
            ..self
        }
    }

    /// Writes `normalized` instead of leaving it to the default.
    #[must_use]
    pub const fn explicit_normalized(self) -> Self {
        Self {
            normalized: NormalizedSpelling::Explicit,
            ..self
        }
    }

    /// Writes the simplified representation of entities as `keyValues`.
    #[must_use]
    pub const fn key_values(self) -> Self {
        Self {
            simplified: SimplifiedSpelling::KeyValues,
            ..self
        }
    }

    /// The token written for `representation` in `format` or `options` alone, `None` when nothing is written:
    /// normalized only when the spelling states it and the table names it.
    fn token<R: RepresentationFamily>(self, representation: R) -> Option<RepresentationToken> {
        match (representation.table_token(), self.normalized, self.simplified) {
            (Some(RepresentationToken::Normalized), NormalizedSpelling::Implicit, _) | (None, _, _) => None,
            (Some(RepresentationToken::Simplified), _, SimplifiedSpelling::KeyValues) => Some(RepresentationToken::KeyValues),
            (Some(token), _, _) => Some(token),
        }
    }

    /// The tokens written in `format` and in `options` for `representation`. Beside an `options` keyword, `format`
    /// states the token of the table even for normalized, so that it takes precedence (Table 6.3.12-1).
    fn written<R: RepresentationFamily>(self, representation: R) -> (Option<RepresentationToken>, Option<RepresentationToken>) {
        match self.parameter {
            RepresentationParameter::Format => (self.token(representation), None),
            RepresentationParameter::Options => (None, self.token(representation)),
            RepresentationParameter::FormatOverOptions(keyword) => (representation.table_token(), Some(keyword)),
        }
    }

    /// Refuses an `options` keyword with no `format` to take precedence over it: the normalized temporal
    /// representation, which Table 6.3.12-1 gives no token, asked for with
    /// [`FormatOverOptions`](RepresentationParameter::FormatOverOptions).
    pub(crate) fn check<R: RepresentationFamily>(self, representation: R) -> Result<(), ArgumentError> {
        match self.written(representation) {
            (None, Some(keyword)) if matches!(self.parameter, RepresentationParameter::FormatOverOptions(_)) => {
                Err(ArgumentError::OptionsKeywordWithoutFormat { keyword })
            }
            (Some(_) | None, Some(_) | None) => Ok(()),
        }
    }

    /// The keyword the spelling writes in `options` for `representation`, if any.
    pub(crate) fn options_keyword<R: RepresentationFamily>(self, representation: R) -> Option<RepresentationToken> {
        self.written(representation).1
    }

    /// Adds `format`, when the spelling writes a token there.
    pub(crate) fn push_format<R: RepresentationFamily>(self, query: &mut QueryString, representation: R) {
        query.push_some(QueryParameter::Format, self.written(representation).0);
    }

    /// Adds `options`: the keyword when the spelling writes one there, then `sysAttrs` when `system_attributes`
    /// includes them (clause 6.3.11).
    pub(crate) fn push_options<R: RepresentationFamily>(self, query: &mut QueryString, representation: R, system_attributes: SystemAttributes) {
        let keyword = self.written(representation).1.map(<&'static str>::from);
        let options: Vec<&'static str> = keyword.into_iter().chain(system_attributes.parameter()).collect();
        query.push_list(QueryParameter::Options, &options);
    }
}

#[cfg(test)]
mod tests;
