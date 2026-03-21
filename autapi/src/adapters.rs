use std::pin::Pin;

use serde::{Deserialize, Serialize, Serializer, ser::Error};

use crate::{
    endpoint::Endpoint,
    request::Request,
    response::Response,
    schema::{SchemaDeserialize, SchemaSerialize},
};

/// Allows (de-)serializing a [`SchemaDeserialize`] or [`SchemaSerialize`] using standard serde
/// (de-)serializers.
#[derive(Debug)]
pub struct SerdeAdapter<T>(pub T);

impl<T> SerdeAdapter<T>
where
    T: SchemaSerialize,
{
    pub fn skip_serializing(&self) -> bool {
        !self.0.is_present()
    }
}

impl<T> Serialize for SerdeAdapter<T>
where
    T: SchemaSerialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if !self.0.is_present() {
            return Err(S::Error::custom("cannot serialize an absent value"));
        }
        self.0.schema_serialize(serializer)
    }
}

impl<'de, T> Deserialize<'de> for SerdeAdapter<T>
where
    T: SchemaDeserialize,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Self(T::schema_deserialize(deserializer)?))
    }
}

/// Wraps an [`Endpoint`] and implements axum's [`Handler`](axum::handler::Handler) trait.
#[derive(Clone)]
pub struct AxumHandlerAdapter<E>(pub E);

impl<V: 'static, S: 'static, E: Endpoint<S, V>> axum::handler::Handler<V, S>
    for AxumHandlerAdapter<E>
{
    type Future = Pin<Box<dyn Future<Output = Response> + Send + 'static>>;

    fn call(self, req: Request, state: S) -> Self::Future {
        Box::pin(self.0.call(req, state))
    }
}

impl<E, V> Endpoint<E, V> for AxumHandlerAdapter<E>
where
    E: Endpoint<E, V>,
{
    fn path(&self) -> std::borrow::Cow<'static, str> {
        self.0.path()
    }

    fn method(&self) -> http::Method {
        self.0.method()
    }

    fn openapi(&self, registry: &mut crate::Registry) -> crate::openapi::Operation {
        self.0.openapi(registry)
    }

    fn call(self, req: Request, state: E) -> impl Future<Output = Response> + Send {
        self.0.call(req, state)
    }
}
