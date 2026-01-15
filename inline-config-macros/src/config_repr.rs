use crate::format::Format;
use crate::path::Key;
use crate::value::Value;

pub struct ConfigBlock<F> {
    format: std::marker::PhantomData<F>,
    items: Vec<ConfigItem<F>>,
}

struct ConfigItem<F> {
    format: std::marker::PhantomData<F>,
    attrs: Vec<syn::Attribute>,
    vis: syn::Visibility,
    mutability: syn::StaticMutability,
    ident: syn::Ident,
    ty: syn::Ident,
    value: Value,
}

impl<F: Format> syn::parse::Parse for ConfigBlock<F> {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut items = vec![];
        while !input.is_empty() {
            items.push(ConfigItem::from_item_static(input.parse()?)?);
        }
        Ok(Self {
            format: std::marker::PhantomData,
            items,
        })
    }
}

impl<F: Format> quote::ToTokens for ConfigBlock<F> {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.items.iter().for_each(|config_item| {
            let (item_static, item_mod, item_struct, item_impl) = config_item.token_items();
            item_static.to_tokens(tokens);
            item_mod.to_tokens(tokens);
            item_struct.to_tokens(tokens);
            item_impl.to_tokens(tokens);
        });
    }
}

impl<F: Format> ConfigItem<F> {
    fn from_item_static(item_static: syn::ItemStatic) -> syn::Result<Self> {
        Ok(Self {
            format: std::marker::PhantomData,
            attrs: item_static.attrs,
            vis: item_static.vis,
            mutability: item_static.mutability,
            ident: item_static.ident,
            ty: Self::ident_from_ty(&item_static.ty)?,
            value: Self::value_from_expr(&item_static.expr)?,
        })
    }

    fn ident_from_ty(ty: &syn::Type) -> syn::Result<syn::Ident> {
        match ty {
            syn::Type::Path(syn::TypePath { qself: None, path }) => path.require_ident().cloned(),
            _ => Err(syn::Error::new_spanned(
                ty,
                "config type must be an identifier",
            )),
        }
    }

