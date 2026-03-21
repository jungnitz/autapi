use http::StatusCode;
use pastey::paste;

use crate::{
    Registry,
    openapi::{self, Map, merge_responses_iter},
    response::{IntoResponse, Response},
};

macro_rules! define {
    ($status:ident) => {
        paste! {
            #[doc = concat!(
                "Converts all status codes returned by an `IntoResponse` type to `",
                stringify!($status),
                "`."
            )]
            #[doc = ""]
            #[doc = "If `T` produces multiple responses, they are merged. See [`merge_responses_iter`] for more details."]
            pub struct [< $status:camel >]<T>(pub T);

            impl<T: IntoResponse> IntoResponse for [<$status:camel>]<T> {
                fn openapi(registry: &mut Registry) -> openapi::Responses {
                    let mut responses = T::openapi(registry);
                    let merged = merge_responses_iter(responses.responses.into_values());
                    responses.responses = match merged {
                        None => Map::default(),
                        Some(response) => Map::from_iter([(StatusCode::$status.as_str().to_owned(), response)]),
                    };
                    responses
                }

                fn into_response(self) -> Response {
                    let mut response = self.0.into_response();
                    *response.status_mut() = StatusCode::$status;
                    response
                }
            }
        }
    };
}

define!(OK);
define!(CREATED);
define!(ACCEPTED);
define!(NOT_MODIFIED);
define!(UNAUTHORIZED);
define!(FORBIDDEN);
