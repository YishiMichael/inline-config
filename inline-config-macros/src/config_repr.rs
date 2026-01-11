use crate::parse::Format;
use crate::path::Key;
use crate::value::Value;

pub struct ConfigItems {
    items: Vec<ConfigItem>,
}

struct ConfigItem {
    attrs: Vec<syn::Attribute>,
    vis: syn::Visibility,
    mutability: syn::StaticMutability,
    ident: syn::Ident,
    ty: syn::Ident,
    value: Value,
}

impl syn::parse::Parse for ConfigItems {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut items = vec![];
        while !input.is_empty() {
            items.push(ConfigItem::from_item_static(input.parse()?)?);
        }
        Ok(Self { items })
    }
}

impl quote::ToTokens for ConfigItems {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.items
            .iter()
            .fold(indexmap::IndexMap::new(), |mut groups, config_item| {
                groups
                    .entry(config_item.ty.clone())
                    .or_insert_with(Vec::new)
                    .push(config_item);
                groups
            })
            .iter()
            .for_each(|(ty, config_items)| {
                let ((variants, variant_tys), (item_statics, item_mods)): (
                (Vec<_>, Vec<_>),
                (Vec<_>, Vec<_>),
            ) = config_items
                .iter()
                .map(
                    |ConfigItem {
                         attrs,
                         vis,
                         mutability,
                         ident,
                         ty: _,
                         value,
                     }| {
                        let mod_ident = quote::format_ident!(
                            "__{}",
                            convert_case::ccase!(upper_snake -> snake, ident.to_string())
                        );
                        let variant = quote::format_ident!(
                            "{}",
                            convert_case::ccase!(upper_snake -> upper_camel, ident.to_string())
                        );
                        let variant_ty: syn::Type = syn::parse_quote! {
                            #mod_ident::Type
                        };
                        let item_static: syn::ItemStatic = syn::parse_quote! {
                            #(#attrs)*
                            #vis #mutability static #ident: #ty = #ty::#variant(#mod_ident::EXPR);
                        };
                        let item_mod = ConfigReprMod::from_value(value).item_mod(&mod_ident);
                        ((variant, variant_ty), (item_static, item_mod))
                    },
                )
                .unzip();
                let item_enum: syn::ItemEnum = syn::parse_quote! {
                    #[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
                    pub enum #ty {
                        #(#variants(#variant_tys),)*
                    }
                };
                let get_impl: syn::ItemImpl = syn::parse_quote! {
                    impl<P, T> ::inline_config::Get<P, T> for #ty
                    where
                        #(
                            #variant_tys:
                                ::inline_config::__private::AccessPath<P>,
                            <
                                #variant_tys
                                    as ::inline_config::__private::AccessPath<P>
                            >::Repr:
                                ::inline_config::__private::ConvertRepr<T>,
                        )*
                    {
                        fn get(&'static self, _path: P) -> T {
                            match self {
                                #(
                                    Self::#variants(value) => <
                                        <
                                            #variant_tys
                                                as ::inline_config::__private::AccessPath<P>
                                        >::Repr
                                            as ::inline_config::__private::ConvertRepr<T>
                                    >::convert_repr(
                                        <
                                            #variant_tys
                                                as ::inline_config::__private::AccessPath<P>
                                        >::access_path(
                                            value,
                                        ),
                                    ),
                                )*
                            }
                        }
                    }
                };
                item_enum.to_tokens(tokens);
                get_impl.to_tokens(tokens);
                item_statics
                    .iter()
                    .for_each(|item_static| item_static.to_tokens(tokens));
                item_mods
                    .iter()
                    .for_each(|item_mod| item_mod.to_tokens(tokens));
            });
    }
}

impl ConfigItem {
    fn from_item_static(item_static: syn::ItemStatic) -> syn::Result<Self> {
        let ty = match &*item_static.ty {
            syn::Type::Path(syn::TypePath { qself: None, path }) => path.require_ident()?.clone(),
            syn::Type::Infer(_) => quote::format_ident!(
                "__{}",
                convert_case::ccase!(upper_snake -> upper_camel, item_static.ident.to_string())
            ),
            _ => Err(syn::Error::new_spanned(
                item_static.ty,
                "config type must be an identifier",
            ))?,
        };
        Ok(Self {
            attrs: item_static.attrs,
            vis: item_static.vis,
            mutability: item_static.mutability,
            ident: item_static.ident,
            ty,
            value: Self::value_from_expr(&item_static.expr)?,
        })
    }

