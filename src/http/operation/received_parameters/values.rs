//! Reading the values of the parameters of a received query string: each value is percent-decoded (IETF RFC 3986
//! section 2.1) and read with the grammar of its parameter. A comma separated list of URIs is split at its literal
//! commas before its items are decoded, since a URI may hold an encoded comma; a list of names or tokens is split
//! after the decoding, since no name or token holds a comma and clients differ in whether they encode the separator.

use crate::{
    http::{
        QueryParameter,
        operation::{ParameterError, ResultsCount, SummaryDetails, SystemAttributes, parameters::Details, received_parameters::ReceivedQuery},
    },
    query::Locality,
};
use percent_encoding::percent_decode_str;
use std::{borrow::Cow, str::FromStr};

/// The one keyword of `options` the listings take: `sysAttrs` (clause 6.3.11).
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::EnumString)]
enum SystemAttributesOption {
    /// `sysAttrs`.
    #[strum(serialize = "sysAttrs")]
    SystemAttributes,
}

impl<'a> ReceivedQuery<'a> {
    /// The percent-decoded value of `parameter`, when it is given.
    ///
    /// # Errors
    ///
    /// Returns [`ParameterError::Encoding`] when the decoded bytes are not UTF-8.
    pub(crate) fn text(&self, parameter: QueryParameter) -> Result<Option<Cow<'a, str>>, ParameterError> {
        self.raw(parameter).map(|raw| decode(raw, parameter)).transpose()
    }

    /// The value of `parameter` read with `FromStr`, when it is given; `error` turns a parse failure into the error
    /// of the parameter.
    ///
    /// # Errors
    ///
    /// Returns [`ParameterError::Encoding`], or what `error` makes of the parse failure.
    pub(crate) fn parsed<T: FromStr>(
        &self,
        parameter: QueryParameter,
        error: impl FnOnce(QueryParameter, T::Err) -> ParameterError,
    ) -> Result<Option<T>, ParameterError> {
        self.text(parameter)?
            .map(|text| text.parse::<T>().map_err(|source| error(parameter, source)))
            .transpose()
    }

    /// The items of the comma separated list `parameter`, empty when it is not given: the raw value is split at its
    /// literal commas, then each item is decoded and read with `FromStr`. For items that may hold a comma, such as
    /// URIs; a list of names is read with [`ReceivedQuery::names`].
    ///
    /// # Errors
    ///
    /// Returns [`ParameterError::Encoding`], or what `error` makes of the first parse failure.
    pub(crate) fn list<T: FromStr>(
        &self,
        parameter: QueryParameter,
        error: impl Fn(QueryParameter, T::Err) -> ParameterError,
    ) -> Result<Vec<T>, ParameterError> {
        self.raw(parameter).map_or_else(
            || Ok(Vec::new()),
            |raw| {
                raw.split(',')
                    .map(|item| decode(item, parameter)?.parse::<T>().map_err(|source| error(parameter, source)))
                    .collect()
            },
        )
    }

    /// The items of the comma separated list of names `parameter`, empty when it is not given: the value is decoded,
    /// then split at its commas, so `a,b` and `a%2Cb` are the same list. No name holds a comma, so the separator is
    /// unambiguous in either spelling.
    ///
    /// # Errors
    ///
    /// Returns [`ParameterError::Encoding`], or what `error` makes of the first parse failure.
    pub(crate) fn names<T: FromStr>(
        &self,
        parameter: QueryParameter,
        error: impl Fn(QueryParameter, T::Err) -> ParameterError,
    ) -> Result<Vec<T>, ParameterError> {
        self.text(parameter)?.map_or_else(
            || Ok(Vec::new()),
            |text| {
                text.split(',')
                    .map(|item| item.parse::<T>().map_err(|source| error(parameter, source)))
                    .collect()
            },
        )
    }

    /// `count` (clause 6.3.13).
    ///
    /// # Errors
    ///
    /// Returns [`ParameterError::NotABoolean`] for a value other than `true` and `false`.
    pub(crate) fn results_count(&self) -> Result<ResultsCount, ParameterError> {
        Ok(match self.flag(QueryParameter::Count)? {
            Some(true) => ResultsCount::Include,
            Some(false) => ResultsCount::Exclude,
            None => ResultsCount::Omit,
        })
    }

    /// `local` (clause 6.3.18).
    ///
    /// # Errors
    ///
    /// Returns [`ParameterError::NotABoolean`] for a value other than `true` and `false`.
    pub(crate) fn locality(&self) -> Result<Locality, ParameterError> {
        Ok(self.flag(QueryParameter::Local)?.map_or(Locality::Unspecified, Locality::from_flag))
    }

    /// The tokens of the comma separated list `parameter`, empty when it is not given: the value is decoded, then
    /// split at its commas, as for [`ReceivedQuery::names`].
    ///
    /// # Errors
    ///
    /// Returns [`ParameterError::Encoding`], or [`ParameterError::UnknownToken`] naming the first item that is not
    /// a token of `T`.
    pub(crate) fn tokens<T: FromStr>(&self, parameter: QueryParameter) -> Result<Vec<T>, ParameterError> {
        self.text(parameter)?.map_or_else(
            || Ok(Vec::new()),
            |text| {
                text.split(',')
                    .map(|item| {
                        item.parse::<T>().map_err(|_| ParameterError::UnknownToken {
                            parameter,
                            token: item.to_owned(),
                        })
                    })
                    .collect()
            },
        )
    }

    /// The value of `parameter` read as one token of `T`, when it is given.
    ///
    /// # Errors
    ///
    /// Returns [`ParameterError::Encoding`], or [`ParameterError::UnknownToken`] when the value is not a token of `T`.
    pub(crate) fn token<T: FromStr>(&self, parameter: QueryParameter) -> Result<Option<T>, ParameterError> {
        self.text(parameter)?
            .map(|text| {
                text.parse::<T>().map_err(|_| ParameterError::UnknownToken {
                    parameter,
                    token: text.into_owned(),
                })
            })
            .transpose()
    }

    /// `details` (Tables 6.25.3.1-1, 6.27.3.1-1 and 6.29.3.2-1).
    ///
    /// # Errors
    ///
    /// Returns [`ParameterError::NotABoolean`] for a value other than `true` and `false`.
    pub(crate) fn details(&self) -> Result<Details, ParameterError> {
        Ok(match self.flag(QueryParameter::Details)? {
            None => Details::Summary(SummaryDetails::Implied),
            Some(false) => Details::Summary(SummaryDetails::Explicit),
            Some(true) => Details::Detailed,
        })
    }

    /// `options` of an operation whose only keyword is `sysAttrs` (clause 6.3.11).
    ///
    /// # Errors
    ///
    /// Returns [`ParameterError::Encoding`], or [`ParameterError::UnknownToken`] naming the first keyword other than
    /// `sysAttrs`.
    pub(crate) fn system_attributes(&self) -> Result<SystemAttributes, ParameterError> {
        let options = self.tokens::<SystemAttributesOption>(QueryParameter::Options)?;
        Ok(if options.is_empty() {
            SystemAttributes::Omit
        } else {
            SystemAttributes::Include
        })
    }

    /// The boolean `parameter`, when it is given.
    ///
    /// # Errors
    ///
    /// Returns [`ParameterError::NotABoolean`] for a value other than `true` and `false`.
    pub(crate) fn flag(&self, parameter: QueryParameter) -> Result<Option<bool>, ParameterError> {
        self.parsed(parameter, |parameter, _| ParameterError::NotABoolean { parameter })
    }
}

/// Percent-decodes the value of `parameter`.
fn decode(raw: &str, parameter: QueryParameter) -> Result<Cow<'_, str>, ParameterError> {
    percent_decode_str(raw).decode_utf8().map_err(|_| ParameterError::Encoding { parameter })
}
