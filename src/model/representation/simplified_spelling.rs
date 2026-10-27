//! The two tokens that name the simplified representation (NGSI-LD clauses 4.5.4, 5.2.14 and 6.3.7).

/// How a document or request names the simplified representation of entities.
///
/// The specification gives the simplified representation two names: `simplified` and its synonym `keyValues`. Table
/// 5.2.14-1 accepts either in a subscription's `notification.format`, and Table 6.3.7-1 in the `format` and `options`
/// parameters of a request. Both ask for the same representation, and a context source may support only one of them,
/// so a client chooses; `simplified` is the default.
///
/// The choice applies only to the simplified representation: any other representation has one name and ignores it.
/// The enum is exhaustive: the specification defines these two names.
///
/// ```
/// use ngsi_ld::model::SimplifiedSpelling;
///
/// assert_eq!(SimplifiedSpelling::default().to_string(), "simplified");
/// assert_eq!(SimplifiedSpelling::KeyValues.to_string(), "keyValues");
/// assert_eq!(
///     "keyValues".parse::<SimplifiedSpelling>()?,
///     SimplifiedSpelling::KeyValues
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, strum::Display, strum::EnumString, strum::IntoStaticStr)]
#[strum(serialize_all = "camelCase")]
pub enum SimplifiedSpelling {
    /// `simplified`.
    #[default]
    Simplified,
    /// `keyValues`, its synonym.
    KeyValues,
}
