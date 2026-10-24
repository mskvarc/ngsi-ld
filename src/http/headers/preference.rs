//! The preference syntax shared by `Prefer` and `Preference-Applied` (RFC 7240 section 2).

use http::HeaderValue;

/// The value of the first preference named `name` in the field lines `lines`, comma-separated preferences each.
///
/// A preference is `token [ "=" word ] *( ";" parameter )` (RFC 7240 section 2): parameters are dropped, names
/// compare case-insensitively, the value may be a quoted string, and the first occurrence of a preference is the one
/// that counts. A line that is not visible ASCII is not a field this crate can read.
pub(crate) fn first_value<'v>(lines: &[&'v HeaderValue], name: &str) -> Result<Option<&'v str>, headers::Error> {
    for line in lines {
        let text = line.to_str().map_err(|_| headers::Error::invalid())?;
        // The values this crate reads (a version, `json`, `ld+json`) hold no comma, so splitting on every comma cannot
        // cut one; a quoted value of another preference that holds one only yields fragments not named `name`.
        if let Some(value) = text.split(',').find_map(|element| named_value(element, name)) {
            return Ok(Some(value));
        }
    }
    Ok(None)
}

/// The value of one comma-separated preference when its name is `name`.
fn named_value<'e>(element: &'e str, name: &str) -> Option<&'e str> {
    let preference = element.split(';').next()?;
    let (found, value) = preference.split_once('=')?;
    if !found.trim().eq_ignore_ascii_case(name) {
        return None;
    }
    let value = value.trim();
    Some(value.strip_prefix('"').and_then(|quoted| quoted.strip_suffix('"')).unwrap_or(value))
}
