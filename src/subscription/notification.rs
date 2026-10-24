//! Notifications (NGSI-LD clauses 5.3.1 and 5.8.6): what a broker sends a subscriber when subscribed entities
//! change, or periodically.

#[cfg(feature = "linked-data")]
use crate::model::name::mapping::{MapNames, NameMapping, member};
use crate::{
    model::{
        Complete,
        Completeness,
        Context,
        DateTime,
        DocumentError,
        Entity,
        FeatureCollection,
        Lossless,
        Output,
        RawJson,
        SimplifiedEntity,
        feature::{collection_object, read_collection},
        keyword::CONTEXT,
        reader::{array, context, date_time, duplicate, identifier, object, once, parse_json, required, unexpected, within},
        representation::{entity_object, read::read_entity},
    },
    subscription::{DocumentType, NotificationId, ReadErrorKind, SubscriptionId, reader::expect_type},
};
use bon::bon;
use jstrict::{Object, Value as Json, object::Entry};
use std::{convert::Infallible, error::Error};

/// A notification: the entities a subscription reports, as `T`.
///
/// NGSI-LD clause 5.3.1.
///
/// `T` is the type of `data`, decided by the subscription: `Vec<Entity<Output>>` for normalized or concise
/// entities, `Vec<SimplifiedEntity>` for simplified ones, their projected counterparts when `pick` or `omit`
/// removes `id` or `type`, and a [`FeatureCollection`] when the endpoint accepts `application/geo+json`.
///
/// The representation of the notified entities follows from the subscription (its `format`, `pick`, `omit` and
/// endpoint `accept`), which the receiver knows, so the receiver names the type of `data` and the reader never
/// infers it from the shape of the document.
///
/// ```
/// use ngsi_ld::{
///     model::{Entity, Lossless, Output},
///     subscription::Notification,
/// };
///
/// let text = r#"{
///   "id": "urn:ngsi-ld:Notification:1",
///   "type": "Notification",
///   "subscriptionId": "urn:ngsi-ld:Subscription:mySubscription",
///   "notifiedAt": "2024-05-01T10:00:00Z",
///   "data": [
///     {
///       "id": "urn:ngsi-ld:Vehicle:A4567",
///       "type": "Vehicle",
///       "speed": {"type": "Property", "value": 55}
///     }
///   ]
/// }"#;
/// let notification = Notification::<Vec<Entity<Output>>>::parse(text, Lossless::Normalized)?;
/// assert_eq!(
///     notification.subscription_id().as_str(),
///     "urn:ngsi-ld:Subscription:mySubscription"
/// );
/// assert_eq!(notification.data().len(), 1);
/// # Ok::<(), anyhow::Error>(())
/// ```
///
/// A receiver under test gets one built with [`Notification::builder`], the way a broker sends it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notification<T> {
    /// `id`.
    id: NotificationId,
    /// `subscriptionId`.
    subscription_id: SubscriptionId,
    /// `notifiedAt`.
    notified_at: DateTime,
    /// `data`.
    data: T,
    /// The `@context` of a JSON-LD notification (clause 5.2.3).
    context: Option<Context>,
}

#[bon]
impl<T> Notification<T> {
    /// Starts a builder of the notification a broker sends (Table 5.3.1-1). `id`, `subscription_id`, `notified_at`
    /// and `data`, the members the table makes mandatory, are required, which the compiler checks; `type` is always
    /// `Notification`.
    ///
    /// ```
    /// use iri_rs::iri;
    /// use ngsi_ld::{
    ///     model::{Entity, Lossless},
    ///     subscription::Notification,
    /// };
    ///
    /// let vehicle = Entity::output_builder()
    ///     .id(iri!("urn:ngsi-ld:Vehicle:A4567"))
    ///     .entity_type("Vehicle")
    ///     .property("speed", 55)
    ///     .build()?;
    /// let notification = Notification::builder()
    ///     .id(iri!("urn:ngsi-ld:Notification:1"))
    ///     .subscription_id(iri!("urn:ngsi-ld:Subscription:mySubscription"))
    ///     .notified_at("2024-05-01T10:00:00Z".parse()?)
    ///     .data(vec![vehicle])
    ///     .build();
    /// assert_eq!(
    ///     notification.render(Lossless::Concise).to_string(),
    ///     concat!(
    ///         r#"{"id":"urn:ngsi-ld:Notification:1","#,
    ///         r#""type":"Notification","#,
    ///         r#""subscriptionId":"urn:ngsi-ld:Subscription:mySubscription","#,
    ///         r#""notifiedAt":"2024-05-01T10:00:00Z","#,
    ///         r#""data":[{"id":"urn:ngsi-ld:Vehicle:A4567","#,
    ///         r#""type":"Vehicle","speed":55}]}"#
    ///     )
    /// );
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    #[builder]
    pub const fn new(
        #[builder(into)] id: NotificationId,
        #[builder(into)] subscription_id: SubscriptionId,
        notified_at: DateTime,
        data: T,
        #[builder(into)] context: Option<Context>,
    ) -> Self {
        Self {
            id,
            subscription_id,
            notified_at,
            data,
            context,
        }
    }
}

