use std::{borrow::Cow, ops::Deref};

use crate::{
    doc_attr::DocAttr,
    to_schema::attrs::{
        DeriveToSchemaAttrs, EnumAttrs, EnumAttrsInner, EnumVariantAttrs, FieldAttrs,
        NamedFieldAttrs, NamedFieldsAttrs, NamedFieldsEnumVariantAttrs, NamedFieldsStructAttrs,
        SchemaMetaAttrs, TupleEnumVariantAttrs, TupleStructAttrs, UnitEnumVariantAttrs,
        UnitEnumVariantAttrsInner, UnitStructAttrs,
    },
    utils::{Case, Either, GenericsHelper, get_single_unnamed_field, ident_to_lit},
};
use darling::{FromDeriveInput, FromField, FromVariant, util::Flag};
use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use syn::{
    Data, DeriveInput, Fields, Generics, Ident, Lifetime, LitStr, Token, Type, WherePredicate,
    parse::Parser, parse_quote, parse_str, punctuated::Punctuated, spanned::Spanned,
};

#[expect(clippy::large_enum_variant)]
pub enum ToSchemaDeriveInput {
    Enum(Enum),
    UnitStruct(UnitStruct),
    TupleStruct(TupleStruct),
    NamedFieldsStruct(NamedFieldsStruct),
}

impl ToSchemaDeriveInput {
    pub fn from_syn(input: &DeriveInput) -> Result<Self, TokenStream> {
        Ok(match &input.data {
            Data::Enum(data) => {
                let attrs =
                    EnumAttrs::from_derive_input(input).map_err(darling::Error::write_errors)?;
                Self::Enum(Enum::from_syn(data, attrs)?)
            }
            Data::Struct(data) => match &data.fields {
                Fields::Named(fields) => {
                    let attrs = NamedFieldsStructAttrs::from_derive_input(input)
                        .map_err(darling::Error::write_errors)?;
                    Self::NamedFieldsStruct(NamedFieldsStruct::from_syn(attrs, fields)?)
                }
                Fields::Unnamed(fields) => {
                    let attrs = TupleStructAttrs::from_derive_input(input)
                        .map_err(darling::Error::write_errors)?;
                    Self::TupleStruct(TupleStruct::from_syn(attrs, fields)?)
                }
                Fields::Unit => {
                    let attrs = UnitStructAttrs::from_derive_input(input)
                        .map_err(darling::Error::write_errors)?;
                    Self::UnitStruct(UnitStruct::from_syn(attrs)?)
                }
            },
            _ => {
                return Err(
                    syn::Error::new(input.span(), "must be a struct or enum").into_compile_error()
                );
            }
        })
    }
    pub fn impl_settings(&self) -> &ImplSettings {
        match self {
            Self::Enum(data) => &data.impl_settings,
            Self::UnitStruct(data) => &data.impl_settings,
            Self::TupleStruct(data) => &data.impl_settings,
            Self::NamedFieldsStruct(data) => &data.impl_settings,
        }
    }
}

pub struct Enum {
    pub impl_settings: ImplSettings,
    pub meta: SchemaMetaAttrs,
    pub tag_property: Option<LitStr>,
    pub variants: Vec<EnumVariant>,
}

impl Enum {
    pub fn from_syn(data: &syn::DataEnum, attrs: EnumAttrs) -> Result<Self, TokenStream> {
        let (
            impl_settings,
            docs,
            EnumAttrsInner {
                tag,
                untagged,
                rename_all,
                rename_all_fields,
                mut meta,
            },
        ) = split_attrs(attrs)?;
        if untagged.is_present() == tag.is_some() {
            return Err(syn::Error::new(
                untagged.span(),
                "an enum must either be assigned a tag property or be marked as untagged",
            )
            .into_compile_error());
        }
        meta = meta.or_with_doc_description(&docs);
        let ctx = Context {
            rename_all_fields,
            rename_all_variants: rename_all,
            tag_property: tag.clone(),
        };
        Ok(Self {
            impl_settings,
            meta,
            tag_property: tag,
            variants: data
                .variants
                .iter()
                .map(|variant| EnumVariant::from_syn(ctx.clone(), variant))
                .collect::<Result<_, _>>()?,
        })
    }
    pub fn transparent_variants(&self) -> impl Iterator<Item = &TupleEnumVariant> + Clone {
        self.tuple_variants()
            .filter(|variant| variant.common.tag_property.is_none())
    }
    pub fn tuple_variants(&self) -> impl Iterator<Item = &TupleEnumVariant> + Clone {
        self.variants.iter().filter_map(|variant| match variant {
            EnumVariant::Tuple(variant) => Some(variant),
            _ => None,
        })
    }
    pub fn named_fields_variants(&self) -> impl Iterator<Item = &NamedFieldsEnumVariant> + Clone {
        self.variants.iter().filter_map(|variant| match variant {
            EnumVariant::NamedFields(variant) => Some(variant),
            _ => None,
        })
    }
    pub fn unit_variants(&self) -> impl Iterator<Item = &UnitEnumVariant> + Clone {
        self.variants.iter().filter_map(|variant| match variant {
            EnumVariant::Unit(variant) => Some(variant),
            _ => None,
        })
    }
}

