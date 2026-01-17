// #[derive(FromDeriveInput)]
// #[darling(attributes(config), forward_attrs(allow, cfg, doc))]
// struct ConfigAttrs {
//     ident: syn::Ident,
//     #[darling(with = collect_value)]
//     attrs: Value,
// }
use crate::format::Format;

pub struct ConfigTokenItems {
    // item: syn::Item,
    item_mod: syn::ItemMod,
    item_struct: syn::ItemStruct,
    get_impl: syn::ItemImpl,
}

impl quote::ToTokens for ConfigTokenItems {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        // self.item.to_tokens(tokens);
        self.item_mod.to_tokens(tokens);
        self.item_struct.to_tokens(tokens);
        self.get_impl.to_tokens(tokens);
    }
}

pub fn config(item: syn::ItemType) -> syn::Result<ConfigTokenItems> {
    let syn::ItemType {
        attrs,
        vis,
        ident,
        generics,
        ty,
        ..
    } = item;
    quote::ToTokens::to_token_stream(&generics)
        .is_empty()
        .then_some(())
        .ok_or_else(|| syn::Error::new_spanned(&generics, "unexpected generics"))?;

    let mac = match *ty {
        syn::Type::Macro(syn::TypeMacro { mac }) => Ok(mac),
        ty => Err(syn::Error::new_spanned(&ty, "expecting macro")),
    }?;
    let format = Format::from_str(&mac.path.require_ident()?.to_string())
        .ok_or(syn::Error::new_spanned(&mac.path, "format not supported"))?;
    let sources = syn::parse::Parser::parse2(
        syn::punctuated::Punctuated::<macro_string::MacroString, syn::Token![,]>::parse_terminated,
        mac.tokens,
    )?;
    let value = sources
        .iter()
        .enumerate()
        .map(|(index, source)| {
            format.parse(&source.0).map_err(|e| {
                syn::Error::new(proc_macro2::Span::call_site(), format!("src {index}: {e}"))
            })
        })
        .collect::<syn::Result<Vec<_>>>()?
        .into_iter()
        .sum();

    // let value = item
    //     .attrs
    //     .iter()
    //     .filter_map(|attr| {
    //         attr.meta.require_list().ok().and_then(|meta_list| {
    //             meta_list
    //                 .path
    //                 .is_ident("config")
    //                 .then_some(meta_list.tokens.clone())
    //         })
    //     })
    //     .map(|tokens| {
    //         let meta = syn::parse2::<syn::Meta>(tokens)?;
    //         let meta_list = meta.require_list()?;
    //         let format = Format::from_str(&meta_list.path.require_ident()?.to_string()).ok_or(
    //             syn::Error::new_spanned(&meta_list.path, "format not supported"),
    //         )?;
    //         let content: macro_string::MacroString = syn::parse2(meta_list.tokens.clone())?;
    //         format
    //             .parse(&content.0)
    //             .map_err(|e| syn::Error::new_spanned(&meta_list.tokens, e))
    //     })
    //     .collect::<syn::Result<Vec<_>>>()?
    //     .into_iter()
    //     .sum();

    let (item_mod, ty) =
        expand_mod::mod_ty_from_value(value, quote::format_ident!("__{}", ident.to_string()))
            .map_err(|expand_mod::NilError| {
                syn::Error::new(proc_macro2::Span::call_site(), "unresolved nil")
            })?;

    Ok(ConfigTokenItems {
        item_mod,
        item_struct: syn::parse_quote! {
            #(#attrs)*
            #vis struct #ident;
        },
        get_impl: syn::parse_quote! {
            impl<P, T> ::inline_config::Get<P, T> for #ident
            where
                #ty: ::inline_config::__private::AccessPath<P>,
                <#ty as ::inline_config::__private::AccessPath<P>>::Repr: ::inline_config::__private::Convert<T>,
            {
                fn get(&self, _path: P) -> T {
                    <<#ty as ::inline_config::__private::AccessPath<P>>::Repr as ::inline_config::__private::Convert<T>>::convert()
                }
            }
        },
    })

    // let config_attrs = ConfigAttrs::from_derive_input(&item).map_err(syn::Error::from)?;
    // let format: Format = std::str::FromStr::from_str(&input.to_string())
    //     .map_err(|e| syn::Error::new_spanned(input, e))?;
    // let (ident, ty, expr, item_fn) = match item {
    //     syn::Item::Static(syn::ItemStatic {
    //         attrs,
    //         vis,
    //         static_token,
    //         mutability,
    //         ident,
    //         colon_token,
    //         ty,
    //         eq_token,
    //         expr,
    //         semi_token,
    //     }) => (
    //         ident,
    //         ty,
    //         expr,
    //         Box::new(move |ident, ty, expr| {
    //             syn::parse_quote! {
    //                 #(#attrs)*
    //                 #vis #static_token #mutability #ident #colon_token #ty #eq_token #expr #semi_token
    //             }
    //         }) as Box<dyn Fn(syn::Ident, syn::Type, syn::Expr) -> syn::Item>,
    //     ),
    //     syn::Item::Const(syn::ItemConst {
    //         attrs,
    //         vis,
    //         const_token,
    //         ident,
    //         generics,
    //         colon_token,
    //         ty,
    //         eq_token,
    //         expr,
    //         semi_token,
    //     }) => (
    //         ident,
    //         ty,
    //         expr,
    //         Box::new(move |ident, ty, expr| {
    //             syn::parse_quote! {
    //                 #(#attrs)*
    //                 #vis #const_token #ident #generics #colon_token #ty #eq_token #expr #semi_token
    //             }
    //         }) as Box<dyn Fn(syn::Ident, syn::Type, syn::Expr) -> syn::Item>,
    //     ),
    //     item => Err(syn::Error::new_spanned(
    //         item,
    //         "expected static or const item",
    //     ))?,
    // };

    // fn value_from_expr(expr: &syn::Expr, format: &Format) -> syn::Result<Value> {
    //     match expr {
    //         syn::Expr::Binary(binary) => Ok(
    //             value_from_expr(&binary.left, format)? + value_from_expr(&binary.right, format)?
    //         ),
    //         expr => format
    //             .parse(
    //                 &syn::parse2::<macro_string::MacroString>(quote::ToTokens::to_token_stream(
    //                     expr,
    //                 ))?
    //                 .0,
    //             )
    //             .map_err(|e| syn::Error::new_spanned(expr, e)),
    //     }
    // }

    // // Ensures `ty` is identifier.
    // syn::parse2::<syn::Ident>(quote::ToTokens::to_token_stream(&ty))?;
    // let value = value_from_expr(&expr, &format)?;

    // let mod_ident = quote::format_ident!("__{}", ident.to_string().to_lowercase());
    // Ok(ConfigTokenItems {
    //     item: item_fn(
    //         ident,
    //         syn::parse_quote! { #ty },
    //         syn::parse_quote! { #ty(#mod_ident::expr()) },
    //     ),
    //     item_mod: ConfigReprMod::from_value(&value).item_mod(&mod_ident),
    //     item_struct: syn::parse_quote! {
    //         #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    //         pub struct #ty(pub #mod_ident::Type);
    //     },
    //     get_impl: syn::parse_quote! {
    //         impl<P, T> ::inline_config::Get<P, T> for #ty
    //         where
    //             #mod_ident::Type:
    //                 ::inline_config::__private::AccessPath<P>,
    //             <#mod_ident::Type as ::inline_config::__private::AccessPath<P>>::Repr:
    //                 ::inline_config::__private::ConvertRepr<T>,
    //         {
    //             fn get(&'static self, _path: P) -> T {
    //                 <
    //                     <#mod_ident::Type as ::inline_config::__private::AccessPath<P>>::Repr
    //                         as ::inline_config::__private::ConvertRepr<T>
    //                 >::convert_repr(
    //                     <#mod_ident::Type as ::inline_config::__private::AccessPath<P>>::access_path(
    //                         &self.0,
    //                     ),
    //                 )
    //             }
    //         }
    //     },
    // })
}

