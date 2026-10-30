//! Unit tests of context source registrations, their fragments and context source notifications.

use crate::{
    model::{BuildError, DocumentError, EntityTypeName},
    registration::{CSourceRegistration, EntityInfo, Problem, RegistrationDirection, RegistrationInfo},
    subscription::{ReadErrorKind, read_error::tests::broken_rules},
};
use std::fmt::Debug;

mod building;
mod fragments;
mod notifications;
mod properties;
mod reading;
mod verbatim;

/// The endpoint of Annex C.3, as the crate writes it.
const ENDPOINT: &str = "http://my.csource.org:1026/";

/// An entity type name.
fn entity_type(name: &str) -> EntityTypeName {
    name.parse().unwrap()
}

/// The entities of type `Vehicle`.
fn vehicles() -> EntityInfo {
    EntityInfo::new(entity_type("Vehicle"))
}

/// Information on the `speed` of vehicles.
fn vehicle_speed() -> RegistrationInfo {
    RegistrationInfo::builder().entities([vehicles()]).property_names(["speed"]).build().unwrap()
}

/// The problems of a build that must fail.
fn problems<T: Debug>(result: Result<T, BuildError<Problem>>) -> Vec<Problem> {
    result.unwrap_err().problems().to_vec()
}

/// Reads a registration in direction `D` and checks that it is written back as the same text.
fn round_trip<D: RegistrationDirection>(text: &str) -> CSourceRegistration<D> {
    let registration = CSourceRegistration::<D>::parse(text).unwrap();
    pretty_assertions::assert_eq!(registration.render().to_string(), text);
    registration
}

/// The error of a read that must fail.
fn read_error<D: RegistrationDirection>(text: &str) -> DocumentError<ReadErrorKind<Problem>> {
    CSourceRegistration::<D>::parse(text).unwrap_err()
}

/// The problems of a read refused for breaking the rules.
fn read_problems<D: RegistrationDirection>(text: &str) -> Vec<Problem> {
    broken_rules(read_error::<D>(text))
}

/// A registration of vehicle speeds with `members` inserted after `information` and `endpoint`.
fn registration_text(members: &str) -> String {
    let separator = if members.is_empty() { "" } else { "," };
    format!(
        r#"{{"type":"ContextSourceRegistration","information":[{{"entities":[{{"type":"Vehicle"}}],"propertyNames":["speed"]}}],"endpoint":"{ENDPOINT}"{separator}{members}}}"#
    )
}
