//! The inputs a builder takes for a value it validates: the value itself, or text validated when the builder
//! finishes, so every problem is reported together.

use crate::model::MemberProblem;

/// A value given to a builder, or why the text given in its place is not one. The trait is sealed by its module,
/// which no caller can name: only the drafts [`draft!`] generates implement it.
pub trait Draft {
    /// The value the draft stands for.
    type Value;
    /// Why the text given is not a value.
    type Error;

    /// Returns the value, or why the text given is not one.
    fn into_result(self) -> Result<Self::Value, Self::Error>;
}

/// Generates a draft, `pub struct $draft(Result<$value, $error>)`: a value given to a builder, or text parsed with
/// `$value`'s `FromStr` when it is given, or with the function `parse = ...` names. It converts from `$value`, `&str`
/// and `String`, and its [`Draft`] implementation hands the builder the value or the reason the text is not one.
/// Attributes written first, such as the doc comment, are put on the struct.
macro_rules! draft {
    ($(#[$meta:meta])* $draft:ident($value:ty, $error:ty)) => {
        draft!($(#[$meta])* $draft($value, $error), parse = str::parse);
    };
    ($(#[$meta:meta])* $draft:ident($value:ty, $error:ty), parse = $parse:expr) => {
        $(#[$meta])*
        #[derive(Debug, Clone)]
        pub struct $draft(Result<$value, $error>);

        impl $crate::model::draft::Draft for $draft {
            type Error = $error;
            type Value = $value;

            fn into_result(self) -> Result<$value, $error> {
                self.0
            }
        }

        // No derive: the conversion wraps the value into the draft's `Result`.
        impl From<$value> for $draft {
            fn from(value: $value) -> Self {
                Self(Ok(value))
            }
        }

        // No derive: the conversion parses the text into the draft's `Result`.
        impl From<&str> for $draft {
            fn from(text: &str) -> Self {
                Self($parse(text))
            }
        }

        // No derive: the conversion parses the text into the draft's `Result`.
        impl From<String> for $draft {
            fn from(text: String) -> Self {
                Self($parse(text.as_str()))
            }
        }
    };
}

/// Resolves the drafts given, in order, recording each whose text is not a value with `invalid`, which receives the
/// position of the draft and the reason.
pub(crate) fn resolve_drafts<D: Draft, P>(drafts: impl IntoIterator<Item = D>, problems: &mut Vec<P>, invalid: impl Fn(usize, D::Error) -> P) -> Vec<D::Value> {
    drafts
        .into_iter()
        .enumerate()
        .filter_map(|(index, draft)| draft.into_result().map_err(|source| problems.push(invalid(index, source))).ok())
        .collect()
}

/// Resolves the drafts of a list the specification forbids to be empty, as [`resolve_drafts`] does: absent, it is
/// empty with no problem; given empty, [`MemberProblem::Empty`] is recorded for `member`.
pub(crate) fn resolve_required_drafts<D: Draft, M: Copy, P: From<MemberProblem<M>>>(
    member: M,
    drafts: Option<Vec<D>>,
    problems: &mut Vec<P>,
    invalid: impl Fn(usize, D::Error) -> P,
) -> Vec<D::Value> {
    match drafts {
        Some(drafts) if drafts.is_empty() => {
            problems.push(P::from(MemberProblem::Empty { member }));
            Vec::new()
        }
        Some(drafts) => resolve_drafts(drafts, problems, invalid),
        None => Vec::new(),
    }
}
