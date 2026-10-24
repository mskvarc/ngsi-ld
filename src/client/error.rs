//! Why a call failed.

use crate::{
    http::{RequestError, ResponseError, ResponseHead},
    model::{ErrorType, ProblemDetails},
};

/// Why a call failed: the request was refused before sending, the broker answered with an error or with
/// something the operation does not define, or the transport failed (`E` is the transport's own error).
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ClientError<E> {
    /// The request was not sent.
    #[error("the request was refused before sending")]
    Request(#[source] RequestError),
    /// The broker's response is an error or cannot be read.
    #[error("the broker's response is an error")]
    Response(#[source] ResponseError),
    /// No response was received.
    #[error("the transport failed")]
    Transport(#[source] E),
}

impl<E> ClientError<E> {
    /// The problem the broker reported, if it reported one.
    #[must_use]
    pub const fn problem(&self) -> Option<&ProblemDetails> {
        match self {
            Self::Response(ResponseError::Problem { details, .. }) => Some(details),
            Self::Request(_) | Self::Response(_) | Self::Transport(_) => None,
        }
    }

    /// The head of the broker's response besides its status (version, headers and the transport's extensions),
    /// when the broker answered.
    #[must_use]
    pub fn head(&self) -> Option<&ResponseHead> {
        match self {
            Self::Response(error) => Some(error.head()),
            Self::Request(_) | Self::Transport(_) => None,
        }
    }

    /// The type of the problem the broker reported, if it reported one with a type.
    #[must_use]
    pub const fn error_type(&self) -> Option<&ErrorType> {
        match self.problem() {
            Some(ProblemDetails {
                error_type: Some(error_type), ..
            }) => Some(error_type),
            Some(_) | None => None,
        }
    }
}
