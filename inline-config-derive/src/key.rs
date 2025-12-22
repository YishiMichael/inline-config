#[derive(Clone)]
pub(crate) struct Key(Vec<KeySegment>);

#[derive(Clone)]
pub(crate) enum KeySegment {
    Name(String),
    Index(isize),
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

    fn index(s: &mut &str) -> ModalResult<isize> {
        (opt('-'), digit1)
            .take()
            .try_map(FromStr::from_str)
            .context(StrContext::Expected(StrContextValue::Description("index")))
            .parse_next(s)
    }
}

impl Key {
    pub(crate) fn type_ts(self) -> proc_macro2::TokenStream {
        fn recurse(key_segments: &[KeySegment]) -> proc_macro2::TokenStream {
            if let Some((root, postfix)) = key_segments.split_first() {
                let root_type_ts = match root {
                    KeySegment::Name(name) => KeySegment::name_type_ts(name),
                    KeySegment::Index(index) => KeySegment::index_type_ts(*index),
                };
                let postfix_type_ts = recurse(postfix);
                quote::quote! {
                    ::inline_config::__private::KeyCons<#root_type_ts, #postfix_type_ts>
                }
            } else {
                quote::quote! {
                    ::inline_config::__private::KeyNil
                }
            }
        }
        recurse(&self.0)
    }

    pub(crate) fn value_ts(self) -> proc_macro2::TokenStream {
        let type_ts = self.type_ts();
        quote::quote! {
            <#type_ts>::default()
        }
    }
}

impl KeySegment {
    pub(crate) fn name_type_ts(name: &str) -> proc_macro2::TokenStream {
        let hash = const_fnv1a_hash::fnv1a_hash_str_64(name);
        quote::quote! {
            ::inline_config::__private::KeySegmentName<#hash>
        }
    }

    pub(crate) fn index_type_ts(index: isize) -> proc_macro2::TokenStream {
        quote::quote! {
            ::inline_config::__private::KeySegmentIndex<#index>
        }
    }
}
