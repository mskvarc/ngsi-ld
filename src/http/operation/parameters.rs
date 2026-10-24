//! Parameters several operations share. Each is a named choice rather than a flag.

use crate::{
    http::{QueryParameter, operation::ArgumentError, path::ResourcePath, query_string::QueryString, request::RequestParts},
    model::NgsiLdMediaType,
};
use http::Method;

/// Whether the answer includes the system attributes `createdAt`, `modifiedAt` and `expiresAt` (NGSI-LD clause
/// 6.3.11, `options=sysAttrs`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SystemAttributes {
    /// The broker's default: no system attributes.
    #[default]
    Omit,
    /// `options=sysAttrs`.
    Include,
}

/// Whether a query asks for the total number of matches (NGSI-LD clause 6.3.13, `count`), with an explicit
/// `count=false` kept apart from an absent parameter so that a request read back states what it was sent.
///
/// The enum is exhaustive: the parameter is absent, `true` or `false`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ResultsCount {
    /// No `count`.
    #[default]
    Omit,
    /// `count=true`: the total comes back in `NGSILD-Results-Count`.
    Include,
    /// `count=false`: no total, stated.
    Exclude,
}

impl SystemAttributes {
    /// The `options` parameter value.
    pub(crate) const fn parameter(self) -> Option<&'static str> {
        match self {
            Self::Omit => None,
            Self::Include => Some("sysAttrs"),
        }
    }

    /// Adds the `options` parameter the value stands for, if any (clause 6.3.12 and the tables of the operation).
    pub(crate) fn push_query(self, query: &mut QueryString) {
        query.push_some(QueryParameter::Options, self.parameter());
    }
}

/// How a request for the summary representation of a discovery or stored-context resource writes the `details`
/// parameter (NGSI-LD Tables 6.25.3.1-1, 6.27.3.1-1, 6.29.3.2-1 and 6.30.3.1-1): left out, or stated as
/// `details=false`, which every table allows and which clause 5.13.4.4 answers as an absent parameter.
///
/// `details=true` asks for the detailed representation, whose answer has another shape, so it is a request type of
/// its own rather than a third variant. The enum is exhaustive: a boolean parameter is absent, `false` or `true`.
///
/// ```
/// use ngsi_ld::http::operation::SummaryDetails;
///
/// assert_eq!(SummaryDetails::default(), SummaryDetails::Implied);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SummaryDetails {
    /// No `details` parameter: the summary, `false` being the default.
    #[default]
    Implied,
    /// `details=false`.
    Explicit,
}

/// Which representation a discovery or stored-context request asks for (NGSI-LD Tables 6.25.3.1-1, 6.27.3.1-1,
/// 6.29.3.2-1 and 6.30.3.1-1): the summary, with `details` absent or `false`, or the detailed one `details=true`
/// names. The two answers have different shapes, so each is a request type of its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Details {
    /// The summary.
    Summary(SummaryDetails),
    /// `details=true`.
    Detailed,
}

impl Details {
    /// The `details` parameter value.
    pub(crate) const fn parameter(self) -> Option<bool> {
        match self {
            Self::Summary(SummaryDetails::Implied) => None,
            Self::Summary(SummaryDetails::Explicit) => Some(false),
            Self::Detailed => Some(true),
        }
    }

    /// The `GET` of `path` asking for this representation, answered in JSON; `more` adds the operation's other
    /// parameters after `details`.
    pub(crate) fn get(self, path: ResourcePath, more: impl FnOnce(&mut QueryString)) -> RequestParts {
        let mut query = QueryString::default();
        query.push_some(QueryParameter::Details, self.parameter());
        more(&mut query);
        RequestParts::resource(Method::GET, path, query).accepting(NgsiLdMediaType::Json)
    }
}

/// The page a query of a paginated operation asks for (clauses 4.12, 6.3.10 and 6.3.13): refuses `limit=0`
/// without `count`, which clause 6.3.13 answers with a `BadRequest`.
pub(crate) const fn check_limit(limit: Option<u32>, count: ResultsCount) -> Result<(), ArgumentError> {
    match (limit, count) {
        (Some(0), ResultsCount::Omit | ResultsCount::Exclude) => Err(ArgumentError::ZeroLimitWithoutCount),
        (Some(_) | None, ResultsCount::Omit | ResultsCount::Include | ResultsCount::Exclude) => Ok(()),
    }
}

/// Adds `limit`, `offset` (clause 6.3.10) and `count` (clause 6.3.13), in that order.
pub(crate) fn push_paging(query: &mut QueryString, limit: Option<u32>, offset: Option<u32>, count: ResultsCount) {
    query.push_some(QueryParameter::Limit, limit);
    query.push_some(QueryParameter::Offset, offset);
    match count {
        ResultsCount::Omit => {}
        ResultsCount::Include => query.push(QueryParameter::Count, true),
        ResultsCount::Exclude => query.push(QueryParameter::Count, false),
    }
}

/// Which HTTP binding of Query Entities is used: `GET /entities` (NGSI-LD clause 6.4.3.2) or
/// `POST /entityOperations/query` (clause 6.23.3.1). Both are built from the same parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum QueryMethod {
    /// `GET /entities` with the parameters in the query string.
    #[default]
    Get,
    /// `POST /entityOperations/query` with a `Query` body (clause 5.2.23); useful when the URL would grow too long.
    Post,
}

/// Whether Append Attributes and Batch Entity Update overwrite attributes that already exist (NGSI-LD clauses
/// 6.6.3.1 and 6.16.3.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Overwrite {
    /// Existing attributes are replaced.
    #[default]
    Replace,
    /// `options=noOverwrite`: existing attributes are kept, and only new ones are added.
    Keep,
}

impl Overwrite {
    /// The `options` parameter value.
    pub(crate) const fn parameter(self) -> Option<&'static str> {
        match self {
            Self::Replace => None,
            Self::Keep => Some("noOverwrite"),
        }
    }

    /// Adds the `options` parameter the value stands for, if any (clause 6.3.12 and the tables of the operation).
    pub(crate) fn push_query(self, query: &mut QueryString) {
        query.push_some(QueryParameter::Options, self.parameter());
    }
}

/// What Batch Entity Upsert does with an entity that already exists (NGSI-LD clauses 5.6.8.3 and 6.15.3.1); an
/// entity that does not exist is created in either mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum UpsertMode {
    /// The existing entity is replaced by the one sent: the default mode, sent as no parameter.
    #[default]
    Replace,
    /// `options=update`: the existing entity is updated with the attributes sent (clause 5.6.2).
    Update,
}

impl UpsertMode {
    /// The `options` parameter value.
    pub(crate) const fn parameter(self) -> Option<&'static str> {
        match self {
            Self::Replace => None,
            Self::Update => Some("update"),
        }
    }

    /// Adds the `options` parameter the value stands for, if any (clause 6.3.12 and the tables of the operation).
    pub(crate) fn push_query(self, query: &mut QueryString) {
        query.push_some(QueryParameter::Options, self.parameter());
    }
}
