use crate::format::Format;
use crate::path::Key;
use crate::value::{Map, Value};

pub struct ConfigItem {
    ident: syn::Ident,
    value: Value,
}

pub fn config(item: syn::ItemStruct) -> syn::Result<ConfigItem> {
    let mut errors = darling::Error::accumulator();
    if matches!(item.fields, syn::Fields::Named(_)) {
        errors.push(darling::Error::unsupported_shape("non-unit struct"));
    }
    if matches!(item.fields, syn::Fields::Unnamed(_)) {
        errors.push(darling::Error::unsupported_shape("enum"));
    }
    let value = item
        .attrs
        .into_iter()
        .filter(|attr| attr.path().is_ident("config"))
        .filter_map(|attr| {
            errors.handle_in(|| {
                let meta_list: syn::MetaList =
                    syn::parse2(attr.meta.require_list()?.tokens.clone())?;
                let format = Format::from_str(&meta_list.path.require_ident()?.to_string()).ok_or(
                    syn::Error::new_spanned(&meta_list.path, "format not supported"),
                )?;
                let source: macro_string::MacroString = syn::parse2(meta_list.tokens.clone())?;
                let value = format.parse(&source.eval()?).map_err(|e| source.error(e))?;
                Ok(value)
            })
        })
        .sum();
    errors.finish()?;
    Ok(ConfigItem {
        ident: item.ident,
        value,
    })
}

impl quote::ToTokens for ConfigItem {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.value.to_item_const(&self.ident).to_tokens(tokens);
    }
}

struct ConvertImpl {
    ty: syn::Type,
    expr: syn::Expr,
    generics: syn::Generics,
}

trait ValueVariant {
    fn convert_impls(
        &self,
        _children_tys: &[syn::Type],
        _children_exprs: &[syn::Expr],
    ) -> Vec<ConvertImpl> {
        Vec::new()
    }

    fn children(&self) -> Vec<(Key, &Value)> {
        Vec::new()
    }

    fn to_item_const(&self, ident: &syn::Ident) -> syn::ItemConst {
        let ((children_tys, children_exprs), (item_impls_index, item_mods)): (
            (Vec<syn::Type>, Vec<syn::Expr>),
            (Vec<syn::ItemImpl>, Vec<syn::ItemMod>),
        ) = self
            .children()
            .iter()
            .enumerate()
            .map(|(index, (key, value))| {
                let mod_ident = quote::format_ident!("_{index}");
                let child_ident = quote::format_ident!("Type");
                let item_const = value.to_item_const(&child_ident);
                let child_ty = syn::parse_quote! {
                    #mod_ident::#child_ident
                };
                let child_expr = syn::parse_quote! {
                    #mod_ident::#child_ident
                };
                let item_impl_index = syn::parse_quote! {
                    impl ::std::ops::Index<#key> for #ident {
                        type Output = #child_ty;

                        fn index(&self, _index: #key) -> &Self::Output {
                            <&#child_ty as Default>::default()
                        }
                    }
                };
                let item_mod = syn::parse_quote! {
                    pub mod #mod_ident {
                        pub struct #child_ident;

                        #item_const
                    }
                };
                ((child_ty, child_expr), (item_impl_index, item_mod))
            })
            .unzip();
        let item_impls_from: Vec<syn::ItemImpl> = self
            .convert_impls(&children_tys, &children_exprs)
            .into_iter()
            .map(|ConvertImpl { ty, expr, generics }| {
                syn::parse_quote! {
                    impl #generics From<#ident> for #ty {
                        fn from(_value: #ident) -> Self {
                            #expr
                        }
                    }
                }
            })
            .collect();
        syn::parse_quote! {
            const _: () = {
                impl Clone for #ident {
                    fn clone(&self) -> Self {
                        Self
                    }
                }

                impl Copy for #ident {}

                impl Default for #ident {
                    fn default() -> Self {
                        Self
                    }
                }

                impl Default for &'static #ident {
                    fn default() -> &'static #ident {
                        &#ident
                    }
                }

                #(#item_impls_from)*

                impl ::std::ops::Index<::inline_config::__private::PathNil> for #ident {
                    type Output = #ident;

                    fn index(&self, _index: ::inline_config::__private::PathNil) -> &Self::Output {
                        &#ident
                    }
                }

                impl<__inline_config__K, __inline_config__P, __inline_config__CK, __inline_config__CKP>
                    ::std::ops::Index<::inline_config::__private::PathCons<__inline_config__K, __inline_config__P>> for #ident
                where
                    #ident: ::std::ops::Index<__inline_config__K, Output = __inline_config__CK>,
                    __inline_config__CK: ::std::ops::Index<__inline_config__P, Output = __inline_config__CKP>,
                    &'static __inline_config__CKP: Default + 'static,
                {
                    type Output = __inline_config__CKP;

                    fn index(&self, _index: ::inline_config::__private::PathCons<__inline_config__K, __inline_config__P>) -> &Self::Output {
                        <&__inline_config__CKP>::default()
                    }
                }

                #(#item_impls_index)*

                #(#item_mods)*
            };
        }
    }
}

