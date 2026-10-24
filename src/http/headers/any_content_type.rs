//! The `Content-Type` of a response from outside NGSI-LD, which may name any media type.

use crate::http::headers::field_value::decode_singleton;
use http::{HeaderMap, header::CONTENT_TYPE};
use mediatype::{MediaType, MediaTypeBuf};

/// The media type the one `Content-Type` field of `headers` names, whatever it is, for a response from outside
/// NGSI-LD, such as the server of a remote context (JSON-LD 1.1 API section 9.4). `None` when the field is missing,
/// repeated or not a media type.
pub(crate) fn any_media_type(headers: &HeaderMap) -> Option<MediaTypeBuf> {
    let text = decode_singleton(&mut headers.get_all(CONTENT_TYPE).iter()).ok()?;
    MediaType::parse(text).ok().map(MediaTypeBuf::from)
}
