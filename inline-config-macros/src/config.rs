use crate::format::Format;
use crate::path::Key;
use crate::value::{Map, Value};

// #[derive(FromMeta)]
// struct Sources {
//     #[darling(multiple)]
//     config: Vec<Value>,
// }

pub struct ConfigItem {
    ident: syn::Ident,
    value: Value,
    // #[darling(map = accumulate)]
    // #[darling(flatten)]
    // sources: Sources,
}

pub fn config(item: syn::ItemStruct) -> syn::Result<ConfigItem> {
    let mut errors = darling::Error::accumulator();
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
        // let path_nil = &Path::default();
        // let ident = &self.ident;
        // quote::quote! {
        //     impl std::ops::Deref for #ident {
        //         type Target = ::inline_config::__private::Config<#ident, #path_nil>;

        //         fn deref(&self) -> &Self::Target {
        //             &::inline_config::__private::Config(::std::marker::PhantomData)
        //         }
        //     }
        // }
        // .to_tokens(tokens);
        // impls_from_value(&self.value, ident, path_nil)
        //     .iter()
        //     .for_each(|item_impl| item_impl.to_tokens(tokens));
    }
}

// pub fn config(item: syn::DeriveInput) -> syn::Result<ConfigTokenItems> {
//     let ConfigItem { ident, sources } = ConfigItem::from_derive_input(&item)?;
//     Ok(ConfigTokenItems {
//         item_impl_deref: ,
//         item_impls_from: ,
//     })
// }

