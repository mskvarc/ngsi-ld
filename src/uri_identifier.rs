//! Identifiers of the documents a broker stores besides entities: subscriptions, notifications and context source
//! registrations, each a URI (NGSI-LD clauses 5.2.9, 5.2.12 and 5.3).

/// Generates a URI identifier newtype with its literal and string conversions; no derive converts through the
/// wrapped [`Uri`](crate::model::Uri).
///
/// `uri_identifier!(/// docs\n Name)` expands to a public tuple struct over `Uri` deriving the common traits,
/// `Display` and `FromStr`, with `as_uri`, `as_str`, `From<Iri<&str>>` for a literal checked by `iri_rs::iri!`,
/// and `TryFrom<&str>`.
macro_rules! uri_identifier {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(
            Debug,
            Clone,
            PartialEq,
            Eq,
            Hash,
            derive_more::Display,
            derive_more::From,
            derive_more::Into,
            derive_more::AsRef,
            derive_more::FromStr,
        )]
        pub struct $name($crate::model::Uri);

        impl $name {
            /// Returns the identifier as a URI.
            #[must_use]
            pub const fn as_uri(&self) -> &$crate::model::Uri {
                &self.0
            }

            /// Returns the identifier as written.
            #[must_use]
            pub fn as_str(&self) -> &str {
                self.0.as_str()
            }
        }

        /// A literal checked at compile time by `iri_rs::iri!`.
        impl From<iri_rs::Iri<&str>> for $name {
            fn from(iri: iri_rs::Iri<&str>) -> Self {
                Self($crate::model::Uri::from(iri))
            }
        }

        impl TryFrom<&str> for $name {
            type Error = $crate::model::UriError;

            fn try_from(value: &str) -> Result<Self, Self::Error> {
                value.parse()
            }
        }
    };
}
