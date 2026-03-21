use autapi::{
    Registry,
    adapters::SerdeAdapter,
    schema::{Nullable, SchemaDeserialize, SchemaSerialize, ToSchema},
};
use insta::{assert_debug_snapshot, assert_json_snapshot};
use serde_json::json;

/// A referenced value.
#[derive(Debug, ToSchema)]
struct Referenced(bool);

/// An always inlined value.
#[derive(Debug, ToSchema)]
#[schema(always_inlined)]
struct Inlined(bool);

/// A value that can be omitted.
#[derive(Debug, ToSchema)]
struct OptionalValue(Option<bool>);

#[test]
fn tuple_struct() {
    #[derive(ToSchema)]
    struct TestReferenced(Referenced);
    #[derive(ToSchema)]
    #[schema(inline)]
    struct TestReferencedInline(Referenced);
    #[derive(ToSchema)]
    struct TestInlined(Inlined);
    #[derive(ToSchema)]
    #[schema(inline)]
    struct TestInlinedInline(Inlined);

    assert_json_snapshot!(TestReferenced::schema(&mut Registry::default()));
    assert_json_snapshot!(TestReferencedInline::schema(&mut Registry::default()));
    assert_json_snapshot!(TestInlined::schema(&mut Registry::default()));
    assert_json_snapshot!(TestInlinedInline::schema(&mut Registry::default()));
}

#[test]
fn named_field_struct() {
    /// An exemplary named field struct.
    #[derive(Debug, ToSchema)]
    #[schema(rename_all = "PascalCase")]
    struct Fields {
        /// A simple field.
        foo: i64,
        /// This field should be renamed to baz!
        #[schema(rename = "baz")]
        bar: String,
        /// What happens when we use a raw identifier?
        #[schema(examples = [true], title = "Title")]
        r#type: bool,
        /// This should be a reference and optional!
        referenced_optional: OptionalValue,
        /// This should be optional and not a reference.
        #[schema(inline)]
        referenced_optional_inline: OptionalValue,
        /// An automatically inlined value.
        inlined: Inlined,
        #[schema(skip)]
        #[expect(dead_code)]
        skippo: f64,
    }
    // schema
    assert_eq!(Fields::REQUIRED, true);
    assert_eq!(Fields::ALWAYS_INLINED, false);
    assert_json_snapshot!(Fields::schema(&mut Registry::default()));
    // serializing
    let dummy_value = Fields {
        foo: 42,
        bar: "I_AM_BAZ!".to_owned(),
        r#type: true,
        referenced_optional: OptionalValue(None),
        referenced_optional_inline: OptionalValue(Some(true)),
        inlined: Inlined(false),
        skippo: 1.0,
    };
    assert!(SchemaSerialize::is_present(&dummy_value));
    assert_json_snapshot!(SerdeAdapter(dummy_value));
    // deserializing
    assert!(Fields::deserialize_missing().is_none());
    assert_debug_snapshot!(serde_json::from_value::<SerdeAdapter<Fields>>(json!({
        "Foo": 42,
        "baz": "I_AM_BAZ!",
        "Type": true,
        "ReferencedOptional": false,
        "Inlined": true
    })));
    // deserializing with missing field
    assert_debug_snapshot!(serde_json::from_value::<SerdeAdapter<Fields>>(json!({
        "baz": "I_AM_BAZ!",
        "Type": true,
        "ReferencedOptional": false,
        "Inlined": true
    })));
    // deserializing with additional field
    assert_debug_snapshot!(serde_json::from_value::<SerdeAdapter<Fields>>(json!({
        "Foo": 42,
        "baz": "I_AM_BAZ!",
        "Type": true,
        "Inlined": true,
        "ExtraField": 1.0,
    })));
    // deserializing with null for a non-required field
    assert_debug_snapshot!(serde_json::from_value::<SerdeAdapter<Fields>>(json!({
        "Foo": 42,
        "baz": "I_AM_BAZ!",
        "Type": true,
        "ReferencedOptional": null,
        "Inlined": true
    })));
}

#[test]
fn unit_struct() {
    /// A unit struct.
    #[derive(Debug, ToSchema)]
    struct UnitStruct;

    assert_json_snapshot!(UnitStruct::schema(&mut Registry::default()));
    assert_json_snapshot!(SerdeAdapter(UnitStruct));
    assert_debug_snapshot!(serde_json::from_value::<SerdeAdapter<UnitStruct>>(json!(
        null
    )));
    assert_debug_snapshot!(serde_json::from_value::<SerdeAdapter<UnitStruct>>(
        json!({})
    ));
}

