use crate::format::Format;
use crate::lit_expand::Lit;
use crate::path::Key;
use crate::value::Value;

pub struct ConfigTokenItems {
    item: syn::Item,
    item_mod: syn::ItemMod,
    item_struct: syn::ItemStruct,
    get_impl: syn::ItemImpl,
}

impl quote::ToTokens for ConfigTokenItems {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.item.to_tokens(tokens);
        self.item_mod.to_tokens(tokens);
        self.item_struct.to_tokens(tokens);
        self.get_impl.to_tokens(tokens);
    }
}

pub fn config(input: syn::Ident, item: syn::Item) -> syn::Result<ConfigTokenItems> {
    let format: Format = std::str::FromStr::from_str(&input.to_string())
        .map_err(|e| syn::Error::new_spanned(input, e))?;
    let (ident, ty, expr, item_fn) = match item {
        syn::Item::Static(syn::ItemStatic {
            attrs,
            vis,
            static_token,
            mutability,
            ident,
            colon_token,
            ty,
            eq_token,
            expr,
            semi_token,
        }) => (
            ident,
            ty,
            expr,
            Box::new(move |ident, ty, expr| {
                syn::parse_quote! {
                    #(#attrs)*
                    #vis #static_token #mutability #ident #colon_token #ty #eq_token #expr #semi_token
                }
            }) as Box<dyn Fn(syn::Ident, syn::Type, syn::Expr) -> syn::Item>,
        ),
        syn::Item::Const(syn::ItemConst {
            attrs,
            vis,
            const_token,
            ident,
            generics,
            colon_token,
            ty,
            eq_token,
            expr,
            semi_token,
        }) => (
            ident,
            ty,
            expr,
            Box::new(move |ident, ty, expr| {
                syn::parse_quote! {
                    #(#attrs)*
                    #vis #const_token #ident #generics #colon_token #ty #eq_token #expr #semi_token
                }
            }) as Box<dyn Fn(syn::Ident, syn::Type, syn::Expr) -> syn::Item>,
        ),
        item => Err(syn::Error::new_spanned(
            item,
            "expected static or const item",
        ))?,
    };

    fn value_from_expr(expr: &syn::Expr, format: &Format) -> syn::Result<Value> {
        match expr {
            syn::Expr::Binary(binary) => Ok(
                value_from_expr(&binary.left, format)? + value_from_expr(&binary.right, format)?
            ),
            expr => format
                .parse(&syn::parse2::<Lit>(quote::ToTokens::to_token_stream(expr))?.expand()?)
                .map_err(|e| syn::Error::new_spanned(expr, e)),
        }
    }

    // Ensures `ty` is identifier.
    syn::parse2::<syn::Ident>(quote::ToTokens::to_token_stream(&ty))?;
    let value = value_from_expr(&expr, &format)?;

    let mod_ident = quote::format_ident!("__{}", ident.to_string().to_lowercase());
    Ok(ConfigTokenItems {
        item: item_fn(
            ident,
            syn::parse_quote! { #ty },
            syn::parse_quote! { #ty(#mod_ident::expr()) },
        ),
        item_mod: ConfigReprMod::from_value(&value).item_mod(&mod_ident),
        item_struct: syn::parse_quote! {
            #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
            pub struct #ty(pub #mod_ident::Type);
        },
        get_impl: syn::parse_quote! {
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
        },
    })
}

// fn lit_from_expr(expr: &syn::Expr) -> syn::Result<String> {
//     match expr {
//         syn::Expr::Lit(syn::ExprLit {
//             attrs: _,
//             lit: syn::Lit::Str(text_lit),
//         }) => Ok(text_lit.value()),

//         syn::Expr::Macro(syn::ExprMacro { attrs: _, mac }) => {
//             let path_lit: syn::LitStr = syn::parse2(mac.tokens.clone())?;
//             let path = match mac.path.require_ident()?.to_string().as_str() {
//                 "include_str" => Ok(std::path::PathBuf::from(path_lit.value())),
//                 "include_config_env" => Self::resolve_env(&path_lit.value())
//                     .map(std::path::PathBuf::from)
//                     .map_err(|e| syn::Error::new_spanned(&path_lit, e)),
//                 _ => Err(syn::Error::new_spanned(&mac.path, "expected `include_str`")),
//             }?;

//             // Resolve the path relative to the current file.
//             let path = if path.is_absolute() {
//                 path
//             } else {
//                 // Rust analyzer hasn't implemented `Span::file()`.
//                 // https://github.com/rust-lang/rust-analyzer/issues/15950
//                 std::path::PathBuf::from(proc_macro2::Span::call_site().file())
//                     .parent()
//                     .ok_or(syn::Error::new_spanned(
//                         &path_lit,
//                         "cannot retrieve parent dir",
//                     ))?
//                     .join(path)
//             };

//             let text =
//                 std::fs::read_to_string(path).map_err(|e| syn::Error::new_spanned(&path_lit, e))?;
//             F::parse(&text).map_err(|e| syn::Error::new_spanned(expr, e))
//         }

//         _ => Err(syn::Error::new_spanned(
//             expr,
//             "expected string literal or macro invocation",
//         )),
//     }
// }

// pub struct ConfigItem<F> {
//     format: std::marker::PhantomData<F>,
//     ident: syn::Ident,
//     ty: syn::Ident,
//     value: Value,
//     #[allow(clippy::type_complexity)]
//     item_fn: Box<dyn Fn(&syn::Ident, &syn::Type, &syn::Expr) -> syn::Item>,
// }

// impl<F: Format> syn::parse::Parse for ConfigItem<F> {
//     fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
//         match input.parse()? {
//             syn::Item::Static(syn::ItemStatic {
//                 attrs,
//                 vis,
//                 static_token,
//                 mutability,
//                 ident,
//                 colon_token,
//                 ty,
//                 eq_token,
//                 expr,
//                 semi_token,
//             }) => Ok(Self {
//                 format: std::marker::PhantomData,
//                 ident,
//                 ty: Self::ident_from_ty(&ty)?,
//                 value: Self::value_from_expr(&expr)?,
//                 item_fn: Box::new(move |ident, ty, expr| {
//                     syn::parse_quote! {
//                         #(#attrs)*
//                         #vis #static_token #mutability #ident #colon_token #ty #eq_token #expr #semi_token
//                     }
//                 }),
//             }),
//             syn::Item::Const(syn::ItemConst {
//                 attrs,
//                 vis,
//                 const_token,
//                 ident,
//                 generics,
//                 colon_token,
//                 ty,
//                 eq_token,
//                 expr,
//                 semi_token,
//             }) => Ok(Self {
//                 format: std::marker::PhantomData,
//                 ident,
//                 ty: Self::ident_from_ty(&ty)?,
//                 value: Self::value_from_expr(&expr)?,
//                 item_fn: Box::new(move |ident, ty, expr| {
//                     syn::parse_quote! {
//                         #(#attrs)*
//                         #vis #const_token #ident #generics #colon_token #ty #eq_token #expr #semi_token
//                     }
//                 }),
//             }),
//             item => Err(syn::Error::new_spanned(
//                 item,
//                 "expected static or const item",
//             )),
//         }
//     }
// }

// impl<F: Format> quote::ToTokens for ConfigItem<F> {
//     fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
//         let Self {
//             format: _,
//             ident,
//             ty,
//             value,
//             item_fn,
//         } = self;

//         item.to_tokens(tokens);
//         item_mod.to_tokens(tokens);
//         item_struct.to_tokens(tokens);
//         item_impl.to_tokens(tokens);
//     }
// }

// impl<F: Format> ConfigItem<F> {
//     fn ident_from_ty(ty: &syn::Type) -> syn::Result<syn::Ident> {
//         match ty {
//             syn::Type::Path(syn::TypePath { qself: None, path }) => path.require_ident().cloned(),
//             _ => Err(syn::Error::new_spanned(
//                 ty,
//                 "config type must be an identifier",
//             )),
//         }
//     }

//     // Resolve `$ENV_VAR` in a given path.
//     // Inspired from `include_dir::resolve_env`.
//     fn resolve_env(path: &str) -> Result<String, std::env::VarError> {
//         let mut chars = path.chars().peekable();
//         let mut resolved = String::new();
//         while let Some(c) = chars.next() {
//             if c != '$' {
//                 resolved.push(c);
//                 continue;
//             }
//             if chars.peek() == Some(&'$') {
//                 chars.next();
//                 resolved.push('$');
//                 continue;
//             }
//             let mut variable = String::new();
//             while let Some(&c) = chars.peek() {
//                 if matches!(c, '0'..='9' | 'A'..='Z' | 'a'..='z' | '_') {
//                     chars.next();
//                     variable.push(c);
//                 } else {
//                     break;
//                 }
//             }
//             resolved.push_str(&std::env::var(&variable)?);
//         }
//         Ok(resolved)
//     }
// }

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
                    #mod_ident::expr()
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
                    #(#members: #member_exprs,)*
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
                            #(pub #member_tys,)*
                        );
                    }
                } else {
                    syn::parse_quote! {
                        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
                        pub struct Struct {
                            #(pub #members: #member_tys,)*
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

                            fn access(&'static self) -> &Self::Repr {
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
                                fn convert(&'static self) -> #ty {
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
                pub const fn expr() -> Type {
                    #expr
                }
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
