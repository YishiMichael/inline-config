#[derive(Clone)]
pub(crate) struct Key(Vec<KeySegment>);

#[derive(Clone)]
pub(crate) enum KeySegment {
    Name(String),
    Index(usize),
}

mod parse {
    // Reference: https://docs.rs/config/latest/src/config/path/parser.rs.html
    use super::{Key, KeySegment};
    use std::str::FromStr;
    use winnow::ascii::{digit1, space0};
    use winnow::combinator::{alt, delimited, opt, preceded, repeat, terminated};
    use winnow::error::{StrContext, StrContextValue};
    use winnow::prelude::*;
    use winnow::token::take_while;

    impl syn::parse::Parse for Key {
        fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
            let s: syn::LitStr = input.parse()?;
            key.parse(s.value().as_str())
                .map_err(|e| syn::Error::new(s.span(), e))
        }
    }

    fn key(s: &mut &str) -> ModalResult<Key> {
        terminated(
            (
                opt(preceded(space0, name).map(KeySegment::Name)),
                repeat(
                    0..,
                    preceded(
                        space0,
                        alt((
                            preceded('.', preceded(space0, name)).map(KeySegment::Name),
                            delimited('[', delimited(space0, index, space0), ']')
                                .map(KeySegment::Index),
                        )),
                    ),
                ),
            ),
            space0,
        )
        .map(|(root, postfix)| Key(root.into_iter().chain::<Vec<_>>(postfix).collect()))
        .parse_next(s)
    }

    fn name(s: &mut &str) -> ModalResult<String> {
        take_while(1.., ('a'..='z', 'A'..='Z', '0'..='9', '_', '-'))
            .map(ToOwned::to_owned)
            .context(StrContext::Label("name"))
            .context(StrContext::Expected(StrContextValue::Description(
                "ASCII alphanumeric",
            )))
            .context(StrContext::Expected(StrContextValue::CharLiteral('_')))
            .context(StrContext::Expected(StrContextValue::CharLiteral('-')))
            .parse_next(s)
    }

    fn index(s: &mut &str) -> ModalResult<usize> {
        digit1
            .try_map(FromStr::from_str)
            .context(StrContext::Label("index"))
            .parse_next(s)
    }
}

impl Key {
    pub(crate) fn ty(self) -> syn::Type {
        fn recurse(key_segments: &[KeySegment]) -> syn::Type {
            if let Some((root, postfix)) = key_segments.split_first() {
                let root_ty = match root {
                    KeySegment::Name(name) => KeySegment::name_ty(name),
                    KeySegment::Index(index) => KeySegment::index_ty(*index),
                };
                let postfix_ty = recurse(postfix);
                syn::parse_quote! {
                    ::inline_config::__private::KeyCons<#root_ty, #postfix_ty>
                }
            } else {
                syn::parse_quote! {
                    ::inline_config::__private::KeyNil
                }
            }
        }
        recurse(&self.0)
    }

    pub(crate) fn expr(self) -> syn::Expr {
        let ty = self.ty();
        syn::parse_quote! {
            <#ty>::default()
        }
    }
}

impl KeySegment {
    pub(crate) fn name_ty(name: &str) -> syn::Type {
        let hash = const_fnv1a_hash::fnv1a_hash_str_64(name);
        syn::parse_quote! {
            ::inline_config::__private::KeySegmentName<#hash>
        }
    }

    pub(crate) fn index_ty(index: usize) -> syn::Type {
        syn::parse_quote! {
            ::inline_config::__private::KeySegmentIndex<#index>
        }
    }
}
