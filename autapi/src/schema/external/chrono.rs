use chrono::{DateTime, FixedOffset, NaiveDate, TimeZone};

use crate::{
    Registry,
    openapi::{Format, MaybeRef, Schema, Type},
    schema::{ToSchema, macros::transparent_serde},
};

impl<Tz: TimeZone> ToSchema for DateTime<Tz> {
    type Original = DateTime<FixedOffset>;

    const REQUIRED: bool = true;
    const ALWAYS_INLINED: bool = true;

    fn name() -> std::borrow::Cow<'static, str> {
        "DateTime".into()
    }

    fn schema(_registry: &mut Registry) -> MaybeRef<Schema> {
        Schema::default()
            .with_schema_type(Type::String)
            .with_format(Format::DateTime)
            .into()
    }
}

transparent_serde!(serde DateTime<Tz: TimeZone>);

impl ToSchema for NaiveDate {
    type Original = Self;

    const REQUIRED: bool = true;
    const ALWAYS_INLINED: bool = true;

    fn name() -> std::borrow::Cow<'static, str> {
        "Date".into()
    }

    fn schema(_: &mut Registry) -> MaybeRef<Schema> {
        Schema::default()
            .with_schema_type(Type::String)
            .with_format(Format::Date)
            .into()
    }
}

transparent_serde!(serde NaiveDate<>);
