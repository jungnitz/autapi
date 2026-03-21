# autapi

<small>

_autapi_ is still very much in development.
This means that the documentation is still severely lacking and there might be many breaking changes before a `v0.1`.
I do use this library for my own projects and intend to bring it to said first stable release in due time.

</small>

When developing REST APIs in Rust, there are many tools available that aim to make it easier.
In particular, I have always liked the idea of using [_utoipa_](https://github.com/juhaku/utoipa) together with a web
framework like [_axum_](https://github.com/tokio-rs/axum), which will make generating an OpenAPI description of your API
from the code sort-of easy.
There are some issues with this approach however:
Because both of these tools try to be general-purpose, they integrate only to a certain degree with one another.
This leads to some unfortunate extra-work that might lead to inconsistencies between the OpenAPI description and the
actual endpoint behavior.
In particular, when describing your endpoint with _utoipa_, you still need to manually list all responses, parameters,
security requirements etc.

This crate provides an opinionated approach to solving this problem by

- extracting all OpenAPI metadata from the function signature of your endpoints in an extensible way,
- defining new serialization and deserialization traits and derive macros that
    - allow overriding the default _serde_ implementation of external types
    - clearly separate between whether a value is nullable or optional and most importantly
    - **guarantee** that the generated JSON / OpenAPI schema is equivalent to the serialization or deserialization
      implementation (so long as all manual implementations of `ToSchema` are correct)

For example, a simple TODO API with required authentication might look something like this:

```rust
use utils::{User, IdPath}; // NOT from autapi

/// A TODO with title and body
#[derive(ToSchema)]
pub struct Todo {
    title: String,
    body: Option<String>,
}

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
```

<details>
<summary>Generated OpenAPI specification</summary>

```json
{
  "openapi": "3.1.0",
  "info": {
    "title": "TODO API",
    "description": "",
    "license": {
      "name": "MIT",
      "identifier": "MIT"
    },
    "version": "0.0.1"
  },
  "paths": {
    "/todos": {
      "post": {
        "tags": [
          "todo"
        ],
        "summary": "Create a TODO",
        "operationId": "create_todo",
        "requestBody": {
          "content": {
            "application/json": {
              "schema": {
                "$ref": "#/components/schemas/Todo"
              }
            }
          },
          "required": true
        },
        "responses": {
          "200": {
            "description": "",
            "content": {
              "application/json": {
                "schema": {
                  "type": "null"
                }
              }
            }
          },
          "401": {
            "description": "",
            "content": {
              "application/json": {
                "schema": {
                  "type": "null"
                }
              }
            }
          }
        },
        "security": [
          {
            "basic": []
          }
        ]
      }
    },
    "/todos/{id}": {
      "get": {
        "summary": "Get a TODO",
        "operationId": "get_todo",
        "parameters": [
          {
            "name": "id",
            "in": "path",
            "required": true,
            "schema": {
              "type": "integer",
              "format": "uint64"
            }
          }
        ],
        "responses": {
          "200": {
            "description": "",
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/Todo"
                }
              }
            }
          },
          "401": {
            "description": "",
            "content": {
              "application/json": {
                "schema": {
                  "type": "null"
                }
              }
            }
          }
        },
        "security": [
          {
            "basic": []
          }
        ]
      }
    }
  },
  "components": {
    "schemas": {
      "Todo": {
        "type": "object",
        "properties": {
          "body": {
            "type": "string"
          },
          "title": {
            "type": "string"
          }
        },
        "required": [
          "title"
        ],
        "description": "A TODO with title and body"
      }
    }
  }
}
```

</details>

Note that there is very little actual OpenAPI details exposed in the program code.
Despite this, the generated OpenAPI specification is very detailed.
_autapi_ handles most OpenAPI details without cluttering your code.
For example, you no longer need to annotate your truly optional fields (e.g. `Todo::body`) with
`#[serde(skip_serializing_if = "Option::is_none")]`, `#[schema(nullable = false)]` and similar verbose constructs.

As for implementation, this library does not even attempt to be a fully-fledged web framework like _axum_.
Instead, it mostly wraps the functionality provided by _axum_ and annotates it with OpenAPI metadata.
It is generally allowed and possible to "punch" through the OpenAPI layer of this library and work with _axum_'s types
directly.
Of course, when doing so, you have to be careful to not do something that makes the generated OpenAPI spec invalid.
However, most of your usual endpoints require no such shenanigans, and the only places where you need to circumvent the
wrappers provided by this library is to install _tower_ layers or similar things.