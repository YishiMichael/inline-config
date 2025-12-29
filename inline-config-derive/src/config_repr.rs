use crate::parse::Format;
use crate::path::Key;
use crate::value::Value;

pub struct ConfigItems {
    items: Vec<ConfigItem>,
}

struct ConfigItem {
    attrs: Vec<syn::Attribute>,
    vis: syn::Visibility,
    static_token: syn::Token![static],
    ident: syn::Ident,
    eq_token: syn::Token![=],
    value: Value,
    semi_token: syn::Token![;],
}

impl syn::parse::Parse for ConfigItems {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut items = vec![];
        while !input.is_empty() {
            items.push(input.parse()?);
        }
        Ok(Self { items })
    }
}

impl quote::ToTokens for ConfigItems {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.items
            .iter()
            .for_each(|config_item| config_item.to_tokens(tokens));
    }
}

impl syn::parse::Parse for ConfigItem {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            attrs: input.call(syn::Attribute::parse_outer)?,
            vis: input.parse()?,
            static_token: input.parse()?,
            ident: input.parse()?,
            eq_token: input.parse()?,
            value: value_from_expr(&input.parse()?)?,
            semi_token: input.parse()?,
        })
    }
}

impl quote::ToTokens for ConfigItem {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let ConfigItem {
            attrs,
            vis,
            static_token,
            ident,
            eq_token,
            value,
            semi_token,
        } = self;
        let ConfigReprTokens {
            ty,
            expr,
            struct_items,
            access_key_impls,
            convert_into_impls,
            non_nil_repr_impls,
        } = ConfigReprTokens::from_value(&value, &quote::format_ident!("__{ident}_"), &vis);

        let static_item: syn::ItemStatic = syn::parse_quote! {
            #(#attrs)*
            #vis #static_token #ident: #ty #eq_token #expr #semi_token
        };
        static_item.to_tokens(tokens);
        struct_items
            .iter()
            .for_each(|struct_item| struct_item.to_tokens(tokens));
        access_key_impls
            .iter()
            .for_each(|access_key_impl| access_key_impl.to_tokens(tokens));
        convert_into_impls
            .iter()
            .for_each(|convert_into_impl| convert_into_impl.to_tokens(tokens));
        non_nil_repr_impls
            .iter()
            .for_each(|non_nil_repr_impl| non_nil_repr_impl.to_tokens(tokens));
    }
}

fn value_from_expr(expr: &syn::Expr) -> syn::Result<Value> {
    match expr {
        syn::Expr::Lit(syn::ExprLit {
            attrs,
            lit: syn::Lit::Str(text_lit),
        }) => {
            let format = match attrs.as_slice() {
                [] => proc_macro_error::abort!(text_lit, "must specify format for literal config"),
                [attribute] => {
                    let specifier = attribute.meta.require_path_only()?.require_ident()?;
                    Format::from_specifier(specifier.to_string().as_str())
                        .unwrap_or_else(|| proc_macro_error::abort!(specifier, "unknown specifier"))
                }
                [_, attribute, ..] => {
                    proc_macro_error::abort!(attribute, "multiple format specifier attributes")
                }
            };
            Ok(format
                .parse(text_lit.value().as_str())
                .unwrap_or_else(|e| proc_macro_error::abort!(expr, e)))
        }

        syn::Expr::Macro(syn::ExprMacro { attrs, mac }) if mac.path.is_ident("include_config") => {
            let path_lit: syn::LitStr = syn::parse2(mac.tokens.clone())?;
            let path = resolve_path(path_lit.value().as_str());

            // Resolve the relative path at the directory containing the call site file.
            let path = if path.is_absolute() {
                path
            } else {
                // Rust analyzer hasn't implemented `Span::file()`.
                // https://github.com/rust-lang/rust-analyzer/issues/15950
                std::path::Path::new(proc_macro2::Span::call_site().file().as_str())
                    .parent()
                    .unwrap_or_else(|| {
                        proc_macro_error::abort!(path_lit, "cannot retrieve parent dir")
                    })
                    .join(path)
            };

            let format = match attrs.as_slice() {
                [] => Format::from_extension(
                    path.extension()
                        .unwrap_or_else(|| proc_macro_error::abort!(path_lit, "unknown extension"))
                        .to_str()
                        .unwrap_or_else(|| proc_macro_error::abort!(path_lit, "unknown extension")),
                )
                .unwrap_or_else(|| {
                    proc_macro_error::abort!(path_lit, "cannot select format from extension")
                }),
                [attribute] => {
                    let specifier = attribute.meta.require_path_only()?.require_ident()?;
                    Format::from_specifier(specifier.to_string().as_str())
                        .unwrap_or_else(|| proc_macro_error::abort!(specifier, "unknown specifier"))
                }
                [_, attribute, ..] => {
                    proc_macro_error::abort!(attribute, "multiple format specifier attributes")
                }
            };
            let text = std::fs::read_to_string(path)
                .unwrap_or_else(|e| proc_macro_error::abort!(path_lit, e));
            Ok(format
                .parse(text.as_str())
                .unwrap_or_else(|e| proc_macro_error::abort!(expr, e)))
        }

        syn::Expr::Binary(binary) => {
            Ok(value_from_expr(binary.left.as_ref())? + value_from_expr(binary.right.as_ref())?)
        }

        expr => proc_macro_error::abort!(
            expr,
            r#"expected `#[<format>] "<content>"` or `include_config!("<path>")`"#
        ),
    }
}

