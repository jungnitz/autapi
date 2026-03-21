use darling::{FromDeriveInput, FromField, FromMeta, FromVariant, util::Flag};
use syn::{Generics, Ident, LitStr, Type};

use crate::{
    doc_attr::DocAttr,
    utils::{Case, ExprsMeta, PreserveStringExpr},
};

#[derive(Debug, Clone, FromDeriveInput)]
#[darling(attributes(schema), forward_attrs(doc))]
pub struct DeriveToSchemaAttrs<I> {
    pub ident: Ident,
    pub generics: Generics,
    pub bounds: Option<LitStr>,

    pub rename: Option<LitStr>,
    pub always_inlined: Flag,

    pub no_serialize: Flag,
    pub no_deserialize: Flag,

    pub serde: Flag,

    #[darling(with = DocAttr::try_from)]
    pub attrs: DocAttr,
    #[darling(flatten)]
    pub inner: I,
}

pub type NamedFieldsStructAttrs = DeriveToSchemaAttrs<NamedFieldsAttrs>;
pub type TupleStructAttrs = DeriveToSchemaAttrs<FieldAttrs>;
pub type UnitStructAttrs = DeriveToSchemaAttrs<SchemaMetaAttrs>;
pub type EnumAttrs = DeriveToSchemaAttrs<EnumAttrsInner>;

#[derive(Debug, Clone, FromMeta)]
pub struct EnumAttrsInner {
    pub tag: Option<LitStr>,
    pub untagged: Flag,
    pub rename_all: Option<Case>,
    pub rename_all_fields: Option<Case>,
    #[darling(flatten)]
    pub meta: SchemaMetaAttrs,
}

#[derive(Debug, Clone, FromVariant)]
#[darling(attributes(schema), forward_attrs(doc))]
pub struct EnumVariantAttrs<I> {
    pub ident: Ident,
    pub rename: Option<LitStr>,
    pub untagged: Flag,
    #[darling(with = DocAttr::try_from)]
    pub attrs: DocAttr,
    #[darling(flatten)]
    pub inner: I,
}

pub type NamedFieldsEnumVariantAttrs = EnumVariantAttrs<NamedFieldsAttrs>;
pub type TupleEnumVariantAttrs = EnumVariantAttrs<FieldAttrs>;
pub type UnitEnumVariantAttrs = EnumVariantAttrs<UnitEnumVariantAttrsInner>;

#[derive(Debug, Clone, FromMeta)]
pub struct UnitEnumVariantAttrsInner {
    pub null: Flag,
    #[darling(flatten)]
    pub meta: SchemaMetaAttrs,
}

#[derive(Debug, Clone, FromMeta)]
pub struct NamedFieldsAttrs {
    pub rename_all: Option<Case>,
    #[darling(flatten)]
    pub meta: SchemaMetaAttrs,
}

#[derive(Debug, Clone, FromField)]
#[darling(attributes(schema), forward_attrs(doc))]
pub struct NamedFieldAttrs {
    pub ty: Type,
    pub skip: Flag,
    pub rename: Option<LitStr>,
    #[darling(with = DocAttr::try_from)]
    pub attrs: DocAttr,
    #[darling(flatten)]
    pub inner: FieldAttrs,
}

#[derive(Debug, Clone, FromMeta)]
pub struct FieldAttrs {
    pub inline: Flag,
    pub using: Option<String>,
    pub optional: Flag,
    pub nullable: Flag,
    #[darling(flatten)]
    pub meta: SchemaMetaAttrs,
}

#[derive(Debug, Clone, FromMeta)]
pub struct SchemaMetaAttrs {
    pub description: Option<LitStr>,
    pub title: Option<LitStr>,
    pub deprecated: Flag,
    pub read_only: Flag,
    pub write_only: Flag,
    pub example: Option<PreserveStringExpr>,
    pub examples: Option<ExprsMeta>,
}

impl SchemaMetaAttrs {
    pub fn any_set(&self) -> bool {
        let Self {
            description,
            title,
            deprecated,
            read_only,
            write_only,
            example,
            examples,
        } = self;
        description.is_some()
            || title.is_some()
            || deprecated.is_present()
            || read_only.is_present()
            || write_only.is_present()
            || example.is_some()
            || examples.is_some()
    }
    pub fn or_with_doc_description(self, docs: &DocAttr) -> Self {
        Self {
            description: self.description.or_else(|| docs.to_lines()),
            ..self
        }
    }
}
