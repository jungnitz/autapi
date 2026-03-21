use crate::{
    Registry, UndocumentedAxum,
    adapters::SerdeAdapter,
    openapi::{MaybeRef, MediaTypeContent, Operation, RequestBody},
    request::{FromRequest, Request},
    schema::{SchemaDeserialize, ToSchema},
};

impl<T: ToSchema + SchemaDeserialize, S> FromRequest<S> for T
where
    S: Send + Sync,
{
    type Rejection =
        UndocumentedAxum<<axum::Json<SerdeAdapter<T>> as axum::extract::FromRequest<S>>::Rejection>;

    fn openapi(operation: &mut Operation, registry: &mut Registry) {
        assert!(
            operation.request_body.is_none(),
            "request body should not be set yet"
        );
        let content = MediaTypeContent::default().with_schema(T::schema_ref(registry));
        let body = RequestBody::default()
            .with_content_entry(mime::APPLICATION_JSON.essence_str(), content)
            .with_maybe_required(T::REQUIRED.then_some(true));
        operation.request_body = Some(MaybeRef::T(body))
    }

    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        if request.headers().get(http::header::CONTENT_TYPE).is_none()
            && let Some(value) = T::deserialize_missing()
        {
            Ok(value)
        } else {
            <axum::Json<SerdeAdapter<T>> as axum::extract::FromRequest<S>>::from_request(
                request, state,
            )
            .await
            .map(|json| json.0.0)
            .map_err(UndocumentedAxum)
        }
    }
}
