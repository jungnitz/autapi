use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    GenericParam, Generics, Ident, Lifetime,
    token::{Colon, Comma},
};

use crate::utils::{Either, PunctuatedIter};

pub struct GenericsHelper<'g> {
    pub generics: &'g Generics,
    where_clause: TokenStream,
    immutable_where: bool,
}

impl<'g> GenericsHelper<'g> {
    pub fn from_generics_with_bounds(generics: &'g Generics, bounds: impl ToTokens) -> Self {
        let mut result = Self::from_generics(generics);
        bounds.to_tokens(&mut result.where_clause);
        result.immutable_where = true;
        result
    }
    pub fn from_generics(generics: &'g Generics) -> Self {
        Self {
            generics,
            where_clause: generics
                .where_clause
                .as_ref()
                .map(ToTokens::to_token_stream)
                .unwrap_or_else(|| quote!(where)),
            immutable_where: false,
        }
    }
    pub fn type_param_names(&self) -> impl Iterator<Item = &Ident> {
        self.generics.type_params().map(|param| &param.ident)
    }
    pub fn lifetime_names(&self) -> impl Iterator<Item = &Lifetime> {
        self.generics.lifetimes().map(|param| &param.lifetime)
    }
    pub fn add_bound_for_all_type_params<T: ToTokens>(mut self, bound: T) -> Self {
        if self.immutable_where {
            return self;
        }
        self.generics.type_params().for_each(|param| {
            param.ident.to_tokens(&mut self.where_clause);
            Colon::default().to_tokens(&mut self.where_clause);
            bound.to_tokens(&mut self.where_clause);
            Comma::default().to_tokens(&mut self.where_clause);
        });
        self
    }
    pub fn split_for_impl(&self) -> (impl ToTokens, impl ToTokens, impl ToTokens) {
        (
            PunctuatedIter::comma(self.generics.params.iter()),
            PunctuatedIter::comma(self.generics.params.iter().map(|param| match param {
                GenericParam::Const(param) => Either::Left(&param.ident),
                GenericParam::Lifetime(param) => Either::Right(&param.lifetime),
                GenericParam::Type(param) => Either::Left(&param.ident),
            })),
            &self.where_clause,
        )
    }
    pub fn make_phantom_data(&self) -> TokenStream {
        let ty = self.type_param_names();
        let lt = self.lifetime_names();
        quote!(
            ::std::marker::PhantomData<(
                #(&#lt (),)*
                #(#ty,)*
            )>
        )
    }
}

impl<'g> ToTokens for GenericsHelper<'g> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.where_clause.to_tokens(tokens);
    }
    fn into_token_stream(self) -> TokenStream
    where
        Self: Sized,
    {
        self.where_clause
    }
}
