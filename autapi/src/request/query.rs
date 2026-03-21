use axum::{extract::FromRequestParts as AxumFromRequestParts, response::IntoResponse as _};
use axum_extra::extract::{Query as AxumQuery, QueryRejection as AxumQueryRejection};
use http::request::Parts;

use crate::{
    Registry,
    adapters::SerdeAdapter,
    openapi::{Operation, ParameterIn, Responses},
    request::{FromRequestParts, utils::add_parameters_to_operation},
    response::{IntoResponse, Response},
    schema::SchemaDeserialize,
};

/// Extract data from query strings.
///
/// The type parameter must have an object schema.
/// The schemas for the individual query parameters are then retrieved as the properties of this
/// object schema.
///
/// ### Example
/// ```
/// use autapi::{endpoint::endpoint, request::Query, schema::ToSchema};
///
/// #[derive(ToSchema)]
/// struct UsersQuery {
///     page: i64,
///     /// Requires at least one element
///     user_ids: Vec<i64>,
///     /// Optional list, inner `Vec` will contain at least one element
///     group_ids: Option<Vec<i64>>,
/// }
///
/// #[endpoint(method = GET, path = "/users")]
/// async fn get_users(Query(UsersQuery { page, user_ids, group_ids }): Query<UsersQuery>) {
///     // ...
/// }
/// ```
///
/// ### Axum counterpart
/// This is a slim wrapper around [`axum_extra::extract::Query`].
/// It uses [`SchemaDeserialize`] instead of serde's `Deserialize` and provides OpenAPI integration.
pub struct Query<T>(pub T);

pub struct QueryRejection(AxumQueryRejection);

impl<S, T> FromRequestParts<S> for Query<T>
where
    T: SchemaDeserialize,
    S: Send + Sync + 'static,
{
    type Rejection = QueryRejection;

    fn openapi(operation: &mut Operation, registry: &mut Registry) {
        add_parameters_to_operation::<T>(operation, registry, ParameterIn::Query, "query");
    }

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        <AxumQuery<SerdeAdapter<T>> as AxumFromRequestParts<S>>::from_request_parts(parts, state)
            .await
            .map(|query| Self(query.0.0))
            .map_err(QueryRejection)
    }
}

impl IntoResponse for QueryRejection {
    fn openapi(_: &mut Registry) -> Responses {
        Responses::default()
    }

    fn into_response(self) -> Response {
        self.0.into_response()
    }
}
