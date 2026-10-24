//! The operations that delete one resource named by its identifier and are answered 204 with no body: Delete
//! Temporal Evolution, Delete Context Source Registration, Delete Snapshot and Delete `EntityMap` (NGSI-LD clauses
//! 6.19.3.2, 6.9.3.3, 6.37.3.3 and 6.32.3.3).

/// Implements the operation of a deletion: `delete_resource!(Operation => ResourcePath::path)` expands to the
/// sealing, an [`Operation`](crate::http::operation::Operation) whose only success is 204 with no body, and a
/// request `DELETE` of the resource `path(self.id)` with no query string.
macro_rules! delete_resource {
    ($operation:ident => $path:expr) => {
        impl $crate::http::operation::sealed::Sealed for $operation<'_> {}

        impl $crate::http::operation::Operation for $operation<'_> {
            type Outcome = ();

            fn decode(&self, _: &$crate::http::ResponseContext, response: ::http::Response<::bytes::Bytes>) -> Result<(), $crate::http::ResponseError> {
                $crate::http::operation::no_content::decode_no_content(response)
            }
        }

        impl $crate::http::operation::sealed::BuildsRequest for $operation<'_> {
            fn scoped_request(
                &self,
                endpoint: &$crate::http::Endpoint,
                scope: $crate::http::request::scope::RequestScope<'_>,
            ) -> Result<::http::Request<::bytes::Bytes>, $crate::http::RequestError> {
                $crate::http::request::RequestParts::resource(
                    ::http::Method::DELETE,
                    ($path)(self.id),
                    $crate::http::query_string::QueryString::default(),
                )
                .into_request(endpoint, scope)
            }
        }
    };
}
