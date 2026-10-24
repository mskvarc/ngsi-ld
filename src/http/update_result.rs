//! The result of an attribute operation that succeeded only partly (NGSI-LD clauses 5.2.18 and 5.2.19).

use crate::model::{AttributeName, RawJson, Uri};
use jstrict::{Object, Value as Json};
use serde::{Deserialize, Serialize};

/// Which attributes an append or update changed and which it did not (NGSI-LD clause 5.2.18), the body of a
/// 207 response to an attribute operation.
///
/// Error and partial-success bodies carry no `@context` (clause 6.3.6), so the names are usually fully
/// qualified IRIs.
///
/// ```
/// use ngsi_ld::http::UpdateResult;
///
/// let result: UpdateResult = serde_json::from_str(
///     r#"{
///       "updated": ["https://example.org/speed"],
///       "notUpdated": [
///         {"attributeName": "https://example.org/brandName", "reason": "attribute exists"}
///       ]
///     }"#,
/// )?;
/// assert_eq!(result.updated[0].to_string(), "https://example.org/speed");
/// assert_eq!(result.not_updated[0].reason, "attribute exists");
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateResult {
    /// The attributes that were appended or updated.
    #[serde(default)]
    pub updated: Vec<AttributeName>,
    /// The attributes that were not, each with the reason.
    #[serde(default)]
    pub not_updated: Vec<NotUpdatedDetails>,
}

/// Why one attribute was not changed (NGSI-LD clause 5.2.19).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotUpdatedDetails {
    /// The attribute.
    pub attribute_name: AttributeName,
    /// The broker's explanation, free text.
    pub reason: String,
    /// The registration of the failed distributed operation, if it was one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registration_id: Option<Uri>,
}

impl UpdateResult {
    /// Writes the result as the body of a 207 answer: `updated`, then `notUpdated`, each entry with its
    /// `attributeName`, its `reason` and its `registrationId` when it has one (clauses 5.2.18 and 5.2.19). A 207
    /// answer carries no `@context` (clause 6.3.6), so the names are written as they are held, which for an answer
    /// should be their fully qualified IRIs.
    ///
    /// ```
    /// use ngsi_ld::http::{NotUpdatedDetails, UpdateResult};
    ///
    /// let result = UpdateResult {
    ///     updated: vec!["https://example.org/color".parse()?],
    ///     not_updated: vec![NotUpdatedDetails {
    ///         attribute_name: "https://example.org/speed".parse()?,
    ///         reason: "attribute exists".to_owned(),
    ///         registration_id: None,
    ///     }],
    /// };
    /// assert_eq!(
    ///     result.render().to_string(),
    ///     concat!(
    ///         r#"{"updated":["https://example.org/color"],"#,
    ///         r#""notUpdated":[{"attributeName":"https://example.org"#,
    ///         r#"/speed","reason":"attribute exists"}]}"#
    ///     )
    /// );
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    #[must_use]
    pub fn render(&self) -> RawJson {
        let mut object = Object::new();
        let updated = self.updated.iter().map(|name| Json::from(name.to_string())).collect();
        object.push("updated".into(), Json::Array(updated));
        object.push(
            "notUpdated".into(),
            Json::Array(self.not_updated.iter().map(NotUpdatedDetails::to_json).collect()),
        );
        RawJson::from_json(Json::Object(object))
    }
}

impl NotUpdatedDetails {
    /// Writes the entry as an element of `notUpdated`.
    fn to_json(&self) -> Json {
        let mut object = Object::new();
        object.push("attributeName".into(), Json::from(self.attribute_name.to_string()));
        object.push("reason".into(), Json::from(self.reason.as_str()));
        if let Some(registration_id) = &self.registration_id {
            object.push("registrationId".into(), Json::from(registration_id.to_string()));
        }
        Json::Object(object)
    }
}

#[cfg(test)]
mod tests {
    use crate::http::{NotUpdatedDetails, UpdateResult};
    use pretty_assertions::assert_eq;

    #[test]
    fn a_result_with_updated_and_not_updated_attributes_round_trips() {
        let text = r#"{"updated":["speed"],"notUpdated":[{"attributeName":"brandName","reason":"attribute exists"}]}"#;
        let result: UpdateResult = serde_json::from_str(text).unwrap();
        assert_eq!(
            result,
            UpdateResult {
                updated: vec!["speed".parse().unwrap()],
                not_updated: vec![NotUpdatedDetails {
                    attribute_name: "brandName".parse().unwrap(),
                    reason: "attribute exists".to_owned(),
                    registration_id: None
                }],
            }
        );
        assert_eq!(serde_json::to_string(&result).unwrap(), text);
        assert_eq!(result.render().to_string(), text);
    }

    #[test]
    fn a_result_missing_both_lists_reads_as_empty() {
        assert_eq!(serde_json::from_str::<UpdateResult>("{}").unwrap(), UpdateResult::default());
    }
}
