use std::{fmt, mem::swap, ops::Deref, slice};

use serde::{Deserialize, Deserializer, Serialize, de::Visitor};
use serde_json::{Number as JsonNumber, Value};
use serde_value::ValueDeserializer;
use serde_with::skip_serializing_none;

use crate::openapi::{Map, MaybeRef, macros};

macros::define_openapi_spec_object! {
    [override_with]:
    #[derive(Default)]
    pub struct Schema {
        // -- Validation for any instance type
        #[serde(rename = "type")]
        pub schema_type: Option<SchemaType>,
        #[serde(rename = "enum")]
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub enum_values: Vec<Value>,
        #[serde(rename = "const")]
        pub const_value: Option<Value>,
        pub format: Option<Format>,

        // -- Validation for objects
        #[serde(skip_serializing_if = "Map::is_empty")]
        pub properties: Map<String, MaybeRef<Schema>>,
        pub property_names: Option<Box<MaybeRef<Schema>>>,
        pub additional_properties: Option<Box<MaybeRef<Schema>>>,
        pub max_properties: Option<u64>,
        pub min_properties: Option<u64>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub required: Vec<String>,
        #[serde(skip_serializing_if = "Map::is_empty")]
        pub dependent_required: Map<String, Vec<String>>,

        // -- Validation for numbers
        pub multiple_of: Option<JsonNumber>,
        pub maximum: Option<JsonNumber>,
        pub exclusive_maximum: Option<JsonNumber>,
        pub minimum: Option<JsonNumber>,
        pub exclusive_minimum: Option<JsonNumber>,

        // -- Validation for strings
        pub max_length: Option<u64>,
        pub min_length: Option<u64>,
        pub pattern: Option<String>,
        pub content_encoding: Option<String>,
        pub content_media_type: Option<String>,
        pub content_schema: Option<Box<MaybeRef<Schema>>>,

        // -- Validation for arrays
        pub items: Option<ArrayItems>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub prefix_items: Vec<MaybeRef<Schema>>,

        pub max_items: Option<u64>,
        pub min_items: Option<u64>,
        pub unique_items: Option<bool>,
        pub max_contains: Option<u64>,
        pub min_contains: Option<u64>,

        // -- Meta information
        pub title: Option<String>,
        pub description: Option<String>,
        pub default: Option<Value>,
        pub deprecated: Option<bool>,
        pub read_only: Option<bool>,
        pub write_only: Option<bool>,
        pub example: Option<Value>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub examples: Vec<Value>,

        // -- Polymorphism
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub all_of: Vec<MaybeRef<Schema>>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub one_of: Vec<MaybeRef<Schema>>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub any_of: Vec<MaybeRef<Schema>>,
        pub discriminator: Option<Discriminator>,
    }
}

impl Schema {
    pub fn new_string_constant(string: impl Into<String>) -> Self {
        Schema::default()
            .with_schema_type(Type::String)
            .with_enum_values(vec![Value::String(string.into())])
    }
    pub fn null() -> Self {
        Schema::default().with_schema_type(Type::Null)
    }
}

#[derive(PartialEq, Eq, Clone)]
pub enum ArrayItems {
    False,
    Schema(Box<MaybeRef<Schema>>),
}

impl<I: Into<MaybeRef<Schema>>> From<I> for ArrayItems {
    fn from(value: I) -> Self {
        Self::Schema(Box::new(value.into()))
    }
}

impl Serialize for ArrayItems {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::False => serializer.serialize_bool(false),
            Self::Schema(schema) => schema.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for ArrayItems {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Vis;
        impl<'de> Visitor<'de> for Vis {
            type Value = ArrayItems;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("false")
            }

            fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if !v {
                    Ok(ArrayItems::False)
                } else {
                    Err(E::custom("array items must be false or a schema object"))
                }
            }
        }
        let value = deserializer.deserialize_any(serde_value::ValueVisitor)?;
        if let Ok(schema) = Box::deserialize(ValueDeserializer::<D::Error>::new(value.clone())) {
            return Ok(Self::Schema(schema));
        }
        ValueDeserializer::<D::Error>::new(value).deserialize_bool(Vis)
    }
}

#[derive(PartialEq, Eq, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SchemaType {
    Single(Type),
    Multiple(Vec<Type>),
}

impl SchemaType {
    pub fn push(&mut self, ty: Type) {
        match self {
            Self::Single(prev_ty) => {
                let mut swap_target = Type::Null;
                swap(prev_ty, &mut swap_target);
                *self = Self::Multiple(vec![swap_target, ty])
            }
            Self::Multiple(prev_types) => prev_types.push(ty),
        }
    }
}

impl Deref for SchemaType {
    type Target = [Type];

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Single(single) => slice::from_ref(single),
            Self::Multiple(multiple) => multiple,
        }
    }
}

impl From<Type> for SchemaType {
    fn from(value: Type) -> Self {
        Self::Single(value)
    }
}

#[derive(PartialEq, Eq, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub enum Type {
    Null,
    Boolean,
    Object,
    Array,
    Number,
    String,
    Integer,
}

/// A subset of the [OpenAPI Format Registry](https://spec.openapis.org/registry/format/)
#[derive(PartialEq, Eq, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum Format {
    // -- Primitive data types --
    Int8,
    Int16,
    Int32,
    Int64,
    Uint8,
    Uint16,
    Uint32,
    Uint64,
    Float,
    Double,
    /// A single character
    Char,
    /// Any sequence of octets
    Binary,

    // -- string formats --
    /// RFC3339 date-time without the timezone component
    DateTimeLocal,
    /// Date and time as defined by date-time - RFC3339
    DateTime,
    /// Date as defined by full-date - RFC3339
    Date,
    /// Time as defined by full-time - RFC3339
    Time,
    /// Duration as defined by duration - RFC3339
    Duration,
    /// Base64 encoded data as defined in RFC4648
    Byte,
    /// Binary data encoded as a url-safe string as defined in RFC4648
    Base64Url,
    /// Commonmark-formatted text
    Commonmark,
    /// A fixed point decimal number of unspecified precision and range
    Decimal,

    Email,
    IdnEmail,

    Hostname,
    IdnHostname,

    IPv4,
    IPv6,

    Uri,
    UriReference,
    Iri,
    IriReference,
    Uuid,

    UriTemplate,

    JsonPointer,
    RelativeJsonPointer,

    Regex,

    #[serde(untagged)]
    Other(String),
}

#[skip_serializing_none]
#[derive(PartialEq, Eq, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct Discriminator {
    pub property_name: String,
    #[serde(skip_serializing_if = "Map::is_empty")]
    pub mapping: Map<String, String>,
    pub default_mapping: Option<String>,
}

impl Discriminator {
    pub fn new(property_name: String) -> Self {
        Self {
            property_name,
            mapping: Default::default(),
            default_mapping: Default::default(),
        }
    }
}
