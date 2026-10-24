//! Reading registrations in both directions, and every refusal of the readers.

use crate::{
    model::{Input, MemberError, OneOrMany, Output},
    query::Locality,
    registration::{
        CSourceRegistration,
        ContextSourceInfoError,
        ContextSourceInfoKey,
        Problem,
        RegistrationManagementInfo,
        RegistrationMode,
        RegistrationStatus,
        tests::{building::EVERY_MEMBER, read_error, read_problems, registration_text, round_trip},
    },
    subscription::{DocumentType, ReadErrorKind},
};
use pretty_assertions::assert_eq;

/// The members only a broker writes, as a retrieved registration carries them.
const OUTPUT_MEMBERS: [&str; 7] = [
    r#""status":"failed""#,
    r#""timesSent":3"#,
    r#""timesFailed":1"#,
    r#""lastSuccess":"2024-05-01T09:00:00Z""#,
    r#""lastFailure":"2024-05-01T10:00:00Z""#,
    r#""createdAt":"2024-04-01T10:00:00Z""#,
    r#""modifiedAt":"2024-04-02T10:00:00Z""#,
];

/// A member of a registration, the path of the error it causes, and the test the error's kind passes.
type Refusal = (&'static str, &'static [&'static str], fn(&ReadErrorKind<Problem>) -> bool);

/// A registration with an `id`, as a broker returns it, with `members` at the end.
fn retrieved(members: &str) -> String {
    registration_text(members).replacen('{', r#"{"id":"urn:ngsi-ld:ContextSourceRegistration:csr1","#, 1)
}

/// Asserts that `text` is refused with an error of the kind `matches` accepts, at `path`.
fn assert_refused(text: &str, path: &[&str], matches: impl Fn(&ReadErrorKind<Problem>) -> bool) {
    let error = read_error::<Input>(text);
    assert_eq!(error.path.to_string(), path.join("."), "{error}");
    assert!(matches(&error.kind), "{error}");
}

#[test]
fn the_registration_with_every_member_round_trips_as_a_request() {
    let registration = round_trip::<Input>(EVERY_MEMBER);
    assert_eq!(registration.mode(), Some(RegistrationMode::Redirect));
    assert_eq!(registration.management().map(RegistrationManagementInfo::locality), Some(Locality::Local));
    assert_eq!(registration.context_source_info().map(<[_]>::len), Some(2));
}

#[test]
fn a_retrieved_registration_keeps_the_members_only_a_broker_writes() {
    let text = retrieved(&OUTPUT_MEMBERS.join(","));
    let registration = round_trip::<Output>(&text);
    let output = registration.output();
    assert_eq!(output.status, Some(RegistrationStatus::Failed));
    assert_eq!((output.times_sent, output.times_failed), (Some(3), Some(1)));
    assert!(output.last_success.is_some() && output.last_failure.is_some() && output.created_at.is_some() && output.modified_at.is_some());
    let input = registration.into_input();
    assert_eq!(input.render().to_string(), retrieved(""));
}

#[test]
fn a_request_refuses_each_member_only_a_broker_writes_and_a_response_needs_an_id() {
    for member in OUTPUT_MEMBERS {
        let error = read_error::<Input>(&retrieved(member));
        let name = member.split('"').nth(1).unwrap();
        assert!(
            matches!(&error.kind, ReadErrorKind::Member(MemberError::UnexpectedMember { member }) if member == name),
            "{error}"
        );
    }
    let error = read_error::<Output>(&registration_text(""));
    assert!(
        matches!(error.kind, ReadErrorKind::Member(MemberError::MissingMember { member: "id" })),
        "{error}"
    );
}

#[test]
fn the_type_information_and_endpoint_are_required_and_no_member_but_a_property_is_read() {
    let wrong = registration_text("").replace("ContextSourceRegistration", "Registration");
    assert_refused(&wrong, &["type"], |kind| {
        matches!(
            kind,
            ReadErrorKind::WrongType {
                expected: DocumentType::ContextSourceRegistration,
                ..
            }
        )
    });
    let no_type = registration_text("").replace(r#""type":"ContextSourceRegistration","#, "");
    assert_refused(&no_type, &[], |kind| {
        matches!(kind, ReadErrorKind::Member(MemberError::MissingMember { member: "type" }))
    });
    let no_endpoint = r#"{"type":"ContextSourceRegistration","information":[{"propertyNames":["speed"]}]}"#;
    assert_refused(no_endpoint, &[], |kind| {
        matches!(kind, ReadErrorKind::Member(MemberError::MissingMember { member: "endpoint" }))
    });
    let no_information = r#"{"type":"ContextSourceRegistration","endpoint":"http://my.csource.org:1026/"}"#;
    assert_refused(no_information, &[], |kind| {
        matches!(kind, ReadErrorKind::Member(MemberError::MissingMember { member: "information" }))
    });
    assert_refused(
        &registration_text(r#""time stamp":{"startAt":"2017-11-29T14:53:15Z"}"#),
        &[],
        |kind| matches!(kind, ReadErrorKind::Member(MemberError::UnexpectedMember { member }) if member == "time stamp"),
    );
}

#[test]
fn the_information_and_its_entities_are_checked_where_they_stand() {
    let with_information =
        |information: &str| format!(r#"{{"type":"ContextSourceRegistration","information":{information},"endpoint":"http://my.csource.org:1026/"}}"#);
    assert_refused(&with_information("[]"), &["information"], |kind| {
        matches!(kind, ReadErrorKind::Member(MemberError::Empty))
    });
    let error = read_error::<Input>(&with_information("[{}]"));
    assert_eq!(error.path.to_string(), "information.0");
    assert!(
        matches!(&error.kind, ReadErrorKind::Invalid { source } if source.problems() == [Problem::EmptyRegistrationInfo]),
        "{error}"
    );
    assert_refused(&with_information(r#"[{"entities":[]}]"#), &["information", "0", "entities"], |kind| {
        matches!(kind, ReadErrorKind::Member(MemberError::Empty))
    });
    assert_refused(
        &with_information(r#"[{"entities":[{"id":"urn:a"}]}]"#),
        &["information", "0", "entities", "0"],
        |kind| matches!(kind, ReadErrorKind::Member(MemberError::MissingMember { member: "type" })),
    );
    assert_refused(
        &with_information(r#"[{"entities":[{"type":"1Vehicle"}]}]"#),
        &["information", "0", "entities", "0", "type"],
        |kind| matches!(kind, ReadErrorKind::Member(MemberError::Name { .. })),
    );
    assert_refused(
        &with_information(r#"[{"entities":[{"type":"Vehicle","idPattern":"(a|b"}]}]"#),
        &["information", "0", "entities", "0", "idPattern"],
        |kind| matches!(kind, ReadErrorKind::IdPattern { .. }),
    );
    let alternation = with_information(r#"[{"entities":[{"type":"Vehicle","idPattern":"^a$|^b$"}]}]"#);
    assert_eq!(
        round_trip::<Input>(&alternation).information()[0].entities()[0]
            .id_pattern()
            .map(ToString::to_string)
            .as_deref(),
        Some("^a$|^b$")
    );
    let several = with_information(r#"[{"entities":[{"type":["Vehicle","Car"],"idPattern":".*downtown$"}],"relationshipNames":["isParked"]}]"#);
    let registration = round_trip::<Input>(&several);
    assert!(registration.information()[0].entities()[0].entity_type().is_many());
}

#[test]
fn each_mode_and_operation_name_round_trips_and_others_are_refused() {
    for mode in ["inclusive", "exclusive", "redirect", "auxiliary"] {
        let operations = if mode == "auxiliary" { r#","operations":["retrieveOps"]"# } else { "" };
        let information = if mode == "exclusive" {
            r#"[{"entities":[{"type":"Vehicle","id":"urn:ngsi-ld:Vehicle:A456"}],"propertyNames":["speed"]}]"#
        } else {
            r#"[{"propertyNames":["speed"]}]"#
        };
        round_trip::<Input>(&format!(
            r#"{{"type":"ContextSourceRegistration","information":{information},"endpoint":"http://my.csource.org:1026/","mode":"{mode}"{operations}}}"#
        ));
    }
    assert_refused(&registration_text(r#""mode":"proxy""#), &["mode"], |kind| {
        matches!(kind, ReadErrorKind::Member(MemberError::InvalidToken { member: "mode", .. }))
    });
    let registration = round_trip::<Input>(&registration_text(
        r#""operations":["retrieveOps","queryEntity","upsertTemporal","federationOps"]"#,
    ));
    assert_eq!(registration.operations().map(<[_]>::len), Some(4));
    assert_refused(&registration_text(r#""operations":["fooOps"]"#), &["operations"], |kind| {
        matches!(kind, ReadErrorKind::Member(MemberError::InvalidToken { member: "operations", .. }))
    });
}

#[test]
fn geometries_scopes_tenants_durations_and_intervals_are_checked() {
    let cases: [Refusal; 6] = [
        (r#""location":{"type":"Point","coordinates":[1]}"#, &["location"], |kind| {
            matches!(kind, ReadErrorKind::Member(MemberError::Geometry { .. }))
        }),
        (r#""scope":"/a//b""#, &["scope"], |kind| {
            matches!(kind, ReadErrorKind::Member(MemberError::Scope { .. }))
        }),
        (r#""tenant":"""#, &["tenant"], |kind| matches!(kind, ReadErrorKind::Tenant { .. })),
        (r#""refreshRate":"1 minute""#, &["refreshRate"], |kind| {
            matches!(kind, ReadErrorKind::Member(MemberError::Duration { .. }))
        }),
        (r#""managementInterval":{"endAt":"2017-11-29T14:53:15Z"}"#, &["managementInterval"], |kind| {
            matches!(kind, ReadErrorKind::Member(MemberError::MissingMember { member: "startAt" }))
        }),
        (
            r#""managementInterval":{"startAt":" 2017-11-29T14:53:15Z"}"#,
            &["managementInterval", "startAt"],
            |kind| matches!(kind, ReadErrorKind::Member(MemberError::Temporal { .. })),
        ),
    ];
    for (member, path, matches) in cases {
        assert_refused(&registration_text(member), path, matches);
    }
    let outside_wgs_84 = r#""location":{"type":"Point","coordinates":[-1,100]}"#;
    let read = CSourceRegistration::<Input>::parse(&registration_text(outside_wgs_84)).unwrap();
    assert_eq!(
        read.location().map(ToString::to_string).as_deref(),
        Some(r#"{"type":"Point","coordinates":[-1,100]}"#)
    );
}

#[test]
fn a_reader_checks_the_rules_of_the_modes_and_of_the_context_source_information() {
    assert_eq!(
        read_problems::<Input>(&registration_text(r#""mode":"auxiliary""#)),
        [Problem::AuxiliaryOperations]
    );
    assert_eq!(
        read_problems::<Input>(&registration_text(r#""mode":"exclusive""#)),
        [Problem::ExclusiveWithoutEntityIdAndAttributes]
    );
    assert_eq!(
        read_problems::<Input>(&registration_text(r#""contextSourceInfo":[{"key":"accept","value":"text/plain"}]"#)),
        [Problem::ContextSourceInfo {
            index: 0,
            source: ContextSourceInfoError::MediaType {
                key: ContextSourceInfoKey::Accept
            }
        }]
    );
    assert_refused(&registration_text(r#""description":"""#), &["description"], |kind| {
        matches!(kind, ReadErrorKind::Member(MemberError::Empty))
    });
}

#[test]
fn one_scope_stays_a_string_and_an_explicit_false_stays_written() {
    let registration = round_trip::<Input>(&registration_text(r#""management":{"localOnly":false},"scope":"/Madrid""#));
    assert!(!registration.scope().is_some_and(OneOrMany::is_many));
    assert_eq!(registration.management().map(RegistrationManagementInfo::locality), Some(Locality::Distributed));
    let read = CSourceRegistration::<Input>::parse(&registration_text(r#""management":{}"#)).unwrap();
    assert_eq!(read.management().map(RegistrationManagementInfo::locality), Some(Locality::Unspecified));
}

#[test]
fn an_empty_context_source_info_and_an_empty_dataset_id_list_round_trip() {
    let empty = round_trip::<Input>(&registration_text(r#""contextSourceInfo":[],"datasetId":[]"#));
    assert_eq!(empty.context_source_info().map(<[_]>::len), Some(0));
    assert_eq!(empty.dataset_ids().map(<[_]>::len), Some(0));
    let absent = round_trip::<Input>(&registration_text(""));
    assert_eq!(absent.context_source_info(), None);
    assert_eq!(absent.dataset_ids(), None);
}
