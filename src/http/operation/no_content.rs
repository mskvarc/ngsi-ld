//! The answer of an operation whose only success is 204 with no body.

use crate::http::{ResponseError, response::classify};
use bytes::Bytes;
use http::{Response, StatusCode};

/// Reads the answer of an operation whose only success is 204 with no body: Update and Delete Subscription
/// (Tables 6.11.3.2-1, 6.11.3.3-1, 6.13.3.2-1 and 6.13.3.3-1), Update and Delete Context Source Registration
/// (Tables 6.9.3.2-1 and 6.9.3.3-1) and the operations that change a temporal evolution (Tables 6.19.3.2-1 and
/// 6.20.3.1-1 to 6.22.3.2-1).
pub(crate) fn decode_no_content(response: Response<Bytes>) -> Result<(), ResponseError> {
    let response = classify(response)?;
    match response.status() {
        StatusCode::NO_CONTENT => Ok(()),
        _ => Err(response.unexpected()),
    }
}
