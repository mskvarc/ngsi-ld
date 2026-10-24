//! The quality value (`qvalue`, IETF RFC 9110 section 12.4.2) that weighs a preference, and the weighted element it
//! is attached to: in the Language Filter (NGSI-LD clause 4.15) and in the content negotiation headers of the HTTP
//! binding.

use std::{fmt, iter, str::FromStr};

/// The highest quality value, in thousandths.
const MAXIMUM_QUALITY: u16 = 1000;

/// Why a string or a number is not a quality value.
///
/// IETF RFC 9110 section 12.4.2 defines quality values.
///
/// The enum is exhaustive: the grammar of a quality value has no other rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
pub enum QualityError {
    /// The text is not `0` or `1`, optionally followed by `.` and decimals.
    #[error("a quality value is 0 or 1, optionally followed by a point and decimals")]
    Syntax,
    /// The number has more than three decimals.
    #[error("a quality value has at most three decimals")]
    TooManyDecimals,
    /// The number is above 1.
    #[error("a quality value is at most 1")]
    AboveOne,
}

/// A quality value: a number from 0 to 1 with at most three decimals.
///
/// It is the `qvalue` of IETF RFC 9110 section 12.4.2, kept in thousandths.
///
/// The parser accepts exactly the grammar `( "0" [ "." 0*3DIGIT ] ) / ( "1" [ "." 0*3("0") ] )`, so `00.5`, `.5`,
/// `1.5` and `0.1234` are refused. The printer writes the shortest decimal form: `1`, `0`, `0.9`, `0.85`.
///
/// ```
/// use ngsi_ld::query::{Quality, QualityError};
///
/// let quality: Quality = "0.80".parse()?;
/// assert_eq!(quality.thousandths(), 800);
/// assert_eq!(quality.to_string(), "0.8");
/// assert_eq!("1.5".parse::<Quality>(), Err(QualityError::AboveOne));
/// assert_eq!("00.5".parse::<Quality>(), Err(QualityError::Syntax));
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Quality(u16);

impl Quality {
    /// The quality `thousandths` / 1000.
    ///
    /// # Errors
    ///
    /// Returns [`QualityError::AboveOne`] when `thousandths` is above 1000.
    pub const fn try_from_thousandths(thousandths: u16) -> Result<Quality, QualityError> {
        if thousandths > MAXIMUM_QUALITY {
            Err(QualityError::AboveOne)
        } else {
            Ok(Quality(thousandths))
        }
    }

    /// The quality in thousandths, from 0 to 1000.
    #[must_use]
    pub const fn thousandths(self) -> u16 {
        self.0
    }
}

// The `qvalue` grammar of RFC 9110 section 12.4.2: `0` with up to three decimals, or `1` with up to three zeros.
impl FromStr for Quality {
    type Err = QualityError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let (whole, fraction) = text.split_once('.').unwrap_or((text, ""));
        let digits = |part: &str| part.bytes().all(|byte| byte.is_ascii_digit());
        // The whole part is one digit; a longer run of digits is above one unless it starts with a zero, which the
        // grammar does not allow at all.
        if whole.is_empty() || !digits(whole) || !digits(fraction) || (whole.len() > 1 && whole.starts_with('0')) {
            return Err(QualityError::Syntax);
        }
        if fraction.len() > 3 {
            return Err(QualityError::TooManyDecimals);
        }
        let decimals = fraction
            .bytes()
            .chain(iter::repeat(b'0'))
            .take(3)
            .fold(0_u16, |thousandths, digit| thousandths * 10 + u16::from(digit - b'0'));
        match whole {
            "0" => Ok(Quality(decimals)),
            "1" if decimals == 0 => Ok(Quality(MAXIMUM_QUALITY)),
            _above_one => Err(QualityError::AboveOne),
        }
    }
}

// The shortest decimal form, which a derive cannot express.
impl fmt::Display for Quality {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0 == MAXIMUM_QUALITY {
            return f.write_str("1");
        }
        f.write_str("0")?;
        if self.0 == 0 {
            return Ok(());
        }
        let digits = format!("{:03}", self.0);
        write!(f, ".{}", digits.trim_end_matches('0'))
    }
}

/// An element of a preference list with an optional weight.
///
/// It is written `value`, or `value;q=<quality>` (IETF RFC 9110 section 12.4.2).
///
/// The Language Filter (NGSI-LD clause 4.15) weighs its language ranges this way, and so do the content negotiation
/// headers of the HTTP binding. An element written without a weight has none here, so a list reads back exactly as
/// it was written; RFC 9110 gives such an element the weight 1.
///
/// ```
/// use ngsi_ld::query::{Quality, Weighted, language_filter::LanguageRange};
///
/// let any: Weighted<LanguageRange> =
///     Weighted::new(LanguageRange::Any).with_quality(Quality::try_from_thousandths(500)?);
/// assert_eq!(any.to_string(), "*;q=0.5");
/// assert_eq!(any.quality(), Some(Quality::try_from_thousandths(500)?));
/// assert_eq!(any.value(), &LanguageRange::Any);
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Weighted<T> {
    /// The element.
    value: T,
    /// The weight, when one is written.
    quality: Option<Quality>,
}

