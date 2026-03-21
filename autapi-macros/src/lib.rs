use proc_macro::TokenStream;
use syn::{DeriveInput, ItemFn, parse_macro_input};

mod doc_attr;
mod endpoint;
mod responses;
mod to_schema;
mod utils;

#[proc_macro_attribute]
pub fn endpoint(attr: TokenStream, item: TokenStream) -> TokenStream {
    TokenStream::from(endpoint::endpoint(
        proc_macro2::TokenStream::from(attr),
        parse_macro_input!(item as ItemFn),
    ))
}

#[proc_macro_derive(DescribeResponse)]
pub fn derive_response(attr: TokenStream) -> TokenStream {
    let input = parse_macro_input!(attr as DeriveInput);
    TokenStream::from(responses::derive_responses(input))
}

/// Implement `ToSchema`, `SchemaSerialize` and `SchemaDeserialize` for a type.
#[proc_macro_derive(ToSchema, attributes(schema))]
pub fn derive_to_schema(attr: TokenStream) -> TokenStream {
    let input = parse_macro_input!(attr as DeriveInput);
    let result = to_schema::derive_to_schema(input).unwrap_or_else(|t| t);
    TokenStream::from(result)
}
