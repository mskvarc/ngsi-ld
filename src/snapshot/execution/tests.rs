//! The details of a query's execution.

use crate::{
    model::{DocumentError, ErrorType, MemberError},
    snapshot::{ExecutionResultDetails, ExecutionStatus, Problem as SnapshotProblem, execution::read_details_list},
    subscription::ReadErrorKind,
};
use jstrict::Value as Json;
use pretty_assertions::assert_eq;

/// Reads one details object, as the only element of a list.
fn read(text: &str) -> Result<ExecutionResultDetails, DocumentError<ReadErrorKind<SnapshotProblem>>> {
    let json: Json = format!("[{text}]").parse().unwrap();
    read_details_list(json).map(|mut list| list.remove(0))
}

#[test]
fn execution_details_hold_a_status_and_a_lenient_problem() {
    let empty = read(r#"{"resultStatus":"empty"}"#).unwrap();
    assert_eq!(empty.result_status, ExecutionStatus::Empty);
    assert!(empty.problem_details.is_none());
    let failure = read(r#"{"problemDetails":{"type":"https://uri.etsi.org/ngsi-ld/errors/InternalError","status":"500"},"resultStatus":"failure"}"#).unwrap();
    assert_eq!(failure.result_status, ExecutionStatus::Failure);
    let problem = failure.problem_details.unwrap();
    assert_eq!((problem.error_type, problem.status), (Some(ErrorType::InternalError), Some(500)));
    let error = read(r#"{"resultStatus":"done"}"#).unwrap_err();
    assert!(
        matches!(error.kind, ReadErrorKind::Member(MemberError::InvalidToken { member: "resultStatus", .. })),
        "{error:?}"
    );
    let error = read("{}").unwrap_err();
    assert!(
        matches!(error.kind, ReadErrorKind::Member(MemberError::MissingMember { member: "resultStatus" })),
        "{error:?}"
    );
    assert_eq!(error.path.to_string(), "0");
}
