/// General-purpose wrapper for things that should not be added to the API description.
///
/// This type implements [`FromRequest`], [`FromRequestParts`] and [`IntoResponse`] for `T`s that
/// implement the respective trait.
///
/// [`FromRequest`]: crate::request::FromRequest
/// [`FromRequestParts`]: crate::request::FromRequestParts
/// [`IntoResponse`]: crate::response::IntoResponse
pub struct Undocumented<T>(pub T);

/// General-purpose wrapper for types that implement axum's traits and should be used with this
/// crate, but without modifying the API description.
///
/// This type implements [`FromRequest`], [`FromRequestParts`] and [`IntoResponse`] for `T`s that
/// implement the corresponding trait in axum.
///
/// [`FromRequest`]: crate::request::FromRequest
/// [`FromRequestParts`]: crate::request::FromRequestParts
/// [`IntoResponse`]: crate::response::IntoResponse
pub struct UndocumentedAxum<T>(pub T);