    fn value_from_expr(expr: &syn::Expr) -> syn::Result<Value> {
        match expr {
            syn::Expr::Lit(syn::ExprLit {
                attrs,
                lit: syn::Lit::Str(text_lit),
            }) => {
                let format = match attrs.as_slice() {
                    [] => Err(syn::Error::new_spanned(
                        text_lit,
                        "must specify format for literal config",
                    )),
                    [attribute] => {
                        let specifier = attribute.meta.require_path_only()?.require_ident()?;
                        Format::from_specifier(&specifier.to_string()).ok_or(
                            syn::Error::new_spanned(specifier, "unknown format specifier"),
                        )
                    }
                    [_, attribute, ..] => Err(syn::Error::new_spanned(
                        attribute,
                        "multiple format specifier attributes",
                    )),
                }?;
                format
                    .parse(&text_lit.value())
                    .map_err(|e| syn::Error::new_spanned(expr, e))
            }

            syn::Expr::Macro(syn::ExprMacro { attrs, mac }) => {
                let path_lit: syn::LitStr = syn::parse2(mac.tokens.clone())?;
                let path = match mac.path.require_ident()?.to_string().as_str() {
                    "include_config" => Ok(std::path::PathBuf::from(path_lit.value())),
                    "include_config_env" => Self::resolve_env(&path_lit.value())
                        .map(std::path::PathBuf::from)
                        .map_err(|e| syn::Error::new_spanned(&path_lit, e)),
                    _ => Err(syn::Error::new_spanned(
                        &mac.path,
                        "expected `include_config` or `include_config_env`",
                    )),
                }?;

                // Resolve the path relative to the current file.
                let path = if path.is_absolute() {
                    path
                } else {
                    // Rust analyzer hasn't implemented `Span::file()`.
                    // https://github.com/rust-lang/rust-analyzer/issues/15950
                    std::path::PathBuf::from(proc_macro2::Span::call_site().file())
                        .parent()
                        .ok_or(syn::Error::new_spanned(
                            &path_lit,
                            "cannot retrieve parent dir",
                        ))?
                        .join(path)
                };

                let format = match attrs.as_slice() {
                    [] => Format::from_extension(
                        path.extension()
                            .ok_or(syn::Error::new_spanned(&path_lit, "unknown extension"))?
                            .to_str()
                            .ok_or(syn::Error::new_spanned(&path_lit, "unknown extension"))?,
                    )
                    .ok_or({
                        syn::Error::new_spanned(&path_lit, "cannot select format from extension")
                    }),
                    [attribute] => {
                        let specifier = attribute.meta.require_path_only()?.require_ident()?;
                        Format::from_specifier(&specifier.to_string()).ok_or(
                            syn::Error::new_spanned(specifier, "unknown format specifier"),
                        )
                    }
                    [_, attribute, ..] => Err(syn::Error::new_spanned(
                        attribute,
                        "multiple format specifier attributes",
                    )),
                }?;
                let text = std::fs::read_to_string(path)
                    .map_err(|e| syn::Error::new_spanned(&path_lit, e))?;
                format
                    .parse(&text)
                    .map_err(|e| syn::Error::new_spanned(expr, e))
            }

            syn::Expr::Binary(binary) => {
                Ok(Self::value_from_expr(&binary.left)? + Self::value_from_expr(&binary.right)?)
            }

            _ => Err(syn::Error::new_spanned(
                expr,
                "expected string literal or macro invocation",
            )),
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
}

struct ConfigReprMod {
    ty: syn::Type,
    expr: syn::Expr,
    item_struct: Option<syn::ItemStruct>,
    field_mods: Vec<syn::ItemMod>,
    access_impls: Vec<syn::ItemImpl>,
    convert_impls: Vec<syn::ItemImpl>,
}

impl ConfigReprMod {
    fn from_value(value: &Value) -> Self {
        match value {
            Value::Nil => Self::from_primitive(
                syn::parse_quote! { ::inline_config::__private::ReprNil },
                syn::parse_quote! { ::inline_config::__private::ReprNil },
            ),
            Value::Boolean(value) => Self::from_primitive(
                syn::parse_quote! { ::inline_config::__private::ReprBoolean },
                syn::parse_quote! { ::inline_config::__private::ReprBoolean(#value) },
            ),
            Value::PosInt(value) => Self::from_primitive(
                syn::parse_quote! { ::inline_config::__private::ReprPosInt },
                syn::parse_quote! { ::inline_config::__private::ReprPosInt(#value) },
            ),
            Value::NegInt(value) => Self::from_primitive(
                syn::parse_quote! { ::inline_config::__private::ReprNegInt },
                syn::parse_quote! { ::inline_config::__private::ReprNegInt(#value) },
            ),
            Value::Float(value) => Self::from_primitive(
                syn::parse_quote! { ::inline_config::__private::ReprFloat },
                syn::parse_quote! { ::inline_config::__private::ReprFloat(::inline_config::__private::OrderedFloat(#value)) },
            ),
            Value::String(value) => Self::from_primitive(
                syn::parse_quote! { ::inline_config::__private::ReprString },
                syn::parse_quote! { ::inline_config::__private::ReprString(#value) },
            ),
            Value::Array(value) => Self::from_container(
                value.iter().enumerate(),
                Key::index_ty,
                Self::array_containers,
            ),
            Value::Table(value) => Self::from_container(
                value.iter().map(|(name, value)| (name.as_ref(), value)),
                Key::name_ty,
                Self::table_containers,
            ),
        }
    }

    fn from_primitive(ty: syn::Type, expr: syn::Expr) -> Self {
        Self {
            ty,
            expr,
            item_struct: None,
            field_mods: Vec::new(),
            access_impls: Vec::new(),
            convert_impls: Vec::new(),
        }
    }

    #[allow(clippy::type_complexity)]
    fn from_container<'v, T: Copy>(
        items: impl Iterator<Item = (T, &'v Value)>,
        key_ty_fn: fn(T) -> syn::Type,
        convert_items_fn: fn(&syn::Ident, Vec<T>, Vec<syn::Expr>) -> Vec<(syn::Type, syn::Expr)>,
    ) -> Self {
        let (field_mods, (idents, tags)): (Vec<_>, (Vec<_>, Vec<_>)) = items
            .enumerate()
            .map(|(index, (tag, value))| {
                let ident = quote::format_ident!("_{index}");
                (Self::from_value(value).item_mod(&ident), (ident, tag))
            })
            .unzip();
        Self {
            ty: syn::parse_quote! {
                ::inline_config::__private::ReprContainer<Struct>
            },
            expr: syn::parse_quote! {
                ::inline_config::__private::ReprContainer(Struct {
                    #(#idents: &#idents::EXPR,)*
                })
            },
            item_struct: Some(syn::parse_quote! {
                #[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
                pub struct Struct {
                    #(pub #idents: &'static #idents::Type,)*
                }
            }),
            field_mods,
            access_impls: idents
                .iter()
                .zip(tags.iter())
                .map(|(ident, tag)| {
                    let key_ty = key_ty_fn(*tag);
                    syn::parse_quote! {
                        impl ::inline_config::__private::Access<#key_ty> for Struct {
                            type Repr = #ident::Type;

                            fn access(&self) -> &Self::Repr {
                                &self.#ident
                            }
                        }
                    }
                })
                .collect(),
            convert_impls: {
                let generic = syn::Ident::new("__inline_config__T", proc_macro2::Span::call_site());
                let convert_items = convert_items_fn(&generic, tags, idents.iter().map(|ident| {
                    syn::parse_quote! {
                        <
                            #ident::Type as ::inline_config::__private::ConvertRepr<#generic>
                        >::convert_repr(&self.#ident)
                    }
                }).collect());
                convert_items.iter()
                    .map(|(ty, expr)| {
                        syn::parse_quote! {
                            impl<#generic> ::inline_config::__private::Convert<#ty> for Struct
                            where
                                #(#idents::Type: ::inline_config::__private::ConvertRepr<#generic>,)*
                            {
                                fn convert(&self) -> #ty {
                                    #expr
                                }
                            }
                        }
                    })
                    .collect()
            },
        }
    }

    fn item_mod(&self, mod_ident: &syn::Ident) -> syn::ItemMod {
        let Self {
            ty,
            expr,
            item_struct,
            field_mods,
            access_impls,
            convert_impls,
        } = self;
        syn::parse_quote! {
            pub mod #mod_ident {
                pub type Type = #ty;
                pub static EXPR: Type = #expr;
                #item_struct
                #(#field_mods)*
                #(#access_impls)*
                #(#convert_impls)*
            }
        }
    }

    fn array_containers(
        generic: &syn::Ident,
        _tags: Vec<usize>,
        exprs: Vec<syn::Expr>,
    ) -> Vec<(syn::Type, syn::Expr)> {
        [(
            syn::parse_quote! { Vec<#generic> },
            syn::parse_quote! { [#(#exprs),*].into() },
        )]
        .into()
    }

    fn table_containers(
        generic: &syn::Ident,
        tags: Vec<&str>,
        exprs: Vec<syn::Expr>,
    ) -> Vec<(syn::Type, syn::Expr)> {
        [
            (
                syn::parse_quote! { ::std::collections::BTreeMap<&'static str, #generic> },
                syn::parse_quote! { [#((#tags, #exprs)),*].into() },
            ),
            (
                syn::parse_quote! { ::std::collections::BTreeMap<String, #generic> },
                syn::parse_quote! { [#((#tags.to_string(), #exprs)),*].into() },
            ),
            #[cfg(feature = "indexmap")]
            (
                syn::parse_quote! { ::inline_config::__private::IndexMap<&'static str, #generic> },
                syn::parse_quote! { [#((#tags, #exprs)),*].into() },
            ),
            #[cfg(feature = "indexmap")]
            (
                syn::parse_quote! { ::inline_config::__private::IndexMap<String, #generic> },
                syn::parse_quote! { [#((#tags.to_string(), #exprs)),*].into() },
            ),
        ]
        .into()
    }
}
