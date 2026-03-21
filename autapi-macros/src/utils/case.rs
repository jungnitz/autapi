use convert_case::Casing;
use darling::FromMeta;
use proc_macro2::Span;
use syn::Lit;

#[derive(Debug, Copy, Clone)]
pub struct Case(Span, convert_case::Case<'static>);

impl Case {
    pub fn format(self, str: &str) -> String {
        str.to_case(self.1)
    }
    #[expect(unused)]
    pub fn span(&self) -> Span {
        self.0
    }
}

impl FromMeta for Case {
    fn from_value(value: &syn::Lit) -> darling::Result<Self> {
        match value {
            Lit::Str(str) => {
                let str = str.value();
                Ok(Self(
                    value.span(),
                    match str.as_str() {
                        "lowercase" => convert_case::Case::Lower,
                        "UPPERCASE" => convert_case::Case::Upper,
                        "PascalCase" => convert_case::Case::Pascal,
                        "camelCase" => convert_case::Case::Camel,
                        "snake_case" => convert_case::Case::Snake,
                        "SCREAMING_SNAKE_CASE" => convert_case::Case::UpperSnake,
                        "kebab-case" => convert_case::Case::Kebab,
                        "SCREAMING-KEBAB-CASE" => convert_case::Case::UpperKebab,
                        _ => return Err(darling::Error::unknown_value(&str)),
                    },
                ))
            }
            _ => Err(darling::Error::unexpected_lit_type(value)),
        }
    }
}