/// Information about an enum variant that applies to all types of variants (tuple, named fields,
/// and unit)
pub struct EnumVariantCommon {
    pub docs: DocAttr,
    pub ident: Ident,
    pub name: LitStr,
    pub tag_property: Option<LitStr>,
}

fn split_enum_variant_attrs<I>(
    ctx: &mut Context,
    attrs: EnumVariantAttrs<I>,
) -> Result<(EnumVariantCommon, I), TokenStream> {
    let EnumVariantAttrs {
        ident,
        rename,
        untagged,
        attrs,
        inner,
    } = attrs;
    let name = rename.unwrap_or_else(|| ident_to_lit(&ident, ctx.rename_all_variants));
    let tag_property = if untagged.is_present() {
        None
    } else {
        ctx.tag_property.clone()
    };
    Ok((
        EnumVariantCommon {
            docs: attrs,
            ident,
            name,
            tag_property,
        },
        inner,
    ))
}

pub struct TupleEnumVariant {
    pub common: EnumVariantCommon,
    pub schema: SchemaRef,
}

pub struct NamedFieldsEnumVariant {
    pub common: EnumVariantCommon,
    pub fields: NamedFields,
}

pub struct UnitEnumVariant {
    pub common: EnumVariantCommon,
    pub null: Flag,
    pub meta: SchemaMetaAttrs,
}

#[expect(clippy::large_enum_variant)]
pub enum EnumVariant {
    Tuple(TupleEnumVariant),
    NamedFields(NamedFieldsEnumVariant),
    Unit(UnitEnumVariant),
}

impl EnumVariant {
    fn from_syn(mut ctx: Context, variant: &syn::Variant) -> Result<Self, TokenStream> {
        match &variant.fields {
            Fields::Named(named) => {
                let attrs = NamedFieldsEnumVariantAttrs::from_variant(variant)
                    .map_err(darling::Error::write_errors)?;
                let (common, named_fields) = split_enum_variant_attrs(&mut ctx, attrs)?;
                let fields = NamedFields::from_syn(ctx, named_fields, &common.docs, named)?;
                Ok(EnumVariant::NamedFields(NamedFieldsEnumVariant {
                    common,
                    fields,
                }))
            }
            Fields::Unnamed(unnamed) => {
                let attrs = TupleEnumVariantAttrs::from_variant(variant)
                    .map_err(darling::Error::write_errors)?;
                let (common, field_attrs) = split_enum_variant_attrs(&mut ctx, attrs)?;
                let schema = SchemaRef::from_syn(
                    get_single_unnamed_field(unnamed)?.ty.clone(),
                    &common.docs,
                    field_attrs,
                )?;
                Ok(EnumVariant::Tuple(TupleEnumVariant { common, schema }))
            }
            Fields::Unit => {
                let attrs = UnitEnumVariantAttrs::from_variant(variant)
                    .map_err(darling::Error::write_errors)?;
                let (common, UnitEnumVariantAttrsInner { meta, null }) =
                    split_enum_variant_attrs(&mut ctx, attrs)?;
                if common.tag_property.is_some() {
                    return Err(syn::Error::new(
                        common.ident.span(),
                        "only untagged unit variants are supported",
                    )
                    .into_compile_error());
                }
                let meta = meta.or_with_doc_description(&common.docs);
                Ok(EnumVariant::Unit(UnitEnumVariant { common, null, meta }))
            }
        }
    }
    pub fn common(&self) -> &EnumVariantCommon {
        match self {
            Self::NamedFields(variant) => &variant.common,
            Self::Tuple(variant) => &variant.common,
            Self::Unit(variant) => &variant.common,
        }
    }
}

impl Deref for EnumVariant {
    type Target = EnumVariantCommon;

    fn deref(&self) -> &Self::Target {
        self.common()
    }
}

