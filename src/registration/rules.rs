//! The rules of NGSI-LD Tables 5.2.9-1 and 5.2.10-1 and of clauses 4.3.6.2 and 4.3.6.3 that relate the
//! members of a registration, checked by the builder and by both readers.

use crate::{
    model::MemberProblem,
    registration::{
        CSourceRegistration,
        DistributedOperation,
        OperationGroup,
        Problem,
        RegistrationDirection,
        RegistrationField,
        RegistrationInfo,
        RegistrationMember,
        RegistrationMode,
        SupportedOperation,
    },
};

/// The members of a registration the rules relate, borrowed from a registration or from the builder's input.
pub(crate) struct Members<'a> {
    /// `information`.
    pub(crate) information: &'a [RegistrationInfo],
    /// `mode`.
    pub(crate) mode: Option<RegistrationMode>,
    /// `operations`.
    pub(crate) operations: Option<&'a [SupportedOperation]>,
    /// `registrationName` and `description`, each a non-empty string when present.
    pub(crate) texts: [(RegistrationMember, Option<&'a str>); 2],
}

/// Every rule `registration` breaks.
pub(crate) fn registration_problems<D: RegistrationDirection>(registration: &CSourceRegistration<D>) -> Vec<Problem> {
    member_problems(&Members {
        information: &registration.information,
        mode: registration.mode,
        operations: registration.operations.as_deref(),
        texts: [
            (RegistrationMember::RegistrationName, registration.name.as_deref()),
            (RegistrationMember::Description, registration.description.as_deref()),
        ],
    })
}

/// Every rule `members` break.
pub(crate) fn member_problems(members: &Members<'_>) -> Vec<Problem> {
    let mut problems = Vec::new();
    if members.information.is_empty() {
        problems.push(Problem::from(MemberProblem::Empty {
            member: RegistrationField::Information,
        }));
    }
    if members.information.iter().any(RegistrationInfo::is_empty) {
        problems.push(Problem::EmptyRegistrationInfo);
    }
    for (member, text) in members.texts {
        if text.is_some_and(str::is_empty) {
            problems.push(Problem::from(MemberProblem::Empty {
                member: RegistrationField::from(member),
            }));
        }
    }
    match members.mode {
        Some(RegistrationMode::Auxiliary) if !members.operations.is_some_and(auxiliary_operations) => problems.push(Problem::AuxiliaryOperations),
        Some(RegistrationMode::Exclusive) if !members.information.iter().all(RegistrationInfo::names_entity_ids_and_attributes) => {
            problems.push(Problem::ExclusiveWithoutEntityIdAndAttributes);
        }
        Some(RegistrationMode::Inclusive | RegistrationMode::Exclusive | RegistrationMode::Redirect | RegistrationMode::Auxiliary) | None => {}
    }
    problems
}

/// Whether `operations` stay within what an auxiliary registration supports: one or more of `retrieveOps`,
/// `retrieveEntity` and `queryEntity` (clause 5.9.2.4).
pub(crate) fn auxiliary_operations(operations: &[SupportedOperation]) -> bool {
    !operations.is_empty()
        && operations.iter().all(|operation| {
            matches!(
                operation,
                SupportedOperation::Group(OperationGroup::RetrieveOps)
                    | SupportedOperation::Operation(DistributedOperation::RetrieveEntity | DistributedOperation::QueryEntity)
            )
        })
}
