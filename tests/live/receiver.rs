//! A receiver of notifications in the test process, which the broker reaches from its container (clause 6.3.8).

use std::{env, net::TcpListener, str, time::Duration};
use tokio::time::sleep;
use wiremock::{Mock, MockServer, Request, ResponseTemplate, matchers::method};

/// The environment variable naming the host under which the broker reaches the test process.
const NOTIFICATION_HOST_VARIABLE: &str = "NGSI_LD_TEST_NOTIFICATION_HOST";

/// A receiver of notifications listening on every interface, so the broker reaches it from its container.
pub(crate) async fn receiver() -> MockServer {
    let listener = TcpListener::bind("0.0.0.0:0").unwrap();
    let server = MockServer::builder().listener(listener).start().await;
    Mock::given(method("POST")).respond_with(ResponseTemplate::new(200)).mount(&server).await;
    server
}

/// The URL under which the broker reaches `receiver`.
pub(crate) fn notification_uri(receiver: &MockServer) -> String {
    let host = env::var(NOTIFICATION_HOST_VARIABLE).expect("NGSI_LD_TEST_NOTIFICATION_HOST names the host under which the broker reaches the tests");
    format!("http://{host}:{}/notify", receiver.address().port())
}

/// Waits up to 30 seconds for a notification `read` accepts, and returns it with its request; or, when none
/// arrives, the bodies of every request the receiver got.
pub(crate) async fn notification<T>(receiver: &MockServer, read: impl Fn(&str) -> Option<T>) -> Result<(T, Request), Vec<String>> {
    for _ in 0..60 {
        for request in receiver.received_requests().await.unwrap_or_default() {
            if let Some(found) = str::from_utf8(&request.body).ok().and_then(&read) {
                return Ok((found, request));
            }
        }
        sleep(Duration::from_millis(500)).await;
    }
    let requests = receiver.received_requests().await.unwrap_or_default();
    Err(requests.iter().map(|request| String::from_utf8_lossy(&request.body).into_owned()).collect())
}
