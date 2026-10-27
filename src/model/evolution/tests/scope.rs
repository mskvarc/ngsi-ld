use crate::model::{
    DocumentErrorKind,
    Input,
    Lossless,
    MemberError,
    NullMember,
    Output,
    evolution::{ScopeValue, TemporalEntity},
};
use pretty_assertions::assert_eq;

/// A vehicle whose scope evolution is `scope`, written as given.
fn vehicle_with_scope(scope: &str) -> String {
    format!(r#"{{"id":"urn:ngsi-ld:Vehicle:B9211","type":"Vehicle","scope":{scope}}}"#)
}

/// A scope evolution in normalized form: a single scope, a list of scopes, and a deletion.
const NORMALIZED: &str = r#"[{"type":"Property","value":"/Madrid/Centro","observedAt":"2018-08-01T12:03:00Z"},{"type":"Property","value":["/Madrid/Centro","/Company123/UnitA"],"observedAt":"2018-08-01T12:05:00Z","modifiedAt":"2018-08-01T12:07:00Z"},{"type":"Property","value":"urn:ngsi-ld:null","deletedAt":"2018-08-01T12:07:00Z"}]"#;

/// `NORMALIZED` in concise form.
const CONCISE: &str = r#"[{"value":"/Madrid/Centro","observedAt":"2018-08-01T12:03:00Z"},{"value":["/Madrid/Centro","/Company123/UnitA"],"observedAt":"2018-08-01T12:05:00Z","modifiedAt":"2018-08-01T12:07:00Z"},{"value":"urn:ngsi-ld:null","deletedAt":"2018-08-01T12:07:00Z"}]"#;

#[test]
fn a_scope_evolution_round_trips_in_both_forms() {
    let normalized = vehicle_with_scope(NORMALIZED);
    let entity = TemporalEntity::<Output>::parse(&normalized, Lossless::Normalized).unwrap();
    let values: Vec<_> = entity.scope.iter().map(|instance| instance.value().clone()).collect();
    assert!(
        matches!(values.as_slice(), [ScopeValue::Scopes(one), ScopeValue::Scopes(two), ScopeValue::Removed] if one.as_slice().len() == 1 && two.as_slice().len() == 2)
    );
    assert_eq!(entity.render(Lossless::Normalized).to_string(), normalized);
    let concise = vehicle_with_scope(CONCISE);
    assert_eq!(entity.render(Lossless::Concise).to_string(), concise);
    assert_eq!(TemporalEntity::<Output>::parse(&concise, Lossless::Concise).unwrap(), entity);
}

#[test]
fn a_plain_scope_value_is_not_a_temporal_representation_of_the_scope() {
    for scope in [r#""/Madrid""#, r#"["/Madrid","/Company123/UnitA"]"#] {
        let input = TemporalEntity::<Input>::parse(&vehicle_with_scope(scope), Lossless::Normalized).unwrap_err();
        assert_eq!(input.path.to_string(), "scope", "{scope}");
        let output = TemporalEntity::<Output>::parse(&vehicle_with_scope(scope), Lossless::Concise).unwrap_err();
        assert_eq!(output.path.to_string(), "scope", "{scope}");
    }
}

#[test]
fn a_scope_instance_holds_nothing_else() {
    let input = |scope: &str| TemporalEntity::<Input>::parse(&vehicle_with_scope(scope), Lossless::Normalized).unwrap_err();
    let output = |scope: &str| TemporalEntity::<Output>::parse(&vehicle_with_scope(scope), Lossless::Normalized).unwrap_err();
    let modified = input(r#"[{"type":"Property","value":"/Madrid","modifiedAt":"2018-08-01T12:07:00Z"}]"#);
    assert_eq!(modified.path.to_string(), "scope");
    assert!(matches!(modified.kind, DocumentErrorKind::Member(MemberError::UnexpectedMember { member }) if member == "modifiedAt"));
    let removed = input(r#"[{"type":"Property","value":"urn:ngsi-ld:null"}]"#);
    assert_eq!(removed.path.to_string(), "scope");
    assert!(matches!(removed.kind, DocumentErrorKind::NgsiLdNull { member: NullMember::Value }));
    let dataset = output(r#"[{"type":"Property","value":"/Madrid","datasetId":"urn:ngsi-ld:dataset:gps"}]"#);
    assert!(matches!(dataset.kind, DocumentErrorKind::Member(MemberError::UnexpectedMember { member }) if member == "datasetId"));
    let relationship = output(r#"[{"type":"Relationship","value":"/Madrid"}]"#);
    assert!(matches!(relationship.kind, DocumentErrorKind::ScopeNotAProperty { found } if found == "Relationship"));
    let invalid = output(r#"[{"type":"Property","value":"Madrid Centro"}]"#);
    assert!(matches!(invalid.kind, DocumentErrorKind::Member(MemberError::Scope { .. })));
    let empty = output("[]");
    assert_eq!(empty.path.to_string(), "scope");
    assert!(matches!(empty.kind, DocumentErrorKind::Member(MemberError::Empty)));
}
