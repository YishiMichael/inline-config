/// Enables eager expansion of built-in macros.
///
/// Mimics the job of [`proc_macro::TokenStream::expand_expr`] which requires `#![feature(proc_macro_expand)]`.
pub enum Lit {
    Str(syn::LitStr),
    IncludeStr(proc_macro2::Span, IncludeStr),
    Concat(proc_macro2::Span, Concat),
    Env(proc_macro2::Span, Env),
}

pub struct IncludeStr(Box<Lit>);
pub struct Concat(Vec<Lit>);
pub struct Env(Box<Lit>, Option<Box<Lit>>);

impl syn::parse::Parse for Lit {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        match input.parse()? {
            syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(s),
                ..
            }) => Ok(Self::Str(s)),
            syn::Expr::Macro(syn::ExprMacro { mac, .. }) => {
                match mac.path.require_ident()?.to_string().as_str() {
                    "include_str" => Ok(Self::IncludeStr(
                        syn::spanned::Spanned::span(&mac.tokens),
                        syn::parse2(mac.tokens)?,
                    )),
                    "concat" => Ok(Self::Concat(
                        syn::spanned::Spanned::span(&mac.tokens),
                        syn::parse2(mac.tokens)?,
                    )),
                    "env" => Ok(Self::Env(
                        syn::spanned::Spanned::span(&mac.tokens),
                        syn::parse2(mac.tokens)?,
                    )),
                    _ => Err(syn::Error::new_spanned(mac.path, "unsupported macro")),
                }
            }
            expr => Err(syn::Error::new_spanned(expr, "expected string literal")),
        }
    }
}

impl syn::parse::Parse for IncludeStr {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let args: Vec<_> = input
            .call(syn::punctuated::Punctuated::<Lit, syn::Token![,]>::parse_terminated)?
            .into_iter()
            .collect();
        match TryInto::<[Lit; 1]>::try_into(args) {
            Ok([arg]) => Ok(Self(Box::new(arg))),
            _ => Err(input.error("invalid token")),
        }
    }
}

impl syn::parse::Parse for Concat {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let args: Vec<_> = input
            .call(syn::punctuated::Punctuated::<Lit, syn::Token![,]>::parse_terminated)?
            .into_iter()
            .collect();
        Ok(Self(args))
    }
}

impl syn::parse::Parse for Env {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let args: Vec<_> = input
            .call(syn::punctuated::Punctuated::<Lit, syn::Token![,]>::parse_terminated)?
            .into_iter()
            .collect();
        match TryInto::<[Lit; 1]>::try_into(args) {
            Ok([arg]) => Ok(Self(Box::new(arg), None)),
            Err(args) => match TryInto::<[Lit; 2]>::try_into(args) {
                Ok([arg, msg]) => Ok(Self(Box::new(arg), Some(Box::new(msg)))),
                _ => Err(input.error("invalid token")),
            },
        }
    }
}

impl Lit {
    pub fn expand(self) -> syn::Result<String> {
        match self {
            Self::Str(lit) => Ok(lit.value()),
            Self::IncludeStr(span, args) => args.expand(span),
            Self::Concat(span, args) => args.expand(span),
            Self::Env(span, args) => args.expand(span),
        }
    }
}

impl IncludeStr {
    fn expand(self, span: proc_macro2::Span) -> syn::Result<String> {
        let arg = self.0.expand()?;
        let path = std::path::PathBuf::from(arg);
        // Resolve the path relative to the current file.
        let path = if path.is_absolute() {
            path
        } else {
            // Rust analyzer hasn't implemented `Span::file()`.
            // https://github.com/rust-lang/rust-analyzer/issues/15950
            std::path::PathBuf::from(proc_macro2::Span::call_site().file())
                .parent()
                .ok_or(syn::Error::new(span, "cannot retrieve parent dir"))?
                .join(path)
        };
        std::fs::read_to_string(path).map_err(|e| syn::Error::new(span, e))
    }
}

impl Concat {
    fn expand(self, _span: proc_macro2::Span) -> syn::Result<String> {
        Ok(self
            .0
            .into_iter()
            .map(Lit::expand)
            .collect::<syn::Result<Vec<_>>>()?
            .join(""))
    }
}

impl Env {
    fn expand(self, span: proc_macro2::Span) -> syn::Result<String> {
        let arg = self.0.expand()?;
        let msg = self.1.map(|msg| msg.expand()).transpose()?;
        std::env::var(arg).map_err(|e| syn::Error::new(span, msg.unwrap_or_else(|| e.to_string())))
    }
}
