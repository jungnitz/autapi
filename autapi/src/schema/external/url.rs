use std::borrow::Cow;

use url::Url;

use crate::{
    Registry,
    openapi::{Format, MaybeRef, Schema, Type},
    schema::{ToSchema, macros::transparent_serde},
};

impl ToSchema for Url {
    type Original = Url;

    const REQUIRED: bool = true;
    const ALWAYS_INLINED: bool = true;

    fn name() -> Cow<'static, str> {
        "Url".into()
    }

    fn schema(_registry: &mut Registry) -> MaybeRef<Schema> {
        Schema::default()
            .with_schema_type(Type::String)
            .with_format(Format::Uri)
            .into()
    }
}

transparent_serde!(serde Url<>);
