//! The context source registration operations (NGSI-LD clauses 5.9.2 to 5.9.4, 5.10.1 and 5.10.2), bound to
//! `csourceRegistrations` and `csourceRegistrations/{registrationId}` (clauses 6.8 and 6.9): one request type per
//! operation, and how their responses are read.

use crate::{
    http::{BodyError, operation::stored_document::DocumentReader},
    model::Output,
    registration::{CSourceRegistration, Problem},
    subscription::ReadErrorKind,
};

pub(super) mod delete;
pub(super) mod parameters;
pub(super) mod query;
pub(super) mod register;
pub(super) mod retrieve;
pub(super) mod update;

/// Reads registrations strictly: a member Tables 5.2.9-1 and 5.2.9-2 do not define fails the response.
const READER: DocumentReader<CSourceRegistration<Output>, ReadErrorKind<Problem>> = DocumentReader {
    read: CSourceRegistration::from_json,
    error: BodyError::Registration,
};
