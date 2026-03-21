use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{Data, DeriveInput, Fields, Ident, Type, spanned::Spanned};

pub struct ResponsesImpl {
    pub name: Ident,
    pub type_idents: Vec<Ident>,
    pub types: Vec<Type>,
}

pub fn derive_responses(input: DeriveInput) -> TokenStream {
    let Data::Enum(data) = input.data else {
        return syn::Error::new(input.span(), "has to be an enum").to_compile_error();
    };
    let mut type_idents = Vec::new();
    let mut types = Vec::new();
    for variant in data.variants {
        let Fields::Unnamed(fields) = variant.fields else {
            return syn::Error::new(variant.span(), "has to contain exactly one unnamed field")
                .to_compile_error();
        };
        if fields.unnamed.len() != 1 {
            return syn::Error::new(fields.span(), "has to contain exactly one unnamed field")
                .to_compile_error();
        }
        let ty = fields.unnamed.into_iter().next().unwrap().ty;
        type_idents.push(variant.ident);
        types.push(ty);
    }
    ResponsesImpl {
        name: input.ident,
        type_idents,
        types,
    }
    .into_token_stream()
}

impl ToTokens for ResponsesImpl {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let base = quote!(autapi);
        let ResponsesImpl {
            name,
            type_idents,
            types,
        } = self;

        tokens.extend(quote! {
            impl #base::DescribeResponse for #name {
                fn describe(
                    description: &mut #base::ResponsesDescription,
                    args: &#base::DescribeResponseArgs
                ) {
                    #(<#types as #base::DescribeResponse>::describe(description, args);)*
                }
                fn into_response(self) -> #base::axum::response::Response {
                    match self {
                        #(Self::#type_idents(response) =>
                            #base::DescribeResponse::into_response(response)),*
                    }
                }
            }

            #(
            impl From<#types> for #name {
                fn from(value: #types) -> Self {
                    Self::#type_idents(value)
                }
            }
            )*
        });
    }
}
