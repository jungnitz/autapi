use axum::{
    extract::{
        FromRequestParts as AxumFromRequestParts, Path as AxumPath,
        rejection::PathRejection as AxumPathRejection,
    },
    response::IntoResponse as _,
};
use http::request::Parts;

use crate::{
    Registry,
    adapters::SerdeAdapter,
    openapi::{Operation, ParameterIn, Responses},
    request::{FromRequestParts, utils::add_parameters_to_operation},
    response::IntoResponse,
    schema::SchemaDeserialize,
};

/// Extract segments from the path of the requested URL.
///
/// The type parameter must have an object schema.
/// The schemas for the individual path segments are then retrieved as the properties of this object
/// schema.
///
/// ### Example
/// ```
/// use autapi::{endpoint::endpoint, request::Path, schema::ToSchema};
///
/// #[derive(ToSchema)]
/// struct UsersPath {
///     user_id: i64,
/// }
///
/// #[endpoint(method = GET, path = "/users/{user_id}")]
/// async fn get_user(Path(UsersPath { user_id }): Path<UsersPath>) {
///     // ...
/// }
/// ```
///
/// ### Axum counterpart
/// This is a slim wrapper around [`axum::extract::Path`].
/// It uses [`SchemaDeserialize`] instead of serde's `Deserialize` and provides OpenAPI integration.
pub struct Path<T>(pub T);

pub struct PathRejection(AxumPathRejection);

impl<T, S> FromRequestParts<S> for Path<T>
where
    T: SchemaDeserialize + Send,
    S: Send + Sync + 'static,
{
    type Rejection = PathRejection;

    fn openapi(operation: &mut Operation, registry: &mut Registry) {
        add_parameters_to_operation::<T>(operation, registry, ParameterIn::Path, "path");
    }

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        <AxumPath<SerdeAdapter<T>> as AxumFromRequestParts<S>>::from_request_parts(parts, state)
            .await
            .map(|query| Self(query.0.0))
            .map_err(PathRejection)
    }
}

impl IntoResponse for PathRejection {
    fn openapi(_: &mut Registry) -> Responses {
        Responses::default()
    }

    fn into_response(self) -> crate::response::Response {
        self.0.into_response()
    }
}
