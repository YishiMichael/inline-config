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
        let ConfigReprModule { item_mod, ty, expr } =
            ConfigReprModule::from_value(ident, ident, &syn::parse_quote! { #ident }, value);

        item_mod.to_tokens(tokens);
        let static_item: syn::ItemStatic = syn::parse_quote! {
            #(#attrs)*
            #vis #static_token #ident: #ty #eq_token #expr #semi_token
        };
        static_item.to_tokens(tokens);
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
                    Format::from_specifier(&specifier.to_string())
                        .unwrap_or_else(|| proc_macro_error::abort!(specifier, "unknown specifier"))
                }
                [_, attribute, ..] => {
                    proc_macro_error::abort!(attribute, "multiple format specifier attributes")
                }
            };
            Ok(format
                .parse(&text_lit.value())
                .unwrap_or_else(|e| proc_macro_error::abort!(expr, e)))
        }

        syn::Expr::Macro(syn::ExprMacro { attrs, mac }) => {
            let path_lit: syn::LitStr = syn::parse2(mac.tokens.clone())?;
            let path = match mac.path.require_ident()?.to_string().as_str() {
                "include_config" => std::path::PathBuf::from(path_lit.value()),
                "include_config_env" => std::path::PathBuf::from(
                    resolve_env(&path_lit.value())
                        .unwrap_or_else(|e| proc_macro_error::abort!(path_lit, e)),
                ),
                _ => proc_macro_error::abort!(
                    mac.path,
                    "expected `include_config` or `include_config_env`"
                ),
            };

            // Resolve the relative path at the directory containing the call site file.
            let path = if path.is_absolute() {
                path
            } else {
                // Rust analyzer hasn't implemented `Span::file()`.
                // https://github.com/rust-lang/rust-analyzer/issues/15950
                std::path::PathBuf::from(proc_macro2::Span::call_site().file())
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
                    Format::from_specifier(&specifier.to_string())
                        .unwrap_or_else(|| proc_macro_error::abort!(specifier, "unknown specifier"))
                }
                [_, attribute, ..] => {
                    proc_macro_error::abort!(attribute, "multiple format specifier attributes")
                }
            };
            let text = std::fs::read_to_string(path)
                .unwrap_or_else(|e| proc_macro_error::abort!(path_lit, e));
            Ok(format
                .parse(&text)
                .unwrap_or_else(|e| proc_macro_error::abort!(expr, e)))
        }

        syn::Expr::Binary(binary) => {
            Ok(value_from_expr(binary.left.as_ref())? + value_from_expr(binary.right.as_ref())?)
        }

        expr => proc_macro_error::abort!(expr, "expected string literal or macro invocation"),
    }
}

// Resolve `$ENV_VAR` in a given path.
// Inspired from `include_dir::resolve_env`.
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

struct ConfigReprModule {
    item_mod: syn::ItemMod,
    ty: syn::Type,
    expr: syn::Expr,
}

impl ConfigReprModule {
    fn from_value(
        ident: &syn::Ident,
        mod_ident: &syn::Ident,
        mod_path: &syn::Path,
        value: &Value,
    ) -> Self {
        match value {
            Value::Nil => Self::from_primitive(
                ident,
                mod_ident,
                syn::parse_quote! { () },
                syn::parse_quote! { () },
            ),
            Value::Boolean(value) => Self::from_primitive(
                ident,
                mod_ident,
                syn::parse_quote! { bool },
                syn::parse_quote! { #value },
            ),
            Value::Integer(value) => Self::from_primitive(
                ident,
                mod_ident,
                syn::parse_quote! { i64 },
                syn::parse_quote! { #value },
            ),
            Value::Float(value) => Self::from_primitive(
                ident,
                mod_ident,
                syn::parse_quote! { f64 },
                syn::parse_quote! { #value },
            ),
            Value::String(value) => Self::from_primitive(
                ident,
                mod_ident,
                syn::parse_quote! { &'static str },
                syn::parse_quote! { #value },
            ),
            Value::Array(value) => Self::from_array(ident, mod_ident, mod_path, value.iter()),
            Value::Table(value) => Self::from_table(ident, mod_ident, mod_path, value.iter()),
        }
    }

    fn from_primitive(
        ident: &syn::Ident,
        mod_ident: &syn::Ident,
        ty: syn::Type,
        expr: syn::Expr,
    ) -> Self {
        let item_mod = syn::parse_quote! {
            #[allow(non_snake_case)]
            pub mod #mod_ident {
                #[allow(non_camel_case_types)]
                pub type #ident = #ty;
            }
        };
        let ty = syn::parse_quote! {
            #mod_ident::#ident
        };
        Self { item_mod, ty, expr }
    }

    fn from_array<'v>(
        ident: &syn::Ident,
        mod_ident: &syn::Ident,
        mod_path: &syn::Path,
        value: impl Iterator<Item = &'v Value>,
    ) -> Self {
        #[allow(clippy::type_complexity)]
        let ((item_mods, (tys, exprs)), (key_tys, members)): (
            (Vec<_>, (Vec<_>, Vec<_>)),
            (Vec<_>, Vec<_>),
        ) = value
            .enumerate()
            .map(|(index, value)| {
                let mod_ident = quote::format_ident!("_{index}");
                let ident = quote::format_ident!("{ident}_{index}");
                let field_module = Self::from_value(
                    &ident,
                    &mod_ident,
                    &syn::parse_quote! { #mod_path::#mod_ident },
                    value,
                );
                (
                    (field_module.item_mod, (field_module.ty, field_module.expr)),
                    (Key::index_ty(index), syn::Member::from(index)),
                )
            })
            .unzip();
        let lifetime = syn::Lifetime::new("'__inline_config__r", proc_macro2::Span::call_site());
        let generic = syn::Ident::new("__inline_config__T", proc_macro2::Span::call_site());
        let convert_into_impls: [syn::Item; _] = [syn::parse_quote! {
            impl<#lifetime, #generic>
                ::inline_config::__private::ConvertInto<#lifetime, Vec<#generic>> for #ident
            where
                #(#tys: ::inline_config::__private::ConvertInto<#lifetime, #generic>,)*
            {
                fn convert_into(&#lifetime self) -> Vec<#generic> {
                    [
                        #(
                            <#tys as ::inline_config::__private::ConvertInto<#lifetime, #generic>>::convert_into(&self.#members),
                        )*
                    ].into()
                }
            }
        }];
        let item_mod = syn::parse_quote! {
            #[allow(non_snake_case)]
            pub mod #mod_ident {
                #(#item_mods)*

                #[allow(non_camel_case_types)]
                pub struct #ident(#(pub #tys),*);

                #(
                    impl ::inline_config::__private::AccessKey<#key_tys> for #ident {
                        type Repr = #tys;

                        fn access_key(&self) -> &Self::Repr {
                            &self.#members
                        }
                    }
                )*

                #(#convert_into_impls)*

                impl ::inline_config::__private::NonNilRepr for #ident {}
            }
        };
        let ty = syn::parse_quote! {
            #mod_ident::#ident
        };
        let expr = syn::parse_quote! {
            #mod_path::#ident(#(#exprs),*)
        };
        Self { item_mod, ty, expr }
    }