// mod parse {
//     use crate::value::Value;

//     pub fn value_from_attrs(attrs: &[syn::Attribute]) -> syn::Result<Value> {
//         Ok(attrs
//             .iter()
//             .filter_map(|attr| {
//                 attr.meta.require_list().ok().and_then(|meta_list| {
//                     meta_list
//                         .path
//                         .is_ident("config")
//                         .then_some(meta_list.tokens.clone())
//                 })
//             })
//             .map(|tokens| {
//                 let meta_list = syn::parse2::<syn::Meta>(tokens)?.require_list()?;
//                 let format = Format::from_str(&meta_list.path.require_ident()?.to_string());
//                 // .and_then(|meta| SourceGroup::from_meta(&meta).map_err(syn::Error::from))
//             })
//             .collect::<syn::Result<Vec<_>>>()?
//             .iter()
//             .map(|source_group| source_group.parse().map_err(syn::Error::from))
//             .collect::<syn::Result<Vec<_>>>()?
//             .into_iter()
//             .flatten()
//             .collect::<Vec<_>>()
//             .into_iter()
//             .sum())
//     }

//     #[derive(FromMeta)]
//     enum SourceGroup {
//         #[cfg(feature = "json")]
//         Json {
//             #[darling(multiple)]
//             sources: Vec<darling::util::SpannedValue<Source>>,
//         },

