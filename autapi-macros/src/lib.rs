use proc_macro::TokenStream;
use syn::{DeriveInput, ItemFn, parse_macro_input};

mod doc_attr;
mod endpoint;
mod responses;
mod to_schema;
mod utils;

#[proc_macro_attribute]
pub fn endpoint(attr: TokenStream, item: TokenStream) -> TokenStream {
    TokenStream::from(endpoint::endpoint(
        proc_macro2::TokenStream::from(attr),
        parse_macro_input!(item as ItemFn),
    ))
}

#[proc_macro_derive(DescribeResponse)]
pub fn derive_response(attr: TokenStream) -> TokenStream {
    let input = parse_macro_input!(attr as DeriveInput);
    TokenStream::from(responses::derive_responses(input))
}

/// Implement `ToSchema`, `SchemaSerialize`, `SchemaDeserialize` and `Validate` for a type.
///
/// Supported are named field structs, newtype structs (exactly one unnamed field), unit structs
/// and enums. All behavior is configured with `#[schema(...)]` attributes. Doc comments are used as
/// the `description` of the schema unless `description` is given explicitly.
///
/// ```ignore
/// /// A TODO item
/// #[derive(ToSchema)]
/// #[schema(rename_all = "camelCase")]
/// struct Todo {
///     title: String,
///     /// Omitted if there is no body
///     body: Option<String>,
///     #[schema(nullable, read_only)]
///     completed_at: Option<String>,
/// }
/// ```
///
/// # Type attributes
///
/// These attributes can be placed on any struct or enum:
///
/// | Attribute | Description |
/// |-----------|-------------|
/// | `rename = "Name"` | The schema name used in the `components` section of the OpenAPI document. Defaults to the type name, with the names of type parameters appended for generic types (e.g. `Page_Todo`). |
/// | `always_inlined` | Never add the schema to `components`, always inline it where it is used instead (sets `ToSchema::ALWAYS_INLINED`). |
/// | `bounds = "T: Trait, ..."` | Replaces the automatically generated trait bounds for type parameters with the given where predicates. |
/// | `no_serialize` | Do not implement `SchemaSerialize`, e.g. for types that are only received. |
/// | `no_deserialize` | Do not implement `SchemaDeserialize`, e.g. for types that are only sent. |
/// | `serde` | Additionally implement serde's `Serialize` and `Deserialize` by forwarding to `SchemaSerialize` and `SchemaDeserialize`. Serializing panics for types whose schema is not required. |
///
/// # Struct attributes
///
/// Named field structs additionally accept:
///
/// | Attribute | Description |
/// |-----------|-------------|
/// | `rename_all = "case"` | Renames all fields to the given case: `lowercase`, `UPPERCASE`, `PascalCase`, `camelCase`, `snake_case`, `SCREAMING_SNAKE_CASE`, `kebab-case` or `SCREAMING-KEBAB-CASE`. |
///
/// Newtype structs are described by their inner field, so they accept the
/// [field attributes](#field-attributes) (e.g. `inline` or validation constraints) on the struct
/// itself.
///
/// Unit structs are represented as `null`.
///
/// Named field structs, unit structs and enums also accept the
/// [metadata attributes](#metadata-attributes).
///
/// # Enum attributes
///
/// | Attribute | Description |
/// |-----------|-------------|
/// | `tag = "property"` | Internally tag variants: the variant name is stored in the given property of the object. |
/// | `untagged` | Do not tag variants: a value matches the first variant it can be deserialized as. |
/// | `rename_all = "case"` | Renames all variants to the given case (see `rename_all` for structs). |
/// | `rename_all_fields = "case"` | Renames the fields of all named field variants to the given case. |
///
/// Exactly one of `tag` and `untagged` is required.
///
/// # Variant attributes
///
/// | Attribute | Description |
/// |-----------|-------------|
/// | `rename = "name"` | The name of the variant, used as the tag value or string constant. |
/// | `untagged` | Do not tag this variant even though the enum has a `tag`. Untagged variants must come after all tagged ones. |
/// | `null` | Unit variants only: represent this variant as `null` instead of a string constant. |
/// | `rename_all = "case"` | Named field variants only: renames the fields of this variant, overriding `rename_all_fields`. |
///
/// Unit variants are represented as a string constant of their name and must be untagged (either
/// via the enum or the variant). If an enum consists only of unit variants without `null` or any
/// metadata, they are merged into a single string schema with `enum` values.
///
/// Tuple variants must have exactly one field and accept the [field attributes](#field-attributes)
/// on the variant itself. Untagged tuple variants are represented directly by their inner value.
///
/// Unit and named field variants accept the [metadata attributes](#metadata-attributes).
///
/// # Field attributes
///
/// These attributes can be placed on named fields (and, as described above, newtype structs and
/// tuple variants):
///
/// | Attribute | Description |
/// |-----------|-------------|
/// | `rename = "name"` | Named fields only: the property name, overriding `rename_all`. |
/// | `skip` | Named fields only: exclude the field from the schema and (de-)serialization. It is set to `Default::default()` when deserializing. |
/// | `inline` | Inline the schema of the field type instead of referencing it in `components`. |
/// | `optional` | Allow omitting the property, using `Default::default()` when it is missing. Equivalent to `using = "Option<T>"`. |
/// | `nullable` | Allow `null` for the property, which is deserialized to `Default::default()`. Equivalent to `using = "Nullable<T>"`. On `Option<T>` fields, the property may additionally be omitted, and `None` is omitted when serializing. |
/// | `using = "Type"` | Describe, serialize and deserialize the field as if it had type `Type`, converting via `SchemaUsing<Type>`. |
///
/// `optional`, `nullable` and `using` are mutually exclusive. Whether a property is required is
/// determined by its (`using`) type: e.g. `Option<T>` properties may be omitted, while properties
/// of most other types are required.
///
/// Fields also accept the [metadata attributes](#metadata-attributes) and
/// [validation attributes](#validation).
///
/// # Metadata attributes
///
/// | Attribute | Description |
/// |-----------|-------------|
/// | `description = "..."` | The description of the schema. Defaults to the doc comments. |
/// | `title = "..."` | The title of the schema. |
/// | `deprecated` | Mark the schema as deprecated. |
/// | `read_only` | Mark the schema as read-only (only sent in responses). |
/// | `write_only` | Mark the schema as write-only (only sent in requests). |
/// | `example = expr` | An example value. The expression is serialized via `SchemaSerialize`, e.g. `example = "foo"` or `example = Todo::example()`. |
/// | `examples = [expr, ...]` | Multiple example values, serialized like `example`. |
///
/// For fields whose schema is a reference, only `description` is applied (as a sibling of
/// `$ref`).
///
/// # Validation
///
/// Fields, newtype structs and tuple variants accept validation constraints, which are added to
/// the schema and checked when deserializing (or manually via `Validate::validate`):
///
/// | Attribute | Applies to | Description |
/// |-----------|------------|-------------|
/// | `minimum = n` | numbers | The value must be at least `n`. |
/// | `maximum = n` | numbers | The value must be at most `n`. |
/// | `exclusive_minimum = n` | numbers | The value must be greater than `n`. |
/// | `exclusive_maximum = n` | numbers | The value must be less than `n`. |
/// | `multiple_of = n` | numbers | The value must be a multiple of `n` (which must be greater than 0). |
/// | `min_length = n` | strings | The string must have at least `n` characters. |
/// | `max_length = n` | strings | The string must have at most `n` characters. |
/// | `pattern = "regex"` | strings | The string must match the regex (requires the `regex` feature). The pattern is not anchored, use `^` and `$` to match the entire string. |
/// | `min_items = n` | sequences | The sequence must have at least `n` items. |
/// | `max_items = n` | sequences | The sequence must have at most `n` items. |
/// | `min_properties = n` | maps | The map must have at least `n` entries. |
/// | `max_properties = n` | maps | The map must have at most `n` entries. |
///
/// Numbers may be negative and/or floats, integers must fit into `i64` or `u64`. Deserialization
/// fails with the first violated constraint. Values of tuple variants and fields of named field
/// variants are validated while deserializing the variant, so untagged enums try the next variant
/// if a constraint is violated.
///
/// If the field's schema is a reference, the constraints are added to an `allOf` wrapping the
/// reference.
///
/// Constraints apply to the inner value of `Option<T>` and `Nullable<T>` and are skipped for absent
/// values. For fields with `optional`, `nullable` or `using`, the constraints apply to the `using`
/// type: when deserializing, they are checked before converting the value (so the default of an
/// omitted `optional` value is not validated), while `Validate::validate` checks the value
/// returned by `SchemaUsing::to_ser`. To use constraints on fields of your own types, implement
/// the corresponding trait (`ValidateNumber`, `ValidateStr`, `ValidateItems` or
/// `ValidateProperties`).
///
/// Values used for omitted properties (`SchemaDeserialize::deserialize_missing`) are validated as
/// well. As no error can be reported there, a value violating the constraints is discarded, so the
/// property is treated as required: newtypes and enums then fail to deserialize with a missing
/// property error, and enums try the next untagged variant. Therefore, the value returned by
/// `deserialize_missing` of a type used with constraints should satisfy them.
///
/// Trait bounds required by constraints on fields of generic types are not added automatically,
/// specify them via `bounds` instead.
///
/// ```ignore
/// #[derive(ToSchema)]
/// #[schema(min_length = 3, max_length = 32, pattern = "^[a-z0-9_]+$")]
/// struct Username(String);
///
/// #[derive(ToSchema)]
/// struct Signup {
///     username: Username,
///     #[schema(minimum = 13)]
///     age: u8,
/// }
/// ```
#[proc_macro_derive(ToSchema, attributes(schema))]
pub fn derive_to_schema(attr: TokenStream) -> TokenStream {
    let input = parse_macro_input!(attr as DeriveInput);
    let result = to_schema::derive_to_schema(input).unwrap_or_else(|t| t);
    TokenStream::from(result)
}