// Replace `${ENV_VAR}` in paths.
// Inspired by crate include_dir.
fn resolve_path(s: &str) -> std::path::PathBuf {
    let mut path = std::path::PathBuf::new();
    path.push(s);
    path
}

struct ConfigReprTokens {
    ty: syn::Type,
    expr: syn::Expr,
    struct_items: Vec<syn::ItemStruct>,
    access_key_impls: Vec<syn::ItemImpl>,
    convert_into_impls: Vec<syn::ItemImpl>,
    non_nil_repr_impls: Vec<syn::ItemImpl>,
}

impl ConfigReprTokens {
    fn from_value(value: &Value, ident: &syn::Ident, vis: &syn::Visibility) -> Self {
        match value {
            Value::Nil => Self::primitive(syn::parse_quote! { () }, syn::parse_quote! { () }),
            Value::Boolean(value) => {
                Self::primitive(syn::parse_quote! { bool }, syn::parse_quote! { #value })
            }
            Value::Integer(value) => {
                Self::primitive(syn::parse_quote! { i64 }, syn::parse_quote! { #value })
            }
            Value::Float(value) => {
                Self::primitive(syn::parse_quote! { f64 }, syn::parse_quote! { #value })
            }
            Value::String(value) => Self::primitive(
                syn::parse_quote! { &'static str },
                syn::parse_quote! { #value },
            ),
            Value::Array(value) => {
                let (slots, values): (Vec<_>, Vec<_>) = value
                    .iter()
                    .enumerate()
                    .map(|(index, value)| (ArraySlot { index }, value))
                    .unzip();
                Self::dispatch(slots, values.as_slice(), ident, vis)
            }
            Value::Table(value) => {
                let (slots, values): (Vec<_>, Vec<_>) = value
                    .iter()
                    .enumerate()
                    .map(|(index, (name, value))| {
                        (
                            TableSlot {
                                name,
                                ident: syn::parse_str::<syn::Ident>(name)
                                    .ok()
                                    .filter(|_| !name.chars().all(|c| matches!(c, '0'..'9' | '_')))
                                    .unwrap_or_else(|| quote::format_ident!("_{index}")),
                            },
                            value,
                        )
                    })
                    .unzip();
                Self::dispatch(slots, values.as_slice(), ident, vis)
            }
        }
    }

    fn primitive(ty: syn::Type, expr: syn::Expr) -> Self {
        Self {
            ty,
            expr,
            struct_items: [].into(),
            access_key_impls: [].into(),
            convert_into_impls: [].into(),
            non_nil_repr_impls: [].into(),
        }
    }

    fn dispatch<S>(
        config_repr_structure: S,
        values: &[&Value],
        ident: &syn::Ident,
        vis: &syn::Visibility,
    ) -> Self
    where
        S: ConfigReprStructure,
    {
        let mut tys = Vec::new();
        let mut exprs = Vec::new();
        let mut struct_items = Vec::new();
        let mut access_key_impls = Vec::new();
        let mut convert_into_impls = Vec::new();
        let mut non_nil_repr_impls = Vec::new();
        for (index, value) in values.iter().enumerate() {
            let Self {
                ty: field_ty,
                expr: field_expr,
                struct_items: field_struct_items,
                access_key_impls: field_access_key_impls,
                convert_into_impls: field_convert_into_impls,
                non_nil_repr_impls: field_non_nil_repr_impls,
            } = Self::from_value(value, &quote::format_ident!("{ident}_{index}"), vis);
            tys.push(field_ty);
            exprs.push(field_expr);
            struct_items.extend(field_struct_items);
            access_key_impls.extend(field_access_key_impls);
            convert_into_impls.extend(field_convert_into_impls);
            non_nil_repr_impls.extend(field_non_nil_repr_impls);
        }
        struct_items.push(config_repr_structure.struct_item(ident, vis, tys.as_slice()));
        access_key_impls.extend(config_repr_structure.access_key_impls(ident, tys.as_slice()));
        convert_into_impls.extend(config_repr_structure.convert_into_impls(ident, tys.as_slice()));
        non_nil_repr_impls.push(syn::parse_quote! {
            impl ::inline_config::__private::NonNilRepr for #ident {}
        });
        Self {
            ty: syn::parse_quote! {
                #ident
            },
            expr: config_repr_structure.expr(ident, exprs.as_slice()),
            struct_items,
            access_key_impls,
            convert_into_impls,
            non_nil_repr_impls,
        }
    }
}

trait ConfigReprStructure {
    fn expr(&self, ident: &syn::Ident, exprs: &[syn::Expr]) -> syn::Expr;
    fn struct_item(
        &self,
        ident: &syn::Ident,
        vis: &syn::Visibility,
        tys: &[syn::Type],
    ) -> syn::ItemStruct;
    fn access_key_impls(&self, ident: &syn::Ident, tys: &[syn::Type]) -> Vec<syn::ItemImpl>;
    fn convert_into_impls(&self, ident: &syn::Ident, tys: &[syn::Type]) -> Vec<syn::ItemImpl>;
}

struct ArraySlot {
    index: usize,
}

struct TableSlot<'s> {
    name: &'s str,
    ident: syn::Ident,
}

impl ConfigReprStructure for Vec<ArraySlot> {
    fn expr(&self, ident: &syn::Ident, exprs: &[syn::Expr]) -> syn::Expr {
        syn::parse_quote! {
            #ident(
                #(#exprs,)*
            )
        }
    }

    fn struct_item(
        &self,
        ident: &syn::Ident,
        vis: &syn::Visibility,
        tys: &[syn::Type],
    ) -> syn::ItemStruct {
        syn::parse_quote! {
            #vis struct #ident(
                #(#vis #tys,)*
            );
        }
    }

    fn access_key_impls(&self, ident: &syn::Ident, tys: &[syn::Type]) -> Vec<syn::ItemImpl> {
        self.iter()
            .zip(tys)
            .map(|(slot, ty)| {
                let key_ty = Key::index_ty(slot.index);
                let member = syn::Index::from(slot.index);
                syn::parse_quote! {
                    impl ::inline_config::__private::AccessKey<#key_ty> for #ident {
                        type Repr = #ty;

                        fn access_key(&self) -> &Self::Repr {
                            &self.#member
                        }
                    }
                }
            })
            .collect()
    }

    fn convert_into_impls(&self, ident: &syn::Ident, tys: &[syn::Type]) -> Vec<syn::ItemImpl> {
        let members = self.iter().map(|slot| syn::Index::from(slot.index));
        let lifetime = syn::Lifetime::new("'__inline_config__r", proc_macro2::Span::call_site());
        let generic = syn::Ident::new("__inline_config__T", proc_macro2::Span::call_site());
        [
            syn::parse_quote! {
                impl<#lifetime, #generic>
                    ::inline_config::__private::ConvertInto<#lifetime, Vec<#generic>> for #ident
                where
                    #(#tys: ::inline_config::__private::ConvertInto<#lifetime, #generic>),*
                {
                    fn convert_into(&#lifetime self) -> Vec<#generic> {
                        [
                            #(
                                <#tys as ::inline_config::__private::ConvertInto<#lifetime, #generic>>::convert_into(&self.#members),
                            )*
                        ].into()
                    }
                }
            },
        ].into()
    }
}

impl ConfigReprStructure for Vec<TableSlot<'_>> {
    fn expr(&self, ident: &syn::Ident, exprs: &[syn::Expr]) -> syn::Expr {
        let members = self.iter().map(|slot| &slot.ident);
        syn::parse_quote! {
            #ident {
                #(#members: #exprs,)*
            }
        }
    }

    fn struct_item(
        &self,
        ident: &syn::Ident,
        vis: &syn::Visibility,
        tys: &[syn::Type],
    ) -> syn::ItemStruct {
        let members = self.iter().map(|slot| &slot.ident);
        syn::parse_quote! {
            #vis struct #ident {
                #(#vis #members: #tys,)*
            }
        }
    }

