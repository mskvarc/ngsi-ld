//! Context source registrations and their notifications as linked documents (NGSI-LD clauses 5.2.9, 5.2.10 and
//! 5.3.2).

use crate::{
    linked_data::document::{
        printed,
        sealed::Sealed,
        shape::{ARRAY, Shape},
    },
    model::DocumentError,
    registration::{CSourceNotification, CSourceRegistration, Problem as RegistrationProblem, RegistrationDirection},
    subscription::ReadErrorKind,
};
use jstrict::Value as Json;

/// The array members of a registration (Table 5.2.9-1) and of its `RegistrationInfo` (Table 5.2.10-1). A Context
/// Source Property (clause 4.9) is read as compaction writes it, as the value of a Property is: its data type allows
/// any value, so a one-element array comes back as its element.
const REGISTRATION: Shape = Shape::Object(&[
    (
        "information",
        Shape::Array(&Shape::Object(&[("entities", ARRAY), ("propertyNames", ARRAY), ("relationshipNames", ARRAY)])),
    ),
    ("contextSourceInfo", ARRAY),
    ("operations", ARRAY),
    ("datasetId", ARRAY),
]);

impl<D: RegistrationDirection> Sealed for CSourceRegistration<D> {
    type ReadError = DocumentError<ReadErrorKind<RegistrationProblem>>;

    const SHAPE: Shape = REGISTRATION;

    fn write(&self) -> Json {
        self.render().into_json()
    }

    fn read(json: Json) -> Result<Self, Self::ReadError> {
        Self::from_json(json)
    }
}

impl Sealed for CSourceNotification {
    type ReadError = DocumentError<ReadErrorKind<RegistrationProblem>>;

    const SHAPE: Shape = Shape::Object(&[("data", Shape::Array(&REGISTRATION))]);

    fn write(&self) -> Json {
        self.render().into_json()
    }

    fn read(json: Json) -> Result<Self, Self::ReadError> {
        Self::parse(&printed(&json))
    }
}
