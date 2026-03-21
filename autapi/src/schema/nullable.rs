use std::borrow::Cow;

use serde::{Deserializer, Serialize, Serializer};

use crate::{
    Registry,
    adapters::SerdeAdapter,
    openapi::{MaybeRef, Schema, Type},
    schema::{SchemaDeserialize, SchemaSerialize, SchemaUsing, ToSchema},
};

/// A field that may be null.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Nullable<T>(pub Option<T>);

impl<T> From<Option<T>> for Nullable<T> {
    fn from(value: Option<T>) -> Self {
        Self(value)
    }
}

impl<T> From<Nullable<T>> for Option<T> {
    fn from(value: Nullable<T>) -> Self {
        value.0
    }
}

impl<T> From<T> for Nullable<T> {
    fn from(value: T) -> Self {
        Self(Some(value))
    }
}

impl<T: ToSchema> ToSchema for Nullable<T> {
    type Original = Nullable<T::Original>;

    const REQUIRED: bool = T::REQUIRED;
    const ALWAYS_INLINED: bool = true;

    fn name() -> Cow<'static, str> {
        format!("Nullable_{}", T::name()).into()
    }
    fn schema(registry: &mut Registry) -> MaybeRef<Schema> {
        MaybeRef::T(make_nullable(T::schema_ref(registry)))
    }
}

impl<T: SchemaSerialize> SchemaSerialize for Nullable<T>
where
    T: SchemaSerialize,
{
    fn schema_serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.as_ref().map(SerdeAdapter).serialize(serializer)
    }

    fn is_present(&self) -> bool {
        self.0.as_ref().map(|t| t.is_present()).unwrap_or(true)
    }
}

impl<T: SchemaDeserialize> SchemaDeserialize for Nullable<T> {
    fn schema_deserialize<'de, D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        <Option<SerdeAdapter<T>> as serde::Deserialize>::deserialize(deserializer)
            .map(|option| Self(option.map(|adapter| adapter.0)))
    }

    fn deserialize_missing() -> Option<Self> {
        T::deserialize_missing().map(Some).map(Self)
    }
}

fn make_nullable(schema: MaybeRef<Schema>) -> Schema {
    match schema {
        MaybeRef::T(
            mut schema @ Schema {
                schema_type: Some(_),
                ..
            },
        ) => {
            schema.schema_type.as_mut().unwrap().push(Type::Null);
            schema
        }
        schema => Schema::default().with_any_of(vec![Schema::null().into(), schema]),
    }
}

impl<T: ToSchema> SchemaUsing<Option<T>> for Nullable<T> {
    type Ser<'a>
        = &'a Option<T>
    where
        T: 'a;

    fn to_ser<'a>(&'a self) -> Self::Ser<'a> {
        &self.0
    }

    fn from_de(de: Option<T>) -> Self {
        Self(de)
    }
}

impl<T: ToSchema> SchemaUsing<Nullable<T>> for Option<T> {
    type Ser<'a>
        = Nullable<&'a T>
    where
        T: 'a;

    fn to_ser<'a>(&'a self) -> Self::Ser<'a> {
        Nullable(self.as_ref())
    }

    fn from_de(de: Nullable<T>) -> Self {
        de.0
    }
}

impl<T: ToSchema> SchemaUsing<Nullable<T>> for T
where
    T: Default,
{
    type Ser<'a>
        = Nullable<&'a T>
    where
        T: 'a;

    fn to_ser<'a>(&'a self) -> Self::Ser<'a> {
        Nullable(Some(self))
    }

    fn from_de(de: Nullable<T>) -> Self {
        de.0.unwrap_or_default()
    }
}