//         #[cfg(feature = "toml")]
//         Toml {
//             #[darling(multiple)]
//             sources: Vec<darling::util::SpannedValue<Source>>,
//         },

//         #[cfg(feature = "yaml")]
//         Yaml {
//             #[darling(multiple)]
//             sources: Vec<darling::util::SpannedValue<Source>>,
//         },
//     }

//     impl SourceGroup {
//         fn parse(&self) -> darling::Result<Vec<Value>> {
//             use crate::format;
//             match self {
//                 #[cfg(feature = "json")]
//                 SourceGroup::Json { sources } => sources
//                     .iter()
//                     .map(|source| source.parse(format::json::parse))
//                     .collect(),

//                 #[cfg(feature = "toml")]
//                 SourceGroup::Toml { sources } => sources
//                     .iter()
//                     .map(|source| source.parse(format::toml::parse))
//                     .collect(),

//                 #[cfg(feature = "yaml")]
//                 SourceGroup::Yaml { sources } => sources
//                     .iter()
//                     .map(|source| source.parse(format::yaml::parse))
//                     .collect(),
//             }
//         }
//     }

//     struct Source(macro_string::MacroString);

//     impl FromMeta for Source {
//         fn from_nested_meta(item: &darling::ast::NestedMeta) -> darling::Result<Self> {
//             Ok(Self(
//                 syn::parse2(quote::ToTokens::to_token_stream(item))
//                     .map_err(darling::Error::from)?,
//             ))
//         }
//     }

//     impl Source {
//         fn parse(
//             &self,
//             parse: fn(&str) -> Result<Value, Box<dyn std::error::Error>>,
//         ) -> darling::Result<Value> {
//             parse(&self.0 .0).map_err(darling::Error::custom)
//         }
//     }
// }

mod expand_mod {
    use crate::path::Key;
    use crate::value::Value;

    pub fn mod_ty_from_value(
        value: Value,
        mod_ident: syn::Ident,
    ) -> Result<(syn::ItemMod, syn::Type), NilError> {
        ModStructure::from_value(value).map(|mod_structure| mod_structure.mod_ty(mod_ident))
    }

    pub struct NilError;

    struct ModStructure {
        // ty: syn::Type,
        // item_struct: syn::ItemStruct,
        item_const: Option<syn::ItemConst>,
        field_mods: Vec<syn::ItemMod>,
        access_impls: Vec<syn::ItemImpl>,
        convert_impls: Vec<syn::ItemImpl>,
    }