// struct ModStructure {
//     ty: syn::Ident,
//     item_impls_from: Vec<syn::ItemImpl>,
//     item_impls_index: Vec<syn::ItemImpl>,
//     item_mods: Vec<syn::ItemMod>,
// }

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
                    impl<P> ::std::ops::Index<::inline_config::__private::PathCons<#key, P>> for #ident
                    where
                        #child_ty: ::std::ops::Index<P>,
                        &'static <#child_ty as ::std::ops::Index<P>>::Output: Default,
                    {
                        type Output = <#child_ty as ::std::ops::Index<P>>::Output;

                        fn index(&self, _index: ::inline_config::__private::PathCons<#key, P>) -> &Self::Output {
                            <&<#child_ty as ::std::ops::Index<P>>::Output as Default>::default()
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
            .map(|from_impl| {
                let ty = from_impl.ty;
                let expr = from_impl.expr;
                let (impl_generics, _, where_clause) = from_impl.generics.split_for_impl();
                syn::parse_quote! {
                    impl #impl_generics From<#ident> for #ty #where_clause {
                        fn from(_value: #ident) -> Self {
                            #expr
                        }
                    }
                }
            })
            .collect();
        syn::parse_quote! {
            const _: () = {
                impl Default for &#ident {
                    fn default() -> &#ident {
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
                [#(<#children_tys as Into<T>>::into(#children_exprs)),*].into()
            },
            generics: syn::parse_quote! {
                <T> where #(#children_tys: Into<T>),*
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
                    [#(#names, (<#children_tys as Into<T>>::into(#children_exprs))),*].into()
                },
                generics: syn::parse_quote! {
                    <T> where #(#children_tys: Into<T>),*
                },
            },
            ConvertImpl {
                ty: syn::parse_quote! { ::std::collections::BTreeMap<String, T> },
                expr: syn::parse_quote! {
                    [#(#names.to_string(), (<#children_tys as Into<T>>::into(#children_exprs))),*].into()
                },
                generics: syn::parse_quote! {
                    <T> where #(#children_tys: Into<T>),*
                },
            },
            #[cfg(feature = "indexmap")]
            ConvertImpl {
                ty: syn::parse_quote! { ::indexmap::IndexMap<&'static str, T> },
                expr: syn::parse_quote! {
                    [#(#names, (<#children_tys as Into<T>>::into(#children_exprs))),*].into()
                },
                generics: syn::parse_quote! {
                    <T> where #(#children_tys: Into<T>),*
                },
            },
            #[cfg(feature = "indexmap")]
            ConvertImpl {
                ty: syn::parse_quote! { ::indexmap::IndexMap<String, T> },
                expr: syn::parse_quote! {
                    [#(#names.to_string(), (<#children_tys as Into<T>>::into(#children_exprs))),*].into()
                },
                generics: syn::parse_quote! {
                    <T> where #(#children_tys: Into<T>),*
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

// impl<'v> NodeImpls<'v> {
//     fn new(
//         convert_impls: impl FnOnce(&Value, &[syn::Type], &[syn::Expr]) -> Vec<ConvertImpl> + 'static,
//         children: impl Iterator<Item = &'v Value>,
//     ) -> Self {
//         Self {
//             convert_impls: Box::new(convert_impls),
//             children: children.collect(),
//         }
//     }

//     fn primitive<const N: usize>(items: [(syn::Type, syn::Expr); N]) -> Self {
//         Self::new(
//             |_, _, _| {
//                 Vec::from(items.map(|(ty, expr)| ConvertImpl {
//                     ty,
//                     expr,
//                     generics: syn::Generics::default(),
//                 }))
//             },
//             std::iter::empty(),
//         )
//     }

//     fn from_value(value: &'v Value) -> Self {
//         match value {
//             Value::Nil => Self::primitive([]),
//             Value::Boolean(value) => {
//                 Self::primitive([(syn::parse_quote! { bool }, syn::parse_quote! { #value })])
//             }
//             Value::PosInt(value) => Self::primitive(
//                 [
//                     syn::parse_quote! { u8 },
//                     syn::parse_quote! { u16 },
//                     syn::parse_quote! { u32 },
//                     syn::parse_quote! { u64 },
//                     syn::parse_quote! { u128 },
//                     syn::parse_quote! { usize },
//                     syn::parse_quote! { i8 },
//                     syn::parse_quote! { i16 },
//                     syn::parse_quote! { i32 },
//                     syn::parse_quote! { i64 },
//                     syn::parse_quote! { i128 },
//                     syn::parse_quote! { isize },
//                     syn::parse_quote! { f32 },
//                     syn::parse_quote! { f64 },
//                 ]
//                 .map(|ty: syn::Type| (ty.clone(), syn::parse_quote! { #value as #ty })),
//             ),
//             Value::NegInt(value) => Self::primitive(
//                 [
//                     syn::parse_quote! { i8 },
//                     syn::parse_quote! { i16 },
//                     syn::parse_quote! { i32 },
//                     syn::parse_quote! { i64 },
//                     syn::parse_quote! { i128 },
//                     syn::parse_quote! { isize },
//                     syn::parse_quote! { f32 },
//                     syn::parse_quote! { f64 },
//                 ]
//                 .map(|ty: syn::Type| (ty.clone(), syn::parse_quote! { #value as #ty })),
//             ),
//             Value::Float(value) => Self::primitive(
//                 [syn::parse_quote! { f32 }, syn::parse_quote! { f64 }]
//                     .map(|ty: syn::Type| (ty.clone(), syn::parse_quote! { #value as #ty })),
//             ),
//             Value::String(value) => Self::primitive([
//                 (
//                     syn::parse_quote! { &'static str },
//                     syn::parse_quote! { #value },
//                 ),
//                 (
//                     syn::parse_quote! { String },
//                     syn::parse_quote! { #value.to_string() },
//                 ),
//             ]),
//             Value::Array(value) => Self::new(
//                 |_, children_tys, children_exprs| {
//                     Vec::from([ConvertImpl {
//                         ty: syn::parse_quote! { Vec<T> },
//                         expr: syn::parse_quote! {
//                             [#(<#children_tys as Into<T>>::into(#children_exprs)),*].into()
//                         },
//                         generics: syn::parse_quote! {
//                             <T> where #(#children_tys: Into<T>),*
//                         },
//                     }])
//                 },
//                 value.iter(),
//             ),
//             Value::Table(value) => Self::new(
//                 |value, children_tys, children_exprs| {
//                     let names = &value.keys();
//                     Vec::from([
//                         ConvertImpl {
//                             ty: syn::parse_quote! { ::std::collections::BTreeMap<&'static str, T> },
//                             expr: syn::parse_quote! {
//                                 [#(#names, (<#children_tys as Into<T>>::into(#children_exprs))),*].into()
//                             },
//                             generics: syn::parse_quote! {
//                                 <T> where #(#children_tys: Into<T>),*
//                             },
//                         },
//                         ConvertImpl {
//                             ty: syn::parse_quote! { ::std::collections::BTreeMap<String, T> },
//                             expr: syn::parse_quote! {
//                                 [#(#names.to_string(), (<#children_tys as Into<T>>::into(#children_exprs))),*].into()
//                             },
//                             generics: syn::parse_quote! {
//                                 <T> where #(#children_tys: Into<T>),*
//                             },
//                         },
//                         #[cfg(feature = "indexmap")]
//                         ConvertImpl {
//                             ty: syn::parse_quote! { ::indexmap::IndexMap<&'static str, T> },
//                             expr: syn::parse_quote! {
//                                 [#(#names, (<#children_tys as Into<T>>::into(#children_exprs))),*].into()
//                             },
//                             generics: syn::parse_quote! {
//                                 <T> where #(#children_tys: Into<T>),*
//                             },
//                         },
//                         #[cfg(feature = "indexmap")]
//                         ConvertImpl {
//                             ty: syn::parse_quote! { ::indexmap::IndexMap<String, T> },
//                             expr: syn::parse_quote! {
//                                 [#(#names.to_string(), (<#children_tys as Into<T>>::into(#children_exprs))),*].into()
//                             },
//                             generics: syn::parse_quote! {
//                                 <T> where #(#children_tys: Into<T>),*
//                             },
//                         },
//                     ])
//                 },
//                 value.values(),
//             ),
//         }
//     }
// }

// fn impls_from_value(value: &Value, ident: &syn::Ident) -> syn::ItemConst {
//     impl<V> NodeImplementor<V> {
//         fn primitive(items: Vec<(syn::Type, fn(&V) -> syn::Expr)>) -> Self {
//             Self {
//                 // from_implementors:
//                 fields: |_| Vec::new(),
//             }
//         }
//     }

//     struct PrimitiveConvertItem<V> {
//         ty_fn: fn() -> syn::Type,
//         expr_fn: fn(&V) -> syn::Expr,
//     }

//     struct ContainerConvertItem<T> {
//         ty_fn: fn(&syn::Ident) -> syn::Type,
//         expr_fn: fn(&[T], &[syn::Expr]) -> syn::Expr,
//     }

//     fn from_primitive<V, const PN: usize>(
//         config_ident: &syn::Ident,
//         path: &Path,
//         value: &V,
//         primitive_convert_items: [PrimitiveConvertItem<V>; PN],
//     ) -> Vec<syn::ItemImpl> {
//         primitive_convert_items
//             .iter()
//             .map(|convert_item| {
//                 let ty = (convert_item.ty_fn)();
//                 let expr = (convert_item.expr_fn)(value);
//                 syn::parse_quote! {
//                     impl From<::inline_config::__private::Config<#config_ident, #path>> for #ty {
//                         fn from(_value: ::inline_config::__private::Config<#config_ident, #path>) -> Self {
//                             #expr
//                         }
//                     }
//                 }
//             })
//             .collect()
//     }

//     fn from_container<'v, T: 'v, const CN: usize>(
//         config_ident: &syn::Ident,
//         path: &Path,
//         field_iter: impl IntoIterator<Item = (T, &'v Value)>,
//         key_fn: fn(&T) -> Key,
//         container_convert_items: [ContainerConvertItem<T>; CN],
//     ) -> Vec<syn::ItemImpl> {
//         let (tags, values): (Vec<_>, Vec<_>) = field_iter.into_iter().unzip();
//         let t_generic = syn::Ident::new("__inline_config__T", proc_macro2::Span::call_site());
//         let ((where_predicates, exprs), fields_impls): (
//             (Vec<syn::WherePredicate>, Vec<syn::Expr>),
//             Vec<_>,
//         ) = tags
//             .iter()
//             .zip(values)
//             .map(|(tag, value)| {
//                 let field_path = path.clone().append(key_fn(tag));
//                 (
//                     (
//                         syn::parse_quote! {
//                             ::inline_config::__private::Config<#config_ident, #field_path>: Into<#t_generic>
//                         },
//                         syn::parse_quote! {
//                             <::inline_config::__private::Config<#config_ident, #field_path> as Into<#t_generic>>::into(
//                                 ::inline_config::__private::Config(::std::marker::PhantomData),
//                             )
//                         },
//                     ),
//                     impls_from_value(value, config_ident, &field_path),
//                 )
//             })
//             .unzip();
//         container_convert_items
//             .iter()
//             .map(|convert_item| {
//                 let ty = (convert_item.ty_fn)(&t_generic);
//                 let expr = (convert_item.expr_fn)(&tags, &exprs);
//                 syn::parse_quote! {
//                     impl<#t_generic> From<::inline_config::__private::Config<#config_ident, #path>> for #ty
//                     where
//                         #(#where_predicates,)*
//                     {
//                         fn from(_value: ::inline_config::__private::Config<#config_ident, #path>) -> Self {
//                             #expr
//                         }
//                     }
//                 }
//             })
//             .chain(fields_impls.into_iter().flatten())
//             .collect()
//     }

//     macro_rules! numeric_convert_item {
//         ($ty:ty) => {
//             PrimitiveConvertItem {
//                 ty_fn: || syn::parse_quote! { $ty },
//                 expr_fn: |expr| syn::parse_quote! { #expr as $ty },
//             }
//         };
//     }

//     match value {
//         Value::Nil => Vec::new(),
//         Value::Boolean(value) => from_primitive(
//             config_ident,
//             path,
//             value,
//             [PrimitiveConvertItem {
//                 ty_fn: || syn::parse_quote! { bool },
//                 expr_fn: |expr| syn::parse_quote! { #expr },
//             }],
//         ),
//         Value::PosInt(value) => from_primitive(
//             config_ident,
//             path,
//             value,
//             [
//                 numeric_convert_item!(u8),
//                 numeric_convert_item!(u16),
//                 numeric_convert_item!(u32),
//                 numeric_convert_item!(u64),
//                 numeric_convert_item!(u128),
//                 numeric_convert_item!(usize),
//                 numeric_convert_item!(i8),
//                 numeric_convert_item!(i16),
//                 numeric_convert_item!(i32),
//                 numeric_convert_item!(i64),
//                 numeric_convert_item!(i128),
//                 numeric_convert_item!(isize),
//                 numeric_convert_item!(f32),
//                 numeric_convert_item!(f64),
//             ],
//         ),
//         Value::NegInt(value) => from_primitive(
//             config_ident,
//             path,
//             value,
//             [
//                 numeric_convert_item!(i8),
//                 numeric_convert_item!(i16),
//                 numeric_convert_item!(i32),
//                 numeric_convert_item!(i64),
//                 numeric_convert_item!(i128),
//                 numeric_convert_item!(isize),
//                 numeric_convert_item!(f32),
//                 numeric_convert_item!(f64),
//             ],
//         ),
//         Value::Float(value) => from_primitive(
//             config_ident,
//             path,
//             value,
//             [numeric_convert_item!(f32), numeric_convert_item!(f64)],
//         ),
//         Value::String(value) => from_primitive(
//             config_ident,
//             path,
//             value,
//             [
//                 PrimitiveConvertItem {
//                     ty_fn: || syn::parse_quote! { &'static str },
//                     expr_fn: |expr| syn::parse_quote! { #expr },
//                 },
//                 PrimitiveConvertItem {
//                     ty_fn: || syn::parse_quote! { String },
//                     expr_fn: |expr| syn::parse_quote! { #expr.to_string() },
//                 },
//             ],
//         ),
//         Value::Array(value) => from_container(
//             config_ident,
//             path,
//             value.iter().enumerate(),
//             |&index| Key::Index(index),
//             [ContainerConvertItem {
//                 ty_fn: |generic| syn::parse_quote! { Vec<#generic> },
//                 expr_fn: |_tags, exprs| syn::parse_quote! { [#(#exprs),*].into() },
//             }],
//         ),
//         Value::Table(value) => from_container(
//             config_ident,
//             path,
//             value.iter(),
//             |name| Key::Name(name.to_string()),
//             [
//                 ContainerConvertItem {
//                     ty_fn: |generic| syn::parse_quote! { ::std::collections::BTreeMap<&'static str, #generic> },
//                     expr_fn: |tags, exprs| syn::parse_quote! { [#((#tags, #exprs)),*].into() },
//                 },
//                 ContainerConvertItem {
//                     ty_fn: |generic| syn::parse_quote! { ::std::collections::BTreeMap<String, #generic> },
//                     expr_fn: |tags, exprs| syn::parse_quote! { [#((#tags.to_string(), #exprs)),*].into() },
//                 },
//                 #[cfg(feature = "indexmap")]
//                 ContainerConvertItem {
//                     ty_fn: |generic| syn::parse_quote! { ::indexmap::IndexMap<&'static str, #generic> },
//                     expr_fn: |tags, exprs| syn::parse_quote! { [#((#tags, #exprs)),*].into() },
//                 },
//                 #[cfg(feature = "indexmap")]
//                 ContainerConvertItem {
//                     ty_fn: |generic| syn::parse_quote! { ::indexmap::IndexMap<String, #generic> },
//                     expr_fn: |tags, exprs| syn::parse_quote! { [#((#tags.to_string(), #exprs)),*].into() },
//                 },
//             ],
//         ),
//     }
// }
