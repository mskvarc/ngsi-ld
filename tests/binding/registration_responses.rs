//! The responses of the context source registration operations (clauses 6.8 and
//! 6.9): the registration created, the registration read, the statuses that succeed and the problems.

use crate::{
    fixtures::{context, response},
    registration_fixtures::{R, RETRIEVED, id, postpone, registration_error, vehicle_source},
};
use http::{HeaderName, header};
use iri_rs::IriRefBuf;
use ngsi_ld::{
    http::{
        ResourceCreated,
        ResponseError,
        headers::OptionalHeader,
        operation::{DeleteRegistration, Operation, RegisterContextSource, RetrieveRegistration, SystemAttributes, UpdateRegistration},
    },
    model::{ErrorType, MemberError},
    registration::{RegistrationId, RegistrationStatus},
    subscription::ReadErrorKind,
};
use pretty_assertions::assert_eq;
use std::str;

/// The 201 answer to Register with the headers given.
fn registered(headers: &[(&HeaderName, &str)]) -> ResourceCreated<RegistrationId> {
    RegisterContextSource {
        registration: &vehicle_source(),
    }
    .decode(&context(), response(201, headers, ""))
    .unwrap()
}

#[test]
fn a_registration_is_named_by_the_location_of_its_201() {
    let created = registered(&[(&header::LOCATION, &format!("/ngsi-ld/v1/csourceRegistrations/{R}"))]);
    let location = IriRefBuf::new(format!("/ngsi-ld/v1/csourceRegistrations/{R}")).unwrap();
    assert_eq!(created.location, OptionalHeader::Valid(location));
    assert_eq!(created.id(), Some(id(R)));
    let absent = registered(&[]);
    assert_eq!(absent.location, OptionalHeader::Absent);
    assert_eq!(absent.id(), None);
}

#[test]
fn retrieve_reads_the_registration_with_its_output_members() {
    let target = id(R);
    let registration = RetrieveRegistration::builder()
        .id(&target)
        .system_attributes(SystemAttributes::Include)
        .build()
        .decode(&context(), response(200, &[], RETRIEVED))
        .unwrap();
    assert_eq!(registration.id(), &target);
    assert_eq!(registration.output().status, Some(RegistrationStatus::Ok));
    assert_eq!(registration.output().times_sent, Some(2));
    assert_eq!(registration.render().to_string(), RETRIEVED);
}

#[test]
fn a_member_the_tables_do_not_define_fails_the_response() {
    let target = id(R);
    // `foo` would be a Context Source Property (clause 4.9); a member no property can be named after is not.
    let body = RETRIEVED.replacen(r#""status":"ok""#, r#""f-o-o":1,"status":"ok""#, 1);
    let error = RetrieveRegistration::builder()
        .id(&target)
        .build()
        .decode(&context(), response(200, &[], &body))
        .unwrap_err();
    assert!(matches!(&error, ResponseError::Body { status, .. } if *status == 200), "{error:?}");
    let error = registration_error(error);
    assert!(
        matches!(&error.kind, ReadErrorKind::Member(MemberError::UnexpectedMember { member }) if member == "f-o-o"),
        "{error:?}"
    );
}

#[test]
fn only_the_statuses_the_tables_list_are_successes() {
    let target = id(R);
    let fragment = postpone();
    let update = UpdateRegistration {
        id: &target,
        fragment: &fragment,
    };
    let delete = DeleteRegistration { id: &target };
    update.decode(&context(), response(204, &[], "")).unwrap();
    delete.decode(&context(), response(204, &[], "")).unwrap();
    let errors = [
        delete.decode(&context(), response(200, &[], "")).unwrap_err(),
        update.decode(&context(), response(201, &[], "")).unwrap_err(),
        RegisterContextSource {
            registration: &vehicle_source(),
        }
        .decode(&context(), response(204, &[], ""))
        .unwrap_err(),
        RetrieveRegistration::builder()
            .id(&target)
            .build()
            .decode(&context(), response(204, &[], ""))
            .unwrap_err(),
    ];
    for error in errors {
        assert!(matches!(error, ResponseError::UnexpectedStatus { .. }), "{error:?}");
    }
}

#[test]
fn problems_are_read_for_every_registration_operation() {
    let conflict = r#"{"type":"https://uri.etsi.org/ngsi-ld/errors/AlreadyExists","title":"exists"}"#;
    let error = RegisterContextSource {
        registration: &vehicle_source(),
    }
    .decode(&context(), response(409, &[(&header::CONTENT_TYPE, "application/json")], conflict))
    .unwrap_err();
    assert!(
        matches!(&error, ResponseError::Problem { details, .. } if details.error_type == Some(ErrorType::AlreadyExists)),
        "{error:?}"
    );
    let unprocessable = r#"{"type":"https://uri.etsi.org/ngsi-ld/errors/OperationNotSupported","title":"unavailable"}"#;
    let error = RegisterContextSource {
        registration: &vehicle_source(),
    }
    .decode(&context(), response(422, &[(&header::CONTENT_TYPE, "application/json")], unprocessable))
    .unwrap_err();
    assert!(
        matches!(&error, ResponseError::Problem { status, details, .. } if *status == 422 && details.error_type == Some(ErrorType::OperationNotSupported)),
        "{error:?}"
    );
    let target = id(R);
    let missing = r#"{"type":"https://uri.etsi.org/ngsi-ld/errors/ResourceNotFound","title":"missing"}"#;
    let error = RetrieveRegistration::builder()
        .id(&target)
        .build()
        .decode(&context(), response(404, &[(&header::CONTENT_TYPE, "application/json")], missing))
        .unwrap_err();
    assert!(
        matches!(&error, ResponseError::Problem { details, .. } if details.error_type == Some(ErrorType::ResourceNotFound)),
        "{error:?}"
    );
}
