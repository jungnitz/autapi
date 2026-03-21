use std::marker::PhantomData;

use http::StatusCode;

use crate::{
    Registry,
    adapters::SerdeAdapter,
    openapi::{self, MediaTypeContent, Responses},
    response::{IntoResponse, Response},
    schema::{SchemaSerialize, ToSchema},
};

fn openapi<T: ?Sized + ToSchema>(registry: &mut Registry) -> Responses {
    let response = openapi::Response::default().with_content_entry(
        mime::APPLICATION_JSON.essence_str(),
        MediaTypeContent::default().with_schema(T::schema_ref(registry)),
    );
    Responses::default().with_responses_entry(StatusCode::OK.as_str(), response)
}

impl<T: ToSchema + SchemaSerialize> IntoResponse for T {
    fn openapi(registry: &mut Registry) -> Responses {
        openapi::<T>(registry)
    }

    fn into_response(self) -> Response {
        axum::response::IntoResponse::into_response(axum::Json(SerdeAdapter(self)))
    }
}

pub struct CompatibleJson<T: ?Sized>(PhantomData<fn(T)>, Response);

impl<T: ToSchema + ?Sized> CompatibleJson<T> {
    pub fn new<C>(value: C) -> Self
    where
        C: ToSchema<Original = T::Original> + SchemaSerialize,
    {
        Self(PhantomData, value.into_response())
    }
}

impl<T: ?Sized> IntoResponse for CompatibleJson<T>
where
    T: SchemaSerialize,
{
    fn openapi(registry: &mut Registry) -> Responses {
        openapi::<T>(registry)
    }

    fn into_response(self) -> Response {
        self.1
    }
}
