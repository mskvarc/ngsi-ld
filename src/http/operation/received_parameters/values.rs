//! Reading the values of the parameters of a received query string: each value is percent-decoded (IETF RFC 3986
//! section 2.1) and read with the grammar of its parameter, a comma separated list split at its literal commas
//! before its items are decoded.

use crate::{
    http::{
        QueryParameter,
        operation::{ParameterError, ResultsCount, received_parameters::ReceivedQuery},
    },
    query::Locality,
};
use percent_encoding::percent_decode_str;
use std::{borrow::Cow, str::FromStr};

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
    /// literal commas, then each item is decoded and read with `FromStr`.
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

    /// The tokens of the comma separated list `parameter`, empty when it is not given.
    ///
    /// # Errors
    ///
    /// Returns [`ParameterError::Encoding`], or [`ParameterError::UnknownToken`] naming the first item that is not
    /// a token of `T`.
    pub(crate) fn tokens<T: FromStr>(&self, parameter: QueryParameter) -> Result<Vec<T>, ParameterError> {
        self.raw(parameter).map_or_else(
            || Ok(Vec::new()),
            |raw| {
                raw.split(',')
                    .map(|item| {
                        let item = decode(item, parameter)?;
                        item.parse::<T>().map_err(|_| ParameterError::UnknownToken {
                            parameter,
                            token: item.into_owned(),
                        })
                    })
                    .collect()
            },
        )
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
