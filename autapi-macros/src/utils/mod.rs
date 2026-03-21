mod case;
mod generics;
mod meta;
mod punctuated;

use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{Ident, LitStr, spanned::Spanned};

pub use self::{case::*, generics::*, meta::*, punctuated::*};

/// Returns the single field in `fields` or a compile error `TokenStream` if there is not exactly
/// one field in `fields`
pub fn get_single_unnamed_field(fields: &syn::FieldsUnnamed) -> Result<&syn::Field, TokenStream> {
    let mut iter = fields.unnamed.iter();
    if let Some(field) = iter.next()
        && iter.next().is_none()
    {
        Ok(field)
    } else {
        Err(
            syn::Error::new(fields.span(), "only one field can be specified here")
                .into_compile_error(),
        )
    }
}

/// Returns the name of the given ident, stripping any `r#` prefixes
pub fn ident_name(ident: &Ident) -> String {
    let name = ident.to_string();
    match name.strip_prefix("r#") {
        None => name,
        Some(name) => name.to_owned(),
    }
}

/// Returns a string literal that contains the ident's name without any `r#` prefixes.
pub fn ident_to_lit(ident: &Ident, rename: Option<Case>) -> LitStr {
    let mut ident_name = ident_name(ident);
    if let Some(rename) = rename {
        ident_name = rename.format(&ident_name);
    }
    LitStr::new(&ident_name, ident.span())
}

pub enum Either<L, R> {
    Left(L),
    Right(R),
}

impl<L, R> ToTokens for Either<L, R>
where
    L: ToTokens,
    R: ToTokens,
{
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Either::Left(left) => left.to_tokens(tokens),
            Either::Right(right) => right.to_tokens(tokens),
        }
    }
}
