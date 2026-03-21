use proc_macro2::TokenStream;
use quote::{format_ident, quote, quote_spanned};
use syn::{Ident, Lifetime, spanned::Spanned};

use crate::{
    to_schema::model::{
        Enum, EnumVariant, EnumVariantCommon, NamedFields, NamedFieldsStruct, ToSchemaDeriveInput,
        TupleStruct, UnitStruct,
    },
    utils::GenericsHelper,
};

struct DeriveContent {
    schema_serialize: TokenStream,
    is_present: TokenStream,
}

pub fn impl_serialize(input: &ToSchemaDeriveInput) -> TokenStream {
    if input.impl_settings().no_serialize.is_present() {
        return quote!();
    }
    let generics = input
        .impl_settings()
        .generics()
        .add_bound_for_all_type_params(quote!(_autapi::schema::SchemaSerialize));
    let DeriveContent {
        schema_serialize,
        is_present,
    } = match input {
        ToSchemaDeriveInput::UnitStruct(input) => impl_serialize_unit_struct(input),
        ToSchemaDeriveInput::Enum(input) => impl_serialize_enum(input, &generics),
        ToSchemaDeriveInput::NamedFieldsStruct(input) => {
            impl_serialize_named_fields_struct(input, &generics)
        }
        ToSchemaDeriveInput::TupleStruct(input) => impl_serialize_tuple_struct(input),
    };
    let ty = &input.impl_settings().type_ident;
    let (gen_impl, gen_ty, where_clause) = generics.split_for_impl();
    let serde = if input.impl_settings().serde.is_present() {
        quote! {
            impl<#gen_impl> _autapi::private::serde::Serialize for #ty<#gen_ty> #where_clause {
                fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                where
                    S: _autapi::private::serde::Serializer
                {
                    const _: () = {
                        if !<#ty<#gen_ty> as _autapi::schema::ToSchema>::REQUIRED {
                            panic!("type must be a required schema for auto-implementing `serde::Serialize`")
                        }
                    };
                    _autapi::schema::SchemaSerialize::schema_serialize(self, serializer)
                }
            }
        }
    } else {
        quote!()
    };
    quote! {
        impl<#gen_impl> _autapi::schema::SchemaSerialize for #ty<#gen_ty> #where_clause {
            fn schema_serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: _autapi::private::serde::Serializer
            {
                #schema_serialize
            }
            fn is_present(&self) -> bool {
                #is_present
            }
        }
        #serde
    }
}

fn impl_serialize_unit_struct(_: &UnitStruct) -> DeriveContent {
    DeriveContent {
        schema_serialize: quote!(_autapi::private::serde::Serialize::serialize(
            &(),
            serializer
        )),
        is_present: quote!(true),
    }
}

