use std::{borrow::Cow, convert::Infallible, marker::PhantomData};

use axum::handler::{Handler, HandlerService};
use tower::{Layer, Service, ServiceExt};

#[cfg(doc)]
use crate::Router;
use crate::{
    Registry, adapters::AxumHandlerAdapter, openapi::Operation, request::Request,
    response::Response,
};

/// Create an [`Endpoint`] implementation based on a handler function.
///
/// ```rust
/// use autapi::endpoint::endpoint;
///
/// #[endpoint(method=GET, path="/")]
/// async fn hello_world() -> String {
///     String::from("Hello World!")
/// }
/// ```
pub use autapi_macros::endpoint;

/// Describes an API endpoint through its method, path and operation details.
///
/// This trait is usually implemented via the [`endpoint`] macro.
pub trait Endpoint<S, V>: Clone + Send + Sync + Sized + 'static {
    /// Returns the path of this endpoint, possibly containing path parameters.
    ///
    /// The returned value is used during registration of the endpoint with the [`Router`].
    /// Therefore, it should not change during the lifetime of a value implementing this trait.
    fn path(&self) -> Cow<'static, str>;

    /// Returns the method of this endpoint.
    ///
    /// The returned value is used during registration of the endpoint with the [`Router`].
    /// Therefore, it should not change during the lifetime of a value implementing this trait.
    fn method(&self) -> http::Method;

    /// Generates the OpenAPI description for this endpoint.
    fn openapi(&self, registry: &mut Registry) -> Operation;

    /// Handle a request directed at this endpoint.
    fn call(self, req: Request, state: S) -> impl Future<Output = Response> + Send;

    fn layer_undocumented<L>(self, layer: L) -> Layered<L, Self, S, V> {
        Layered {
            layer,
            endpoint: self,
            _pd: PhantomData,
        }
    }
}

/// An [`Endpoint`] wrapped in a tower [`Layer`].
///
/// Returned from [`Endpoint::layer_undocumented`].
pub struct Layered<L, E, S, V> {
    layer: L,
    endpoint: E,
    _pd: PhantomData<fn(&S) -> V>,
}

impl<L, E, S, V> Clone for Layered<L, E, S, V>
where
    L: Clone,
    E: Clone,
{
    fn clone(&self) -> Self {
        Self {
            layer: self.layer.clone(),
            endpoint: self.endpoint.clone(),
            _pd: self._pd,
        }
    }
}

impl<L, E, S, V> Endpoint<S, V> for Layered<L, E, S, V>
where
    E: Endpoint<S, V>,
    L: Layer<HandlerService<AxumHandlerAdapter<E>, V, S>> + Clone + Send + Sync + 'static,
    L::Service: Service<Request, Error = Infallible> + Clone + Send + 'static,
    <L::Service as Service<Request>>::Response: axum::response::IntoResponse,
    <L::Service as Service<Request>>::Future: Send,
    E: 'static,
    S: Send + 'static,
    V: 'static,
{
    fn path(&self) -> Cow<'static, str> {
        self.endpoint.path()
    }
    fn method(&self) -> http::Method {
        self.endpoint.method()
    }
    fn openapi(&self, registry: &mut Registry) -> Operation {
        self.endpoint.openapi(registry)
    }
    async fn call(self, req: Request, state: S) -> Response {
        let handler = AxumHandlerAdapter(self.endpoint).with_state(state);
        let service = self.layer.layer(handler);
        match service.oneshot(req).await {
            Ok(res) => axum::response::IntoResponse::into_response(res),
            Err(err) => match err {},
        }
    }
}
