extern crate self as autapi;

/// Adapters for bridging between the traits in this crate and traits in axum and serde
pub mod adapters;
/// Request handlers with OpenAPI description
pub mod endpoint;
/// Implementation of the OpenAPI specification
pub mod openapi;
/// Extraction from and description of requests
pub mod request;
/// Conversion to and description of responses
pub mod response;
/// JSON Schema for Rust types
pub mod schema;

mod rapidoc;
mod registry;
mod router;
mod utils;

#[doc(hidden)]
pub mod private;

pub use axum;
pub use http;

pub use self::{registry::Registry, router::*, utils::*};
