//! Choices the specification writes as an optional JSON boolean, where an absent member and an explicit `false`
//! both have to survive a round trip: `localOnly`, `splitEntities` and `isActive` of a subscription, `showChanges`
//! and `sysAttrs` of its notification, and the `local` and `splitEntities` query parameters.

/// Implements the boolean conversions of a three-way choice enum with an `Unspecified` variant.
///
/// `flag_choice!(Choice, On, Off)` expands to an inherent `impl Choice` with two `pub(crate)` `const fn`s:
/// `flag(self) -> Option<bool>`, which maps `Unspecified` to `None`, `On` to `Some(true)` and `Off` to
/// `Some(false)`, and `from_flag(bool) -> Choice`, its inverse for a written boolean.
macro_rules! flag_choice {
    ($choice:ident, $on:ident, $off:ident) => {
        impl $choice {
            /// The boolean written for the choice, `None` when it is not stated.
            pub(crate) const fn flag(self) -> Option<bool> {
                match self {
                    Self::Unspecified => None,
                    Self::$on => Some(true),
                    Self::$off => Some(false),
                }
            }

            /// The choice a written boolean states.
            pub(crate) const fn from_flag(flag: bool) -> Self {
                if flag { Self::$on } else { Self::$off }
            }
        }
    };
}
