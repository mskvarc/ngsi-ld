//! Reading the query string of a request in typed form, as a context source receives the requests a broker forwards
//! to it (NGSI-LD clause 5.7.2.4 and clause 6.3): the parameters are split at `&`, each name is one of the
//! operation's table, and each value is percent-decoded (IETF RFC 3986 section 2.1) and read with the grammar of
//! its parameter.

use crate::{
    http::{QueryParameter, operation::EntityMapLifetimeError},
    model::{DocumentError, NameError, UriError},
    query::{ParseError, RegularExpressionError, geoquery::GeoQueryError, ordering::OrderingParamsError, temporal_query::TemporalQueryError},
};
use percent_encoding::percent_decode_str;
use std::num::ParseIntError;

pub(super) mod selection;
mod values;
pub(crate) mod write_options;

/// Why the query string of a request is not one the operation defines.
///
/// The enum is `#[non_exhaustive]`: new parameters bring new ways to be invalid.
// `ParseError` carries no `Clone` or equality, so neither can the error that chains it.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ParameterError {
    /// A parameter the operation's table does not define.
    #[error("`{name}` is not a parameter of the operation")]
    Unknown {
        /// The name, percent-decoded.
        name: String,
    },
    /// A parameter given twice.
    #[error("the parameter `{parameter}` is given twice")]
    Repeated {
        /// The parameter.
        parameter: QueryParameter,
    },
    /// A value whose percent-decoded bytes are not UTF-8.
    #[error("the parameter `{parameter}` is not percent-encoded UTF-8")]
    Encoding {
        /// The parameter.
        parameter: QueryParameter,
    },
    /// A value that does not follow the grammar of its query language.
    #[error("the parameter `{parameter}` does not follow its grammar")]
    Syntax {
        /// The parameter.
        parameter: QueryParameter,
        /// Why.
        #[source]
        source: ParseError,
    },
    /// A value that is not an attribute name (clause 4.6.2).
    #[error("the parameter `{parameter}` holds an invalid name")]
    Name {
        /// The parameter.
        parameter: QueryParameter,
        /// Why.
        #[source]
        source: NameError,
    },
    /// A value that is not a URI: an entity id or a dataset id.
    #[error("the parameter `{parameter}` holds an invalid URI")]
    Uri {
        /// The parameter.
        parameter: QueryParameter,
        /// Why.
        #[source]
        source: UriError,
    },
    /// `idPattern` is not a regular expression.
    #[error("invalid id pattern")]
    IdPattern {
        /// Why.
        #[source]
        source: RegularExpressionError,
    },
    /// `limit`, `offset` or `joinLevel` is not an integer of its range.
    #[error("the parameter `{parameter}` is not an integer of its range")]
    Integer {
        /// The parameter.
        parameter: QueryParameter,
        /// Why.
        #[source]
        source: ParseIntError,
    },
    /// `count`, `local`, `details`, `splitEntities`, `entityMap` or `deleteAll` is neither `true` nor `false`.
    #[error("the parameter `{parameter}` must be true or false")]
    NotABoolean {
        /// The parameter.
        parameter: QueryParameter,
    },
    /// A `format` or an `options` token Table 6.3.7-1, clause 6.3.11 or the table of the write operation does not
    /// define.
    #[error("`{token}` is not a value of the parameter `{parameter}`")]
    UnknownToken {
        /// The parameter.
        parameter: QueryParameter,
        /// The token, percent-decoded.
        token: String,
    },
    /// `join` is not `flat`, `inline` or `@none` (clause 4.5.23).
    #[error("invalid join")]
    Join {
        /// Why.
        #[source]
        source: strum::ParseError,
    },
    /// `entityMapLifetime` is not a positive duration (clause 4.5.25).
    #[error("invalid entity map lifetime")]
    Lifetime {
        /// Why.
        #[source]
        source: EntityMapLifetimeError,
    },
    /// `options` names two representations, which Table 6.3.12-1 forbids.
    #[error("the representation is asked for twice")]
    RepresentationTwice,
    /// The `options` of Batch Entity Upsert name both `replace` and `update`, of which clause 6.15.3.1 makes the
    /// mode one.
    #[error("the options of an upsert name both replace and update")]
    UpsertModeTwice,
    /// `georel`, `geometry`, `coordinates` and `geoproperty` do not make a geoquery (clause 4.10).
    #[error("invalid geoquery")]
    GeoQuery {
        /// Why.
        #[source]
        source: DocumentError<GeoQueryError>,
    },
    /// `orderBy`, `collation`, `orderFrom` and `orderGeometry` do not make an entity ordering (clause 4.23).
    #[error("invalid entity ordering")]
    Ordering {
        /// Why.
        #[source]
        source: DocumentError<OrderingParamsError>,
    },
    /// `orderFrom` does not hold JSON coordinates (clauses 4.10 and 4.23.3).
    #[error("the parameter `{parameter}` must hold JSON coordinates")]
    CoordinatesNotJson {
        /// The parameter.
        parameter: QueryParameter,
    },
    /// `timerel`, `timeAt`, `endTimeAt`, `timeproperty`, `lastN`, `aggrMethods` and `aggrPeriodDuration` do not make
    /// the temporal selection of the operation (clause 4.11): a value out of its grammar, a `between` without
    /// `endTimeAt`, or a Query Temporal Evolution without `timerel` (Table 6.18.3.2-1).
    #[error("invalid temporal query")]
    TemporalQuery {
        /// Why.
        #[source]
        source: DocumentError<TemporalQueryError>,
    },
    /// An ordering parameter given without `orderBy`, which the others complete (clause 4.23.3).
    #[error("the parameter `{parameter}` needs `orderBy`")]
    OrderingWithoutOrderBy {
        /// The parameter.
        parameter: QueryParameter,
    },
}

