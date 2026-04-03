use std::{borrow::Cow, mem};

use axum::routing::{MethodFilter, MethodRouter};

use crate::{
    Registry, adapters::AxumHandlerAdapter, endpoint::Endpoint, openapi::OpenApi, rapidoc,
};

/// Wrapper around axum's `Router`, allowing registration of endpoints.
pub struct Router<S = ()> {
    axum: axum::Router<S>,
    registry: Registry,
    serve_spec_at: Vec<String>,
    #[expect(clippy::type_complexity)]
    modify_openapi: Vec<Box<dyn FnOnce(&mut Registry, &S) + Send + Sync + 'static>>,
}

impl<S> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    pub fn endpoint<E: Endpoint<S, V>, V: 'static>(mut self, endpoint: E) -> Self {
        self.endpoint_with_base("", endpoint);
        self
    }
    fn endpoint_with_base<E: Endpoint<S, V>, V: 'static>(&mut self, base: &str, endpoint: E) {
        let path = make_path(base, endpoint.path().as_ref());
        let method = endpoint.method();
        let filter = MethodFilter::try_from(endpoint.method())
            .expect("a matching method filter should exist");
        let operation = endpoint
            .openapi(&mut self.registry)
            .with_operation_id(endpoint.operation_id());
        self.modify_axum(|router| {
            router.route(
                path.as_ref(),
                MethodRouter::default().on(filter, AxumHandlerAdapter(endpoint)),
            )
        });
        let operation_entry = self
            .registry
            .openapi_mut()
            .paths
            .paths
            .entry(path.clone())
            .or_default()
            .operation_by_method_mut(method.clone())
            .expect("a matching operation entry should exist in PathItem");
        if operation_entry.is_some() {
            panic!("colliding operations for path {path:?} and method {method}");
        }
        *operation_entry = Some(operation);
    }
    pub fn nest<'r>(&'r mut self, base: &'r str) -> NestedRouter<'r, S> {
        NestedRouter {
            router: self,
            base: base.into(),
        }
    }
    pub fn with_state(self, state: S) -> Router {
        let cloned_state = state.clone();
        Router {
            axum: self.axum.with_state(state),
            serve_spec_at: self.serve_spec_at,
            registry: self.registry,
            modify_openapi: vec![Box::new(move |openapi, _| {
                for modifier in self.modify_openapi {
                    modifier(openapi, &cloned_state);
                }
            })],
        }
    }
    pub fn modify_axum(&mut self, modifier: impl FnOnce(axum::Router<S>) -> axum::Router<S>) {
        self.axum = modifier(mem::take(&mut self.axum));
    }
    pub fn registry_mut(&mut self) -> &mut Registry {
        &mut self.registry
    }
    pub fn modify_openapi(
        &mut self,
        modifier: impl FnOnce(&mut Registry, &S) + Send + Sync + 'static,
    ) {
        self.modify_openapi.push(Box::new(modifier));
    }
    pub fn serve_docs(mut self, path: &str) -> Self {
        let serve_spec_at = make_path(path, "openapi.json");
        self.modify_axum(|router| {
            router.route(
                path,
                axum::routing::get(rapidoc::RapiDoc {
                    spec_url: serve_spec_at.clone(),
                }),
            )
        });
        self.serve_spec_at.push(serve_spec_at);
        self
    }
}

impl Router {
    pub fn into_parts(mut self) -> (axum::Router, OpenApi) {
        for modifier in self.modify_openapi {
            modifier(&mut self.registry, &());
        }
        let openapi = self.registry.into_openapi();

        let mut axum = self.axum;
        for spec_at in self.serve_spec_at {
            axum = axum.route(&spec_at, axum::routing::get(axum::Json(openapi.clone())));
        }
        (axum, openapi)
    }

    #[cfg(all(feature = "tokio", any(feature = "http1", feature = "http2")))]
    pub fn serve<L>(self, listener: L) -> axum::serve::Serve<L, axum::Router, axum::Router>
    where
        L: axum::serve::Listener,
    {
        axum::serve(listener, self.into_parts().0)
    }
}

impl<S: Clone + Send + Sync + 'static> Default for Router<S> {
    fn default() -> Self {
        Self {
            axum: Default::default(),
            registry: Default::default(),
            serve_spec_at: Default::default(),
            modify_openapi: Default::default(),
        }
    }
}

/// Configure endpoints for a router starting with a path.
pub struct NestedRouter<'r, S = ()> {
    router: &'r mut Router<S>,
    base: Cow<'r, str>,
}

impl<'r, S> NestedRouter<'r, S>
where
    S: Clone + Send + Sync + 'static,
{
    pub fn endpoint<E: Endpoint<S, V>, V: 'static>(self, endpoint: E) -> Self {
        self.router.endpoint_with_base(&self.base, endpoint);
        self
    }
    pub fn nest(&mut self, base: &str) -> NestedRouter<'_, S> {
        NestedRouter {
            router: self.router,
            base: make_path(&self.base, base).into(),
        }
    }
    pub fn into_nested(self, base: &str) -> NestedRouter<'r, S> {
        NestedRouter {
            router: self.router,
            base: make_path(&self.base, base).into(),
        }
    }
}

fn make_path(base: &str, path: &str) -> String {
    if base.is_empty() {
        format!("/{}", path.trim_start_matches('/'))
    } else {
        format!(
            "/{}/{}",
            base.trim_end_matches('/').trim_start_matches("/"),
            path.trim_start_matches('/')
        )
    }
}