    impl ModStructure {
        fn from_value(value: Value) -> Result<Self, NilError> {
            match value {
                Value::Nil => Err(NilError),
                Value::Boolean(value) => Self::from_primitive(
                    syn::parse_quote! { bool },
                    syn::parse_quote! { #value },
                    BOOLEAN_IMPLS,
                ),
                Value::PosInt(value) => Self::from_primitive(
                    syn::parse_quote! { u64 },
                    syn::parse_quote! { #value },
                    POS_INT_IMPLS,
                ),
                Value::NegInt(value) => Self::from_primitive(
                    syn::parse_quote! { i64 },
                    syn::parse_quote! { #value },
                    NEG_INT_IMPLS,
                ),
                Value::Float(value) => Self::from_primitive(
                    syn::parse_quote! { f64 },
                    syn::parse_quote! { #value },
                    FLOAT_IMPLS,
                ),
                Value::String(value) => Self::from_primitive(
                    syn::parse_quote! { &'static str },
                    syn::parse_quote! { #value },
                    STRING_IMPLS,
                ),
                Value::Array(value) => Self::from_container(
                    value.into_iter().enumerate(),
                    |index| Key::index_ty(*index),
                    ARRAY_IMPLS,
                ),
                Value::Table(value) => {
                    Self::from_container(value.into_iter(), |name| Key::name_ty(name), TABLE_IMPLS)
                }
            }
        }

        fn from_primitive(
            ty: syn::Type,
            expr: syn::Expr,
            convert_items: &[PrimitiveConvertItem],
        ) -> Result<Self, NilError> {
            let repr_ty = Self::repr_ty();
            Ok(Self {
                item_const: Some(syn::parse_quote! {
                    const EXPR: #ty = #expr;
                }),
                field_mods: Vec::new(),
                access_impls: Vec::new(),
                convert_impls: convert_items
                    .iter()
                    .map(|convert_item| {
                        let ty = (convert_item.ty_fn)();
                        let expr = (convert_item.expr_fn)(&syn::parse_quote! { EXPR });
                        syn::parse_quote! {
                            impl ::inline_config::__private::Convert<#ty> for #repr_ty {
                                fn convert() -> #ty {
                                    #expr
                                }
                            }
                        }
                    })
                    .collect(),
            })
        }

        fn from_container<T>(
            items: impl Iterator<Item = (T, Value)>,
            key_ty_fn: fn(&T) -> syn::Type,
            convert_items: &[ContainerConvertItem<T>],
        ) -> Result<Self, NilError> {
            let repr_ty = Self::repr_ty();
            let mut tags = Vec::new();
            let mut field_mods = Vec::new();
            let mut field_tys = Vec::new();
            for (index, (tag, value)) in items.enumerate() {
                let (field_mod, field_ty) =
                    Self::from_value(value)?.mod_ty(quote::format_ident!("_{index}"));
                tags.push(tag);
                field_mods.push(field_mod);
                field_tys.push(field_ty);
            }
            Ok(Self {
                item_const: None,
                field_mods,
                access_impls: tags
                    .iter()
                    .zip(field_tys.iter())
                    .map(|(tag, field_ty)| {
                        let key_ty = key_ty_fn(tag);
                        syn::parse_quote! {
                            impl ::inline_config::__private::AccessKey<#key_ty> for #repr_ty {
                                type Repr = #field_ty;
                            }
                        }
                    })
                    .collect(),
                convert_impls: {
                    let generic =
                        syn::Ident::new("__inline_config__T", proc_macro2::Span::call_site());
                    let where_predicates: Vec<syn::WherePredicate> = field_tys
                        .iter()
                        .map(|field_ty| {
                            syn::parse_quote! {
                                #field_ty: ::inline_config::__private::Convert<#generic>
                            }
                        })
                        .collect();
                    let exprs: Vec<syn::Expr> = field_tys
                        .iter()
                        .map(|field_ty| {
                            syn::parse_quote! {
                                <#field_ty as ::inline_config::__private::Convert<#generic>>::convert()
                            }
                        })
                        .collect();
                    convert_items
                        .iter()
                        .map(|convert_item| {
                            let ty = (convert_item.ty_fn)(&generic);
                            let expr = (convert_item.expr_fn)(&tags, &exprs);
                            syn::parse_quote! {
                                impl<#generic> ::inline_config::__private::Convert<#ty> for #repr_ty
                                where
                                    #(#where_predicates,)*
                                {
                                    fn convert() -> #ty {
                                        #expr
                                    }
                                }
                            }
                        })
                        .collect()
                },
            })
        }

        fn repr_ty() -> syn::Ident {
            syn::parse_quote! { Repr }
        }

        fn mod_ty(&self, mod_ident: syn::Ident) -> (syn::ItemMod, syn::Type) {
            let repr_ty = Self::repr_ty();
            let Self {
                // ty,
                // expr,
                // item_struct,
                item_const,
                field_mods,
                access_impls,
                convert_impls,
            } = self;
            (
                syn::parse_quote! {
                    pub mod #mod_ident {
                        // pub type Type = #ty;
                        // pub const fn expr() -> Type {
                        //     #expr
                        // }
                        // #item_struct
                        // #item_struct
                        pub struct #repr_ty;
                        #item_const
                        #(#field_mods)*
                        #(#access_impls)*
                        #(#convert_impls)*
                    }
                },
                syn::parse_quote! {
                    #mod_ident::#repr_ty
                },
            )
        }
    }