impl<T> Notification<T> {
    /// Returns the notification's identifier (`id`).
    #[must_use]
    pub const fn id(&self) -> &NotificationId {
        &self.id
    }

    /// Returns the identifier of the subscription that caused the notification (`subscriptionId`).
    #[must_use]
    pub const fn subscription_id(&self) -> &SubscriptionId {
        &self.subscription_id
    }

    /// Returns when the broker generated the notification (`notifiedAt`).
    #[must_use]
    pub const fn notified_at(&self) -> &DateTime {
        &self.notified_at
    }

    /// Returns the notified entities (`data`).
    #[must_use]
    pub const fn data(&self) -> &T {
        &self.data
    }

    /// Takes the notified entities.
    #[must_use]
    pub fn into_data(self) -> T {
        self.data
    }

    /// Returns the `@context` of a JSON-LD notification.
    #[must_use]
    pub const fn context(&self) -> Option<&Context> {
        self.context.as_ref()
    }

    /// Reads a notification, reading `data` with `read`.
    fn read<P: Error>(text: &str, read: impl FnOnce(Json) -> Result<T, DocumentError<ReadErrorKind<P>>>) -> Result<Self, DocumentError<ReadErrorKind<P>>> {
        Self::read_document(text, DocumentType::Notification, read, |_, json| Ok(Some(json)))
    }

    /// Reads a notification document whose `type` is `document_type`, reading `data` with `read`. Every member the
    /// notification does not define goes to `extra`, which reads it as a member of its own document or gives it
    /// back, unread, for the reader to refuse.
    pub(crate) fn read_document<P: Error>(
        text: &str,
        document_type: DocumentType,
        read: impl FnOnce(Json) -> Result<T, DocumentError<ReadErrorKind<P>>>,
        mut extra: impl FnMut(&str, Json) -> Result<Option<Json>, DocumentError<ReadErrorKind<P>>>,
    ) -> Result<Self, DocumentError<ReadErrorKind<P>>> {
        let mut id = None;
        let mut typed = None;
        let mut subscription_id = None;
        let mut notified_at = None;
        let mut data = None;
        let mut notification_context = None;
        let mut read = Some(read);
        for Entry { key, value } in object(parse_json(text)?)? {
            let key = key.as_str();
            match key {
                "id" => once(&mut id, key, within(key, identifier(value, "id"))?)?,
                "type" => once(&mut typed, key, within(key, expect_type(value, document_type))?)?,
                "subscriptionId" => once(&mut subscription_id, key, within(key, identifier(value, "subscriptionId"))?)?,
                "notifiedAt" => once(&mut notified_at, key, within(key, date_time(value, "notifiedAt"))?)?,
                "data" => {
                    let read = read.take().ok_or(duplicate(key))?;
                    once(&mut data, key, within(key, read(value))?)?;
                }
                CONTEXT => once(&mut notification_context, key, within(key, context(value))?)?,
                other => {
                    if extra(other, value)?.is_some() {
                        return Err(unexpected(other));
                    }
                }
            }
        }
        required(typed, "type")?;
        Ok(Self {
            id: required(id, "id")?,
            subscription_id: required(subscription_id, "subscriptionId")?,
            notified_at: required(notified_at, "notifiedAt")?,
            data: required(data, "data")?,
            context: notification_context,
        })
    }

    /// Writes the notification in canonical member order, `data` written by `write`: `id`, `type`,
    /// `subscriptionId`, `notifiedAt`, `data`, `@context`.
    fn write(&self, write: impl FnOnce(&T) -> Json) -> RawJson {
        self.write_document(DocumentType::Notification, |_| {}, write)
    }

    /// Writes a notification document whose `type` is `document_type`: `id`, `type`, `subscriptionId`,
    /// `notifiedAt`, the members `extra` writes, `data` written by `write`, `@context`.
    pub(crate) fn write_document(&self, document_type: DocumentType, extra: impl FnOnce(&mut Object), write: impl FnOnce(&T) -> Json) -> RawJson {
        let mut object = Object::new();
        object.push("id".into(), Json::from(self.id.as_str()));
        object.push("type".into(), Json::from(<&str>::from(document_type)));
        object.push("subscriptionId".into(), Json::from(self.subscription_id.as_str()));
        object.push("notifiedAt".into(), Json::from(self.notified_at.to_string()));
        extra(&mut object);
        object.push("data".into(), write(&self.data));
        if let Some(context) = &self.context {
            object.push(CONTEXT.into(), context.to_json());
        }
        RawJson::from_json(Json::Object(object))
    }
}

