use crate::schema::SchemaDeserialize;
use crate::{
    Registry,
    openapi::{MaybeRef, Schema},
    schema::{
        ToSchema,
        macros::{transparent_deserialize, transparent_serialize},
    },
};
use serde::Deserializer;
use std::{borrow::Cow, cell::RefCell, ops::Deref, rc::Rc, sync::Arc};

macro_rules! impl_transparent {
    (
        $ty:ident <
            $($lt:lifetime,)?
            T $(: $($bound:ident)*)?
        > via $func:expr
    ) => {
        impl_transparent!(
            @no_deserialize
            $ty < $($lt,)? T $(: $($bound)*)? >
            via $func
        );
        transparent_deserialize!(schema $ty<$($lt;)? T $(: $($bound)*)?> via T);
    };
    (
        @no_deserialize
        $ty:ident <
            $($lt:lifetime,)?
            T $(: $($bound:ident)*)?
        > via $func:expr
    ) => {
        impl<$($lt,)? T: ?Sized> ToSchema for $ty<$($lt,)? T>
        where
            T: ToSchema,
            $($(T: $bound,)*)?
        {
            type Original = T::Original;

            const REQUIRED: bool = T::REQUIRED;
            const ALWAYS_INLINED: bool = T::ALWAYS_INLINED;

            fn name() -> Cow<'static, str> {
                T::name()
            }
            fn schema(registry: &mut Registry) -> MaybeRef<Schema> {
                T::schema(registry)
            }

            fn schema_ref(registry: &mut Registry) -> MaybeRef<Schema> {
                T::schema_ref(registry)
            }
        }

        transparent_serialize!(schema $ty<$($lt;)? T $(: $($bound)*)?> via T <- $func);
    };
}

impl_transparent!(Box<T> via Deref::deref);
impl_transparent!(Arc<T> via Deref::deref);
impl_transparent!(Rc<T> via Deref::deref);
impl_transparent!(RefCell<T> via RefCell::borrow);
impl_transparent!(@no_deserialize Cow<'a, T: ToOwned> via Deref::deref);

impl<'a, T> SchemaDeserialize for Cow<'a, T>
where
    T: ?Sized + ToOwned + ToSchema,
    T::Owned: SchemaDeserialize + ToSchema<Original = T::Original>,
{
    fn schema_deserialize<'de, D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Cow::Owned(T::Owned::schema_deserialize(deserializer)?))
    }

    fn deserialize_missing() -> Option<Self> {
        Some(Cow::Owned(T::Owned::deserialize_missing()?))
    }
}

type Ref<'a, T> = &'a T;
impl_transparent!(@no_deserialize Ref<'a, T> via Deref::deref);
