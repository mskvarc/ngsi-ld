//! Linked entity retrieval (NGSI-LD clause 4.5.23): how the entities that relationships target are returned
//! alongside the entities a query, a retrieval or a notification carries.

/// How the entities that relationships target are returned.
///
/// It is the `join` parameter of NGSI-LD Tables 6.4.3.2-1 and 6.5.3.1-1, and the `join` member of a subscription's
/// notification (Table 5.2.14.1-1; clause 4.5.23).
///
/// The enum is exhaustive: the specification allows exactly `flat`, `inline` and `@none`.
///
/// ```
/// use ngsi_ld::query::Join;
///
/// assert_eq!(Join::Inline.to_string(), "inline");
/// assert_eq!("@none".parse::<Join>()?, Join::None);
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::Display, strum::EnumString, strum::IntoStaticStr)]
pub enum Join {
    /// `flat`: the linked entities are appended to the array of entities (clause 4.5.23.3).
    #[strum(serialize = "flat")]
    Flat,
    /// `inline`: each linked entity is returned in an `entity` or `entityList` member of the relationship that
    /// targets it (clause 4.5.23.2).
    #[strum(serialize = "inline")]
    Inline,
    /// `@none`: no linked entity is returned, the default.
    #[strum(serialize = "@none")]
    None,
}

impl Join {
    /// Whether the join returns linked entities, which is what gives a `joinLevel` meaning (Tables 6.4.3.2-1,
    /// 6.5.3.1-1 and 5.2.14.1-1).
    #[must_use]
    pub const fn links(self) -> bool {
        matches!(self, Self::Flat | Self::Inline)
    }
}
