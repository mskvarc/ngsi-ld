//! Subscriptions whose URLs are written as the caller gave them.

use crate::{
    model::{Input, UrlDraft},
    subscription::{Endpoint, NotificationParams, Subscription, SubscriptionFragment},
};
use pretty_assertions::assert_eq;

fn subscription(endpoint: UrlDraft, jsonld_context: UrlDraft) -> Subscription<Input> {
    let endpoint = Endpoint::builder(endpoint).build().unwrap();
    Subscription::builder()
        .watched_attributes(["speed"])
        .notification(NotificationParams::builder(endpoint).build().unwrap())
        .jsonld_context(jsonld_context)
        .build()
        .unwrap()
}

#[test]
fn a_subscription_endpoint_and_context_given_verbatim_are_written_as_given() {
    let verbatim = subscription(UrlDraft::verbatim("http://localhost:8080"), UrlDraft::verbatim("https://example.org"));
    let text = verbatim.render().to_string();
    assert!(text.contains(r#""endpoint":{"uri":"http://localhost:8080""#), "{text}");
    assert!(text.contains(r#""jsonldContext":"https://example.org""#), "{text}");
    assert_eq!(verbatim.jsonld_context().map(url::Url::as_str), Some("https://example.org/"));
    assert_eq!(verbatim.notification().endpoint().uri().as_str(), "http://localhost:8080/");
}

#[test]
fn subscriptions_differing_only_in_the_spelling_of_their_urls_are_equal() {
    let verbatim = subscription(UrlDraft::verbatim("http://localhost:8080"), UrlDraft::verbatim("https://example.org"));
    let canonical = subscription("http://localhost:8080".into(), "https://example.org".into());
    assert_eq!(verbatim, canonical);
    assert!(canonical.render().to_string().contains(r#""jsonldContext":"https://example.org/""#));
}

#[test]
fn a_subscription_fragment_context_given_verbatim_is_written_as_given() {
    let fragment = SubscriptionFragment::builder()
        .jsonld_context(UrlDraft::verbatim("https://example.org"))
        .build()
        .unwrap();
    assert_eq!(
        fragment.render().to_string(),
        r#"{"type":"Subscription","jsonldContext":"https://example.org"}"#
    );
}