    fn value_from_expr(expr: &syn::Expr) -> syn::Result<Value> {
        match expr {
            syn::Expr::Lit(syn::ExprLit {
                attrs: _,
                lit: syn::Lit::Str(text_lit),
            }) => F::parse(&text_lit.value()).map_err(|e| syn::Error::new_spanned(expr, e)),

            syn::Expr::Macro(syn::ExprMacro { attrs: _, mac }) => {
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

                let text = std::fs::read_to_string(path)
                    .map_err(|e| syn::Error::new_spanned(&path_lit, e))?;
                F::parse(&text).map_err(|e| syn::Error::new_spanned(expr, e))
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

    fn token_items(
        &self,
    ) -> (
        syn::ItemStatic,
        syn::ItemMod,
        syn::ItemStruct,
        syn::ItemImpl,
    ) {
        let Self {
            format: _,
            attrs,
            vis,
            mutability,
            ident,
            ty,
            value,
        } = self;
        let mod_ident = quote::format_ident!("__{}", ident.to_string().to_lowercase());
        let item_static: syn::ItemStatic = syn::parse_quote! {
            #(#attrs)*
            #vis #mutability static #ident: #ty = #ty(#mod_ident::EXPR);
        };
        let item_mod = ConfigReprMod::from_value(value).item_mod(&mod_ident);
        let item_struct: syn::ItemStruct = syn::parse_quote! {
            #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
            pub struct #ty(pub #mod_ident::Type);
        };
        let item_impl: syn::ItemImpl = syn::parse_quote! {
            impl<P, T> ::inline_config::Get<P, T> for #ty
            where
                #mod_ident::Type:
                    ::inline_config::__private::AccessPath<P>,
                <#mod_ident::Type as ::inline_config::__private::AccessPath<P>>::Repr:
                    ::inline_config::__private::ConvertRepr<T>,
            {
                fn get(&'static self, _path: P) -> T {
                    <
                        <#mod_ident::Type as ::inline_config::__private::AccessPath<P>>::Repr
                            as ::inline_config::__private::ConvertRepr<T>
                    >::convert_repr(
                        <#mod_ident::Type as ::inline_config::__private::AccessPath<P>>::access_path(
                            &self.0,
                        ),
                    )
                }
            }
        };
        (item_static, item_mod, item_struct, item_impl)
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
                value
                    .iter()
                    .enumerate()
                    .map(|(index, value)| (index, syn::Member::from(index), value)),
                Key::index_ty,
                Self::array_containers,
            ),
            Value::Table(value) => Self::from_container(
                value.iter().enumerate().map(|(index, (name, value))| {
                    (
                        name.as_ref(),
                        syn::Member::from(
                            Some(name)
                                .filter(|name| {
                                    !(name.starts_with('_')
                                        && name.chars().skip(1).all(|c| c.is_ascii_digit()))
                                })
                                .and_then(|name| syn::parse_str::<syn::Ident>(name).ok())
                                .unwrap_or_else(|| quote::format_ident!("_{index}")),
                        ),
                        value,
                    )
                }),
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
        items: impl Iterator<Item = (T, syn::Member, &'v Value)>,
        key_ty_fn: fn(T) -> syn::Type,
        convert_items_fn: fn(&syn::Ident, &[T], &[syn::Expr]) -> Vec<(syn::Type, syn::Expr)>,
    ) -> Self {
        let (field_mods, (tags, (members, (member_tys, member_exprs)))): (
            Vec<_>,
            (Vec<_>, (Vec<_>, (Vec<_>, Vec<_>))),
        ) = items
            .enumerate()
            .map(|(index, (tag, member, value))| {
                let mod_ident = quote::format_ident!("_{index}");
                let member_ty: syn::Type = syn::parse_quote! {
                    #mod_ident::Type
                };
                let member_expr: syn::Expr = syn::parse_quote! {
                    #mod_ident::EXPR
                };
                (
                    Self::from_value(value).item_mod(&mod_ident),
                    (tag, (member, (member_ty, member_expr))),
                )
            })
            .unzip();
        Self {
            ty: syn::parse_quote! {
                ::inline_config::__private::ReprContainer<Struct>
            },
            expr: syn::parse_quote! {
                ::inline_config::__private::ReprContainer(Struct {
                    #(#members: &#member_exprs,)*
                })
            },
            item_struct: Some(
                if members
                    .iter()
                    .all(|member| matches!(member, syn::Member::Unnamed(_)))
                {
                    syn::parse_quote! {
                        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
                        pub struct Struct(
                            #(pub &'static #member_tys,)*
                        );
                    }
                } else {
                    syn::parse_quote! {
                        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
                        pub struct Struct {
                            #(pub #members: &'static #member_tys,)*
                        }
                    }
                },
            ),
            field_mods,
            access_impls: tags
                .iter()
                .zip(members.iter().zip(member_tys.iter()))
                .map(|(tag, (member, member_ty))| {
                    let key_ty = key_ty_fn(*tag);
                    syn::parse_quote! {
                        impl ::inline_config::__private::Access<#key_ty> for Struct {
                            type Repr = #member_ty;

                            fn access(&self) -> &Self::Repr {
                                &self.#member
                            }
                        }
                    }
                })
                .collect(),
            convert_impls: {
                let generic = syn::Ident::new("__inline_config__T", proc_macro2::Span::call_site());
                let (exprs, predicates): (Vec<syn::Expr>, Vec<syn::WherePredicate>) = members
                    .iter()
                    .zip(member_tys.iter())
                    .map(|(member, member_ty)| {
                        (
                            syn::parse_quote! {
                                <
                                    #member_ty as ::inline_config::__private::ConvertRepr<#generic>
                                >::convert_repr(&self.#member)
                            },
                            syn::parse_quote! {
                                #member_ty: ::inline_config::__private::ConvertRepr<#generic>
                            },
                        )
                    })
                    .unzip();
                convert_items_fn(&generic, &tags, &exprs)
                    .iter()
                    .map(|(ty, expr)| {
                        syn::parse_quote! {
                            impl<#generic> ::inline_config::__private::Convert<#ty> for Struct
                            where
                                #(#predicates,)*
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
        _tags: &[usize],
        exprs: &[syn::Expr],
    ) -> Vec<(syn::Type, syn::Expr)> {
        [(
            syn::parse_quote! { Vec<#generic> },
            syn::parse_quote! { [#(#exprs),*].into() },
        )]
        .into()
    }

    fn table_containers(
        generic: &syn::Ident,
        tags: &[&str],
        exprs: &[syn::Expr],
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