    fn from_table<'v>(
        ident: &syn::Ident,
        mod_ident: &syn::Ident,
        mod_path: &syn::Path,
        value: impl Iterator<Item = (&'v String, &'v Value)>,
    ) -> Self {
        #[allow(clippy::type_complexity)]
        let ((item_mods, (tys, exprs)), (names, (key_tys, members))): (
            (Vec<_>, (Vec<_>, Vec<_>)),
            (Vec<_>, (Vec<_>, Vec<_>)),
        ) = value
            .enumerate()
            .map(|(index, (name, value))| {
                let mod_ident = syn::parse_str::<syn::Ident>(name)
                    .ok()
                    .or_else(|| syn::parse_str::<syn::Ident>(&format!("r#{name}")).ok())
                    .filter(|_| {
                        !(name.starts_with('_') && name.chars().skip(1).all(|c| c.is_ascii_digit()))
                    })
                    .unwrap_or_else(|| quote::format_ident!("_{index}"));
                let ident = quote::format_ident!("{ident}_{index}");
                let field_module = Self::from_value(
                    &ident,
                    &mod_ident,
                    &syn::parse_quote! { #mod_path::#mod_ident },
                    value,
                );
                (
                    (field_module.item_mod, (field_module.ty, field_module.expr)),
                    (name, (Key::name_ty(name), syn::Member::from(mod_ident))),
                )
            })
            .unzip();
        let lifetime = syn::Lifetime::new("'__inline_config__r", proc_macro2::Span::call_site());
        let generic = syn::Ident::new("__inline_config__T", proc_macro2::Span::call_site());
        let convert_into_impls: [syn::Item; _] = [
            syn::parse_quote! {
                impl<#lifetime, #generic>
                    ::inline_config::__private::ConvertInto<#lifetime, ::std::collections::BTreeMap<&#lifetime str, #generic>> for #ident
                where
                    #(#tys: ::inline_config::__private::ConvertInto<#lifetime, #generic>,)*
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
                    #(#tys: ::inline_config::__private::ConvertInto<#lifetime, #generic>,)*
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
                    #(#tys: ::inline_config::__private::ConvertInto<#lifetime, #generic>,)*
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
                    #(#tys: ::inline_config::__private::ConvertInto<#lifetime, #generic>,)*
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
        ];
        let item_mod = syn::parse_quote! {
            #[allow(non_snake_case)]
            pub mod #mod_ident {
                #(#item_mods)*

                #[allow(non_camel_case_types)]
                pub struct #ident {
                    #(pub #members: #tys,)*
                }

                #(
                    impl ::inline_config::__private::AccessKey<#key_tys> for #ident {
                        type Repr = #tys;

                        fn access_key(&self) -> &Self::Repr {
                            &self.#members
                        }
                    }
                )*

                #(#convert_into_impls)*

                impl ::inline_config::__private::NonNilRepr for #ident {}
            }
        };
        let ty = syn::parse_quote! {
            #mod_ident::#ident
        };
        let expr = syn::parse_quote! {
            #mod_path::#ident {
                #(#members: #exprs,)*
            }
        };
        Self { item_mod, ty, expr }
    }
}