    struct PrimitiveConvertItem {
        ty_fn: fn() -> syn::Type,
        expr_fn: fn(&syn::Expr) -> syn::Expr,
    }

    struct ContainerConvertItem<T> {
        ty_fn: fn(&syn::Ident) -> syn::Type,
        expr_fn: fn(&[T], &[syn::Expr]) -> syn::Expr,
    }

    macro_rules! numeric_convert_item {
        ($ty:ty) => {
            PrimitiveConvertItem {
                ty_fn: || syn::parse_quote! { $ty },
                expr_fn: |expr| syn::parse_quote! { #expr as $ty },
            }
        };
    }

    static BOOLEAN_IMPLS: &[PrimitiveConvertItem] = &[PrimitiveConvertItem {
        ty_fn: || syn::parse_quote! { bool },
        expr_fn: |expr| syn::parse_quote! { #expr },
    }];

    static POS_INT_IMPLS: &[PrimitiveConvertItem] = &[
        numeric_convert_item!(u8),
        numeric_convert_item!(u16),
        numeric_convert_item!(u32),
        numeric_convert_item!(u64),
        numeric_convert_item!(u128),
        numeric_convert_item!(usize),
        numeric_convert_item!(i8),
        numeric_convert_item!(i16),
        numeric_convert_item!(i32),
        numeric_convert_item!(i64),
        numeric_convert_item!(i128),
        numeric_convert_item!(isize),
        numeric_convert_item!(f32),
        numeric_convert_item!(f64),
    ];

    static NEG_INT_IMPLS: &[PrimitiveConvertItem] = &[
        numeric_convert_item!(i8),
        numeric_convert_item!(i16),
        numeric_convert_item!(i32),
        numeric_convert_item!(i64),
        numeric_convert_item!(i128),
        numeric_convert_item!(isize),
        numeric_convert_item!(f32),
        numeric_convert_item!(f64),
    ];

    static FLOAT_IMPLS: &[PrimitiveConvertItem] =
        &[numeric_convert_item!(f32), numeric_convert_item!(f64)];

    static STRING_IMPLS: &[PrimitiveConvertItem] = &[
        PrimitiveConvertItem {
            ty_fn: || syn::parse_quote! { &'static str },
            expr_fn: |expr| syn::parse_quote! { #expr },
        },
        PrimitiveConvertItem {
            ty_fn: || syn::parse_quote! { String },
            expr_fn: |expr| syn::parse_quote! { #expr.to_string() },
        },
    ];

    static ARRAY_IMPLS: &[ContainerConvertItem<usize>] = &[ContainerConvertItem {
        ty_fn: |generic| syn::parse_quote! { Vec<#generic> },
        expr_fn: |_tags, exprs| syn::parse_quote! { [#(#exprs),*].into() },
    }];

    static TABLE_IMPLS: &[ContainerConvertItem<String>] = &[
        ContainerConvertItem {
            ty_fn: |generic| syn::parse_quote! { ::std::collections::BTreeMap<&'static str, #generic> },
            expr_fn: |tags, exprs| syn::parse_quote! { [#((#tags, #exprs)),*].into() },
        },
        ContainerConvertItem {
            ty_fn: |generic| syn::parse_quote! { ::std::collections::BTreeMap<String, #generic> },
            expr_fn: |tags, exprs| syn::parse_quote! { [#((#tags.to_string(), #exprs)),*].into() },
        },
        #[cfg(feature = "indexmap")]
        ContainerConvertItem {
            ty_fn: |generic| syn::parse_quote! { indexmap::IndexMap<&'static str, #generic> },
            expr_fn: |tags, exprs| syn::parse_quote! { [#((#tags, #exprs)),*].into() },
        },
        #[cfg(feature = "indexmap")]
        ContainerConvertItem {
            ty_fn: |generic| syn::parse_quote! { indexmap::IndexMap<String, #generic> },
            expr_fn: |tags, exprs| syn::parse_quote! { [#((#tags.to_string(), #exprs)),*].into() },
        },
    ];
}
