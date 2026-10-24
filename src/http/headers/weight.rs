//! The weight a content negotiation header gives each of its elements (RFC 9110 section 12.4.2), read from the
//! element's text; the weighted element itself is [`Weighted`](crate::query::Weighted).

use crate::{
    http::headers::field_list::{Unterminated, split, trim_ows},
    query::Quality,
};

/// Splits an element without parameters of its own into its text and its weight: `text [ OWS ";" OWS "q=" qvalue ]`
/// (RFC 9110 section 12.4.2), the `q` case-insensitive.
pub(crate) fn split_weight(element: &str) -> Result<(&str, Option<Quality>), headers::Error> {
    match split(element, b';').map_err(|Unterminated| headers::Error::invalid())?.as_slice() {
        [text] => Ok((trim_ows(text), None)),
        [text, weight] => {
            let qvalue = trim_ows(weight)
                .strip_prefix(['q', 'Q'])
                .and_then(|rest| rest.strip_prefix('='))
                .ok_or_else(headers::Error::invalid)?;
            let quality = qvalue.parse().map_err(|_| headers::Error::invalid())?;
            Ok((trim_ows(text), Some(quality)))
        }
        [..] => Err(headers::Error::invalid()),
    }
}

#[cfg(test)]
mod tests {
    use crate::{http::headers::weight::split_weight, query::Quality};
    use pretty_assertions::assert_eq;

    fn quality(thousandths: u16) -> Quality {
        Quality::from_thousandths(thousandths).unwrap()
    }

    #[test]
    fn a_weight_is_read_with_optional_whitespace_and_any_case_of_q() {
        assert_eq!(split_weight("fr").ok(), Some(("fr", None)));
        assert_eq!(split_weight("fr;q=0.5").ok(), Some(("fr", Some(quality(500)))));
        assert_eq!(split_weight("fr ; Q=1.000").ok(), Some(("fr", Some(quality(1000)))));
    }

    #[test]
    fn a_parameter_other_than_q_a_malformed_qvalue_or_a_second_parameter_is_rejected() {
        for element in ["fr;level=1", "fr;q", "fr;q=", "fr;q=1.5", "fr;q=0.1234", "fr;q = 0.5", "fr;q=0.5;q=0.4", "fr;"] {
            assert!(split_weight(element).is_err(), "{element}");
        }
    }
}
