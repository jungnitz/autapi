#![allow(unused)]

use autapi::openapi::{Operation, SecurityRequirement};
use autapi::request::{FromRequestParts, Path};
use autapi::response::Unauthorized;
use autapi::{Registry, Router, endpoint::endpoint, info_from_env, openapi::Tag, schema::ToSchema};
use http::request::Parts;

/// Create a TODO
#[endpoint(method = POST, path = "/todos", tags = ["todo"])]
async fn create_todo(user: User, todo: Todo) {
    todo!("create todo")
}

/// Get a TODO
#[endpoint(method = GET, path = "/todos/{id}")]
async fn get_todo(user: User, Path(IdPath { id }): Path<IdPath>) -> Todo {
    todo!("check permissions and return Todo with given id")
}

#[tokio::main]
async fn main() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080")
        .await
        .unwrap();
    let mut router = Router::default();
    router.registry_mut().openapi_mut().info = info_from_env!().with_title("TODO API");
    router
        .endpoint(create_todo)
        .endpoint(get_todo)
        .serve_docs("/api-docs/")
        .serve(listener)
        .await
        .unwrap();
}

/// A TODO with title and body
#[derive(ToSchema)]
pub struct Todo {
    title: String,
    body: Option<String>,
}

#[derive(ToSchema)]
pub struct IdPath {
    id: u64,
}

pub struct User {
    id: u64,
}

impl<S: 'static + Send + Sync> FromRequestParts<S> for User {
    type Rejection = Unauthorized<()>;

    fn openapi(operation: &mut Operation, _registry: &mut Registry) {
        operation
            .security
            .get_or_insert_default()
            .push(SecurityRequirement::default().with_schemes_entry("basic", []))
    }

    async fn from_request_parts(_parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        todo!("validate username & password")
    }
}