#[test]
fn test_enum() {
    #[derive(Debug, ToSchema)]
    struct NamedFieldsStruct {
        thing: bool,
    }
    #[derive(Debug, ToSchema)]
    #[schema(
        tag = "tag",
        rename_all = "camelCase",
        rename_all_fields = "PascalCase"
    )]
    enum Enum {
        Struct(NamedFieldsStruct),
        r#NamedFields {
            foo_bar: i32,
        },
        /// Another thing.
        #[schema(rename = "AnOtHeR", rename_all = "SCREAMING_SNAKE_CASE")]
        Another {
            bar_foo: i64,
        },
        /// A constant string.
        #[schema(untagged)]
        Constant,
        /// An untagged string.
        #[schema(untagged)]
        Untagged(String),
        #[schema(null, untagged)]
        Null,
    }

    // schema
    assert_eq!(Enum::REQUIRED, true);
    assert_eq!(Enum::ALWAYS_INLINED, false);
    assert_json_snapshot!(Enum::schema(&mut Registry::default()));

    // serialize
    assert_json_snapshot!(SerdeAdapter(Enum::Struct(NamedFieldsStruct {
        thing: true
    })));
    assert_json_snapshot!(SerdeAdapter(Enum::NamedFields { foo_bar: 32 }));
    assert_json_snapshot!(SerdeAdapter(Enum::Another { bar_foo: 42 }));
    assert_json_snapshot!(SerdeAdapter(Enum::Constant));
    assert_json_snapshot!(SerdeAdapter(Enum::Untagged("hello".to_owned())));
    assert_json_snapshot!(SerdeAdapter(Enum::Null));

    // deserialize
    assert!(Enum::deserialize_missing().is_none());
    assert_debug_snapshot!(serde_json::from_value::<SerdeAdapter<Enum>>(json!({
      "tag": "struct",
      "thing": true
    })));
    assert_debug_snapshot!(serde_json::from_value::<SerdeAdapter<Enum>>(json!({
      "tag": "namedFields",
      "FooBar": 32
    })));
    assert_debug_snapshot!(serde_json::from_value::<SerdeAdapter<Enum>>(json!({
        "tag": "AnOtHeR",
        "BAR_FOO": 42
    })));
    assert_debug_snapshot!(serde_json::from_value::<SerdeAdapter<Enum>>(json!(
        "constant"
    )));
    assert_debug_snapshot!(serde_json::from_value::<SerdeAdapter<Enum>>(json!("hello")));
    assert_debug_snapshot!(serde_json::from_value::<SerdeAdapter<Enum>>(json!(null)));
}

#[test]
fn enum_optional() {
    #[derive(Debug, ToSchema)]
    #[schema(tag = "tag")]
    enum Enum {
        Str {
            val: String,
        },
        #[schema(untagged)]
        Thing(OptionalValue),
        #[schema(untagged)]
        AnotherThing(OptionalValue),
    }
    assert_eq!(Enum::ALWAYS_INLINED, false);
    assert_eq!(Enum::REQUIRED, false);

    assert_json_snapshot!(Enum::schema(&mut Registry::default()));

    assert_debug_snapshot!(Enum::deserialize_missing());

    #[derive(Debug, ToSchema)]
    struct UseEnum {
        value: Enum,
    }
    assert_json_snapshot!(UseEnum::schema(&mut Registry::default()));
    assert_json_snapshot!(SerdeAdapter(UseEnum {
        value: Enum::AnotherThing(OptionalValue(None))
    }));
    assert_debug_snapshot!(serde_json::from_value::<SerdeAdapter<UseEnum>>(json!({})));

    assert_json_snapshot!(SerdeAdapter(UseEnum {
        value: Enum::Str {
            val: "val_string".to_owned()
        }
    }))
}

#[test]
pub fn enum_unit_only() {
    /// A unit-only enum
    #[derive(Debug, ToSchema)]
    #[schema(untagged)]
    pub enum Enum {
        Val1,
        Val2,
    }
    assert_json_snapshot!(Enum::schema(&mut Registry::default()));
    assert_json_snapshot!(SerdeAdapter(Enum::Val1));
    assert_debug_snapshot!(serde_json::from_value::<SerdeAdapter<Enum>>(json!("Val1")));
    assert_debug_snapshot!(serde_json::from_value::<SerdeAdapter<Enum>>(json!(
        "Invalid"
    )));
}