pub struct UnitStruct {
    pub impl_settings: ImplSettings,
    pub meta: SchemaMetaAttrs,
}

impl UnitStruct {
    pub fn from_syn(attrs: UnitStructAttrs) -> Result<Self, TokenStream> {
        let (impl_settings, docs, meta) = split_attrs(attrs)?;
        Ok(Self {
            impl_settings,
            meta: meta.or_with_doc_description(&docs),
        })
    }
}

pub struct TupleStruct {
    pub impl_settings: ImplSettings,
    pub schema: SchemaRef,
}

impl TupleStruct {
    pub fn from_syn(
        attrs: TupleStructAttrs,
        fields: &syn::FieldsUnnamed,
    ) -> Result<Self, TokenStream> {
        let (impl_settings, docs, field_attrs) = split_attrs(attrs)?;
        let field = get_single_unnamed_field(fields)?;
        Ok(Self {
            impl_settings,
            schema: SchemaRef::from_syn(field.ty.clone(), &docs, field_attrs)?,
        })
    }
}

pub struct NamedFieldsStruct {
    pub impl_settings: ImplSettings,
    pub named_fields: NamedFields,
}

impl NamedFieldsStruct {
    pub fn from_syn(
        attrs: NamedFieldsStructAttrs,
        fields: &syn::FieldsNamed,
    ) -> Result<Self, TokenStream> {
        let (impl_settings, docs, named_fields) = split_attrs(attrs)?;
        Ok(Self {
            impl_settings,
            named_fields: NamedFields::from_syn(Context::default(), named_fields, &docs, fields)?,
        })
    }
}

pub struct NamedFields {
    pub meta: SchemaMetaAttrs,
    pub fields: Vec<NamedField>,
}

impl NamedFields {
    fn from_syn(
        ctx: Context,
        attrs: NamedFieldsAttrs,
        docs: &DocAttr,
        syn_fields: &syn::FieldsNamed,
    ) -> Result<Self, TokenStream> {
        let NamedFieldsAttrs { rename_all, meta } = attrs;
        let ctx = ctx.or_rename_all_fields(rename_all);
        let meta = meta.or_with_doc_description(docs);

        let fields = syn_fields
            .named
            .iter()
            .map(|field| NamedField::from_syn(ctx.clone(), field))
            .collect::<Result<_, _>>()?;

        Ok(Self { meta, fields })
    }
    pub fn non_skipped(&self) -> impl Iterator<Item = &NamedField> + Clone {
        self.fields.iter().filter(|field| !field.skip.is_present())
    }
}

pub struct NamedField {
    pub ident: Ident,
    pub schema: SchemaRef,

    pub skip: Flag,
    pub schema_name: LitStr,
}

impl NamedField {
    fn from_syn(ctx: Context, field: &syn::Field) -> Result<Self, TokenStream> {
        let ident = field
            .ident
            .as_ref()
            .expect("ident of named field should be present");
        let NamedFieldAttrs {
            ty,
            skip,
            rename,
            attrs,
            inner,
        } = NamedFieldAttrs::from_field(field).map_err(darling::Error::write_errors)?;
        let schema_name = rename.unwrap_or_else(|| ident_to_lit(ident, ctx.rename_all_fields));
        Ok(Self {
            ident: ident.clone(),
            schema_name,

            skip,
            schema: SchemaRef::from_syn(ty, &attrs, inner)?,
        })
    }
}

/// A reference to another `ToSchema` type.
pub struct SchemaRef {
    /// The type of the referenced schema
    pub ty: Type,
    /// Determines whether `ToSchema::schema` or `ToSchema::schema_ref` is used
    pub inline: Flag,
    /// The `SchemaUsing` type to use for serializing / deserializing this value.
    pub using: Option<Type>,
    /// Meta-information for the schema
    pub meta: SchemaMetaAttrs,
}

