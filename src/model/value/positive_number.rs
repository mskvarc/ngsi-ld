//! Numbers the specification restricts to "greater than 0".

use crate::model::{MemberProblem, Number};

/// A number is zero or negative where the specification requires it above 0.
#[derive(Debug, Clone, PartialEq, Eq, Hash, thiserror::Error)]
#[error("{number} is not greater than 0")]
pub struct NotPositive {
    /// The number refused.
    pub number: Number,
}

/// A JSON number greater than 0, kept in its lexical form.
///
/// It is the restriction NGSI-LD places on periods and counts such as a subscription's `throttling` and `timeInterval`
/// (Table 5.2.12-1) and an endpoint's `timeout` and `cooldown` (Table 5.2.15-1).
///
/// ```
/// use ngsi_ld::model::{Number, PositiveNumber};
///
/// let period = PositiveNumber::try_from(Number::from(5))?;
/// assert_eq!(period.to_string(), "5");
/// assert!(PositiveNumber::try_from(Number::from(0)).is_err());
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display, derive_more::AsRef, derive_more::Into)]
#[display("{_0}")]
pub struct PositiveNumber(Number);

impl PositiveNumber {
    /// Returns the number.
    #[must_use]
    pub const fn as_number(&self) -> &Number {
        &self.0
    }
}

// The check reads the lexical form, so no derive expresses it.
impl TryFrom<Number> for PositiveNumber {
    type Error = NotPositive;

    fn try_from(number: Number) -> Result<Self, Self::Error> {
        if is_positive(&number) {
            Ok(Self(number))
        } else {
            Err(NotPositive { number })
        }
    }
}

/// Whether `number` is greater than zero. The lexical form decides, so no number is rounded to zero on the way: a
/// number is zero exactly when its significand holds no digit other than `0`.
pub(crate) fn is_positive(number: &Number) -> bool {
    let text = number.as_str();
    let significand = text.split(['e', 'E']).next().unwrap_or(text);
    !text.starts_with('-') && significand.bytes().any(|digit| matches!(digit, b'1'..=b'9'))
}

/// Checks a number the specification requires to be greater than 0 for `member`, recording a refused one.
pub(crate) fn resolve_positive<M, P: From<MemberProblem<M>>>(member: M, number: Option<Number>, problems: &mut Vec<P>) -> Option<PositiveNumber> {
    number.and_then(|number| {
        PositiveNumber::try_from(number)
            .map_err(|source| problems.push(P::from(MemberProblem::NotPositive { member, source })))
            .ok()
    })
}

#[cfg(test)]
mod tests {
    use crate::model::{Number, PositiveNumber};
    use jstrict::Value as Json;

    /// Reads the lexical form of a JSON number.
    fn number(text: &str) -> Number {
        let Json::Number(number) = text.parse::<Json>().unwrap() else {
            panic!("not a number: {text}");
        };
        Number::from_json(number)
    }

    #[test]
    fn numbers_above_zero_are_kept_in_their_lexical_form() {
        for text in ["5", "0.5", "1e3", "0.001E2"] {
            assert_eq!(PositiveNumber::try_from(number(text)).unwrap().to_string(), text);
        }
    }

    #[test]
    fn zero_and_negative_numbers_are_refused() {
        for text in ["0", "0.0", "0e5", "-1", "-0.5"] {
            assert!(PositiveNumber::try_from(number(text)).is_err(), "{text}");
        }
    }
}
