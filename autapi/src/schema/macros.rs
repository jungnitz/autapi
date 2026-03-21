macro_rules! transparent_serialize {
    (
        serde $ty:ident <
            $($lt:lifetime;)?
            $($gen:ident $(: $($bound:path)* )? ),*
        > $(via $via_ty:ident <- $func:expr)?
    ) => {
        impl<$($lt,)? $($gen: ?Sized),*> crate::schema::SchemaSerialize for $ty<$($lt,)? $($gen,)*>
        where
            crate::schema::macros::transparent_serialize!(@via $($via_ty)?): serde::Serialize,
            $($($($gen: $bound,)*)?)*
        {
            fn schema_serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                <
                    crate::schema::macros::transparent_serialize!(@via $($via_ty)?) as serde::Serialize
                >::serialize($($func)?(self), serializer)
            }
            fn is_present(&self) -> bool {
                true
            }
        }
    };
    (
        schema $ty:ident <
            $($lt:lifetime;)?
            $($gen:ident $(: $($bound:path)* )? ),*
        > via $via_ty:ident <- $func:expr
    ) => {
        impl<$($lt,)? $($gen: ?Sized),*> crate::schema::SchemaSerialize for $ty<$($lt,)? $($gen,)*>
        where
            $via_ty: crate::schema::SchemaSerialize,
            $($($($gen: $bound,)*)?)*
        {
            fn schema_serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                <$via_ty as crate::schema::SchemaSerialize>::schema_serialize(
                    &*$func(self),
                    serializer
                )
            }
            fn is_present(&self) -> bool {
                <$via_ty as crate::schema::SchemaSerialize>::is_present(
                    &*$func(self),
                )
            }
        }
    };
    (@via $via:ident) => { $via };
    (@via) => { Self };
}

macro_rules! transparent_deserialize {
    (
        serde $ty:ident <
            $($lt:lifetime;)?
            $($gen:ident $(: $($bound:path)* )? ),*
        > $(via $via:ident)?
    ) => {
        #[allow(dead_code)]
        impl<$($lt,)? $($gen: ?Sized,)*> crate::schema::SchemaDeserialize for $ty <$($lt)? $($gen,)*>
        where
            crate::schema::macros::transparent_deserialize!(@via $($via)?): for<'de> serde::Deserialize<'de>,
            $($($($gen: $bound),*)?)*
        {
            fn schema_deserialize<'de, D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                <
                    crate::schema::macros::transparent_deserialize!(@via $($via)?) as serde::Deserialize
                >::deserialize(deserializer).map(Into::into)
            }
            fn deserialize_missing() -> Option<Self> {
                None
            }
        }
    };
    (
        schema $ty:ident <
            $($lt:lifetime;)?
            $($gen:ident $(: $($bound:path)* )? ),*
        > via $via:ident $(-> $func:expr)?
    ) => {
        impl<$($lt,)? $($gen: ?Sized,)*> crate::schema::SchemaDeserialize for $ty <$($lt,)? $($gen,)*>
        where
            $via: crate::schema::SchemaDeserialize,
            $($($($gen: $bound),*)?)*
        {
            fn schema_deserialize<'de, D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                <
                    $via as crate::schema::SchemaDeserialize
                >::schema_deserialize(deserializer).map(
                    crate::schema::macros::transparent_deserialize!(@via_func $($func)?)
                )
            }
            fn deserialize_missing() -> Option<Self> {
                <
                    $via as crate::schema::SchemaDeserialize
                >::deserialize_missing().map(
                    crate::schema::macros::transparent_deserialize!(@via_func $($func)?)
                )
            }
        }
    };
    (@via $via:ident) => { $via };
    (@via) => { Self };
    (@via_func $via_func:expr) => { $via_func };
    (@via_func) => { Into::into };
}

macro_rules! transparent_serde {
    (
        $serde_or_schema:ident $ty:ident <
            $($lt:lifetime;)?
            $($gen:ident $(: $($bound:path)* )? ),*
        >
    ) => {
        crate::schema::macros::transparent_serialize!(
            $serde_or_schema $ty <
                $($lt;)?
                $($gen $(: $($bound)*)?),*
            >
        );
        crate::schema::macros::transparent_deserialize!(
            $serde_or_schema $ty <
                $($lt;)?
                $($gen $(: $($bound)*)?),*
            >
        );
    };
}

pub(crate) use transparent_deserialize;
pub(crate) use transparent_serde;
pub(crate) use transparent_serialize;
