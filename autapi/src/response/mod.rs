mod json;
mod status_codes;
mod text;
mod undocumented;

use std::convert::Infallible;

pub use self::{json::*, status_codes::*, text::*};
use crate::{Registry, openapi::Responses};

pub type Response = axum::response::Response;

/// A type that is convertible to an HTTP response with OpenAPI description.
///
/// Most notably, this trait is implemented for
/// - `T: ToSchema + SchemaSerialize`, returning a `200 Ok` response with
///   `Content-Type: application/json`
/// - `Result<T, E>` with `T: IntoResponse` and `E: IntoResponse`
/// - The new-type structs in this module named after status codes (e.g. [`Created`])
pub trait IntoResponse {
    /// Describes the responses generated via the [`into_response`](Self::into_response) function.
    fn openapi(registry: &mut Registry) -> Responses;
    /// Converts this value into an axum response.
    fn into_response(self) -> Response;
}

impl<T: IntoResponse, E: IntoResponse> IntoResponse for Result<T, E> {
    fn openapi(registry: &mut Registry) -> Responses {
        let mut responses = T::openapi(registry);
        responses.merge_with(E::openapi(registry));
        responses
    }

    fn into_response(self) -> Response {
        match self {
            Self::Ok(t) => t.into_response(),
            Self::Err(e) => e.into_response(),
        }
    }
}

impl IntoResponse for Infallible {
    fn openapi(_: &mut Registry) -> Responses {
        Default::default()
    }
    fn into_response(self) -> Response {
        match self {}
    }
}
