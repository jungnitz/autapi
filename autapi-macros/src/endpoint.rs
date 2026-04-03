use darling::{FromMeta, ast::NestedMeta};
use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident, quote};
use syn::{Error, FnArg, Ident, ItemFn, LitStr, ReturnType, spanned::Spanned};

use crate::doc_attr::DocAttr;

#[derive(Debug, FromMeta)]
pub struct Args {
    method: Ident,
    path: LitStr,
    tags: Option<Vec<LitStr>>,
}

pub fn endpoint(attr: TokenStream, mut input: ItemFn) -> TokenStream {
    // parse args
    let args = match NestedMeta::parse_meta_list(attr) {
        Ok(v) => v,
        Err(e) => {
            return darling::Error::from(e).write_errors();
        }
    };
    let args = match Args::from_list(&args) {
        Ok(v) => v,
        Err(e) => {
            return e.write_errors();
        }
    };
    let path = &args.path;
    let method = &args.method;
    let return_type = match &input.sig.output {
        ReturnType::Default => quote!(()),
        ReturnType::Type(_, t) => t.to_token_stream(),
    };
    let docs = DocAttr::from(input.attrs.as_slice());
    let description_mod = get_description_modifier(&args, &docs);

    // Helper variables
    let fn_name = &input.sig.ident;
    let vis = &input.vis;

    // extract names and types for the input arguments of the endpoint
    let n_args = input.sig.inputs.len();
    let mut arg_types = Vec::with_capacity(n_args);
    let mut arg_names = Vec::with_capacity(n_args);
    for (i, arg) in input.sig.inputs.iter_mut().enumerate() {
        let FnArg::Typed(arg) = arg else {
            return Error::new(arg.span(), "receiver arg not supported").to_compile_error();
        };
        arg_types.push(arg.ty.clone());
        arg_names.push(format_ident!("arg{i}"));
    }

    // we will need to differentiate between the last and the rest of the function arguments later
    // (DescribeParameter vs DescribePartsParameter).
    // keep the last parameter in a slice to allow using repetitions in quote
    let (arg_types_no_last, arg_names_no_last, arg_type_last, arg_name_last) = if n_args == 0 {
        (
            arg_types.as_slice(),
            arg_names.as_slice(),
            [].as_slice(),
            [].as_slice(),
        )
    } else {
        (
            &arg_types[..n_args - 1],
            &arg_names[..n_args - 1],
            &arg_types[n_args - 1..],
            &arg_names[n_args - 1..],
        )
    };

    let where_clause = quote! {
        where
            #(
            #arg_types_no_last: FromRequestParts<S>,
            )*
            #(#arg_type_last: FromRequest<S, V>,)*
            S: Send + Sync
    };
    let (gen_v, ty_v) = if arg_types.is_empty() {
        (None, quote!(()))
    } else {
        (Some(quote!(V)), quote!(V))
    };

    let operation_name = input.sig.ident.to_string();
    quote! {
        #[derive(Clone)]
        #[allow(non_camel_case_types)]
        #docs
        #vis struct #fn_name;

        const _: () = {
            use autapi as _autapi;
            use _autapi::response::IntoResponse;
            use _autapi::request::FromRequest;
            use _autapi::request::FromRequestParts;
            use _autapi::Registry;

            impl<S, #gen_v> _autapi::endpoint::Endpoint<S, #ty_v> for #fn_name #where_clause {
                fn path(&self) -> std::borrow::Cow<'static, str> {
                    #path.into()
                }
                fn method(&self) -> _autapi::http::Method {
                    _autapi::http::Method::#method
                }
                fn operation_id(&self) -> std::borrow::Cow<'static, str> {
                    std::borrow::Cow::Borrowed(#operation_name)
                }
                fn openapi(
                    &self,
                    registry: &mut Registry
                ) -> _autapi::openapi::Operation {
                    let mut operation = _autapi::openapi::Operation::default();
                    operation.responses = _autapi::openapi::Responses::merge_iter(
                        [
                            <#return_type as IntoResponse>::openapi(registry),
                            #(
                                <
                                    <#arg_types as FromRequest<S, _>>::Rejection as IntoResponse
                                >::openapi(registry),
                            )*
                        ]
                    ).unwrap();
                    #(
                    <#arg_types as FromRequest<S, _>>::openapi(&mut operation, registry);
                    )*
                    #description_mod
                    operation
                }
                async fn call(self, req: _autapi::request::Request, state: S) -> _autapi::response::Response {
                    let (mut parts, body) = req.into_parts();
                    #(
                        let #arg_names_no_last = match <#arg_types_no_last as FromRequestParts<S>>::from_request_parts(&mut parts, &state).await {
                            Ok(arg) => arg,
                            Err(err) => return IntoResponse::into_response(err),
                        };
                    )*
                    let req = _autapi::request::Request::from_parts(parts, body);
                    #(
                        let #arg_name_last = match <#arg_type_last as FromRequest<S, _>>::from_request(req, &state).await {
                            Ok(arg) => arg,
                            Err(err) => return IntoResponse::into_response(err),
                        };
                    )*
                    #[allow(clippy::unused_async)]
                    #input
                    IntoResponse::into_response(
                        #fn_name(#(#arg_names),*).await
                    )
                }
            }
        };
    }
}

fn get_description_modifier(args: &Args, docs: &DocAttr) -> TokenStream {
    let mut description_mod = TokenStream::new();

    // handle openapi operation tags
    if let Some(tags) = &args.tags {
        quote! {
            operation.tags.extend([#(#tags),*].into_iter().map(|str| str.to_owned()));
        }
        .to_tokens(&mut description_mod);
    }

    // handle openapi operation summary and description fields
    if let Some((title, body)) = docs.clone().into_title_and_body() {
        quote!(operation.summary = Some(#title.to_owned());).to_tokens(&mut description_mod);

        if let Some(description) = body {
            quote!(operation.description = Some(#description.to_owned());)
                .to_tokens(&mut description_mod)
        }
    }

    description_mod
}
