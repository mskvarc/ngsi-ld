//! A client stating the `@context` kept in a local file.

use crate::{
    client::{Answering, Client, Transport},
    context_file::{self, ContextFileError},
    http::{ContextDelivery, EndpointError},
};
use std::path::Path;

/// Why a client could not state the `@context` of a file.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ContextFileClientError {
    /// The file could not be read into a self-contained context.
    #[error("the context file could not be read")]
    File(#[from] ContextFileError),
    /// The context cannot be stated on requests.
    #[error("the context cannot be stated on requests")]
    Endpoint(#[from] EndpointError),
}

impl<T: Transport + Clone, M: Answering> Client<T, M> {
    /// A client stating the `@context` of the JSON-LD document at `path` (`{"@context": ...}`), read with
    /// [`context_file::load`], which inlines every local file it references. Reading is synchronous: call it when the
    /// client is set up, or on a blocking pool from code that must not block. A context that is one absolute reference
    /// travels in the `Link` header as this client's would; any other context travels whole as the `@context` of an
    /// `application/ld+json` body (clause 6.3.5).
    ///
    /// ```no_run
    /// # use bytes::Bytes;
    /// # use http::{Request, Response};
    /// # use std::convert::Infallible;
    /// # #[derive(Clone)]
    /// # struct Offline;
    /// # impl ngsi_ld::client::Transport for Offline {
    /// #     type Error = Infallible;
    /// #     async fn send(&self, _: Request<Bytes>) -> Result<Response<Bytes>, Infallible> { Ok(Response::new(Bytes::new())) }
    /// # }
    /// use ngsi_ld::client::Client;
    /// use url::Url;
    ///
    /// let client = Client::builder().base_url(Url::parse("http://localhost:9090/ngsi-ld/v1/")?).transport(Offline).build()?;
    /// let fleet = client.with_context_file("contexts/fleet.jsonld")?;
    /// # let _ = fleet;
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`ContextFileClientError`] when the file cannot be read into a self-contained context, or the context
    /// cannot be stated on requests.
    pub fn with_context_file(&self, path: impl AsRef<Path>) -> Result<Client<T, M>, ContextFileClientError> {
        let context = context_file::load(path)?;
        let linkable = context.link_target().is_some();
        let client = self.with_context(context)?;
        Ok(if linkable {
            client
        } else {
            client.with_context_delivery(ContextDelivery::LdJsonBody)
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        client::{Client, ContextFileClientError, Transport},
        http::{ContextDelivery, EndpointError},
    };
    use bytes::Bytes;
    use http::{Request, Response};
    use iri_rs::IriBuf;
    use std::{convert::Infallible, env, fs, future, path::PathBuf, process};
    use url::Url;

    /// A transport that is never asked to send.
    #[derive(Clone)]
    struct Offline;

    impl Transport for Offline {
        type Error = Infallible;

        fn send(&self, _: Request<Bytes>) -> impl Future<Output = Result<Response<Bytes>, Infallible>> + Send {
            future::ready(Ok(Response::new(Bytes::new())))
        }
    }

    fn client() -> Client<Offline> {
        Client::builder()
            .base_url(Url::parse("http://broker.test/ngsi-ld/v1/").unwrap())
            .transport(Offline)
            .build()
            .unwrap()
    }

    /// Writes `document` to a file of its own for the test `label`.
    fn file(label: &str, document: &str) -> PathBuf {
        let directory = env::temp_dir().join(format!("ngsi-ld-client-context-file-{}-{label}", process::id()));
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("context.jsonld");
        fs::write(&path, document).unwrap();
        path
    }

    #[test]
    fn one_absolute_reference_keeps_the_link_header_and_anything_else_travels_in_the_body() {
        let linked = client()
            .with_context_file(file("linked", r#"{"@context": "https://example.org/fleet.jsonld"}"#))
            .unwrap();
        assert_eq!(linked.endpoint().context_delivery(), ContextDelivery::JsonWithLink);
        let inline = client()
            .with_context_file(file(
                "inline",
                r#"{"@context": [{"speed": "https://example.org/speed"}, "https://example.org/core.jsonld"]}"#,
            ))
            .unwrap();
        assert_eq!(inline.endpoint().context_delivery(), ContextDelivery::LdJsonBody);
    }

    #[test]
    fn a_context_profile_refuses_a_file_whose_context_no_link_carries() {
        let profiled = client()
            .with_context_profile(IriBuf::new("https://uri.etsi.org/ngsi-ld/profile/version").unwrap())
            .unwrap();
        let error = profiled
            .with_context_file(file("profiled", r#"{"@context": [{"speed": "https://example.org/speed"}]}"#))
            .err();
        assert!(matches!(
            error,
            Some(ContextFileClientError::Endpoint(EndpointError::ProfileWithoutContextLink))
        ));
    }
}
