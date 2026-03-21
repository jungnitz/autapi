use std::{
    borrow::Cow,
    collections::{BTreeMap, HashMap},
    fmt,
    hash::{BuildHasher, Hash},
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
    ($ty:ident<K, V $(,)? $($gen:ident),*> $(, $reserve:expr)?) => {
        impl<K, V, $($gen),*> ToSchema for $ty<K, V, $($gen),*>
        where
            K: ToSchema,
            V: ToSchema,
        {
            type Original = BTreeMap<K::Original, V::Original>;

            const REQUIRED: bool = true;
            const ALWAYS_INLINED: bool = true;

            fn name() -> Cow<'static, str> {
                format!("Map_{}_{}", K::name(), V::name()).into()
            }
            fn schema(registry: &mut Registry) -> MaybeRef<Schema> {
                MaybeRef::T(Schema::default()
                    .with_schema_type(Type::Object)
                    .with_property_names(K::schema_ref(registry))
                    .with_additional_properties(V::schema_ref(registry))
                )
            }
        }

        impl<K, V, $($gen),*> SchemaSerialize for $ty<K, V, $($gen),*>
        where
            Self: ToSchema,
            K: SchemaSerialize,
            V: SchemaSerialize,
        {
            fn schema_serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                serializer.collect_map(
                    self.iter().map(|(k, v)| (SerdeAdapter(k), SerdeAdapter(v)))
                        .filter(|(k, v)| !k.skip_serializing() && !v.skip_serializing())
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
            K: SchemaDeserialize,
            V: SchemaDeserialize,
            $($($($gen: $bound,)*)?)*
        {
            fn schema_deserialize<'de, D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct Vis<$($gen),*>(PhantomData<($($gen,)*)>);
                impl<'de, $($gen),*> Visitor<'de> for Vis<$($gen),*>
                where
                    K: SchemaDeserialize,
                    V: SchemaDeserialize,
                    $($($($gen: $bound,)*)?)*
                {
                    type Value = $ty<$($gen),*>;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("an object")
                    }

                    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
                    where
                        A: serde::de::MapAccess<'de>,
                    {
                        let mut result = <$ty<$($gen),*> as Default>::default();
                        $(
                            $reserve(&mut result, size_hint::<(K, V)>(map.size_hint()));
                        )?
                        while let Some((k, v)) = map.next_entry::<SerdeAdapter<_>, SerdeAdapter<_>>()? {
                            result.insert(k.0, v.0);
                        }
                        Ok(result)
                    }
                }
                deserializer.deserialize_map(Vis(PhantomData))
            }

            fn deserialize_missing() -> Option<Self> {
                None
            }
        }
    }
}

impl_to_schema_and_serialize!(BTreeMap<K, V>);
impl_deserialize!(BTreeMap<K: Ord, V>);

impl_to_schema_and_serialize!(HashMap<K, V, H>);
impl_deserialize!(HashMap<K: Hash & Eq, V, H: BuildHasher & Default>, HashMap::reserve);

#[cfg(test)]
mod tests {
    use insta::{assert_debug_snapshot, assert_json_snapshot};
    use serde_json::json;

    use super::*;

    #[test]
    pub fn maps() {
        assert_json_snapshot!(HashMap::<String, i64>::schema(&mut Registry::default()));
        assert_json_snapshot!(SerdeAdapter(BTreeMap::<&str, i32>::from_iter([
            ("one", 1),
            ("two", 2)
        ])));
        assert_debug_snapshot!(
            serde_json::from_value::<SerdeAdapter<BTreeMap<String, i32>>>(json!({
                "two": 2,
                "three": 3,
            }))
        )
    }
}
