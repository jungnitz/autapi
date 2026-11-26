use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

use crate::to_schema::{
    deserialize::impl_deserialize, model::ToSchemaDeriveInput, schema::impl_to_schema,
    serialize::impl_serialize, validate::impl_validate,
};

mod attrs;
mod deserialize;
mod model;
mod schema;
mod serialize;
mod validate;

pub fn derive_to_schema(input: DeriveInput) -> Result<TokenStream, TokenStream> {
    let input = ToSchemaDeriveInput::from_syn(&input)?;
    let to_schema = impl_to_schema(&input);
    let serialize = impl_serialize(&input);
    let deserialize = impl_deserialize(&input);
    let validate = impl_validate(&input);

    Ok(quote! {
        #[allow(non_camel_case_types)]
        const _: () = {
            use autapi as _autapi;
            #to_schema
            #serialize
            #deserialize
            #validate
        };
    })
}
