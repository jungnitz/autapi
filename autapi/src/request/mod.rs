mod json;
mod state;
mod undocumented;
mod utils;

mod path;
pub use path::{Path, PathRejection};

#[cfg(feature = "query")]
mod query;
#[cfg(feature = "query")]
pub use query::{Query, QueryRejection};

pub use self::state::State;

use http::request::Parts;

use crate::{Registry, openapi::Operation, response::IntoResponse};

pub type Request = axum::extract::Request;

/// Data that can be derived from a request.
pub trait FromRequest<S, V = via::Request>: Sized {
    type Rejection: IntoResponse;

    fn openapi(operation: &mut Operation, registry: &mut Registry);
    fn from_request(
        request: Request,
        state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send;
}

/// Data that can be derived from request parts.
pub trait FromRequestParts<S>: Sized {
    type Rejection: IntoResponse;

    fn openapi(operation: &mut Operation, registry: &mut Registry);
    fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send;
}

impl<S, T> FromRequest<S, via::Parts> for T
where
    T: FromRequestParts<S>,
    S: Send + Sync,
{
    type Rejection = T::Rejection;

    fn openapi(operation: &mut Operation, registry: &mut Registry) {
        T::openapi(operation, registry);
    }

    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        let mut parts = request.into_parts().0;
        T::from_request_parts(&mut parts, state).await
    }
}

mod via {
    pub enum Parts {}
    pub enum Request {}
}