    fn access_key_impls(&self, ident: &syn::Ident, tys: &[syn::Type]) -> Vec<syn::ItemImpl> {
        self.iter()
            .zip(tys)
            .map(|(slot, ty)| {
                let key_ty = Key::name_ty(slot.name);
                let member = &slot.ident;
                syn::parse_quote! {
                    impl ::inline_config::__private::AccessKey<#key_ty> for #ident {
                        type Repr = #ty;

                        fn access_key(&self) -> &Self::Repr {
                            &self.#member
                        }
                    }
                }
            })
            .collect()
    }

    fn convert_into_impls(&self, ident: &syn::Ident, tys: &[syn::Type]) -> Vec<syn::ItemImpl> {
        let (names, members): (Vec<_>, Vec<_>) =
            self.iter().map(|slot| (slot.name, &slot.ident)).unzip();
        let lifetime = syn::Lifetime::new("'__inline_config__r", proc_macro2::Span::call_site());
        let generic = syn::Ident::new("__inline_config__T", proc_macro2::Span::call_site());
        [
            syn::parse_quote! {
                impl<#lifetime, #generic>
                    ::inline_config::__private::ConvertInto<#lifetime, ::std::collections::BTreeMap<&#lifetime str, #generic>> for #ident
                where
                    #(#tys: ::inline_config::__private::ConvertInto<#lifetime, #generic>),*
                {
                    fn convert_into(&#lifetime self) -> ::std::collections::BTreeMap<&#lifetime str, #generic> {
                        [
                            #(
                                (
                                    #names,
                                    <#tys as ::inline_config::__private::ConvertInto<#lifetime, #generic>>::convert_into(&self.#members),
                                ),
                            )*
                        ].into()
                    }
                }
            },

            syn::parse_quote! {
                impl<#lifetime, #generic>
                    ::inline_config::__private::ConvertInto<#lifetime, ::std::collections::BTreeMap<String, #generic>> for #ident
                where
                    #(#tys: ::inline_config::__private::ConvertInto<#lifetime, #generic>),*
                {
                    fn convert_into(&#lifetime self) -> ::std::collections::BTreeMap<String, #generic> {
                        [
                            #(
                                (
                                    #names.to_string(),
                                    <#tys as ::inline_config::__private::ConvertInto<#lifetime, #generic>>::convert_into(&self.#members),
                                ),
                            )*
                        ].into()
                    }
                }
            },

            #[cfg(feature = "indexmap")]
            syn::parse_quote! {
                impl<#lifetime, #generic>
                    ::inline_config::__private::ConvertInto<#lifetime, ::indexmap::IndexMap<&#lifetime str, #generic>> for #ident
                where
                    #(#tys: ::inline_config::__private::ConvertInto<#lifetime, #generic>),*
                {
                    fn convert_into(&#lifetime self) -> ::indexmap::IndexMap<&#lifetime str, #generic> {
                        [
                            #(
                                (
                                    #names,
                                    <#tys as ::inline_config::__private::ConvertInto<#lifetime, #generic>>::convert_into(&self.#members),
                                ),
                            )*
                        ].into()
                    }
                }
            },

            #[cfg(feature = "indexmap")]
            syn::parse_quote! {
                impl<#lifetime, #generic>
                    ::inline_config::__private::ConvertInto<#lifetime, ::indexmap::IndexMap<String, #generic>> for #ident
                where
                    #(#tys: ::inline_config::__private::ConvertInto<#lifetime, #generic>),*
                {
                    fn convert_into(&#lifetime self) -> ::indexmap::IndexMap<String, #generic> {
                        [
                            #(
                                (
                                    #names.to_string(),
                                    <#tys as ::inline_config::__private::ConvertInto<#lifetime, #generic>>::convert_into(&self.#members),
                                ),
                            )*
                        ].into()
                    }
                }
            },
        ].into()
    }
}
