use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident, quote};
use syn::LitStr;

use crate::{
    to_schema::model::{Enum, EnumVariant, SchemaRef, ToSchemaDeriveInput, Validation},
    utils::NumberMeta,
};

pub fn impl_validate(input: &ToSchemaDeriveInput) -> TokenStream {
    let settings = input.impl_settings();
    let generics = settings.generics();
    let (gen_impl, gen_ty, gen_where) = generics.split_for_impl();
    let ty = &settings.type_ident;

    let checks = match input {
        ToSchemaDeriveInput::UnitStruct(_) => quote!(),
        ToSchemaDeriveInput::TupleStruct(input) => {
            validate_stmt(&input.schema, quote!(&self.0), None)
        }
        ToSchemaDeriveInput::NamedFieldsStruct(input) => input
            .named_fields
            .non_skipped()
            .map(|field| {
                let ident = &field.ident;
                validate_stmt(
                    &field.schema,
                    quote!(&self.#ident),
                    Some(&field.schema_name),
                )
            })
            .collect(),
        ToSchemaDeriveInput::Enum(input) => validate_enum(input),
    };

    quote! {
        impl<#gen_impl> _autapi::schema::Validate for #ty<#gen_ty> #gen_where {
            fn validate(&self) -> Result<(), _autapi::schema::ValidationError> {
                #checks
                Ok(())
            }
        }
    }
}

fn validate_enum(input: &Enum) -> TokenStream {
    let arms = input.variants.iter().filter_map(|variant| match variant {
        EnumVariant::Tuple(variant) if variant.schema.validation.any_set() => {
            let variant_name = &variant.common.ident;
            let checks = validate_stmt(&variant.schema, quote!(value), None);
            Some(quote!(Self::#variant_name(value) => { #checks }))
        }
        EnumVariant::NamedFields(variant) => {
            let fields = variant
                .fields
                .non_skipped()
                .filter(|field| field.schema.validation.any_set());
            fields.clone().next()?;
            let variant_name = &variant.common.ident;
            let idents = fields.clone().map(|field| &field.ident);
            let checks = fields.map(|field| {
                let ident = &field.ident;
                validate_stmt(&field.schema, quote!(#ident), Some(&field.schema_name))
            });
            Some(quote!(Self::#variant_name { #(#idents,)* .. } => { #(#checks)* }))
        }
        _ => None,
    });
    quote! {
        #[allow(unreachable_patterns)]
        match self {
            #(#arms)*
            _ => {}
        }
    }
}

/// Returns a statement that validates the reference expression `value` to a value of type
/// `schema.ty`, returning early on the first violation.
///
/// For `using` fields, the constraints describe the `using` type, so the value is converted via
/// `SchemaUsing::to_ser` first.
fn validate_stmt(schema: &SchemaRef, value: TokenStream, property: Option<&LitStr>) -> TokenStream {
    if !schema.validation.any_set() {
        return quote!();
    }
    let value = match &schema.using {
        Some(using) => {
            let ty = &schema.ty;
            quote!(&<#ty as _autapi::schema::SchemaUsing<#using>>::to_ser(#value))
        }
        None => value,
    };
    let validation = validation_expr(schema, value, property);
    quote!(#validation?;)
}

/// Returns an expression of type `Result<(), ValidationError>` that checks the value of the
/// reference expression `value` against all validation constraints of `schema`.
///
/// `value` must be a reference to a value of the deserialize type of `schema`.
pub fn validation_expr(
    schema: &SchemaRef,
    value: impl ToTokens,
    property: Option<&LitStr>,
) -> TokenStream {
    let checks = checks(&schema.validation, property);
    quote! {
        (|| -> Result<(), _autapi::schema::ValidationError> {
            let value = #value;
            #checks
            Ok(())
        })()
    }
}

/// Returns statements that check the value of the reference `value` against all validation
/// constraints, returning early on the first violation.
fn checks(validation: &Validation, property: Option<&LitStr>) -> TokenStream {
    let Validation {
        minimum,
        maximum,
        exclusive_minimum,
        exclusive_maximum,
        multiple_of,
        min_length,
        max_length,
        min_items,
        max_items,
        min_properties,
        max_properties,
        pattern,
    } = validation;
    let property = match property {
        Some(property) => quote!(Some(#property)),
        None => quote!(None),
    };

    let mut checks = TokenStream::new();
    let mut check = |name: &str, bound: TokenStream| {
        let func = format_ident!("{name}");
        checks.extend(quote!(
            _autapi::private::validation::#func(value, #bound, #property)?;
        ));
    };
    for (name, bound) in [
        ("minimum", minimum),
        ("maximum", maximum),
        ("exclusive_minimum", exclusive_minimum),
        ("exclusive_maximum", exclusive_maximum),
        ("multiple_of", multiple_of),
    ] {
        if let Some(bound) = bound {
            check(name, numeric(*bound));
        }
    }
    if min_length.is_some() || max_length.is_some() {
        // checked together to count the characters only once
        let (min, max) = (option(min_length), option(max_length));
        check("length", quote!(#min, #max));
    }
    for (name, bound) in [
        ("min_items", min_items),
        ("max_items", max_items),
        ("min_properties", min_properties),
        ("max_properties", max_properties),
    ] {
        if let Some(bound) = bound {
            check(name, quote!(#bound));
        }
    }
    if let Some(pattern) = pattern {
        checks.extend(quote!(
            _autapi::__validate_pattern!(value, #pattern, #property)?;
        ));
    }
    checks
}

/// Returns a `Numeric` expression for the given number.
fn numeric(number: NumberMeta) -> TokenStream {
    match number {
        NumberMeta::Int(n) => {
            let n = i128::from(n);
            quote!(_autapi::schema::Numeric::Int(#n))
        }
        NumberMeta::UInt(n) => {
            let n = u128::from(n);
            quote!(_autapi::schema::Numeric::UInt(#n))
        }
        NumberMeta::Float(n) => quote!(_autapi::schema::Numeric::Float(#n)),
    }
}

/// Returns an `Option<u64>` expression for the given value.
fn option(value: &Option<u64>) -> TokenStream {
    match value {
        Some(value) => quote!(Some(#value)),
        None => quote!(None),
    }
}
