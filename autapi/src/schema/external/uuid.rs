use std::borrow::Cow;

use uuid::Uuid;

use crate::{
    Registry,
    openapi::{Format, MaybeRef, Schema, Type},
    schema::{ToSchema, macros::transparent_serde},
};

impl ToSchema for Uuid {
    type Original = Uuid;

    const REQUIRED: bool = true;
    const ALWAYS_INLINED: bool = true;

    fn name() -> Cow<'static, str> {
        "Uuid".into()
    }

    fn schema(_registry: &mut Registry) -> MaybeRef<Schema> {
        Schema::default()
            .with_schema_type(Type::String)
            .with_format(Format::Uuid)
            .into()
    }
}

transparent_serde!(serde Uuid<>);
