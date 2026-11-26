use darling::FromMeta;
use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{Expr, ExprLit, ExprUnary, Lit, UnOp};

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

/// A numeric literal, optionally negated (e.g. `3`, `-2.5`).
///
/// Integers are limited to the range of `i64` and `u64`, as only these can be represented exactly
/// as JSON numbers.
#[derive(Clone, Copy, Debug)]
pub enum NumberMeta {
    Int(i64),
    UInt(u64),
    Float(f64),
}

impl NumberMeta {
    /// Returns an approximation of this number, used for sanity checks at compile-time.
    pub fn as_f64(self) -> f64 {
        match self {
            Self::Int(n) => n as f64,
            Self::UInt(n) => n as f64,
            Self::Float(n) => n,
        }
    }
}

impl FromMeta for NumberMeta {
    fn from_expr(expr: &Expr) -> darling::Result<Self> {
        let (negative, lit) = match expr {
            Expr::Lit(ExprLit { lit, .. }) => (false, lit),
            Expr::Unary(ExprUnary {
                op: UnOp::Neg(_),
                expr: inner,
                ..
            }) => match &**inner {
                Expr::Lit(ExprLit { lit, .. }) => (true, lit),
                _ => return Err(darling::Error::unexpected_expr_type(expr)),
            },
            _ => return Err(darling::Error::unexpected_expr_type(expr)),
        };
        match lit {
            Lit::Int(int) => {
                let out_of_range = || {
                    darling::Error::custom("integers must fit into `i64` or `u64`").with_span(expr)
                };
                let n: u64 = int.base10_parse().map_err(|_| out_of_range())?;
                if negative {
                    0i64.checked_sub_unsigned(n)
                        .map(Self::Int)
                        .ok_or_else(out_of_range)
                } else {
                    Ok(i64::try_from(n).map(Self::Int).unwrap_or(Self::UInt(n)))
                }
            }
            Lit::Float(float) => {
                let n: f64 = float.base10_parse()?;
                if !n.is_finite() {
                    return Err(darling::Error::custom("number must be finite").with_span(expr));
                }
                Ok(Self::Float(if negative { -n } else { n }))
            }
            _ => Err(darling::Error::unexpected_lit_type(lit)),
        }
    }
}
