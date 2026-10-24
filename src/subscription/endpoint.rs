//! Where and how notifications are delivered (NGSI-LD clauses 5.2.15 and 5.2.22).

use crate::{
    model::{BuildError, KeyValuePair, NgsiLdMediaType, Number, PositiveNumber, UrlDraft, value::positive_number::resolve_positive},
    subscription::{EndpointMember, Problem, SubscriptionField},
};
use bon::bon;
use jstrict::{Object, Value as Json};
use url::Url;

/// The endpoint a notification is delivered to, and how (NGSI-LD clause 5.2.15).
///
/// ```
/// use ngsi_ld::{model::NgsiLdMediaType, subscription::Endpoint};
///
/// let endpoint = Endpoint::builder("http://my.endpoint.org/notify")
///     .accept(NgsiLdMediaType::JsonLd)
///     .timeout(2000)
///     .receiver_info("Authorization", "Bearer abc")
///     .build()?;
/// assert_eq!(endpoint.uri().as_str(), "http://my.endpoint.org/notify");
/// assert_eq!(endpoint.receiver_info().len(), 1);
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Endpoint {
    /// `uri`: the dereferenceable URI that receives the notifications.
    pub(crate) uri: Url,
    /// `accept`.
    pub(crate) accept: Option<NgsiLdMediaType>,
    /// `timeout`, in milliseconds.
    pub(crate) timeout: Option<PositiveNumber>,
    /// `cooldown`, in milliseconds.
    pub(crate) cooldown: Option<PositiveNumber>,
    /// `receiverInfo`.
    pub(crate) receiver_info: Vec<KeyValuePair>,
    /// `notifierInfo`.
    pub(crate) notifier_info: Vec<KeyValuePair>,
}

#[bon]
impl Endpoint {
    /// Starts a builder from the URI that receives the notifications, a [`Url`] or text validated when the
    /// endpoint is built. `receiver_info` and `notifier_info` add one pair each time they are called.
    ///
    /// # Errors
    ///
    /// `build()` returns [`BuildError`] listing a URI that is not a URL and a `timeout` or `cooldown` that is not
    /// greater than 0 (Table 5.2.15-1).
    #[builder]
    pub fn new(
        #[builder(start_fn, into)] uri: UrlDraft,
        #[builder(field)] receiver_info: Vec<KeyValuePair>,
        #[builder(field)] notifier_info: Vec<KeyValuePair>,
        accept: Option<NgsiLdMediaType>,
        #[builder(into)] timeout: Option<Number>,
        #[builder(into)] cooldown: Option<Number>,
    ) -> Result<Self, BuildError<Problem>> {
        let mut problems = Vec::new();
        let uri = uri.resolve(SubscriptionField::from(EndpointMember::Uri), &mut problems);
        let timeout = resolve_positive(SubscriptionField::from(EndpointMember::Timeout), timeout, &mut problems);
        let cooldown = resolve_positive(SubscriptionField::from(EndpointMember::Cooldown), cooldown, &mut problems);
        let endpoint = uri.map(|uri| Self {
            uri,
            accept,
            timeout,
            cooldown,
            receiver_info,
            notifier_info,
        });
        BuildError::finish_partial(problems, endpoint)
    }
}

impl<S: endpoint_builder::State> EndpointBuilder<S> {
    /// Adds a pair to `receiverInfo`: for the HTTP binding, a header the notification request carries (clause
    /// 6.3.8).
    pub fn receiver_info(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.receiver_info.push(KeyValuePair::new(key, value));
        self
    }

    /// Adds a pair to `notifierInfo`: a setting of the channel to the receiver, such as an MQTT version (clause
    /// 7.2).
    pub fn notifier_info(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.notifier_info.push(KeyValuePair::new(key, value));
        self
    }
}

impl Endpoint {
    /// Returns the URI that receives the notifications (`uri`).
    #[must_use]
    pub const fn uri(&self) -> &Url {
        &self.uri
    }

    /// Returns the media type of the notifications (`accept`); `None` means `application/json`.
    #[must_use]
    pub const fn accept(&self) -> Option<NgsiLdMediaType> {
        self.accept
    }

    /// Returns the time after which a notification counts as failed, in milliseconds (`timeout`).
    #[must_use]
    pub const fn timeout(&self) -> Option<&PositiveNumber> {
        self.timeout.as_ref()
    }

    /// Returns the time to wait after a failure before notifying again, in milliseconds (`cooldown`).
    #[must_use]
    pub const fn cooldown(&self) -> Option<&PositiveNumber> {
        self.cooldown.as_ref()
    }

    /// Returns the pairs conveyed to the receiver (`receiverInfo`).
    #[must_use]
    pub fn receiver_info(&self) -> &[KeyValuePair] {
        &self.receiver_info
    }

    /// Returns the pairs that set up the channel (`notifierInfo`).
    #[must_use]
    pub fn notifier_info(&self) -> &[KeyValuePair] {
        &self.notifier_info
    }

    /// Writes the `Endpoint` object in canonical order: `uri`, `accept`, `timeout`, `cooldown`, `receiverInfo`,
    /// `notifierInfo`.
    pub(crate) fn to_json(&self) -> Json {
        let mut object = Object::new();
        object.push(<&str>::from(EndpointMember::Uri).into(), Json::from(self.uri.as_str()));
        if let Some(accept) = self.accept {
            object.push(<&str>::from(EndpointMember::Accept).into(), Json::from(accept.to_string()));
        }
        push_number(&mut object, EndpointMember::Timeout.into(), self.timeout.as_ref());
        push_number(&mut object, EndpointMember::Cooldown.into(), self.cooldown.as_ref());
        push_pairs(&mut object, EndpointMember::ReceiverInfo.into(), &self.receiver_info);
        push_pairs(&mut object, EndpointMember::NotifierInfo.into(), &self.notifier_info);
        Json::Object(object)
    }
}

/// Appends a number member when present.
pub(crate) fn push_number(object: &mut Object, member: &str, number: Option<&PositiveNumber>) {
    if let Some(number) = number {
        object.push(member.into(), number.as_number().to_json());
    }
}

/// Appends an array of pairs when there is one.
pub(crate) fn push_pairs(object: &mut Object, member: &str, pairs: &[KeyValuePair]) {
    if !pairs.is_empty() {
        object.push(member.into(), Json::Array(pairs.iter().map(KeyValuePair::to_json).collect()));
    }
}
