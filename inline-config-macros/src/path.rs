#[derive(Clone)]
pub enum Key {
    Index(usize),
    Name(String),
}

impl Key {
    pub fn index_ty(index: usize) -> syn::Type {
        syn::parse_quote! {
            ::inline_config::__private::KeyIndex<#index>
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

    fn ty(self) -> syn::Type {
        match self {
            Self::Index(index) => Self::index_ty(index),
            Self::Name(name) => Self::name_ty(&name),
        }
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

#[derive(Clone)]
pub enum Path {
    Nil,
    LCons(Key, Box<Self>),
    RCons(Box<Self>, Key),
}

impl Path {
    pub fn nil() -> Self {
        Self::Nil
    }

    pub fn left_cons(self, key: Key) -> Self {
        Self::LCons(key, Box::new(self))
    }

    pub fn right_cons(self, key: Key) -> Self {
        Self::RCons(Box::new(self), key)
    }

    pub fn ty(self) -> syn::Type {
        match self {
            Self::Nil => syn::parse_quote! {
                ::inline_config::__private::PathNil
            },
            Self::LCons(key, path) => {
                let key_ty = key.ty();
                let path_ty = path.ty();
                syn::parse_quote! {
                    ::inline_config::__private::PathLCons<#key_ty, #path_ty>
                }
            }
            Self::RCons(path, key) => {
                let path_ty = path.ty();
                let key_ty = key.ty();
                syn::parse_quote! {
                    ::inline_config::__private::PathRCons<#path_ty, #key_ty>
                }
            }
        }
    }

    pub fn expr(self) -> syn::Expr {
        let ty = self.ty();
        syn::parse_quote! {
            <#ty>::default()
        }
    }
}

impl syn::parse::Parse for Path {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(if input.is_empty() {
            Self::nil()
        } else {
            syn::punctuated::Punctuated::<Key, syn::Token![.]>::parse_separated_nonempty(input)?
                .into_iter()
                .rfold(Self::nil(), |path, key| path.left_cons(key))
        })
    }
}