impl ParameterError {
    /// The error for a value of `parameter` that does not follow its grammar.
    pub(crate) const fn syntax(parameter: QueryParameter, source: ParseError) -> Self {
        Self::Syntax { parameter, source }
    }

    /// The error for a value of `parameter` that is not a name.
    pub(crate) const fn name(parameter: QueryParameter, source: NameError) -> Self {
        Self::Name { parameter, source }
    }

    /// The error for a value of `parameter` that is not a URI.
    pub(crate) const fn uri(parameter: QueryParameter, source: UriError) -> Self {
        Self::Uri { parameter, source }
    }

    /// The error for a value of `parameter` that is not an integer of its range.
    pub(crate) const fn integer(parameter: QueryParameter, source: ParseIntError) -> Self {
        Self::Integer { parameter, source }
    }

    /// The error for a `join` that is not a join.
    pub(crate) const fn join(_: QueryParameter, source: strum::ParseError) -> Self {
        Self::Join { source }
    }

    /// The error for an `entityMapLifetime` that is not a lifetime.
    pub(crate) const fn lifetime(_: QueryParameter, source: EntityMapLifetimeError) -> Self {
        Self::Lifetime { source }
    }

    /// The error for an `idPattern` that is not an id pattern.
    pub(crate) const fn id_pattern(_: QueryParameter, source: RegularExpressionError) -> Self {
        Self::IdPattern { source }
    }
}

/// The parameters of a query string as received, each value still percent-encoded.
///
/// Splitting the query string and finding a parameter are here; reading a value with the grammar of its parameter
/// is in the `values` module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReceivedQuery<'a> {
    /// The parameters, in the order received.
    entries: Vec<(QueryParameter, &'a str)>,
}

impl<'a> ReceivedQuery<'a> {
    /// Splits `query`, the part of a URI after its `?`, into its parameters.
    ///
    /// # Errors
    ///
    /// Returns [`ParameterError::Unknown`] for a parameter `accepted` does not list, and
    /// [`ParameterError::Repeated`] for a parameter given twice.
    pub(crate) fn read(query: &'a str, accepted: &[&[QueryParameter]]) -> Result<Self, ParameterError> {
        let mut entries: Vec<(QueryParameter, &'a str)> = Vec::new();
        for pair in query.split('&').filter(|pair| !pair.is_empty()) {
            let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
            let decoded = percent_decode_str(name).decode_utf8_lossy();
            let parameter = decoded
                .parse::<QueryParameter>()
                .ok()
                .filter(|parameter| accepted.iter().any(|group| group.contains(parameter)))
                .ok_or_else(|| ParameterError::Unknown { name: decoded.into_owned() })?;
            if entries.iter().any(|&(seen, _)| seen == parameter) {
                return Err(ParameterError::Repeated { parameter });
            }
            entries.push((parameter, value));
        }
        Ok(ReceivedQuery { entries })
    }

    /// The raw value of `parameter`, when it is given.
    fn raw(&self, parameter: QueryParameter) -> Option<&'a str> {
        self.entries.iter().find(|&&(seen, _)| seen == parameter).map(|&(_, value)| value)
    }

    /// Whether `parameter` is given.
    pub(crate) fn contains(&self, parameter: QueryParameter) -> bool {
        self.raw(parameter).is_some()
    }
}

#[cfg(test)]
mod tests {
    use crate::http::{
        QueryParameter,
        operation::{ParameterError, received_parameters::ReceivedQuery},
    };
    use pretty_assertions::assert_eq;

    const ACCEPTED: [QueryParameter; 3] = [QueryParameter::Id, QueryParameter::Q, QueryParameter::Count];

    #[test]
    fn values_are_percent_decoded_and_a_list_is_split_at_its_literal_commas() {
        let received = ReceivedQuery::read("id=urn%3Aa,urn%3Ab%2Cc&q=a%2Bb&count=true", &[&ACCEPTED]).unwrap();
        let ids: Vec<String> = received.list(QueryParameter::Id, |_, never| match never {}).unwrap();
        assert_eq!(ids, ["urn:a", "urn:b,c"]);
        assert_eq!(received.text(QueryParameter::Q).unwrap().as_deref(), Some("a+b"));
        assert_eq!(received.flag(QueryParameter::Count).unwrap(), Some(true));
        assert!(received.contains(QueryParameter::Q));
    }

    #[test]
    fn an_unknown_or_repeated_parameter_is_reported() {
        assert!(matches!(
            ReceivedQuery::read("q=a&qq=b", &[&ACCEPTED]),
            Err(ParameterError::Unknown { name }) if name == "qq"
        ));
        assert!(matches!(
            ReceivedQuery::read("type=Vehicle", &[&ACCEPTED]),
            Err(ParameterError::Unknown { name }) if name == "type"
        ));
        assert!(matches!(
            ReceivedQuery::read("q=a&q=b", &[&ACCEPTED]),
            Err(ParameterError::Repeated { parameter: QueryParameter::Q })
        ));
    }

    #[test]
    fn an_invalid_value_is_reported_with_its_parameter() {
        let received = ReceivedQuery::read("count=yes&q=%FF", &[&ACCEPTED]).unwrap();
        assert!(matches!(
            received.flag(QueryParameter::Count),
            Err(ParameterError::NotABoolean {
                parameter: QueryParameter::Count
            })
        ));
        assert!(matches!(
            received.text(QueryParameter::Q),
            Err(ParameterError::Encoding { parameter: QueryParameter::Q })
        ));
        assert!(ReceivedQuery::read("", &[&ACCEPTED]).unwrap().text(QueryParameter::Q).unwrap().is_none());
    }
}
