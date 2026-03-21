use std::borrow::Cow;

use serde_json::Value;

use crate::{
    Registry,
    openapi::{Format, MaybeRef, Schema, Type},
    schema::{ToSchema, macros::transparent_serde},
};

macro_rules! impl_primitive {
    ($ty:ident $(, $schema_type:expr $(, $format:expr)?)? $(; $original:ty)?) => {
        impl ToSchema for $ty {
            type Original = impl_primitive!(@original $($original)?);

            const REQUIRED: bool = true;
            const ALWAYS_INLINED: bool = true;
            fn name() -> Cow<'static, str> {
                stringify!($ty).into()
            }
            fn schema(#[expect(unused)] registry: &mut Registry) -> MaybeRef<Schema> {
                MaybeRef::T(Schema::default()
                   $(
                       .with_schema_type($schema_type)
                       $(.with_format($format),)?
                   )?
                )
            }
        }

        transparent_serde!(serde $ty<>);
    };
    (@original) => { Self };
    (@original $ty:ty) => { $ty };
}

impl_primitive!(bool, Type::Boolean);

type Unit = ();
impl_primitive!(Unit, Type::Null);

impl_primitive!(i8, Type::Integer, Format::Int8);
impl_primitive!(i16, Type::Integer, Format::Int16);
impl_primitive!(i32, Type::Integer, Format::Int32);
impl_primitive!(i64, Type::Integer, Format::Int64);
impl_primitive!(i128, Type::Integer);

impl_primitive!(u8, Type::Integer, Format::Uint8);
impl_primitive!(u16, Type::Integer, Format::Uint16);
impl_primitive!(u32, Type::Integer, Format::Uint32);
impl_primitive!(u64, Type::Integer, Format::Uint64);
impl_primitive!(u128, Type::Integer);

impl_primitive!(f32, Type::Number, Format::Float);
impl_primitive!(f64, Type::Number, Format::Double);

impl_primitive!(String, Type::String);
impl_primitive!(str, Type::String; String);

impl_primitive!(Value);
