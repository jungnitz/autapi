use std::borrow::Cow;

use serde::{Deserializer, Serializer, ser::Error as _};

use crate::{
    Registry,
    openapi::{MaybeRef, Schema},
    schema::{SchemaDeserialize, SchemaSerialize, SchemaUsing, ToSchema},
};

impl<T: ToSchema> ToSchema for Option<T> {
    type Original = Option<T::Original>;

    const REQUIRED: bool = false;
    const ALWAYS_INLINED: bool = T::ALWAYS_INLINED;

    fn name() -> Cow<'static, str> {
        T::name()
    }
    fn schema(registry: &mut Registry) -> MaybeRef<Schema> {
        T::schema(registry)
    }
}

impl<T> SchemaSerialize for Option<T>
where
    T: SchemaSerialize,
{
    fn schema_serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let value: &T = self
            .as_ref()
            .ok_or_else(|| S::Error::custom("schema_serialize called for an absent value"))?;
        value.schema_serialize(serializer)
    }

    fn is_present(&self) -> bool {
        match &self {
            Some(value) => value.is_present(),
            None => false,
        }
    }
}

impl<T: SchemaDeserialize> SchemaDeserialize for Option<T> {
    fn schema_deserialize<'de, D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Some(<T as SchemaDeserialize>::schema_deserialize(
            deserializer,
        )?))
    }
    fn deserialize_missing() -> Option<Self> {
        Some(None)
    }
}

impl<T> SchemaUsing<Option<T>> for T
where
    T: Default + ToSchema,
{
    type Ser<'a>
        = Option<&'a T>
    where
        T: 'a;

    fn to_ser<'a>(&'a self) -> Self::Ser<'a> {
        Some(self)
    }

    fn from_de(de: Option<T>) -> Self {
        de.unwrap_or_default()
    }
}