impl ValueVariant for Value {
    fn convert_impls(
        &self,
        children_tys: &[syn::Type],
        children_exprs: &[syn::Expr],
    ) -> Vec<ConvertImpl> {
        match self {
            Self::Nil => ().convert_impls(children_tys, children_exprs),
            Self::Boolean(value) => value.convert_impls(children_tys, children_exprs),
            Self::PosInt(value) => value.convert_impls(children_tys, children_exprs),
            Self::NegInt(value) => value.convert_impls(children_tys, children_exprs),
            Self::Float(value) => value.convert_impls(children_tys, children_exprs),
            Self::String(value) => value.convert_impls(children_tys, children_exprs),
            Self::Array(value) => value.convert_impls(children_tys, children_exprs),
            Self::Table(value) => value.convert_impls(children_tys, children_exprs),
        }
    }

    fn children(&self) -> Vec<(Key, &Value)> {
        match self {
            Self::Nil => ().children(),
            Self::Boolean(value) => value.children(),
            Self::PosInt(value) => value.children(),
            Self::NegInt(value) => value.children(),
            Self::Float(value) => value.children(),
            Self::String(value) => value.children(),
            Self::Array(value) => value.children(),
            Self::Table(value) => value.children(),
        }
    }
}

impl ValueVariant for () {}

