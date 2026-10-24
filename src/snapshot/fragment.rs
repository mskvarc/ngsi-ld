//! Snapshot fragments (NGSI-LD clauses 5.4, 5.5.8 and 5.16.4): the changes Update Snapshot Status applies to a
//! snapshot.

use crate::{
    model::{BuildError, Duration, KeyValuePair, MemberProblem, RawJson, UrlDraft, null::NGSI_LD_NULL},
    snapshot::{Problem, SnapshotField, SnapshotMember, settings::Settings},
    subscription::DocumentType,
};
use bon::bon;
use jstrict::{Object, Value as Json, object::Entry};

/// The changes Update Snapshot Status applies to a snapshot.
///
/// NGSI-LD clauses 5.4 and 5.16.4.
///
/// `snapshotStatus` and the other members of Table 5.2.41-2 are output-only, the queries become read-only once the
/// snapshot exists (Table 5.2.41-1) and the `id` cannot be modified, so a fragment changes the settings only: it
/// sets each, or deletes it with the NGSI-LD Null (clause 5.5.8). No status transition can be requested.
///
/// It is written with `"type": "Snapshot"` and without `id`: the binding names the snapshot. Like the fragments of
/// the other documents, it is only written.
///
/// ```
/// use ngsi_ld::snapshot::{SnapshotFragment, SnapshotMember};
///
/// let longer = SnapshotFragment::builder()
///     .lifetime("PT2H".parse()?)
///     .delete(SnapshotMember::Endpoint)
///     .build()?;
/// assert_eq!(
///     longer.render().to_string(),
///     r#"{"type":"Snapshot","snapshotLifetime":"PT2H","endpoint":"urn:ngsi-ld:null"}"#
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotFragment {
    /// The members set, written in the order of Table 5.2.41-1.
    settings: Settings,
    /// The members deleted.
    deletions: Vec<SnapshotMember>,
}

#[bon]
impl SnapshotFragment {
    /// Starts the builder of a fragment. Each setter replaces a member; `delete` removes one, and may be called for
    /// several.
    ///
    /// # Errors
    ///
    /// `build()` returns [`BuildError`] listing a priority outside 1 to 10, an endpoint that is not a URL, and a
    /// member both set and deleted.
    #[builder]
    pub fn new(
        #[builder(field)] deletions: Vec<SnapshotMember>,
        lifetime: Option<Duration>,
        priority: Option<u8>,
        #[builder(into)] endpoint: Option<UrlDraft>,
        #[builder(with = |pairs: impl IntoIterator<Item = KeyValuePair>| pairs.into_iter().collect())] receiver_info: Option<Vec<KeyValuePair>>,
    ) -> Result<Self, BuildError<Problem>> {
        let mut problems = Vec::new();
        let settings = Settings::from_builder(lifetime, priority, endpoint, receiver_info, &mut problems);
        let mut set = Object::new();
        settings.write(&mut set);
        let mut deleted = Vec::new();
        for member in deletions {
            if set.get_unique(<&str>::from(member)).ok().flatten().is_some() {
                problems.push(Problem::from(MemberProblem::SetAndDeleted {
                    member: SnapshotField::from(member),
                }));
            } else if !deleted.contains(&member) {
                deleted.push(member);
            }
        }
        BuildError::finish(problems, Self { settings, deletions: deleted })
    }
}

impl<S: snapshot_fragment_builder::State> SnapshotFragmentBuilder<S> {
    /// Deletes `member` from the snapshot: the fragment writes the NGSI-LD Null for it (clause 5.5.8).
    pub fn delete(mut self, member: SnapshotMember) -> Self {
        self.deletions.push(member);
        self
    }
}

impl SnapshotFragment {
    /// Whether the fragment deletes `member`.
    #[must_use]
    pub fn deletes(&self, member: SnapshotMember) -> bool {
        self.deletions.contains(&member)
    }

    /// Writes the fragment: `type`, then the members set or deleted in the order of Table 5.2.41-1.
    #[must_use]
    pub fn render(&self) -> RawJson {
        RawJson::from_json(Json::Object(self.to_object()))
    }

    /// Writes the fragment without an `@context`, which the request pipeline delivers (clause 6.3.5).
    pub(crate) fn to_object(&self) -> Object {
        let mut changes = Object::new();
        self.settings.write(&mut changes);
        for member in &self.deletions {
            changes.push(<&str>::from(*member).into(), Json::from(NGSI_LD_NULL.as_str()));
        }
        let mut entries: Vec<Entry> = changes.into_iter().collect();
        // Every member the fragment writes is a `SnapshotMember`, whose order is the canonical one.
        entries.sort_by_key(|entry| entry.key.as_str().parse::<SnapshotMember>().ok());
        let mut object = Object::new();
        object.push("type".into(), Json::from(<&str>::from(DocumentType::Snapshot)));
        for Entry { key, value } in entries {
            object.push(key, value);
        }
        object
    }
}
