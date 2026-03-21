use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::LitStr;

use crate::{
    to_schema::model::{
        Enum, EnumVariant, EnumVariantCommon, NamedFields, NamedFieldsStruct, ToSchemaDeriveInput,
        TupleStruct, UnitStruct,
    },
    utils::GenericsHelper,
};

struct DeserializeImpl {
    schema_deserialize: TokenStream,
    deserialize_missing: TokenStream,
}

pub fn impl_deserialize(input: &ToSchemaDeriveInput) -> TokenStream {
    if input.impl_settings().no_deserialize.is_present() {
        return quote!();
    }
    let generics = input
        .impl_settings()
        .generics()
        .add_bound_for_all_type_params(quote!(_autapi::schema::SchemaDeserialize));
    let DeserializeImpl {
        schema_deserialize,
        deserialize_missing,
    } = match input {
        ToSchemaDeriveInput::UnitStruct(input) => impl_deserialize_unit_struct(input),
        ToSchemaDeriveInput::TupleStruct(input) => impl_deserialize_tuple_struct(input),
        ToSchemaDeriveInput::NamedFieldsStruct(input) => {
            impl_deserialize_named_fields_struct(input, &generics)
        }
        ToSchemaDeriveInput::Enum(input) => impl_deserialize_enum(input, &generics),
    };
    let (gen_impl, gen_ty, gen_where) = generics.split_for_impl();
    let ty = &input.impl_settings().type_ident;

    let serde = if input.impl_settings().serde.is_present() {
        quote! {
            impl<'de, #gen_impl> _autapi::private::serde::Deserialize<'de> for #ty<#gen_ty> #gen_where {
                fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                where
                    D: _autapi::private::serde::Deserializer<'de>,
                {
                    <Self as _autapi::schema::SchemaDeserialize>::schema_deserialize(deserializer)
                }
            }
        }
    } else {
        quote!()
    };

    quote! {
        impl<#gen_impl> _autapi::schema::SchemaDeserialize for #ty<#gen_ty> #gen_where {
            fn schema_deserialize<'de, D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: _autapi::private::serde::Deserializer<'de>,
            {
                #schema_deserialize
            }
            fn deserialize_missing() -> Option<Self> {
                #deserialize_missing
            }
        }
        #serde
    }
}

fn impl_deserialize_unit_struct(_: &UnitStruct) -> DeserializeImpl {
    DeserializeImpl {
        schema_deserialize: quote! {
            <() as _autapi::private::serde::Deserialize<'de>>::deserialize(deserializer)?;
            Ok(Self)
        },
        deserialize_missing: quote!(None),
    }
}

