#[derive(Clone)]
pub enum Key {
    Index(usize),
    Name(String),
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

impl quote::ToTokens for Key {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            Self::Index(index) => quote::quote! {
                ::inline_config::__private::KeyIndex<#index>
            }
            .to_tokens(tokens),
            Self::Name(name) => {
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
                quote::quote! {
                    ::inline_config::__private::KeyName<(#(#tys,)*)>
                }
                .to_tokens(tokens);
            }
        }
    }
}

#[derive(Clone, Default)]
pub struct Path {
    keys: Vec<Key>,
}

impl Path {
    pub fn ty(self) -> syn::Type {
        syn::parse_quote! {
            #self
        }
    }

    pub fn expr(self) -> syn::Expr {
        syn::parse_quote! {
            <#self>::default()
        }
    }
}

impl syn::parse::Parse for Path {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(if input.is_empty() {
            Self::default()
        } else {
            Self {
                keys: syn::punctuated::Punctuated::<Key, syn::Token![.]>::parse_separated_nonempty(
                    input,
                )?
                .into_iter()
                .collect(),
            }
        })
    }
}

impl quote::ToTokens for Path {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.keys
            .iter()
            .rfold(
                quote::quote! {
                    ::inline_config::__private::PathNil
                },
                |path, key| {
                    quote::quote! {
                        ::inline_config::__private::PathCons<#key, #path>
                    }
                },
            )
            .to_tokens(tokens)
    }
}
