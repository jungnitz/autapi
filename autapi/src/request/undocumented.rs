use http::request::Parts;

use crate::{
    Registry, Undocumented, UndocumentedAxum,
    openapi::Operation,
    request::{FromRequest, FromRequestParts, Request},
};

impl<S, T> FromRequest<S> for Undocumented<T>
where
    T: FromRequest<S>,
    S: Send + Sync,
{
    type Rejection = Undocumented<T::Rejection>;

    fn openapi(_: &mut Operation, _: &mut Registry) {}

    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        T::from_request(request, state)
            .await
            .map(Self)
            .map_err(Undocumented)
    }
}

impl<S, T> FromRequestParts<S> for Undocumented<T>
where
    T: FromRequestParts<S>,
    S: Send + Sync,
{
    type Rejection = Undocumented<T::Rejection>;

    fn openapi(_: &mut Operation, _: &mut Registry) {}

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        T::from_request_parts(parts, state)
            .await
            .map(Self)
            .map_err(Undocumented)
    }
}

impl<S, T> FromRequest<S> for UndocumentedAxum<T>
where
    T: axum::extract::FromRequest<S>,
    T::Rejection: Send,
    S: Send + Sync,
{
    type Rejection = UndocumentedAxum<T::Rejection>;

    fn openapi(_: &mut Operation, _: &mut Registry) {}

    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        T::from_request(request, state)
            .await
            .map(Self)
            .map_err(UndocumentedAxum)
    }
}

impl<S, T> FromRequestParts<S> for UndocumentedAxum<T>
where
    T: axum::extract::FromRequestParts<S>,
    T::Rejection: Send,
    S: Send + Sync,
{
    type Rejection = UndocumentedAxum<T::Rejection>;

    fn openapi(_: &mut Operation, _: &mut Registry) {}

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        T::from_request_parts(parts, state)
            .await
            .map(Self)
            .map_err(UndocumentedAxum)
    }
}