fn impl_deserialize_tuple_struct(input: &TupleStruct) -> DeserializeImpl {
    let ty = input.schema.deserialize_type();
    let from_de = |value: TokenStream| {
        let map = input.schema.deserialize_value(quote!(value));
        quote!(#value.map(|value| #map).map(Self))
    };
    DeserializeImpl {
        schema_deserialize: from_de(quote!(
            <#ty as _autapi::schema::SchemaDeserialize>::schema_deserialize(deserializer)
        )),
        deserialize_missing: from_de(quote!(
            <#ty as _autapi::schema::SchemaDeserialize>::deserialize_missing()
        )),
    }
}

fn impl_deserialize_named_fields_struct(
    input: &NamedFieldsStruct,
    generics: &GenericsHelper<'_>,
) -> DeserializeImpl {
    let deserialize = deserialize_named_fields(
        &input.named_fields,
        quote!(Self),
        generics,
        &input.impl_settings.name(),
    );
    DeserializeImpl {
        schema_deserialize: quote!(Ok(#deserialize)),
        deserialize_missing: quote!(None),
    }
}

fn impl_deserialize_enum(input: &Enum, generics: &GenericsHelper<'_>) -> DeserializeImpl {
    // mapping our enum deserialization to serde is a bit tricky, at least for named field variants:
    // We have to create a struct for each of these variants that contains effectively the same
    // fields.
    // We then implement serde::Deserialize using `deserialize_named_fields` for this struct.

    let enum_name = &input.impl_settings.type_ident;
    let (gen_impl, gen_ty, where_clause) = generics.split_for_impl();

    let helper_type_name =
        |variant: &EnumVariantCommon| format_ident!("__autapi_{}", variant.ident);
    // definitions for the "replacement structs" for all named fields variants
    let deserialize_structs = input.named_fields_variants().map(|variant| {
        let struct_name = helper_type_name(&variant.common);
        let variant_name = &variant.common.ident;
        let value = deserialize_named_fields(
            &variant.fields,
            quote!(#enum_name::#variant_name),
            generics,
            &variant.common.name,
        );
        quote! {
            pub struct #struct_name<#gen_impl> (#enum_name<#gen_ty>) #where_clause;
            impl <'de, #gen_impl> _autapi::private::serde::Deserialize<'de>
                for #struct_name<#gen_ty>
            #where_clause
            {
                fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                where
                    D: _autapi::private::serde::Deserializer<'de>
                {
                    Ok(Self(#value))
                }
            }
        }
    });
    // definitions for the "replacement enums" for all unit variants
    let deserialize_enums = input
        .unit_variants()
        .filter(|variant| !variant.null.is_present())
        .map(|variant| {
            let enum_name = helper_type_name(&variant.common);
            let variant_name = &variant.common.ident;
            let rename = &variant.common.name;
            quote! {
                #[derive(_autapi::private::serde::Deserialize)]
                #[serde(crate="_autapi::private::serde", rename = #rename)]
                pub enum #enum_name {
                    #[serde(rename = #rename)]
                    #variant_name
                }
            }
        });

    // types of the variant definition: Variant(<this type>)
    let variant_types = input.variants.iter().map(|variant| match variant {
        EnumVariant::NamedFields(variant) => {
            let struct_name = helper_type_name(&variant.common);
            quote!(#struct_name<#gen_ty>)
        }
        EnumVariant::Tuple(variant) => {
            let ty = &variant.schema.deserialize_type();
            quote!(_autapi::adapters::SerdeAdapter<#ty>)
        }
        EnumVariant::Unit(variant) => {
            if variant.null.is_present() {
                quote!(())
            } else {
                let enum_name = helper_type_name(&variant.common);
                quote!(#enum_name)
            }
        }
    });

    // in deserialize implementation for the original enum:
    //  given inner variant value `variant` of the serde enum:
    //    the corresponding value of type `Self`
    let variant_values = input.variants.iter().map(|variant| match variant {
        EnumVariant::NamedFields(_) => {
            quote!(variant.0)
        }
        EnumVariant::Tuple(variant) => {
            let value = variant.schema.deserialize_value(quote!(variant.0));
            let variant_name = &variant.common.ident;
            quote!(Self::#variant_name(#value))
        }
        EnumVariant::Unit(variant) => {
            let variant_name = &variant.common.ident;
            quote!(Self::#variant_name)
        }
    });

    // attributes of the serde variants
    let variant_attrs = input.variants.iter().map(|variant| {
        let name = &variant.name;
        let mut attrs = quote!(rename = #name);
        if variant.tag_property.is_none() {
            attrs.extend(quote!(, untagged));
        }
        attrs
    });

    // attributes of the serde enum
    let enum_attrs = match &input.tag_property {
        Some(tag) => quote!(tag = #tag),
        _ => quote!(untagged),
    };

    let name = input.impl_settings.name();
    let variant_names = input.variants.iter().map(|variant| &variant.ident);

    DeserializeImpl {
        schema_deserialize: {
            let variant_names2 = variant_names.clone();
            quote! {
                #(#deserialize_structs)*
                #(#deserialize_enums)*

                #[derive(_autapi::private::serde::Deserialize)]
                #[serde(
                    crate="_autapi::private::serde",
                    bound="",
                    deny_unknown_fields,
                    rename=#name,
                    #enum_attrs
                )]
                enum __autapi_Value<#gen_impl> #where_clause {
                    #(
                        #[serde(#variant_attrs)]
                        #variant_names(#variant_types)
                    ),*
                }
                let value: __autapi_Value<#gen_ty> = _autapi::private::serde::Deserialize::deserialize(deserializer)?;
                Ok(match value {
                    #(
                        __autapi_Value::#variant_names2(variant) => #variant_values,
                    )*
                })
            }
        },
        deserialize_missing: {
            let transparent_types = input
                .transparent_variants()
                .map(|variant| variant.schema.deserialize_type());
            let transparent_values = input
                .transparent_variants()
                .map(|variant| variant.schema.deserialize_value(quote!(value)));
            let transparent_names = input
                .transparent_variants()
                .map(|variant| &variant.common.ident);
            quote! {
                #(
                    if let Some(value) = <#transparent_types as _autapi::schema::SchemaDeserialize>::deserialize_missing() {
                        return Some(Self::#transparent_names(#transparent_values));
                    }
                )*
                None
            }
        },
    }
}

/// Uses the deserializer `deserializer` to deserialize the current value into a named field value.
///
/// The resulting expression value is constructed via
/// ```text
/// #block_prefix {
///   <fields>
///   #additional_fields
/// }
/// ```
fn deserialize_named_fields(
    fields: &NamedFields,
    block_prefix: TokenStream,
    generics: &GenericsHelper<'_>,
    name: &LitStr,
) -> TokenStream {
    let (gen_impl, gen_ty, where_clause) = generics.split_for_impl();
    let phantom_data = generics.make_phantom_data();
    let struct_fields = fields.non_skipped().map(|field| {
        let (ident, ty, name) = (
            &field.ident,
            field.schema.deserialize_type(),
            &field.schema_name,
        );
        quote! {
            #[serde(
                default,
                rename = #name,
                deserialize_with="_autapi::private::deserialize_some"
            )]
            #ident: Option<_autapi::adapters::SerdeAdapter<#ty>>
        }
    });

    let field_values = fields.fields.iter().map(|field| {
        if field.skip.is_present() {
            quote!(Default::default())
        } else {
            let ident = &field.ident;
            let ty = field.schema.deserialize_type();
            let value = field.schema.deserialize_value(quote!(value));
            let missing_error_message = format!("missing property `{}`", field.schema_name.value());
            quote!(
                value.#ident
                    .map(|value| value.0)
                    .or_else(|| <#ty as _autapi::schema::SchemaDeserialize>::deserialize_missing())
                    .map(|value| #value)
                    .ok_or_else(|| <D::Error as _autapi::private::serde::de::Error>::custom(#missing_error_message))?
            )
        }
    });
    let field_names = fields.fields.iter().map(|field| &field.ident);

    quote!({
        #[derive(_autapi::private::serde::Deserialize)]
        #[serde(crate="_autapi::private::serde", deny_unknown_fields, bound="", rename=#name)]
        struct __autapi_Value<#gen_impl> #where_clause {
            #(#struct_fields,)*
            #[serde(skip)]
            __generic_phantom: #phantom_data,
        }
        let value = <__autapi_Value<#gen_ty> as _autapi::private::serde::Deserialize>::deserialize(deserializer)?;
        #block_prefix {
            #(#field_names: #field_values,)*
        }
    })
}
