use std::{
    convert::Infallible,
    ops::{Deref, DerefMut},
};

use http::request::Parts;

use crate::{Registry, openapi::Operation, request::FromRequestParts};

/// Extraction of application state.
///
/// The extracted state must be provided after endpoint registration via
/// [`Router::<S>::with_state`](crate::Router::with_state).
pub struct State<S>(pub S);

impl<S> FromRequestParts<S> for State<S>
where
    S: Send + Sync + Clone,
{
    type Rejection = Infallible;

    fn openapi(_: &mut Operation, _: &mut Registry) {}

    async fn from_request_parts(_: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        Ok(Self(state.clone()))
    }
}

impl<S> Deref for State<S> {
    type Target = S;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<S> DerefMut for State<S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
