use crate::model::{Scope, ScopeError};
use pretty_assertions::assert_eq;

#[test]
fn scope_accepts_absolute_and_relative_forms() {
    for input in ["/Madrid/Gardens/ParqueNorte", "Madrid", "/Madrid", "/CompanyA/OrganizationB/UnitC"] {
        assert_eq!(input.parse::<Scope>().unwrap().to_string(), input);
    }
}

#[test]
fn scope_rejects_an_empty_level() {
    assert_eq!("/Madrid//Gardens".parse::<Scope>(), Err(ScopeError::EmptyLevel { position: 8 }));
}

#[test]
fn scope_rejects_an_empty_string_and_a_trailing_slash() {
    assert_eq!("".parse::<Scope>(), Err(ScopeError::EmptyLevel { position: 0 }));
    assert_eq!("/Madrid/".parse::<Scope>(), Err(ScopeError::EmptyLevel { position: 8 }));
}

#[test]
fn scope_rejects_a_level_starting_with_a_digit_and_wildcards() {
    assert_eq!("/1Madrid".parse::<Scope>(), Err(ScopeError::InvalidStart { found: '1', position: 1 }));
    assert_eq!("/Madrid/#".parse::<Scope>(), Err(ScopeError::InvalidStart { found: '#', position: 8 }));
    assert_eq!("/Ma-drid".parse::<Scope>(), Err(ScopeError::InvalidCharacter { found: '-', position: 3 }));
}