impl ValueVariant for bool {
    fn convert_impls(
        &self,
        _children_tys: &[syn::Type],
        _children_exprs: &[syn::Expr],
    ) -> Vec<ConvertImpl> {
        [ConvertImpl {
            ty: syn::parse_quote! { bool },
            expr: syn::parse_quote! { #self },
            generics: syn::Generics::default(),
        }]
        .into()
    }
}

impl ValueVariant for u64 {
    fn convert_impls(
        &self,
        _children_tys: &[syn::Type],
        _children_exprs: &[syn::Expr],
    ) -> Vec<ConvertImpl> {
        macro_rules! cast_impl {
            ($ty:ty) => {
                ConvertImpl {
                    ty: syn::parse_quote! { $ty },
                    expr: syn::parse_quote! { #self as $ty },
                    generics: syn::Generics::default(),
                }
            };
        }
        [
            cast_impl!(u8),
            cast_impl!(u16),
            cast_impl!(u32),
            cast_impl!(u64),
            cast_impl!(u128),
            cast_impl!(usize),
            cast_impl!(i8),
            cast_impl!(i16),
            cast_impl!(i32),
            cast_impl!(i64),
            cast_impl!(i128),
            cast_impl!(isize),
            cast_impl!(f32),
            cast_impl!(f64),
        ]
        .into()
    }
}

impl ValueVariant for i64 {
    fn convert_impls(
        &self,
        _children_tys: &[syn::Type],
        _children_exprs: &[syn::Expr],
    ) -> Vec<ConvertImpl> {
        macro_rules! cast_impl {
            ($ty:ty) => {
                ConvertImpl {
                    ty: syn::parse_quote! { $ty },
                    expr: syn::parse_quote! { #self as $ty },
                    generics: syn::Generics::default(),
                }
            };
        }
        [
            cast_impl!(i8),
            cast_impl!(i16),
            cast_impl!(i32),
            cast_impl!(i64),
            cast_impl!(i128),
            cast_impl!(isize),
            cast_impl!(f32),
            cast_impl!(f64),
        ]
        .into()
    }
}

impl ValueVariant for f64 {
    fn convert_impls(
        &self,
        _children_tys: &[syn::Type],
        _children_exprs: &[syn::Expr],
    ) -> Vec<ConvertImpl> {
        macro_rules! cast_impl {
            ($ty:ty) => {
                ConvertImpl {
                    ty: syn::parse_quote! { $ty },
                    expr: syn::parse_quote! { #self as $ty },
                    generics: syn::Generics::default(),
                }
            };
        }
        [cast_impl!(f32), cast_impl!(f64)].into()
    }
}

impl ValueVariant for String {
    fn convert_impls(
        &self,
        _children_tys: &[syn::Type],
        _children_exprs: &[syn::Expr],
    ) -> Vec<ConvertImpl> {
        [
            ConvertImpl {
                ty: syn::parse_quote! { &'static str },
                expr: syn::parse_quote! { #self },
                generics: syn::Generics::default(),
            },
            ConvertImpl {
                ty: syn::parse_quote! { String },
                expr: syn::parse_quote! { #self.to_string() },
                generics: syn::Generics::default(),
            },
        ]
        .into()
    }
}

impl ValueVariant for Vec<Value> {
    fn convert_impls(
        &self,
        children_tys: &[syn::Type],
        children_exprs: &[syn::Expr],
    ) -> Vec<ConvertImpl> {
        [ConvertImpl {
            ty: syn::parse_quote! { Vec<T> },
            expr: syn::parse_quote! {
                [#(<T as From<#children_tys>>::from(#children_exprs)),*].into()
            },
            generics: syn::parse_quote! {
                <T: #(From<#children_tys>)+*>
            },
        }]
        .into()
    }

    fn children(&self) -> Vec<(Key, &Value)> {
        self.iter()
            .enumerate()
            .map(|(index, value)| (Key::Index(index), value))
            .collect()
    }
}

impl ValueVariant for Map<String, Value> {
    fn convert_impls(
        &self,
        children_tys: &[syn::Type],
        children_exprs: &[syn::Expr],
    ) -> Vec<ConvertImpl> {
        let names: Vec<_> = self.keys().collect();
        [
            ConvertImpl {
                ty: syn::parse_quote! { ::std::collections::BTreeMap<&'static str, T> },
                expr: syn::parse_quote! {
                    [#((#names, <T as From<#children_tys>>::from(#children_exprs))),*].into()
                },
                generics: syn::parse_quote! {
                    <T: #(From<#children_tys>)+*>
                },
            },
            ConvertImpl {
                ty: syn::parse_quote! { ::std::collections::BTreeMap<String, T> },
                expr: syn::parse_quote! {
                    [#((#names.to_string(), <T as From<#children_tys>>::from(#children_exprs))),*].into()
                },
                generics: syn::parse_quote! {
                    <T: #(From<#children_tys>)+*>
                },
            },
            #[cfg(feature = "indexmap")]
            ConvertImpl {
                ty: syn::parse_quote! { ::indexmap::IndexMap<&'static str, T> },
                expr: syn::parse_quote! {
                    [#((#names, <T as From<#children_tys>>::from(#children_exprs))),*].into()
                },
                generics: syn::parse_quote! {
                    <T: #(From<#children_tys>)+*>
                },
            },
            #[cfg(feature = "indexmap")]
            ConvertImpl {
                ty: syn::parse_quote! { ::indexmap::IndexMap<String, T> },
                expr: syn::parse_quote! {
                    [#((#names.to_string(), <T as From<#children_tys>>::from(#children_exprs))),*].into()
                },
                generics: syn::parse_quote! {
                    <T: #(From<#children_tys>)+*>
                },
            },
        ].into()
    }

    fn children(&self) -> Vec<(Key, &Value)> {
        self.iter()
            .map(|(name, value)| (Key::Name(name.clone()), value))
            .collect()
    }
}