#[test]
pub fn using() {
    // in named fields struct

    #[derive(Debug, ToSchema)]
    struct UsingStruct {
        /// Optional in schema
        #[schema(using = "Option<i64>")]
        value: Nullable<i64>,
    }
    assert_json_snapshot!(UsingStruct::schema(&mut Registry::default()));
    assert_json_snapshot!(SerdeAdapter(UsingStruct {
        value: Nullable(Some(42)),
    }));
    assert_json_snapshot!(SerdeAdapter(UsingStruct {
        value: Nullable(None),
    }));
    assert_debug_snapshot!(serde_json::from_value::<SerdeAdapter<UsingStruct>>(json!({
        "value": 42,
    })));
    assert_debug_snapshot!(serde_json::from_value::<SerdeAdapter<UsingStruct>>(json!(
        {}
    )));

    // in tuple struct

    #[derive(Debug, ToSchema)]
    #[schema(using = "Option<i64>")]
    /// Optional integer.
    struct UsingTupleStruct(Nullable<i64>);
    assert_eq!(UsingTupleStruct::REQUIRED, false);
    assert_json_snapshot!(UsingTupleStruct::schema(&mut Registry::default()));
    assert_json_snapshot!(SerdeAdapter(UsingTupleStruct(Nullable(Some(42)))));
    assert_eq!(UsingTupleStruct(Nullable(None)).is_present(), false);
    assert_debug_snapshot!(serde_json::from_value::<SerdeAdapter<UsingTupleStruct>>(
        json!(42)
    ));
    assert_debug_snapshot!(UsingTupleStruct::deserialize_missing());

    // in enum

    #[derive(Debug, ToSchema)]
    #[schema(untagged)]
    enum UsingEnum {
        Fields {
            /// optional int value in named fields
            #[schema(using = "Option<i64>")]
            value: Nullable<i64>,
        },
        /// optional int
        #[schema(using = "Option<i64>")]
        Tuple(Nullable<i64>),
    }
    assert_eq!(UsingEnum::REQUIRED, false);
    assert_eq!(
        UsingEnum::Fields {
            value: Nullable(None)
        }
        .is_present(),
        true
    );
    assert_eq!(UsingEnum::Tuple(Nullable(None)).is_present(), false);

    assert_json_snapshot!(UsingEnum::schema(&mut Registry::default()));
    assert_json_snapshot!(SerdeAdapter(UsingEnum::Tuple(Nullable(Some(42)))));
    assert_json_snapshot!(SerdeAdapter(UsingEnum::Fields {
        value: Nullable(Some(42))
    }));
    assert_json_snapshot!(SerdeAdapter(UsingEnum::Fields {
        value: Nullable(None)
    }));

    assert_debug_snapshot!(serde_json::from_value::<SerdeAdapter<UsingEnum>>(json!(42)));
    assert_debug_snapshot!(serde_json::from_value::<SerdeAdapter<UsingEnum>>(json!({})));
    assert_debug_snapshot!(serde_json::from_value::<SerdeAdapter<UsingEnum>>(
        json!({"value": 42})
    ));
    assert_debug_snapshot!(UsingEnum::deserialize_missing());

    // optional
    #[derive(Debug, ToSchema)]
    struct OptionalField {
        #[schema(optional)]
        field: String,
    }
    assert_json_snapshot!(OptionalField::schema(&mut Registry::default()));
    assert_debug_snapshot!(serde_json::from_value::<SerdeAdapter<OptionalField>>(
        json!({})
    ));
    assert_debug_snapshot!(serde_json::from_value::<SerdeAdapter<OptionalField>>(
        json!({"field": "str"})
    ));
    assert_json_snapshot!(SerdeAdapter(OptionalField {
        field: String::from("str")
    }));

    // optional
    #[derive(Debug, ToSchema)]
    struct NullableField {
        #[schema(nullable)]
        field: String,
    }
    assert_json_snapshot!(NullableField::schema(&mut Registry::default()));
    assert_debug_snapshot!(serde_json::from_value::<SerdeAdapter<NullableField>>(
        json!({"field": null})
    ));
    assert_debug_snapshot!(serde_json::from_value::<SerdeAdapter<NullableField>>(
        json!({"field": "str"})
    ));
    assert_json_snapshot!(SerdeAdapter(NullableField {
        field: String::from("str")
    }));
}
