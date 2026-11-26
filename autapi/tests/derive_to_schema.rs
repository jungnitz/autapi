use autapi::{
    Registry,
    adapters::SerdeAdapter,
    openapi::Format,
    schema::{Nullable, SchemaDeserialize, SchemaSerialize, ToSchema, Validate, ValidateStr},
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
    }));
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

#[test]
fn validation() {
    /// A validated newtype.
    #[derive(Debug, ToSchema)]
    #[schema(min_length = 3, max_length = 8)]
    struct Username(String);

    // allows adding string constraints to fields of type `Username`
    impl ValidateStr for Username {
        fn validation_str(&self) -> Option<&str> {
            Some(&self.0)
        }
    }

    #[derive(Debug, ToSchema)]
    struct Validated {
        #[schema(minimum = -10, exclusive_maximum = 2.5, multiple_of = 0.5)]
        number: f64,
        #[schema(maximum = 100)]
        optional: Option<u8>,
        #[schema(min_items = 1, max_items = 2)]
        items: Vec<bool>,
        #[schema(min_properties = 1)]
        map: std::collections::BTreeMap<String, bool>,
        /// A referenced type with additional constraints
        #[schema(max_length = 5)]
        username: Username,
    }

    #[derive(Debug, ToSchema)]
    #[schema(tag = "type")]
    enum ValidatedEnum {
        #[schema(rename = "named")]
        Named {
            #[schema(minimum = 0)]
            value: i32,
            other: i32,
        },
        #[schema(untagged, minimum = 1)]
        Tuple(i32),
    }

    // schema
    assert_json_snapshot!(Username::schema(&mut Registry::default()));
    assert_json_snapshot!(Validated::schema(&mut Registry::default()));

    // deserializing
    let deserialize = |value| {
        serde_json::from_value::<SerdeAdapter<Validated>>(value)
            .map(|value| value.0)
            .map_err(|err| err.to_string())
    };
    let valid = json!({
        "number": -1.5,
        "items": [true],
        "map": { "a": true },
        "username": "abcd",
    });
    assert!(deserialize(valid.clone()).is_ok());
    let with = |key: &str, value: serde_json::Value| {
        let mut object = valid.clone();
        object[key] = value;
        deserialize(object).unwrap_err()
    };
    assert_eq!(
        with("number", json!(-10.5)),
        "`number`: must be at least -10"
    );
    assert_eq!(
        with("number", json!(2.5)),
        "`number`: must be less than 2.5"
    );
    assert_eq!(
        with("number", json!(0.3)),
        "`number`: must be a multiple of 0.5"
    );
    assert_eq!(
        with("optional", json!(101)),
        "`optional`: must be at most 100"
    );
    assert_eq!(
        with("items", json!([])),
        "`items`: must contain at least 1 items"
    );
    assert_eq!(
        with("items", json!([true, false, true])),
        "`items`: must contain at most 2 items"
    );
    assert_eq!(
        with("map", json!({})),
        "`map`: must contain at least 1 properties"
    );
    // the newtype's own constraint is checked on deserializing the newtype
    assert_eq!(
        with("username", json!("ab")),
        "must be at least 3 characters long"
    );
    assert_eq!(
        with("username", json!("abcdef")),
        "`username`: must be at most 5 characters long"
    );

    // validating manually
    assert!(Username("abc".to_owned()).validate().is_ok());
    let err = Username("a".to_owned()).validate().unwrap_err();
    assert_eq!(err.property(), None);
    assert_eq!(err.to_string(), "must be at least 3 characters long");

    // enums
    assert!(
        ValidatedEnum::Named {
            value: 0,
            other: -1
        }
        .validate()
        .is_ok()
    );
    assert_eq!(
        ValidatedEnum::Named {
            value: -1,
            other: 0
        }
        .validate()
        .unwrap_err()
        .to_string(),
        "`value`: must be at least 0"
    );
    assert!(
        serde_json::from_value::<SerdeAdapter<ValidatedEnum>>(
            json!({"type": "named", "value": -1, "other": 0})
        )
        .is_err()
    );
    assert!(ValidatedEnum::Tuple(1).validate().is_ok());
    assert!(serde_json::from_value::<SerdeAdapter<ValidatedEnum>>(json!(0)).is_err());
}

