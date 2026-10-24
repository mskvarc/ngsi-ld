//! The members of an entity that are not attributes (NGSI-LD clauses 4.5.1, 4.5.4 and 5.2.2): the ones a reader
//! recognises by name before it reads any other member as an attribute.

/// A structural member of an entity (clause 4.5.1). The specification closes the set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::EnumString, strum::IntoStaticStr)]
pub(crate) enum EntityMember {
    /// `id`.
    #[strum(serialize = "id")]
    Id,
    /// `type`.
    #[strum(serialize = "type")]
    Type,
    /// `scope` (clause 4.18).
    #[strum(serialize = "scope")]
    Scope,
    /// `expiresAt` (clause 4.22).
    #[strum(serialize = "expiresAt")]
    ExpiresAt,
    /// `@context` (clause 4.4).
    #[strum(serialize = "@context")]
    Context,
}

/// A member of an entity only a broker writes (clause 5.2.2). The specification closes the set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::EnumString, strum::IntoStaticStr)]
pub(crate) enum SystemMember {
    /// `createdAt`.
    #[strum(serialize = "createdAt")]
    Created,
    /// `modifiedAt`.
    #[strum(serialize = "modifiedAt")]
    Modified,
    /// `deletedAt`.
    #[strum(serialize = "deletedAt")]
    Deleted,
}

impl EntityMember {
    /// Returns the member as it is spelled in a document.
    pub(crate) fn as_str(self) -> &'static str {
        self.into()
    }
}

impl SystemMember {
    /// Returns the member as it is spelled in a document.
    pub(crate) fn as_str(self) -> &'static str {
        self.into()
    }
}
