#[derive(Clone)]
pub(crate) struct Path(Vec<Key>);

#[derive(Clone)]
pub(crate) enum Key {
    Index(usize),
    Name(String),
}

mod parse {
    // Reference: https://docs.rs/config/latest/src/config/path/parser.rs.html
    use super::{Key, Path};
    use std::str::FromStr;
    use winnow::ascii::{digit1, space0};
    use winnow::combinator::{alt, delimited, opt, preceded, repeat, terminated};
    use winnow::error::{StrContext, StrContextValue};
    use winnow::prelude::*;
    use winnow::token::take_while;

    impl syn::parse::Parse for Path {
        fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
            let s: syn::LitStr = input.parse()?;
            path.parse(s.value().as_str())
                .map_err(|e| syn::Error::new(s.span(), e))
        }
    }

    fn path(s: &mut &str) -> ModalResult<Path> {
        terminated(
            (
                opt(preceded(space0, name).map(Key::Name)),
                repeat(0.., preceded(space0, key)),
            ),
            space0,
        )
        .map(|(root, postfix)| Path(root.into_iter().chain::<Vec<_>>(postfix).collect()))
        .parse_next(s)
    }

    fn key(s: &mut &str) -> ModalResult<Key> {
        alt((
            delimited('[', delimited(space0, index, space0), ']').map(Key::Index),
            preceded('.', preceded(space0, name)).map(Key::Name),
        ))
        .parse_next(s)
    }

    fn index(s: &mut &str) -> ModalResult<usize> {
        digit1
            .try_map(FromStr::from_str)
            .context(StrContext::Label("index"))
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
}

impl Path {
    pub(crate) fn ty(self) -> syn::Type {
        self.0.iter().rfold(
            syn::parse_quote! {
                ::inline_config::__private::key::PathNil
            },
            |tail_ty, key| {
                let head_ty = match key {
                    Key::Index(index) => Key::index_ty(*index),
                    Key::Name(name) => Key::name_ty(name),
                };
                syn::parse_quote! {
                    ::inline_config::__private::key::PathCons<#head_ty, #tail_ty>
                }
            },
        )
    }

    pub(crate) fn expr(self) -> syn::Expr {
        self.0.iter().rfold(
            syn::parse_quote! {
                ::inline_config::__private::key::PathNil
            },
            |tail_expr, key| {
                let head_expr = match key {
                    Key::Index(index) => Key::index_expr(*index),
                    Key::Name(name) => Key::name_expr(name),
                };
                syn::parse_quote! {
                    ::inline_config::__private::key::PathCons(#head_expr, #tail_expr)
                }
            },
        )
    }
}

impl Key {
    fn index_ty_param(index: usize) -> syn::Type {
        let index_str = index.to_string();
        let tys = index_str.chars().map(|c| -> syn::Type {
            let ident = quote::format_ident!("_{c}");
            syn::parse_quote! {
                ::inline_config::__private::key::chars::#ident
            }
        });
        syn::parse_quote! {
            (#(#tys,)*)
        }
    }

    pub(crate) fn index_ty(index: usize) -> syn::Type {
        let index_ty_param = Self::index_ty_param(index);
        syn::parse_quote! {
            ::inline_config::__private::key::KeyIndex<#index_ty_param>
        }
    }

    pub(crate) fn index_expr(index: usize) -> syn::Expr {
        let index_ty_param = Self::index_ty_param(index);
        syn::parse_quote! {
            ::inline_config::__private::key::KeyIndex {
                phantom: ::std::marker::PhantomData::<#index_ty_param>,
                index: #index,
            }
        }
    }

    fn name_ty_param(name: &str) -> syn::Type {
        // Referenced from frunk_proc_macro_helpers/lib.rs
        let tys = name.chars().map(|c| -> syn::Type {
            match c {
                'a'..'z' | 'A'..'Z' => {
                    let ident = quote::format_ident!("{c}");
                    syn::parse_quote! {
                        ::inline_config::__private::key::chars::#ident
                    }
                }
                '0'..'9' | '_' => {
                    let ident = quote::format_ident!("_{c}");
                    syn::parse_quote! {
                        ::inline_config::__private::key::chars::#ident
                    }
                }
                _ => {
                    let codepoint = c as u32;
                    syn::parse_quote! {
                        ::inline_config::__private::key::chars::UC<#codepoint>
                    }
                }
            }
        });
        syn::parse_quote! {
            (#(#tys,)*)
        }
    }

    pub(crate) fn name_ty(name: &str) -> syn::Type {
        let name_ty_param = Self::name_ty_param(name);
        syn::parse_quote! {
            ::inline_config::__private::key::KeyName<#name_ty_param>
        }
    }

    pub(crate) fn name_expr(name: &str) -> syn::Expr {
        let name_ty_param = Self::name_ty_param(name);
        syn::parse_quote! {
            ::inline_config::__private::key::KeyName {
                phantom: ::std::marker::PhantomData::<#name_ty_param>,
                name: #name,
            }
        }
    }
}
