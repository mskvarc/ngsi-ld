//! The digital signature of an attribute: its `ngsildproof` member (NGSI-LD clauses 4.5.2.2, 4.5.3.2 and C.11).

use crate::model::{
    AttributeMeta,
    BuildError,
    BuildProblem,
    Direction,
    EntityId,
    EntityTypeName,
    IdDraft,
    Input,
    OneOrMany,
    Output,
    Patch,
    ProblemKind,
    Property,
    TypeDraft,
    Value,
    null,
};
use bon::bon;

/// The `ngsildproof` of an attribute: a Property whose value is a W3C Data Integrity `proof` object, with the
/// non-reified members `entityIdSealed` and `entityTypeSealed` naming the entity the signature is sealed to
/// (NGSI-LD clauses 4.5.2.2 and 4.5.3.2, and the example of Annex C.11).
///
/// Every attribute kind may carry one, in a request as in a response: a Context Producer signs what it sends.
/// The proof is carried as given; the crate neither creates nor verifies signatures.
///
/// ```
/// use indexmap::IndexMap;
/// use iri_rs::iri;
/// use ngsi_ld::model::{Proof, Property, Value};
///
/// let signature = Value::Object(IndexMap::from([
///     ("type".to_owned(), Value::from("DataIntegrityProof")),
///     ("cryptosuite".to_owned(), Value::from("eddsa-rdfc-2022")),
/// ]));
/// let proof = Proof::builder(signature)
///     .entity_id_sealed(iri!("urn:ngsi-ld:Store:002"))
///     .entity_type_sealed("Store")
///     .build()?;
/// let address = Property::builder("Tiger Street 4").proof(proof).build()?;
/// assert!(address.meta().proof.is_some());
/// assert!(
///     Proof::builder("not an object")
///         .entity_id_sealed(iri!("urn:ngsi-ld:Store:002"))
///         .entity_type_sealed("Store")
///         .build()
///         .is_err()
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proof<D: Direction> {
    /// `entityIdSealed`: the id of the entity the signature is sealed to.
    pub(crate) entity_id_sealed: EntityId,
    /// `entityTypeSealed`: the type or types of that entity.
    pub(crate) entity_type_sealed: OneOrMany<EntityTypeName>,
    /// The Property itself, whose `value` is the proof object.
    pub(crate) property: Property<D>,
}

#[bon]
impl Proof<Input> {
    /// Starts a builder from the proof object, the `value` of the `ngsildproof` Property. `entity_id_sealed` and
    /// `entity_type_sealed` are required, which the compiler checks.
    ///
    /// # Errors
    ///
    /// `build()` returns [`BuildError`] listing an invalid sealed id or type, a value that is not a JSON object
    /// (clause 4.5.2.2), and a value that holds the NGSI-LD Null (clause 5.5.4).
    #[builder]
    pub fn new(
        #[builder(start_fn, into)] value: Value,
        #[builder(into)] entity_id_sealed: IdDraft,
        #[builder(into)] entity_type_sealed: TypeDraft,
    ) -> Result<Self, BuildError> {
        let mut problems = Vec::new();
        let entity_id_sealed = entity_id_sealed.resolve(&mut problems);
        let entity_type_sealed = entity_type_sealed.resolve(&mut problems);
        if !value.is_object() {
            problems.push(BuildProblem::top(ProblemKind::ProofNotAnObject));
        }
        null::report(null::check_value(&value), &mut problems);
        let property = Property::plain(value);
        // An unresolved member has recorded its problem, so the error is never empty.
        if let (Some(entity_id_sealed), Some(entity_type_sealed)) = (entity_id_sealed, entity_type_sealed) {
            BuildError::finish(
                problems,
                Self {
                    entity_id_sealed,
                    entity_type_sealed,
                    property,
                },
            )
        } else {
            Err(BuildError::new(problems))
        }
    }

    /// Turns the proof into one a fragment writes.
    pub(crate) fn into_patch(self) -> Proof<Patch> {
        Proof {
            entity_id_sealed: self.entity_id_sealed,
            entity_type_sealed: self.entity_type_sealed,
            property: self.property.map_meta(AttributeMeta::into_patch),
        }
    }

    /// Turns the proof into one a response carries, with no output-only member.
    pub(crate) fn into_output(self) -> Proof<Output> {
        Proof {
            entity_id_sealed: self.entity_id_sealed,
            entity_type_sealed: self.entity_type_sealed,
            property: self.property.map_meta(AttributeMeta::into_output),
        }
    }
}

impl<D: Direction> Proof<D> {
    /// Returns the id of the entity the signature is sealed to (`entityIdSealed`).
    #[must_use]
    pub const fn entity_id_sealed(&self) -> &EntityId {
        &self.entity_id_sealed
    }

    /// Returns the type or types of the entity the signature is sealed to (`entityTypeSealed`).
    #[must_use]
    pub const fn entity_type_sealed(&self) -> &OneOrMany<EntityTypeName> {
        &self.entity_type_sealed
    }

    /// Returns the `ngsildproof` Property; its value is the proof object.
    #[must_use]
    pub const fn property(&self) -> &Property<D> {
        &self.property
    }
}

impl Proof<Output> {
    /// Drops the output-only members of the proof Property, recording an NGSI-LD Null a request cannot carry.
    pub(crate) fn into_input(self, problems: &mut Vec<BuildProblem>) -> Proof<Input> {
        let property = self.property.map_meta(|meta| meta.into_input(problems));
        null::report(null::check_value(property.value()), problems);
        Proof {
            entity_id_sealed: self.entity_id_sealed,
            entity_type_sealed: self.entity_type_sealed,
            property,
        }
    }
}