/// Reads `data` as an array of entities, each with `read`, placing an error under the entity's index.
fn entities<E, P: Error>(json: Json, read: impl Fn(Json) -> Result<E, DocumentError>) -> Result<Vec<E>, DocumentError<ReadErrorKind<P>>> {
    array(json)?
        .into_iter()
        .enumerate()
        .map(|(index, item)| read(item).map_err(|source| DocumentError::top(ReadErrorKind::Entity { source: Box::new(source) }).at_index(index)))
        .collect()
}

/// Writes `data` as an array of entities, each written by `write`.
fn entity_array<E>(entities: &[E], write: impl Fn(&E) -> Object) -> Json {
    Json::Array(entities.iter().map(|entity| Json::Object(write(entity))).collect())
}

/// Reads `data` as a `FeatureCollection`, every entity read with `read`.
fn collection<E, P: Error>(json: Json, read: impl FnMut(Json) -> Result<E, DocumentError>) -> Result<FeatureCollection<E>, DocumentError<ReadErrorKind<P>>> {
    read_collection(json, read).map_err(|source| DocumentError::top(ReadErrorKind::Entity { source: Box::new(source) }))
}

impl<C: Completeness> Notification<Vec<Entity<Output, C>>> {
    /// Reads a notification of entities in normalized or concise representation (`format` normalized or concise,
    /// clause 5.8.6). The concise reader also reads normalized attributes (clause 4.5.2.3), and both read the
    /// bare NGSI-LD Null of a deleted attribute (clause 5.8.6).
    ///
    /// # Errors
    ///
    /// Returns [`DocumentError`] naming the member at fault, an entity's error under `data` and its index.
    pub fn parse(text: &str, form: Lossless) -> Result<Self, DocumentError<ReadErrorKind<Infallible>>> {
        Self::read(text, |data| entities(data, |entity| read_entity(entity, form)))
    }

    /// Writes the notification with its entities in `form`.
    #[must_use]
    pub fn render(&self, form: Lossless) -> RawJson {
        self.write(|data| entity_array(data, |entity| entity_object(entity, form)))
    }
}

impl<C: Completeness> Notification<Vec<SimplifiedEntity<C>>> {
    /// Reads a notification of entities in simplified representation (`format` simplified or `keyValues`,
    /// clause 5.8.6).
    ///
    /// # Errors
    ///
    /// Returns [`DocumentError`] naming the member at fault, an entity's error under `data` and its index.
    pub fn parse(text: &str) -> Result<Self, DocumentError<ReadErrorKind<Infallible>>> {
        Self::read(text, |data| entities(data, SimplifiedEntity::from_json))
    }

    /// Writes the notification.
    #[must_use]
    pub fn render(&self) -> RawJson {
        self.write(|data| entity_array(data, SimplifiedEntity::to_object))
    }
}

impl Notification<FeatureCollection<Entity<Output>>> {
    /// Reads a notification whose `data` is a `FeatureCollection` of normalized or concise entities (endpoint
    /// `accept` `application/geo+json`, Table 5.3.1-1).
    ///
    /// # Errors
    ///
    /// Returns [`DocumentError`] naming the member at fault, a `FeatureCollection` error under `data`.
    pub fn parse(text: &str, form: Lossless) -> Result<Self, DocumentError<ReadErrorKind<Infallible>>> {
        Self::read(text, |data| collection(data, |entity| read_entity::<Output, Complete>(entity, form)))
    }

    /// Writes the notification with its entities in `form`.
    #[must_use]
    pub fn render(&self, form: Lossless) -> RawJson {
        self.write(|data| Json::Object(collection_object(data, |entity| entity_object(entity, form))))
    }
}

impl Notification<FeatureCollection<SimplifiedEntity>> {
    /// Reads a notification whose `data` is a `FeatureCollection` of simplified entities (endpoint `accept`
    /// `application/geo+json` with `format` simplified, Table 5.3.1-1).
    ///
    /// # Errors
    ///
    /// Returns [`DocumentError`] naming the member at fault, a `FeatureCollection` error under `data`.
    pub fn parse(text: &str) -> Result<Self, DocumentError<ReadErrorKind<Infallible>>> {
        Self::read(text, |data| collection(data, SimplifiedEntity::from_json))
    }

    /// Writes the notification.
    #[must_use]
    pub fn render(&self) -> RawJson {
        self.write(|data| Json::Object(collection_object(data, SimplifiedEntity::to_object)))
    }
}

#[cfg(feature = "linked-data")]
impl<T: MapNames> MapNames for Notification<T> {
    fn map_names<M: NameMapping>(self, mapping: &M) -> Result<Self, DocumentError<M::Error>> {
        Ok(Self {
            data: member(self.data, "data", mapping)?,
            ..self
        })
    }
}

#[cfg(feature = "linked-data")]
carries_context!(
    impl<T> for Notification<T> => context
);

#[cfg(test)]
mod tests;
