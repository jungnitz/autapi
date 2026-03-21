mod external;
mod impls;
mod iterator;
mod macros;
mod nullable;

use std::borrow::Cow;

use serde::{Deserializer, Serializer};

use crate::{
    Registry,
    openapi::{MaybeRef, Schema},
};

pub use self::{iterator::*, nullable::*};
pub use autapi_macros::ToSchema;

/// A type that is described with a [`Schema`].
pub trait ToSchema {
    /// A type with a schema that allows the same values as this type.
    ///
    /// This type is used to check equivalence and compatibility of schemas at compile-time.
    /// Therefore, when manually implementing `ToSchema`, you should attempt to make all types with
    /// the same underlying schema have the same `Original` type.
    type Original: ToSchema;

    /// Determines whether a property of this type may be omitted.
    ///
    /// See also [`SchemaSerialize::is_present`] and [`SchemaDeserialize::deserialize_missing`] for
    /// more information and effects of this constant.
    const REQUIRED: bool;

    /// Indicates whether `self.schema_ref(registry) == self.schema(registry)`.
    const ALWAYS_INLINED: bool;

    /// Returns the name to use for adding this schema to the OpenAPI `components` section.
    fn name() -> Cow<'static, str>;

    /// Returns the schema of this type.
    ///
    /// The returned value may either be a schema or a reference to **another** schema (e.g. when
    /// using a newtype struct with the `ToSchema` derive macro).
    fn schema(registry: &mut Registry) -> MaybeRef<Schema>;

    /// Returns a schema or reference to be used within other schemas for values of this type.
    ///
    /// This will be usually a reference, unless this type's schema is [`ALWAYS_INLINED`], in which
    /// case this function is equivalent to [`schema`].
    ///
    /// [`ALWAYS_INLINED`]: Self::ALWAYS_INLINED
    /// [`schema`]: Self::schema
    fn schema_ref(registry: &mut Registry) -> MaybeRef<Schema> {
        if Self::ALWAYS_INLINED {
            Self::schema(registry)
        } else {
            MaybeRef::Ref(registry.register_schema::<Self>())
        }
    }
}

/// A type that is serializable according to its schema specification.
///
/// This trait is similar to serde's [`Serialize`](serde::Serialize), but allows
/// - "automatic", type-based optional properties, which prevents littering your codebase with
///   `#[serde(skip_serializing_if="...")]`
/// - deviating from the default or third-party serde implementations (e.g. for the time crate)
pub trait SchemaSerialize: ToSchema {
    /// Serializes an object of this type.
    ///
    /// Users of this trait should only call this method if [`is_present`](Self::is_present) is
    /// `true` and implementations should return an error in this case.
    fn schema_serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer;

    /// Determines whether a property of this type should be serialized within a JSON object.
    ///
    /// If `false` is returned for a property value, it is not serialized (i.e. omitted from the
    /// JSON object).
    /// Therefore, if `<Self as ToSchema>::REQUIRED == true`, this method must **never** return
    /// `false`, otherwise the schema will be violated in these cases.
    fn is_present(&self) -> bool;
}

/// A type that is deserializable according to its schema specification.
///
/// This trait is similar to serde's [`Deserialize`](serde::Deserialize), but allows
/// - "automatic", type-based optional properties, which prevents littering your codebase with
///   `#[serde(default = "...")]`
/// - deviating from the default or third-party serde implementations (e.g. for the time crate)
pub trait SchemaDeserialize: Sized + ToSchema {
    /// Deserializes an object of this type.
    fn schema_deserialize<'de, D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>;

    /// Returns `Some(_)` with the value to be used if a property of this type is omitted or `None`
    /// if properties of this type cannot be omitted.
    ///
    /// Therefore, if `<Self as ToSchema>::REQUIRED`, this method must return `Some(_)`, otherwise
    /// deserialization will fail despite
    fn deserialize_missing() -> Option<Self>;
}

/// Allows serializing or deserializing a type using another compatible one.
///
/// This trait is used in combination with the `#[schema(using = "<T>")]` attribute.
/// Annotating a field of original type `O` with `O: SchemaUsing<T>` results in the schema,
/// serialization and deserialization to be defined as if the field had type `T`.
/// Conversions between the two types are determined by the implementation of this trait.
pub trait SchemaUsing<T: ToSchema> {
    /// The type to use for serializing a reference to a value of this type.
    type Ser<'a>: ToSchema<Original = T::Original>
    where
        Self: 'a,
        T: 'a;

    /// Converts a reference to the value to use for serializing.
    fn to_ser<'a>(&'a self) -> Self::Ser<'a>;
    /// Converts a deserialized value to the target value.
    fn from_de(de: T) -> Self;
}
