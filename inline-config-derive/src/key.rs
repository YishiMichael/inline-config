#[derive(Clone)]
pub(crate) struct Key(Vec<KeySegment>);

#[derive(Clone)]
pub(crate) enum KeySegment {
    Name(syn::Ident),
    Index(syn::LitInt),
}

impl syn::parse::Parse for Key {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut key_segments = vec![];
        if let Ok(root) = input.parse::<syn::Ident>() {
            key_segments.push(KeySegment::Name(root));
        }
        while !input.is_empty() {
            key_segments.push(input.parse()?);
        }
        Ok(Self(key_segments))
    }
}

impl syn::parse::Parse for KeySegment {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if input.parse::<syn::Token![.]>().is_ok() {
            let name = input.parse()?;
            return Ok(KeySegment::Name(name));
        }
        if input.peek(syn::token::Bracket) {
            let index;
            syn::bracketed!(index in input);
            let index = input.parse()?;
            return Ok(KeySegment::Index(index));
        }
        Err(input.error("expected .<Ident> | [<LitInt>]"))
    }
}

impl Key {
    pub(crate) fn type_ts(self) -> proc_macro2::TokenStream {
        fn recurse(key_segments: &[KeySegment]) -> proc_macro2::TokenStream {
            if let Some((root, postfix)) = key_segments.split_first() {
                let root_type_ts = match root {
                    KeySegment::Name(name) => KeySegment::name_type_ts(name.to_string().as_str()),
                    KeySegment::Index(index) => match index.base10_parse() {
                        Ok(index) => KeySegment::index_type_ts(index),
                        Err(e) => proc_macro_error::abort!(e.span(), e),
                    },
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

    pub(crate) fn index_type_ts(index: usize) -> proc_macro2::TokenStream {
        quote::quote! {
            ::inline_config::__private::KeySegmentIndex<#index>
        }
    }
}
