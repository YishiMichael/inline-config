use crate::format::Format;
use crate::path::Key;
use crate::value::{Map, Value};
use darling::{FromDeriveInput, FromMeta};

#[derive(FromDeriveInput)]
#[darling(supports(struct_unit), attributes(config), forward_attrs)]
struct ConfigItem {
    ident: syn::Ident,
    format: Option<Format>,
    #[darling(multiple)]
    src: Vec<darling::util::SpannedValue<Source>>,
}

enum Source {
    Include(std::path::PathBuf),
    Lit(String),
}

impl Source {
    fn content(&self) -> std::io::Result<std::borrow::Cow<'_, str>> {
        match self {
            Self::Include(path) => {
                // Resolve the path relative to the current file.
                let path = if path.is_absolute() {
                    path.clone()
                } else {
                    // Rust analyzer hasn't implemented `Span::file()`.
                    // https://github.com/rust-lang/rust-analyzer/issues/15950
                    std::path::PathBuf::from(proc_macro2::Span::call_site().file())
                        .parent()
                        .ok_or(std::io::ErrorKind::AddrNotAvailable)?
                        .join(path)
                };
                Ok(std::borrow::Cow::Owned(std::fs::read_to_string(path)?))
            }
            Self::Lit(content) => Ok(std::borrow::Cow::Borrowed(content)),
        }
    }

    fn extension(&self) -> Option<&std::ffi::OsStr> {
        match self {
            Self::Include(path) => path.extension(),
            Self::Lit(_) => None,
        }
    }

    fn resolve_env(path: &str) -> Result<String, std::env::VarError> {
        let mut chars = path.chars().peekable();
        let mut resolved = String::new();
        while let Some(c) = chars.next() {
            if c != '$' {
                resolved.push(c);
                continue;
            }
            if chars.peek() == Some(&'$') {
                chars.next();
                resolved.push('$');
                continue;
            }
            let mut variable = String::new();
            while let Some(&c) = chars.peek() {
                if matches!(c, '0'..='9' | 'A'..='Z' | 'a'..='z' | '_') {
                    chars.next();
                    variable.push(c);
                } else {
                    break;
                }
            }
            resolved.push_str(&std::env::var(&variable)?);
        }
        Ok(resolved)
    }
}

impl FromMeta for Source {
    fn from_expr(expr: &syn::Expr) -> darling::Result<Self> {
        match expr {
            syn::Expr::Macro(syn::ExprMacro {
                mac: syn::Macro { path, tokens, .. },
                ..
            }) if path.is_ident("include") => Ok(Self::Include(std::path::PathBuf::from(
                syn::parse2::<syn::LitStr>(tokens.clone())?.value(),
            ))),
            syn::Expr::Macro(syn::ExprMacro {
                mac: syn::Macro { path, tokens, .. },
                ..
            }) if path.is_ident("include_env") => Ok(Self::Include(std::path::PathBuf::from(
                Self::resolve_env(&syn::parse2::<syn::LitStr>(tokens.clone())?.value())
                    .map_err(|e| syn::Error::new_spanned(expr, e))?,
            ))),
            syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(lit_str),
                ..
            }) => Ok(Self::Lit(lit_str.value())),
            syn::Expr::Lit(lit) => Self::from_value(&lit.lit),
            syn::Expr::Group(group) => Self::from_expr(&group.expr),
            _ => Err(darling::Error::unexpected_expr_type(expr)),
        }
        .map_err(|e| e.with_span(expr))
    }
}

pub fn config(item: syn::DeriveInput) -> syn::Result<syn::ItemConst> {
    let config_item: ConfigItem = ConfigItem::from_derive_input(&item)?;
    let format = config_item.format.map(Ok).unwrap_or_else(|| {
        let mut extensions = config_item.src.iter().filter_map(|source| {
            source
                .extension()
                .and_then(std::ffi::OsStr::to_str)
                .and_then(Format::from_extension)
        });
        let first = extensions
            .next()
            .ok_or(syn::Error::new_spanned(&item, "Missing format"))?;
        let other = extensions.find(|x| x != &first);
        if let Some(other) = other {
            Err(syn::Error::new_spanned(
                &item,
                format!("Multiple formats: {first:?}, {other:?}"),
            ))
        } else {
            Ok(first)
        }
    })?;
    let mut errors = darling::Error::accumulator();
    let value: Value = config_item
        .src
        .into_iter()
        .filter_map(|source| {
            errors.handle_in(|| {
                Ok(source
                    .content()
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
                    .and_then(|content| format.parse(content.as_ref()))
                    .map_err(|e| syn::Error::new(source.span(), e))?)
            })
        })
        .sum();
    errors.finish()?;
    Ok(value.to_item_const(&config_item.ident))
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
