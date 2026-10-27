//! How the instances of a temporal evolution are spelled in a request (NGSI-LD clauses 4.5.6 to 4.5.8).

use jstrict::Value as Json;

/// How an evolution that holds exactly one instance is written.
///
/// Clauses 4.5.7 and 4.5.8 give the temporal representation of a Property or Relationship as "an Array of JSON-LD
/// objects (or a single JSON-LD object in case only one is present)", and clause 4.5.6 gives the `scope` the same
/// representation. Both spellings are conformant. The array is the crate's canonical one, so it is the default; the
/// single object is for a client that sends the other spelling on purpose, to Create or Update Temporal Evolution
/// (clause 5.6.11.3). Add Attributes and Modify Attribute Instance require an array (clauses 5.6.12.3 and 5.6.14.3),
/// so their operations take no spelling.
///
/// An evolution of several instances is an array in both spellings. The enum is exhaustive: an evolution is written
/// as an array, or as the single object when it holds only one instance.
///
/// ```
/// use iri_rs::iri;
/// use ngsi_ld::model::{
///     Lossless,
///     Property,
///     evolution::{InstanceSpelling, TemporalEntity},
/// };
///
/// let vehicle = TemporalEntity::builder()
///     .id(iri!("urn:ngsi-ld:Vehicle:B9211"))
///     .entity_type("Vehicle")
///     .property(
///         "speed",
///         Property::builder(120).observed_at("2018-08-01T12:03:00Z".parse()?),
///     )
///     .build()?;
/// assert_eq!(
///     vehicle
///         .render_spelled(Lossless::Concise, InstanceSpelling::SingleWhenOne)
///         .to_string(),
///     concat!(
///         r#"{"id":"urn:ngsi-ld:Vehicle:B9211","type":"Vehicle","#,
///         r#""speed":{"value":120,"observedAt":"2018-08-01T12:03:00Z"}}"#
///     )
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum InstanceSpelling {
    /// Always an array of instances, whatever their number: the canonical spelling.
    #[default]
    Array,
    /// The single instance as one object when the evolution holds only one, an array otherwise.
    SingleWhenOne,
}

impl InstanceSpelling {
    /// Writes `instances` as an array, or as the one object when this spelling says so and there is only one.
    pub(crate) fn list(self, instances: impl Iterator<Item = Json>) -> Json {
        let mut items: Vec<Json> = instances.collect();
        match (self, items.pop()) {
            (Self::SingleWhenOne, Some(only)) if items.is_empty() => only,
            (Self::Array | Self::SingleWhenOne, last) => {
                items.extend(last);
                Json::Array(items.into())
            }
        }
    }
}
