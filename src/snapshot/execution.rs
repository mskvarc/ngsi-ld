//! The result of running one query of a snapshot (NGSI-LD clause 5.2.42).

use crate::{
    model::{
        DocumentError,
        MemberError,
        ProblemDetails,
        reader::{object, once, required, token, unexpected, within},
    },
    subscription::{
        ReadErrorKind,
        reader::{Emptiness, error, list},
    },
};
use jstrict::{Object, Value as Json, object::Entry};
use std::error::Error;

/// How the execution of a request ended.
///
/// It is the `resultStatus` of NGSI-LD Table 5.2.42-1.
///
/// The enum is exhaustive: the table allows exactly these three values.
///
/// ```
/// use ngsi_ld::snapshot::ExecutionStatus;
///
/// assert_eq!("empty".parse::<ExecutionStatus>()?, ExecutionStatus::Empty);
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::Display, strum::EnumString, strum::IntoStaticStr)]
#[strum(serialize_all = "lowercase")]
pub enum ExecutionStatus {
    /// `success`: the request ran and its result is not empty.
    Success,
    /// `failure`: the request reported an error.
    Failure,
    /// `empty`: the request ran and its result is empty.
    Empty,
}

/// The details of the result of executing a request, such as a snapshot query.
///
/// They say how the execution ended and, especially for an error, the problem it reported (NGSI-LD clause 5.2.42).
///
/// A broker writes it, in the `snapshotQueriesDetails` and `snapshotTemporalQueriesDetails` of a snapshot (Table
/// 5.2.41-2) and in a snapshot notification (Table 5.3.4-1), one per query in the order of the queries.
///
/// ```
/// use ngsi_ld::snapshot::{ExecutionStatus, Snapshot};
/// # use ngsi_ld::model::Output;
///
/// let snapshot = Snapshot::<Output>::parse(
///     r#"{
///       "id": "urn:ngsi-ld:Snapshot:1",
///       "type": "Snapshot",
///       "snapshotStatus": "empty",
///       "snapshotQueriesDetails": [{"resultStatus": "empty"}],
///       "expiresAt": "2030-01-01T00:00:00Z"
///     }"#,
/// )?;
/// let details = &snapshot.output().queries_details[0];
/// assert_eq!(details.result_status, ExecutionStatus::Empty);
/// assert!(details.problem_details.is_none());
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ExecutionResultDetails {
    /// `problemDetails`: more about the result, especially when it is an error.
    pub problem_details: Option<ProblemDetails>,
    /// `resultStatus`: how the execution ended.
    pub result_status: ExecutionStatus,
}

impl ExecutionResultDetails {
    /// Details of an execution that ended with `result_status`.
    #[must_use]
    pub const fn new(result_status: ExecutionStatus) -> ExecutionResultDetails {
        ExecutionResultDetails {
            problem_details: None,
            result_status,
        }
    }

    /// The same details, with the problem the execution reported.
    #[must_use]
    pub fn with_problem(self, problem: ProblemDetails) -> Self {
        Self {
            problem_details: Some(problem),
            ..self
        }
    }
}

/// Reads one `ExecutionResultDetails` object (Table 5.2.42-1); its `problemDetails` is read leniently, as every
/// problem is.
fn read_details<P: Error>(json: Json) -> Result<ExecutionResultDetails, DocumentError<ReadErrorKind<P>>> {
    let mut problem_details = None;
    let mut result_status = None;
    for Entry { key, value } in object(json)? {
        let key = key.as_str();
        match key {
            "problemDetails" => {
                let problem = match &value {
                    Json::Object(_) => ProblemDetails::from_json(&value),
                    Json::Null | Json::Boolean(_) | Json::Number(_) | Json::String(_) | Json::Array(_) => {
                        return Err(error(MemberError::NotAnObject.into()).within(key));
                    }
                };
                once(&mut problem_details, key, problem)?;
            }
            "resultStatus" => once(&mut result_status, key, within(key, token(value, "resultStatus"))?)?,
            other => return Err(unexpected(other)),
        }
    }
    Ok(ExecutionResultDetails {
        problem_details,
        result_status: required(result_status, "resultStatus")?,
    })
}

/// Reads a non-empty list of execution details, placing an error under the index of the details at fault.
pub(crate) fn read_details_list<P: Error>(json: Json) -> Result<Vec<ExecutionResultDetails>, DocumentError<ReadErrorKind<P>>> {
    list(json, Emptiness::Refused)?
        .into_iter()
        .enumerate()
        .map(|(index, item)| read_details(item).map_err(|error| error.at_index(index)))
        .collect()
}

/// Writes one `ExecutionResultDetails` object: `problemDetails`, `resultStatus`.
fn details_json(details: &ExecutionResultDetails) -> Json {
    let mut object = Object::new();
    if let Some(problem) = &details.problem_details {
        object.push("problemDetails".into(), problem.to_json());
    }
    object.push("resultStatus".into(), Json::from(<&str>::from(details.result_status)));
    Json::Object(object)
}

/// Appends a list of execution details when there is one.
pub(crate) fn push_details_list(object: &mut Object, member: &str, list: &[ExecutionResultDetails]) {
    if !list.is_empty() {
        object.push(member.into(), Json::Array(list.iter().map(details_json).collect()));
    }
}

#[cfg(test)]
mod tests;