#[cfg(feature = "regex")]
#[test]
fn validation_pattern() {
    #[derive(Debug, ToSchema)]
    struct Pattern {
        #[schema(pattern = "^[a-z]+$")]
        name: String,
        #[schema(pattern = "^[0-9]+$")]
        optional: Option<String>,
    }

    assert_json_snapshot!(Pattern::schema(&mut Registry::default()));
    let deserialize = |value| serde_json::from_value::<SerdeAdapter<Pattern>>(value);
    assert!(deserialize(json!({"name": "abc"})).is_ok());
    assert!(deserialize(json!({"name": "abc", "optional": "123"})).is_ok());
    assert_eq!(
        deserialize(json!({"name": "ABC"})).unwrap_err().to_string(),
        "`name`: must match the pattern `^[a-z]+$`"
    );
    assert_eq!(
        deserialize(json!({"name": "abc", "optional": "x"}))
            .unwrap_err()
            .to_string(),
        "`optional`: must match the pattern `^[0-9]+$`"
    );
}

#[test]
fn format() {
    /// An email address.
    #[derive(Debug, ToSchema)]
    #[schema(format = "idn-email", max_length = 254)]
    struct Email(String);

    #[derive(Debug, ToSchema)]
    struct Formatted {
        #[schema(format = "uri")]
        url: String,
        #[schema(format = "custom-format")]
        custom: String,
    }

    assert_json_snapshot!(Email::schema(&mut Registry::default()));
    assert_eq!(
        Email::schema(&mut Registry::default())
            .into_inner()
            .unwrap()
            .format,
        Some(Format::IdnEmail)
    );
    assert_json_snapshot!(Formatted::schema(&mut Registry::default()));

    // the format is not validated
    assert!(serde_json::from_value::<SerdeAdapter<Email>>(json!("not an email")).is_ok());
}

#[test]
fn validation_while_deserializing() {
    fn deserialize<T: SchemaDeserialize>(value: serde_json::Value) -> Result<T, String> {
        serde_json::from_value::<SerdeAdapter<T>>(value)
            .map(|value| value.0)
            .map_err(|err| err.to_string())
    }

    // violations of one variant fall through to the next variant
    #[derive(Debug, PartialEq, ToSchema)]
    #[schema(untagged)]
    enum Name {
        #[schema(max_length = 3)]
        Short(String),
        Long(String),
    }
    assert_eq!(deserialize(json!("abc")), Ok(Name::Short("abc".to_owned())));
    assert_eq!(
        deserialize(json!("abcdef")),
        Ok(Name::Long("abcdef".to_owned()))
    );

    // constraints of `using` fields apply to the `using` type, absent values are not validated
    #[derive(Debug, ToSchema)]
    struct Optional {
        #[schema(optional, minimum = 1)]
        count: u32,
    }
    assert_eq!(deserialize::<Optional>(json!({})).unwrap().count, 0);
    assert_eq!(
        deserialize::<Optional>(json!({"count": 0})).unwrap_err(),
        "`count`: must be at least 1"
    );
    assert!(Optional { count: 1 }.validate().is_ok());
    assert!(Optional { count: 0 }.validate().is_err());

    // lengths are counted in characters
    #[derive(Debug, ToSchema)]
    #[schema(min_length = 2, max_length = 3)]
    struct Short(String);
    assert!(deserialize::<Short>(json!("äö")).is_ok());
    assert_eq!(
        deserialize::<Short>(json!("ä")).unwrap_err(),
        "must be at least 2 characters long"
    );
    assert_eq!(
        deserialize::<Short>(json!("äöüß")).unwrap_err(),
        "must be at most 3 characters long"
    );
}
