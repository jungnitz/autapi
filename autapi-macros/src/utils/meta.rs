use darling::FromMeta;
use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::Expr;

/// An array of expressions.
#[derive(Clone, Debug)]
pub struct ExprsMeta(pub Vec<Expr>);

impl FromMeta for ExprsMeta {
    fn from_expr(expr: &Expr) -> darling::Result<Self> {
        let Expr::Array(array) = expr else {
            return Err(darling::Error::unexpected_expr_type(expr));
        };
        let examples = array.elems.iter().cloned().collect();
        Ok(Self(examples))
    }
}

#[derive(Clone, Debug)]
pub struct PreserveStringExpr(pub Expr);

impl FromMeta for PreserveStringExpr {
    fn from_meta(item: &syn::Meta) -> darling::Result<Self> {
        darling::util::parse_expr::preserve_str_literal(item).map(Self)
    }
}

impl ToTokens for PreserveStringExpr {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.0.to_tokens(tokens);
    }
}
