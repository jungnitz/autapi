use http::StatusCode;

use crate::{
    Registry,
    openapi::{MediaTypeContent, Response, Responses},
    response::IntoResponse,
};

/// A `text/plain` response.
pub struct Text(pub String);

impl IntoResponse for Text {
    fn openapi(_registry: &mut Registry) -> Responses {
        Responses::default().with_responses_entry(
            StatusCode::OK.as_str(),
            Response::default().with_content_entry(
                mime::TEXT_PLAIN_UTF_8.essence_str(),
                MediaTypeContent::default(),
            ),
        )
    }

    fn into_response(self) -> super::Response {
        axum::response::IntoResponse::into_response(self.0)
    }
}

impl From<String> for Text {
    fn from(value: String) -> Self {
        Self(value)
    }
}