impl SchemaRef {
    fn from_syn(
        ty: Type,
        doc: &DocAttr,
        FieldAttrs {
            inline,
            using,
            nullable,
            optional,
            mut meta,
        }: FieldAttrs,
    ) -> Result<Self, TokenStream> {
        meta.description = meta.description.or_else(|| doc.to_lines());
        if nullable.is_present() as u8 + optional.is_present() as u8 + using.is_some() as u8 > 1 {
            return Err(syn::Error::new(
                Span::call_site(),
                "attributes `nullable`, `optional` and `using` are mutually exclusive",
            )
            .into_compile_error());
        }
        let using = if nullable.is_present() {
            Some(parse_quote!(_autapi::schema::Nullable<#ty>))
        } else if optional.is_present() {
            Some(parse_quote!(Option<#ty>))
        } else if let Some(using) = using {
            Some(parse_str(&using).map_err(syn::Error::into_compile_error)?)
        } else {
            None
        };
        Ok(Self {
            ty,
            inline,
            using,
            meta,
        })
    }

    /// Returns a `true` value that is true if a property with values described by this object can
    /// be omitted.
    pub fn required_bool(&self) -> TokenStream {
        let ty = self.schema_type();
        quote!(<#ty as _autapi::schema::ToSchema>::REQUIRED)
    }

    pub fn deserialize_type(&self) -> impl ToTokens {
        self.using.as_ref().unwrap_or(&self.ty)
    }
    pub fn schema_type(&self) -> impl ToTokens {
        self.deserialize_type()
    }
    pub fn serialize_type(&self, lt: Lifetime) -> impl ToTokens {
        let ty = &self.ty;
        match &self.using {
            Some(using) => quote!(
                <#ty as _autapi::schema::SchemaUsing<#using>>::Ser<#lt>
            ),
            None => quote!(&#lt #ty),
        }
    }

    /// Creates a value of type `self.ty` given a value `de` of the deserialize type.
    pub fn deserialize_value(&self, de: impl ToTokens) -> impl ToTokens {
        let ty = &self.ty;
        match &self.using {
            Some(using) => Either::Left(quote!(
                <#ty as _autapi::schema::SchemaUsing<#using>>::from_de(#de)
            )),
            None => Either::Right(de),
        }
    }

    /// Creates a value of the serialize type given a reference to a value of `self.ty`.
    pub fn serialize_value(&self, ser: impl ToTokens) -> impl ToTokens {
        let ty = &self.ty;
        match &self.using {
            Some(using) => Either::Left(quote!(
                <#ty as _autapi::schema::SchemaUsing<#using>>::to_ser(#ser)
            )),
            None => Either::Right(ser),
        }
    }
}

/// General settings for implementations, always applicable.
pub struct ImplSettings {
    pub type_ident: Ident,
    type_generics: Generics,
    pub bounds: Option<TokenStream>,

    pub rename: Option<LitStr>,
    pub always_inlined: Flag,

    pub no_serialize: Flag,
    pub no_deserialize: Flag,
    pub serde: Flag,
}

impl ImplSettings {
    pub fn generics(&self) -> GenericsHelper<'_> {
        match &self.bounds {
            Some(bounds) => GenericsHelper::from_generics_with_bounds(&self.type_generics, bounds),
            None => GenericsHelper::from_generics(&self.type_generics),
        }
    }
    pub fn name(&self) -> Cow<'_, LitStr> {
        self.rename
            .as_ref()
            .map(Cow::Borrowed)
            .unwrap_or_else(|| Cow::Owned(ident_to_lit(&self.type_ident, None)))
    }
}

fn split_attrs<I>(
    DeriveToSchemaAttrs {
        ident,
        generics,
        bounds,
        rename,
        always_inlined,
        no_serialize,
        no_deserialize,
        attrs,
        inner,
        serde,
    }: DeriveToSchemaAttrs<I>,
) -> Result<(ImplSettings, DocAttr, I), TokenStream> {
    let bounds = match bounds {
        None => None,
        Some(bounds) => {
            let parser = Punctuated::<WherePredicate, Token![,]>::parse_terminated;
            let bounds = parser
                .parse_str(&bounds.value())
                .map_err(|err| syn::Error::new(bounds.span(), err).into_compile_error())?
                .into_token_stream();
            Some(bounds)
        }
    };
    Ok((
        ImplSettings {
            type_ident: ident,
            type_generics: generics,
            bounds,
            rename,
            always_inlined,
            no_serialize,
            no_deserialize,
            serde,
        },
        attrs,
        inner,
    ))
}

/// Used to forward information from parent elements to child elements.
///
/// For example, the `rename_all` attribute for structs is forwarded from the struct level to field
/// level as the `rename_all_fields` field of the context.
#[derive(Default, Clone)]
struct Context {
    /// Case for all named fields
    rename_all_fields: Option<Case>,
    /// Case for all enum variants
    rename_all_variants: Option<Case>,
    /// Tag property name of the current enum
    tag_property: Option<LitStr>,
}

impl Context {
    /// Returns a new `Context` with `rename_all_fields` set to the given value if it is present.
    fn or_rename_all_fields(self, case: Option<Case>) -> Self {
        Context {
            rename_all_fields: case.or(self.rename_all_fields),
            ..self
        }
    }
}
