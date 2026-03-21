use std::{
    borrow::Cow,
    collections::{HashSet, LinkedList},
    hash::BuildHasher,
    hash::Hash,
    iter,
    marker::PhantomData,
};

use serde::de::Visitor;

use crate::{
    Registry,
    adapters::SerdeAdapter,
    openapi::{MaybeRef, Schema, Type},
    private::size_hint,
    schema::{SchemaDeserialize, SchemaSerialize, ToSchema},
};

macro_rules! impl_to_schema_and_serialize {
    ($ty:ident < T $(,)? $($gen:ident),* >) => {

        impl<T, $($gen),*> ToSchema for $ty<T, $($gen),*>
        where
            T: ToSchema
        {
            type Original = Vec<T::Original>;

            const REQUIRED: bool = true;
            const ALWAYS_INLINED: bool = true;

            fn name() -> Cow<'static, str> {
                format!("Array_{}", T::name()).into()
            }
            fn schema(registry: &mut Registry) -> MaybeRef<Schema> {
                MaybeRef::T(Schema::default()
                    .with_schema_type(Type::Array)
                    .with_items(T::schema_ref(registry))
                )
            }
        }

        impl<T, $($gen),*> SchemaSerialize for $ty<T, $($gen),*>
        where
            Self: ToSchema,
            T: SchemaSerialize
        {
            fn schema_serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                serializer.collect_seq(
                    self.iter().map(SerdeAdapter)
                        .filter(|val| !val.skip_serializing())
                )
            }

            fn is_present(&self) -> bool {
                true
            }
        }

    };
}

macro_rules! impl_deserialize {
    ($ty:ident<$($gen:ident $(: $($bound:ident)&+)?),*> $(, $reserve:expr)?) => {
        impl<$($gen),*> SchemaDeserialize for $ty<$($gen),*>
        where
            Self: ToSchema,
            T: SchemaDeserialize,
            $($($($gen: $bound,)*)?)*
        {
            fn schema_deserialize<'de, D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct Vis<$($gen),*>(PhantomData<($($gen,)*)>);
                impl<'de, $($gen),*> Visitor<'de> for Vis<$($gen),*>
                where
                    T: SchemaDeserialize,
                    $($($($gen: $bound,)*)?)*
                {
                    type Value = $ty<$($gen),*>;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                        formatter.write_str("an array")
                    }

                    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
                    where
                        A: serde::de::SeqAccess<'de>,
                    {
                        let mut result = <$ty<$($gen),*> as Default>::default();
                        $(
                            $reserve(&mut result, size_hint::<T>(seq.size_hint()));
                        )?
                        while let Some(value) = seq.next_element::<SerdeAdapter<T>>()? {
                            result.extend(iter::once(value.0));
                        }
                        Ok(result)
                    }
                }
                deserializer.deserialize_seq(Vis(PhantomData))
            }

            fn deserialize_missing() -> Option<Self> {
                None
            }
        }
    };
}

type Slice<T> = [T];
impl_to_schema_and_serialize!(Slice<T>);

impl_to_schema_and_serialize!(Vec<T>);
impl_deserialize!(Vec<T>, Vec::reserve);

impl_to_schema_and_serialize!(LinkedList<T>);
impl_deserialize!(LinkedList<T>);

impl_to_schema_and_serialize!(HashSet<T, H>);
impl_deserialize!(HashSet<T: Eq & Hash, H: BuildHasher & Default>, HashSet::reserve);

#[cfg(test)]
mod tests {
    use insta::{assert_debug_snapshot, assert_json_snapshot};
    use serde_json::json;

    use super::*;

    #[test]
    pub fn array_schema() {
        assert_json_snapshot!(Vec::<bool>::schema(&mut Registry::default()));
        assert_json_snapshot!(LinkedList::<String>::schema(&mut Registry::default()));
        assert_json_snapshot!(HashSet::<i64>::schema(&mut Registry::default()));

        assert_json_snapshot!(SerdeAdapter(vec![1, 2, 3]));
        assert_debug_snapshot!(serde_json::from_value::<SerdeAdapter<LinkedList<String>>>(
            json!(["this", "is", "a", "linked", "list"])
        ))
    }
}
