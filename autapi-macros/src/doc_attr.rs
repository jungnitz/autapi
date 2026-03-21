use proc_macro2::Span;
use quote::{ToTokens, quote_spanned};
use syn::{Attribute, Expr, ExprLit, Lit, LitStr, Meta, spanned::Spanned};

#[derive(Debug, Clone)]
pub struct DocAttr(Option<(Vec<String>, Span)>);

impl DocAttr {
    pub fn try_from<E>(attrs: impl AsRef<[Attribute]>) -> Result<Self, E> {
        Ok(Self::from(attrs.as_ref()))
    }

    pub fn to_lines(&self) -> Option<LitStr> {
        self.0
            .as_ref()
            .map(|(lines, span)| LitStr::new(&lines.join("\n"), *span))
    }

    pub fn into_title_and_body(self) -> Option<(String, Option<String>)> {
        let docs = self.0?.0;

        // remove lines until we reach an empty line, which gives us the title
        let mut i = 0;
        let mut title = String::new();
        while i < docs.len() {
            if docs[i].is_empty() {
                break;
            }
            if !title.is_empty() {
                title.push(' ');
            }
            title.push_str(&docs[i]);
            i += 1;
        }

        let body = if i == docs.len() {
            None
        } else {
            Some(docs[i + 1..].join("\n"))
        };

        Some((title, body))
    }
}

impl From<&[Attribute]> for DocAttr {
    fn from(attrs: &[Attribute]) -> Self {
        // collect all attributes
        let mut docs = None::<(Vec<_>, Span)>;
        for attr in attrs {
            let (docs, _) = docs.get_or_insert_with(|| (Default::default(), attr.span()));
            if let Meta::NameValue(meta) = &attr.meta {
                if !meta.path.is_ident("doc") {
                    continue;
                }
                let Expr::Lit(ExprLit {
                    lit: Lit::Str(str), ..
                }) = &meta.value
                else {
                    continue;
                };
                let mut doc = str.value();
                doc.truncate(doc.trim_end().len());
                docs.push(doc);
            };
        }
        // remove leading whitespace
        if let Some((docs, _)) = &mut docs {
            let indent = docs
                .iter()
                .filter(|doc| !doc.is_empty())
                .map(|s| s.len() - s.trim_ascii_start().len())
                .min()
                .unwrap_or(0);
            docs.iter_mut()
                .filter(|doc| !doc.is_empty())
                .for_each(|doc| {
                    doc.drain(0..indent);
                });
        }
        Self(docs)
    }
}

impl ToTokens for DocAttr {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let Some((comments, span)) = &self.0 else {
            return;
        };
        tokens.extend(quote_spanned!(*span=> #(#[doc = #comments])*));
    }
}