impl<T> Weighted<T> {
    /// The element `value`, without a weight.
    #[must_use]
    pub fn new(value: impl Into<T>) -> Weighted<T> {
        Weighted {
            value: value.into(),
            quality: None,
        }
    }

    /// Weights the element with `quality`.
    #[must_use]
    pub fn with_quality(self, quality: Quality) -> Weighted<T> {
        Weighted {
            quality: Some(quality),
            ..self
        }
    }

    /// The element.
    #[must_use]
    pub const fn value(&self) -> &T {
        &self.value
    }

    /// Takes the element.
    #[must_use]
    pub fn into_value(self) -> T {
        self.value
    }

    /// The weight, when one is written.
    #[must_use]
    pub const fn quality(&self) -> Option<Quality> {
        self.quality
    }

    /// The element turned into another with `convert`, the weight kept.
    #[must_use]
    pub fn map<U>(self, convert: impl FnOnce(T) -> U) -> Weighted<U> {
        Weighted {
            value: convert(self.value),
            quality: self.quality,
        }
    }

    /// The element `value` with the weight `quality`, if any.
    pub(crate) const fn from_parts(value: T, quality: Option<Quality>) -> Weighted<T> {
        Weighted { value, quality }
    }
}

// The weight is optional and introduced by `;q=`, which a derive cannot express.
impl<T: fmt::Display> fmt::Display for Weighted<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.value.fmt(f)?;
        self.quality.map_or(Ok(()), |quality| write!(f, ";q={quality}"))
    }
}

#[cfg(test)]
mod tests {
    use crate::query::{Quality, QualityError, Weighted};
    use pretty_assertions::assert_eq;

    #[test]
    fn every_form_the_grammar_allows_is_read() {
        for (text, thousandths) in [
            ("0", 0),
            ("0.", 0),
            ("0.5", 500),
            ("0.05", 50),
            ("0.123", 123),
            ("0.000", 0),
            ("1", 1000),
            ("1.", 1000),
            ("1.0", 1000),
            ("1.000", 1000),
        ] {
            assert_eq!(text.parse::<Quality>().map(Quality::thousandths), Ok(thousandths), "{text}");
        }
    }

    #[test]
    fn a_leading_zero_a_missing_whole_part_or_a_non_digit_is_a_syntax_error() {
        for text in [
            "00.5", "01", "00", ".5", "", ".", "x", "0.a", "a.5", "+0.5", "-0", " 0.5", "0.5 ", "0,5", "1e0", "0..5",
        ] {
            assert_eq!(text.parse::<Quality>(), Err(QualityError::Syntax), "{text}");
        }
    }

    #[test]
    fn more_than_three_decimals_is_rejected() {
        for text in ["0.1234", "0.0000", "1.0000"] {
            assert_eq!(text.parse::<Quality>(), Err(QualityError::TooManyDecimals), "{text}");
        }
    }

    #[test]
    fn a_value_above_one_is_rejected() {
        for text in ["1.5", "1.001", "2", "10", "9.0"] {
            assert_eq!(text.parse::<Quality>(), Err(QualityError::AboveOne), "{text}");
        }
        assert_eq!(Quality::try_from_thousandths(1001), Err(QualityError::AboveOne));
        assert_eq!(Quality::try_from_thousandths(1000).map(Quality::thousandths), Ok(1000));
    }

    #[test]
    fn a_quality_prints_its_shortest_form_and_reads_back() {
        for (thousandths, text) in [(1000, "1"), (0, "0"), (900, "0.9"), (850, "0.85"), (5, "0.005")] {
            let quality = Quality::try_from_thousandths(thousandths).unwrap();
            assert_eq!(quality.to_string(), text);
            assert_eq!(text.parse::<Quality>(), Ok(quality));
        }
    }

    #[test]
    fn an_element_renders_its_weight_only_when_it_has_one() {
        let quality = |thousandths| Quality::try_from_thousandths(thousandths).unwrap();
        assert_eq!(Weighted::<&str>::new("gzip").to_string(), "gzip");
        assert_eq!(Weighted::<&str>::new("gzip").with_quality(quality(0)).to_string(), "gzip;q=0");
        assert_eq!(Weighted::<&str>::new("gzip").with_quality(quality(1000)).to_string(), "gzip;q=1");
        let weighted = Weighted::<&str>::new("br").with_quality(quality(850));
        assert_eq!((weighted.value(), weighted.quality()), (&"br", Some(quality(850))));
        assert_eq!(weighted.into_value(), "br");
    }

    #[test]
    fn mapping_an_element_keeps_its_weight() {
        let weighted = Weighted::<&str>::new("fr").with_quality(Quality::try_from_thousandths(900).unwrap());
        assert_eq!(
            weighted.map(str::len),
            Weighted::from_parts(2, Some(Quality::try_from_thousandths(900).unwrap()))
        );
        assert_eq!(Weighted::<&str>::new("fr").map(str::len).quality(), None);
    }
}
