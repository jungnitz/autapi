use crate::{
    Registry, Undocumented, UndocumentedAxum,
    openapi::Responses,
    response::{IntoResponse, Response},
};

impl<T: IntoResponse> IntoResponse for Undocumented<T> {
    fn openapi(_: &mut Registry) -> Responses {
        Default::default()
    }
    fn into_response(self) -> axum::response::Response {
        self.0.into_response()
    }
}

impl<T: axum::response::IntoResponse> IntoResponse for UndocumentedAxum<T> {
    fn openapi(_: &mut Registry) -> Responses {
        Default::default()
    }
    fn into_response(self) -> Response {
        self.0.into_response()
    }
}