fn impl_serialize_tuple_struct(input: &TupleStruct) -> DeriveContent {
    let ty_span = input.schema.ty.span();
    let value = input.schema.serialize_value(quote!(&self.0));
    DeriveContent {
        schema_serialize: quote_spanned!(ty_span=>
            _autapi::schema::SchemaSerialize::schema_serialize(
                &#value, serializer
            )
        ),
        is_present: quote_spanned!(ty_span=> _autapi::schema::SchemaSerialize::is_present(&#value)),
    }
}

fn impl_serialize_named_fields_struct(
    input: &NamedFieldsStruct,
    generics: &GenericsHelper<'_>,
) -> DeriveContent {
    let fields = &input.named_fields;
    let (gen_impl, gen_ty, where_clause) = generics.split_for_impl();
    let define_block = define_named_fields_block_for(fields, generics);
    let value_block = map_to_named_fields_block(Some(format_ident!("self")), fields);
    let name = input.impl_settings.name();
    DeriveContent {
        schema_serialize: quote! {
            #[derive(_autapi::private::serde::Serialize)]
            #[serde(crate="_autapi::private::serde", bound="", rename=#name)]
            struct __autapi_Value<'__autapi, #gen_impl> #where_clause #define_block

            _autapi::private::serde::Serialize::serialize(
                &__autapi_Value::<'_, #gen_ty> #value_block, serializer
            )
        },
        is_present: quote!(true),
    }
}

fn impl_serialize_enum(input: &Enum, generics: &GenericsHelper<'_>) -> DeriveContent {
    let (gen_impl, gen_ty, where_clause) = generics.split_for_impl();

    let variant_names = input.variants.iter().map(|variant| &variant.ident);

    let helper_type_name =
        |variant: &EnumVariantCommon| format_ident!("__autapi_{}", variant.ident);
    let serialize_enums = input
        .unit_variants()
        .filter(|variant| !variant.null.is_present())
        .map(|variant| {
            let enum_name = helper_type_name(&variant.common);
            let variant_name = &variant.common.ident;
            let rename = &variant.common.name;
            quote! {
                #[derive(_autapi::private::serde::Serialize)]
                #[serde(crate="_autapi::private::serde", rename = #rename)]
                pub enum #enum_name {
                    #[serde(rename = #rename)]
                    #variant_name
                }
            }
        });

    let variant_contents = input.variants.iter().map(|variant| match variant {
        EnumVariant::NamedFields(variant) => {
            define_named_fields_block_for(&variant.fields, generics)
        }
        EnumVariant::Tuple(variant) => {
            let ty = &variant
                .schema
                .serialize_type(Lifetime::new("'__autapi", variant.schema.ty.span()));
            quote!((_autapi::adapters::SerdeAdapter<#ty>))
        }
        EnumVariant::Unit(variant) => {
            if variant.null.is_present() {
                quote!()
            } else {
                let ty = helper_type_name(&variant.common);
                quote!((#ty))
            }
        }
    });

    let original_variant_bodies = input.variants.iter().map(|variant| match variant {
        EnumVariant::NamedFields(variant) => {
            let fields = variant.fields.non_skipped().map(|field| &field.ident);
            quote!({#(#fields,)* ..})
        }
        EnumVariant::Tuple(_) => quote!((variant)),
        EnumVariant::Unit(_) => quote!(),
    });

    let mapped_variant_values = input.variants.iter().map(|variant| match variant {
        EnumVariant::NamedFields(variant) => map_to_named_fields_block(None, &variant.fields),
        EnumVariant::Tuple(variant) => {
            let value = variant
                .schema
                .serialize_value(Ident::new("variant", variant.schema.ty.span()));
            quote!((_autapi::adapters::SerdeAdapter(#value)))
        }
        EnumVariant::Unit(variant) => {
            if variant.null.is_present() {
                quote!()
            } else {
                let ty = helper_type_name(&variant.common);
                let var = &variant.common.ident;
                quote!((#ty::#var))
            }
        }
    });

    let variant_attrs = input.variants.iter().map(|variant| {
        let name = &variant.name;
        let mut attrs = quote!(rename = #name);
        if variant.tag_property.is_none() {
            attrs.extend(quote!(, untagged));
        }
        attrs
    });

    let enum_attrs = match &input.tag_property {
        Some(tag) => quote!(tag = #tag),
        None => quote!(untagged),
    };
    let name = input.impl_settings.name();

    DeriveContent {
        schema_serialize: {
            let variant_names2 = variant_names.clone();
            quote! {
                #(#serialize_enums)*

                #[derive(_autapi::private::serde::Serialize)]
                #[serde(crate="_autapi::private::serde", bound="", rename=#name, #enum_attrs)]
                enum Value <'__autapi, #gen_impl> #where_clause {
                    #[serde(skip)]
                    Phantom(&'__autapi ()),
                    #(
                        #[serde(#variant_attrs)]
                        #variant_names #variant_contents,
                    )*
                }
                let value = match self {
                    #(
                        Self::#variant_names2 #original_variant_bodies => Value::#variant_names2::<'_, #gen_ty> #mapped_variant_values,
                    )*
                };
                _autapi::private::serde::Serialize::serialize(&value, serializer)
            }
        },
        is_present: {
            let transparent_variant_names = input
                .transparent_variants()
                .map(|variant| &variant.common.ident);
            let transparent_variant_values = input
                .transparent_variants()
                .map(|variant| variant.schema.serialize_value(quote!(variant)));
            quote! {
                match self {
                    #(
                        Self::#transparent_variant_names(variant) =>
                            _autapi::schema::SchemaSerialize::is_present(&#transparent_variant_values),
                    )*
                    _ => true,
                }
            }
        },
    }
}

fn define_named_fields_block_for(
    named_fields: &NamedFields,
    generics: &GenericsHelper<'_>,
) -> TokenStream {
    let fields = named_fields.non_skipped().map(|field| {
        let (ident, ty, name) = (
            &field.ident,
            field
                .schema
                .serialize_type(Lifetime::new("'__autapi", field.ident.span())),
            &field.schema_name,
        );
        quote! {
            #[serde(
                skip_serializing_if="_autapi::adapters::SerdeAdapter::skip_serializing",
                rename=#name
            )]
            #ident: _autapi::adapters::SerdeAdapter<#ty>
        }
    });
    let phantom_data = generics.make_phantom_data();
    quote!({
        #(#fields,)*
        #[serde(skip)]
        __generic_phantom: ::std::marker::PhantomData<(&'__autapi (), #phantom_data)>,
    })
}

fn map_to_named_fields_block(original: Option<Ident>, fields: &NamedFields) -> TokenStream {
    let field_values = fields.non_skipped().map(|field| {
        let ident = &field.ident;
        let val = match &original {
            Some(v) => quote!(&#v.#ident),
            None => quote!(#ident),
        };
        let val = field.schema.serialize_value(val);
        quote!(_autapi::adapters::SerdeAdapter(#val))
    });
    let field_names = fields.non_skipped().map(|field| &field.ident);
    quote!({
        #(#field_names: #field_values,)*
        __generic_phantom: ::std::marker::PhantomData,
    })
}
