use serde_json::Value;

use crate::{
    Registry,
    openapi::{Entry, MaybeRef, Schema, Type},
};

pub fn add_tag_to_schema<'a>(
    registry: &'a mut Registry,
    schema: MaybeRef<Schema>,
    tag_attr: &'a str,
    tag: &'a str,
) -> MaybeRef<Schema> {
    fn is_known_string_constant(
        registry: &Registry,
        schema: &MaybeRef<Schema>,
        value: &str,
    ) -> bool {
        if let Some(schema) = registry.resolve_schema(schema)
            && let Some(types) = &schema.schema_type
            && types.contains(&Type::String)
            && schema.enum_values.len() == 1
            && let Value::String(string) = &schema.enum_values[0]
            && *string == value
        {
            true
        } else {
            false
        }
    }

    fn make_all_of(schema: MaybeRef<Schema>, tag_attr: &str, tag: &str) -> Schema {
        let tag_schema = Schema::default()
            .with_schema_type(Type::Object)
            .with_properties_entry(tag_attr, MaybeRef::T(Schema::new_string_constant(tag)))
            .with_required_entry(tag_attr);
        Schema::default().with_all_of(vec![MaybeRef::T(tag_schema), schema])
    }

    // if the schema already contains the tag field, we don't need to add it again
    if let Some(resolved_schema) = registry.resolve_schema(&schema)
        && let Some(typ) = &resolved_schema.schema_type
        && typ.contains(&Type::Object)
        && let Some(tag_schema) = resolved_schema.properties.get(tag_attr)
        && is_known_string_constant(registry, tag_schema, tag)
        && resolved_schema.properties.contains_key(tag)
    {
        return schema;
    }

    let schema = if let MaybeRef::T(mut schema) = schema {
        // try to inline the tag into an object schema
        if let Some(typ) = &schema.schema_type
            && typ.contains(&Type::Object)
            && let Entry::Vacant(entry) = schema.properties.entry(tag_attr.to_owned())
        {
            entry.insert(MaybeRef::T(Schema::new_string_constant(tag.to_owned())));
            schema.required.push(tag_attr.to_string());
            schema
        } else {
            make_all_of(MaybeRef::T(schema), tag_attr, tag)
        }
    } else {
        make_all_of(schema, tag_attr, tag)
    };
    MaybeRef::T(schema)
}
