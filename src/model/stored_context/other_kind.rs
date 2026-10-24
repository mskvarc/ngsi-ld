//! A kind of stored `@context` clause 5.13.1 does not define, kept as the broker wrote it.

/// The token of a kind of stored `@context` the specification does not define.
///
/// NGSI-LD clause 5.13.1 defines the kinds, and clause 5.13.3.3 lists the kinds "currently" possible, so a broker of a
/// later version may report another. [`StoredContextKind::Other`](crate::model::StoredContextKind::Other) holds it.
///
/// It is only made by reading a kind, which gives it text that is not empty and names no kind of clause 5.13.1, so
/// a defined kind is never held here.
///
/// ```
/// use ngsi_ld::model::StoredContextKind;
///
/// let kind: StoredContextKind = "Ephemeral".parse()?;
/// assert!(matches!(kind, StoredContextKind::Other(_)));
/// assert_eq!(kind.to_string(), "Ephemeral");
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display, derive_more::AsRef)]
#[as_ref(str)]
pub struct OtherContextKind(String);

impl OtherContextKind {
    /// The kind written `token`, which names no kind of clause 5.13.1 and is not empty.
    pub(crate) const fn new(token: String) -> OtherContextKind {
        OtherContextKind(token)
    }
}
