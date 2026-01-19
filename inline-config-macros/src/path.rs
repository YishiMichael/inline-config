pub struct Path(syn::punctuated::Punctuated<Key, syn::Token![.]>);

pub enum Key {
    Index(usize),
    Name(String),
}

impl syn::parse::Parse for Path {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self(if input.is_empty() {
            syn::punctuated::Punctuated::new()
        } else {
            syn::punctuated::Punctuated::parse_separated_nonempty(input)?
        }))
    }
}

impl syn::parse::Parse for Key {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if input.peek(syn::LitInt) {
            input
                .parse()
                .map(|input: syn::Index| Key::Index(input.index as usize))
        } else if input.peek(syn::Ident) {
            input
                .parse()
                .map(|input: syn::Ident| Key::Name(input.to_string()))
        } else if input.peek(syn::LitStr) {
            input
                .parse()
                .map(|input: syn::LitStr| Key::Name(input.value()))
        } else {
            Err(input.error("expected integer or identifier or literal string"))
        }
    }
}

impl Path {
    pub fn ty(self) -> syn::Type {
        self.0.iter().rfold(
            syn::parse_quote! {
                ::inline_config::__private::PathNil
            },
            |tail_ty, key| {
                let head_ty = match key {
                    Key::Index(index) => Key::index_ty(*index),
                    Key::Name(name) => Key::name_ty(name),
                };
                syn::parse_quote! {
                    ::inline_config::__private::PathCons<#head_ty, #tail_ty>
                }
            },
        )
    }

    pub fn expr(self) -> syn::Expr {
        let ty = self.ty();
        syn::parse_quote! {
            <#ty>::default()
        }
    }
}

impl Key {
    pub fn index_ty(index: usize) -> syn::Type {
        let index_str = index.to_string();
        let tys = index_str.chars().map(|c| -> syn::Type {
            let ident = quote::format_ident!("_{c}");
            syn::parse_quote! {
                ::inline_config::__private::chars::#ident
            }
        });
        syn::parse_quote! {
            ::inline_config::__private::KeyIndex<(#(#tys,)*)>
        }
    }

    pub fn name_ty(name: &str) -> syn::Type {
        // Referenced from frunk_proc_macro_helpers/lib.rs
        let tys = name.trim().chars().map(|c| -> syn::Type {
            match c {
                'A'..='Z' | 'a'..='z' => {
                    let ident = quote::format_ident!("{c}");
                    syn::parse_quote! {
                        ::inline_config::__private::chars::#ident
                    }
                }
                '0'..='9' | '_' => {
                    let ident = quote::format_ident!("_{c}");
                    syn::parse_quote! {
                        ::inline_config::__private::chars::#ident
                    }
                }
                c => syn::parse_quote! {
                    ::inline_config::__private::chars::Ch<#c>
                },
            }
        });
        syn::parse_quote! {
            ::inline_config::__private::KeyName<(#(#tys,)*)>
        }
    }
}
