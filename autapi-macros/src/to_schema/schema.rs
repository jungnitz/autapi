use std::borrow::Cow;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{GenericParam, punctuated::Punctuated, token::Comma};

use crate::{
    to_schema::{
        attrs::SchemaMetaAttrs,
        model::{
            Enum, EnumVariant, NamedFields, NamedFieldsEnumVariant, SchemaRef, ToSchemaDeriveInput,
            TupleEnumVariant, UnitEnumVariant,
        },
    },
    utils::{Either, ident_name},
};

pub fn impl_to_schema(input: &ToSchemaDeriveInput) -> TokenStream {
    let settings = input.impl_settings();
    let generics = settings
        .generics()
        .add_bound_for_all_type_params(quote!(_autapi::schema::ToSchema));
    let SchemaExpr {
        schema,
        required,
        original,
    } = schema_ref_from_input(input);

    let ident = &settings.type_ident;
    let always_inlined = settings.always_inlined.is_present();
    let name = settings
        .rename
        .as_ref()
        .map(Either::Left)
        .unwrap_or_else(|| {
            let mut format_str = ident_name(ident);
            for _ in generics.type_param_names() {
                format_str += "_{}";
            }
            let type_params = generics.type_param_names();
            Either::Right(quote! {
                format!(
                    #format_str,
                    #(<#type_params as _autapi::schema::ToSchema>::name(),)*
                )
            })
        });
    let (gen_impl, gen_ty, gen_where) = generics.split_for_impl();
    let original = original.unwrap_or_else(|| {
        let static_lt = quote!('static);
        let generics = generics.generics.params.iter().map(|param| match param {
            GenericParam::Lifetime(_) => Either::Left(Cow::Borrowed(&static_lt)),
            GenericParam::Type(ty) => Either::Left(Cow::Owned(
                quote!(<#ty as _autapi::schema::ToSchema>::Original),
            )),
            _ => Either::Right(param),
        });
        quote!(#ident<#(#generics),*>)
    });
    quote! {
        impl<#gen_impl> _autapi::schema::ToSchema for #ident<#gen_ty> #gen_where {
            type Original = #original;

            const REQUIRED: bool = #required;
            const ALWAYS_INLINED: bool = #always_inlined;

            fn name() -> std::borrow::Cow<'static, str> {
                #name.into()
            }
            fn schema(registry: &mut _autapi::Registry) -> _autapi::openapi::MaybeRef<_autapi::openapi::Schema> {
                #schema
            }
        }
    }
}

fn schema_ref_from_input(input: &ToSchemaDeriveInput) -> SchemaExpr {
    match input {
        ToSchemaDeriveInput::Enum(e) => schema_ref_from_enum(e),
        ToSchemaDeriveInput::UnitStruct(s) => null_schema_ref(&s.meta),
        ToSchemaDeriveInput::NamedFieldsStruct(s) => schema_ref_from_named_fields(&s.named_fields),
        ToSchemaDeriveInput::TupleStruct(s) => schema_ref_from_schema_ref(&s.schema),
    }
}

pub struct SchemaExpr {
    schema: TokenStream,
    required: TokenStream,
    original: Option<TokenStream>,
}

fn unit_enum_variant_schema_ref(
    UnitEnumVariant { common, null, meta }: &UnitEnumVariant,
) -> TokenStream {
    if null.is_present() {
        null_schema_ref(meta).schema
    } else {
        let name = &common.name;
        with_metadata(
            &quote!(_autapi::openapi::MaybeRef::T(
                _autapi::openapi::Schema::new_string_constant(#name)
            )),
            meta,
        )
    }
}

fn null_schema_ref(meta: &SchemaMetaAttrs) -> SchemaExpr {
    SchemaExpr {
        required: quote!(true),
        schema: with_metadata(
            &quote!(_autapi::openapi::MaybeRef::T(
                _autapi::openapi::Schema::null()
            )),
            meta,
        ),
        original: Some(quote!(<() as _autapi::schema::ToSchema>::Original)),
    }
}

fn schema_ref_from_enum(e: &Enum) -> SchemaExpr {
    let Enum {
        impl_settings: _,
        meta,
        tag_property: _,
        variants,
    } = e;

    let schema = if variants.iter().all(|variant| match variant {
        EnumVariant::Unit(UnitEnumVariant {
            meta,
            null,
            common: _,
        }) => !meta.any_set() && !null.is_present(),
        _ => false,
    }) {
        // special case: only untagged unit variants without any meta
        // => we can merge them into one string enum
        let enum_values = e.unit_variants().map(|variant| &variant.common.name);
        let schema = quote!(_autapi::openapi::MaybeRef::T(
            _autapi::openapi::Schema::default()
                .with_schema_type(_autapi::openapi::Type::String)
                .with_enum_values(vec![#(#enum_values.into(),)*])
        ));
        with_metadata(&schema, meta)
    } else {
        // default case: simply use a anyOf
        let variant_schema_refs = variants.iter().map(schema_ref_from_enum_variant);
        quote!(_autapi::openapi::MaybeRef::T(
            _autapi::openapi::Schema::default().with_any_of(vec![#(#variant_schema_refs,)*])
        ))
    };
    let schema = with_metadata(&schema, meta);

    let transparent_inner_required = e
        .transparent_variants()
        .map(|variant| variant.schema.required_bool());
    let required = quote!(true #(&& #transparent_inner_required)*);
    SchemaExpr {
        schema,
        required,
        original: None,
    }
}

fn schema_ref_from_enum_variant(variant: &EnumVariant) -> TokenStream {
    let name = &variant.common().name;
    let schema_ref = match variant {
        EnumVariant::NamedFields(NamedFieldsEnumVariant { fields, .. }) => {
            schema_ref_from_named_fields(fields).schema
        }
        EnumVariant::Tuple(TupleEnumVariant { schema, .. }) => {
            schema_ref_from_schema_ref(schema).schema
        }
        EnumVariant::Unit(variant) => unit_enum_variant_schema_ref(variant),
    };
    match &variant.common().tag_property {
        Some(tag_property) => {
            // these two statements need to be separated, because otherwise registry might be
            // borrowed mutably twice
            quote!({
                let schema = #schema_ref;
                _autapi::private::schema::add_tag_to_schema(
                    registry, schema, #tag_property, #name
                )
            })
        }
        None => schema_ref,
    }
}

/// Returns a `MaybeRef<Schema>` for the given `NamedFields`
pub fn schema_ref_from_named_fields(fields: &NamedFields) -> SchemaExpr {
    let field_names = fields.non_skipped().map(|field| &field.schema_name);
    let field_names2 = field_names.clone();
    let field_schemas = fields
        .non_skipped()
        .map(|field| schema_ref_from_schema_ref(&field.schema).schema);
    let field_required = fields
        .non_skipped()
        .map(|field| field.schema.required_bool());
    let field_required2 = field_required.clone();
    let schema = quote!({
        let mut schema = _autapi::openapi::Schema::default()
            .with_schema_type(_autapi::openapi::Type::Object)
            .with_properties(
                _autapi::openapi::Map::from_iter(
                    [
                        #((String::from(#field_names), #field_schemas),)*
                    ]
                )
            );
        let capacity = 0 #(+ #field_required as usize)*;
        schema.required.reserve(capacity);
        #(
            if #field_required2 {
                schema.required.push(#field_names2.to_owned())
            }
        )*
        _autapi::openapi::MaybeRef::T(schema)
    });
    let schema = with_metadata(&schema, &fields.meta);
    SchemaExpr {
        schema,
        required: quote!(true),
        original: None,
    }
}

/// Returns a `MaybeRef<Schema>` for the given `SchemaRef`
fn schema_ref_from_schema_ref(
    schema @ SchemaRef {
        ty: _,
        inline,
        using: _,
        meta,
    }: &SchemaRef,
) -> SchemaExpr {
    let ty = schema.schema_type();
    let required = schema.required_bool();
    let original = Some(quote!(<#ty as _autapi::schema::ToSchema>::Original));
    let mut schema_expr = if inline.is_present() {
        SchemaExpr {
            schema: quote!(<#ty as _autapi::schema::ToSchema>::schema(registry)),
            required,
            original,
        }
    } else {
        SchemaExpr {
            schema: quote!(<#ty as _autapi::schema::ToSchema>::schema_ref(registry)),
            required,
            original,
        }
    };
    let schema = &mut schema_expr.schema;
    *schema = with_metadata(schema, meta);
    schema_expr
}

/// Returns a `Schema` value that contains only the given metadata.
fn with_metadata(
    schema_ref: &TokenStream,
    SchemaMetaAttrs {
        description,
        title,
        deprecated,
        read_only,
        write_only,
        example,
        examples,
    }: &SchemaMetaAttrs,
) -> TokenStream {
    let example_expr = |expr| {
        quote!(
            _autapi::private::serde_json::to_value(
                &_autapi::adapters::SerdeAdapter(#expr)
            ).expect("serializing example should succeed")
        )
    };
    let (description, title, deprecated, read_only, write_only, example) = (
        description.iter(),
        title.iter(),
        deprecated.is_present().then_some(true).into_iter(),
        read_only.is_present().then_some(true).into_iter(),
        write_only.is_present().then_some(true).into_iter(),
        example.iter().map(|expr| example_expr(&expr.0)),
    );
    let description2 = description.clone();
    let example_exprs = examples
        .iter()
        .map(|examples| Punctuated::<_, Comma>::from_iter(examples.0.iter().map(example_expr)));
    quote!(
        match #schema_ref {
            _autapi::openapi::MaybeRef::T(schema) => _autapi::openapi::MaybeRef::T(
                schema
                    #(.with_description(#description))*
                    #(.with_title(#title))*
                    #(.with_deprecated(#deprecated))*
                    #(.with_read_only(#read_only))*
                    #(.with_write_only(#write_only))*
                    #(.with_example(#example))*
                    #(.with_examples(vec![#example_exprs]))*
            ),
            _autapi::openapi::MaybeRef::Ref(r) => _autapi::openapi::MaybeRef::Ref(
                r
                    #(.with_description(#description2))*
            ),
        }
    )
}
