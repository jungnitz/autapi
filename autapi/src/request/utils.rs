use crate::{
    Registry,
    openapi::{MaybeRef, Operation, Parameter, ParameterIn, Type},
    schema::ToSchema,
};

pub fn add_parameters_to_operation<T: ToSchema>(
    operation: &mut Operation,
    registry: &mut Registry,
    parameter_in: ParameterIn,
    err_name: &'static str,
) {
    let schema = T::schema(registry);
    let MaybeRef::T(schema) = schema else {
        panic!("schema for {err_name} parameter cannot be a Ref");
    };
    if !schema
        .schema_type
        .is_some_and(|typ| typ.contains(&Type::Object))
    {
        panic!("{err_name} parameter schema has to be an object");
    };
    operation.parameters.extend(
        schema
            .properties
            .into_iter()
            .map(|(property_name, property)| {
                let required = schema.required.contains(&property_name);
                Parameter::new(property_name, parameter_in.clone())
                    .with_schema(property)
                    .with_required(required)
                    .into()
            }),
    );
}
