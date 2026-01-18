use crate::format::Format;
use darling::FromMeta;

#[derive(FromMeta)]
#[darling(derive_syn_parse)]
pub struct ConfigArgs {
    #[darling(default)]
    export: ExportSettings,
}

#[derive(Default, FromMeta)]
struct ExportSettings {
    #[darling(rename = "type")]
    r#type: Option<syn::Ident>,
    #[darling(rename = "const")]
    r#const: Option<syn::Ident>,
    #[darling(rename = "static")]
    r#static: Option<syn::Ident>,
}

pub struct ConfigTokenItems {
    item_mod: syn::ItemMod,
    export_item_type: Option<syn::ItemType>,
    export_item_const: Option<syn::ItemConst>,
    export_item_static: Option<syn::ItemStatic>,
}

impl quote::ToTokens for ConfigTokenItems {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.item_mod.to_tokens(tokens);
        self.export_item_type.to_tokens(tokens);
        self.export_item_const.to_tokens(tokens);
        self.export_item_static.to_tokens(tokens);
    }
}

pub fn config(args: ConfigArgs, item: syn::ItemMod) -> syn::Result<ConfigTokenItems> {
    let syn::ItemMod {
        attrs,
        vis,
        ident,
        content,
        ..
    } = item;
    let (brace, items) = content.ok_or(syn::Error::new_spanned(&ident, "no sources found"))?;
    if items.is_empty() {
        return Err(syn::Error::new(
            syn::spanned::Spanned::span(&brace.span),
            "no sources found",
        ));
    }
    let value = items
        .into_iter()
        .map(|item| match item {
            syn::Item::Macro(syn::ItemMacro { mac, .. }) => Ok(mac),
            item => Err(syn::Error::new_spanned(&item, "expecting macro")),
        })
        .enumerate()
        .map(|(index, mac)| {
            let syn::Macro { path, tokens, .. } = mac?;
            let format = Format::from_str(&path.require_ident()?.to_string())
                .ok_or(syn::Error::new_spanned(&path, "format not supported"))?;
            let source: macro_string::MacroString = syn::parse2(tokens)?;
            format.parse(&source.0).map_err(|e| {
                syn::Error::new(proc_macro2::Span::call_site(), format!("src {index}: {e}"))
            })
        })
        .sum::<syn::Result<_>>()?;

    let expand_mod::ModStructure { ty, expr, item_mod } =
        expand_mod::ModStructure::from_value(ident, value);
    Ok(ConfigTokenItems {
        item_mod: syn::ItemMod {
            attrs,
            vis,
            ..item_mod
        },
        export_item_type: args.export.r#type.map(|ident| {
            syn::parse_quote! {
                pub type #ident = #ty;
            }
        }),
        export_item_const: args.export.r#const.map(|ident| {
            syn::parse_quote! {
                pub const #ident: #ty = #expr;
            }
        }),
        export_item_static: args.export.r#static.map(|ident| {
            syn::parse_quote! {
                pub static #ident: #ty = #expr;
            }
        }),
    })
}

mod expand_mod {
    use crate::path::Key;
    use crate::value::Value;

    pub struct ModStructure {
        pub ty: syn::Type,
        pub expr: syn::Expr,
        pub item_mod: syn::ItemMod,
    }

    impl ModStructure {
        pub fn from_value(mod_ident: syn::Ident, value: Value) -> Self {
            match value {
                Value::Nil => Self::from_primitive(
                    mod_ident,
                    syn::parse_quote! { () },
                    syn::parse_quote! { () },
                    NIL_IMPLS,
                ),
                Value::Boolean(value) => Self::from_primitive(
                    mod_ident,
                    syn::parse_quote! { bool },
                    syn::parse_quote! { #value },
                    BOOLEAN_IMPLS,
                ),
                Value::PosInt(value) => Self::from_primitive(
                    mod_ident,
                    syn::parse_quote! { u64 },
                    syn::parse_quote! { #value },
                    POS_INT_IMPLS,
                ),
                Value::NegInt(value) => Self::from_primitive(
                    mod_ident,
                    syn::parse_quote! { i64 },
                    syn::parse_quote! { #value },
                    NEG_INT_IMPLS,
                ),
                Value::Float(value) => Self::from_primitive(
                    mod_ident,
                    syn::parse_quote! { f64 },
                    syn::parse_quote! { #value },
                    FLOAT_IMPLS,
                ),
                Value::String(value) => Self::from_primitive(
                    mod_ident,
                    syn::parse_quote! { &'static str },
                    syn::parse_quote! { #value },
                    STRING_IMPLS,
                ),
                Value::Array(value) => Self::from_container(
                    mod_ident,
                    value.into_iter().enumerate(),
                    |index| Key::index_ty(*index),
                    ARRAY_IMPLS,
                ),
                Value::Table(value) => Self::from_container(
                    mod_ident,
                    value.into_iter(),
                    |name| Key::name_ty(name),
                    TABLE_IMPLS,
                ),
            }
        }

        fn from_primitive(
            mod_ident: syn::Ident,
            ty: syn::Type,
            expr: syn::Expr,
            convert_items: &[PrimitiveConvertItem],
        ) -> Self {
            let from_impls = convert_items
                .iter()
                .map(|convert_item| {
                    let ty = (convert_item.ty_fn)();
                    let expr = (convert_item.expr_fn)(&syn::parse_quote! { SRC });
                    syn::parse_quote! {
                        impl From<Type> for #ty {
                            fn from(_value: Type) -> Self {
                                #expr
                            }
                        }
                    }
                })
                .collect();
            Self::from_fields(
                mod_ident,
                Vec::new(),
                Vec::new(),
                Vec::new(),
                from_impls,
                Some(syn::parse_quote! {
                    const SRC: #ty = #expr;
                }),
            )
        }

        fn from_container<T>(
            mod_ident: syn::Ident,
            items: impl Iterator<Item = (T, Value)>,
            key_ty_fn: fn(&T) -> syn::Type,
            convert_items: &[ContainerConvertItem<T>],
        ) -> Self {
            let (((key_tys, field_tys), field_mods), tags): (((Vec<_>, Vec<_>), Vec<_>), Vec<_>) =
                items
                    .enumerate()
                    .map(|(index, (tag, value))| {
                        let structure = Self::from_value(quote::format_ident!("_{index}"), value);
                        (((key_ty_fn(&tag), structure.ty), structure.item_mod), tag)
                    })
                    .unzip();
            let generic = syn::Ident::new("__inline_config__T", proc_macro2::Span::call_site());
            let (where_predicates, exprs): (Vec<syn::WherePredicate>, Vec<syn::Expr>) = field_tys
                .iter()
                .map(|field_ty| {
                    (
                        syn::parse_quote! {
                            #field_ty: Default + Into<#generic>
                        },
                        syn::parse_quote! {
                            <#field_ty as Into<#generic>>::into(<#field_ty as Default>::default())
                        },
                    )
                })
                .unzip();
            let from_impls = convert_items
                .iter()
                .map(|convert_item| {
                    let ty = (convert_item.ty_fn)(&generic);
                    let expr = (convert_item.expr_fn)(&tags, &exprs);
                    syn::parse_quote! {
                        impl<#generic> From<Type> for #ty
                        where
                            #(#where_predicates,)*
                        {
                            fn from(value: Type) -> Self {
                                #expr
                            }
                        }
                    }
                })
                .collect();
            Self::from_fields(mod_ident, key_tys, field_tys, field_mods, from_impls, None)
        }

        fn from_fields(
            mod_ident: syn::Ident,
            key_tys: Vec<syn::Type>,
            field_tys: Vec<syn::Type>,
            field_mods: Vec<syn::ItemMod>,
            from_impls: Vec<syn::ItemImpl>,
            src_const: Option<syn::ItemConst>,
        ) -> Self {
            let item_struct: syn::ItemStruct = syn::parse_quote! {
                #[derive(Clone, Copy, Default)]
                pub struct Type;
            };
            let item_static: syn::ItemStatic = syn::parse_quote! {
                pub static EXPR: Type = Type;
            };
            let ref_default_impl: syn::ItemImpl = syn::parse_quote! {
                impl Default for &'static Type {
                    fn default() -> Self {
                        &EXPR
                    }
                }
            };
            let index_key_impls: Vec<syn::ItemImpl> = key_tys
                .iter()
                .zip(field_tys.iter())
                .map(|(key_ty, field_ty)| {
                    syn::parse_quote! {
                        impl ::std::ops::Index<#key_ty> for Type {
                            type Output = #field_ty;

                            fn index(&self, _index: #key_ty) -> &Self::Output {
                                <&'static #field_ty>::default()
                            }
                        }
                    }
                })
                .collect();
            let index_path_impls: Vec<syn::ItemImpl> = [
                syn::parse_quote! {
                    impl ::std::ops::Index<::inline_config::__private::PathNil> for Type {
                        type Output = Type;

                        fn index(&self, _index: ::inline_config::__private::PathNil) -> &Self::Output {
                            <&'static Type as Default>::default()
                        }
                    }
                },
                syn::parse_quote! {
                    impl<K, KS> ::std::ops::Index<::inline_config::__private::PathCons<K, KS>> for Type
                    where
                        Type: ::std::ops::Index<K, Output: ::std::ops::Index<KS, Output: 'static>>,
                        &'static <<Type as ::std::ops::Index<K>>::Output as ::std::ops::Index<KS>>::Output: Default,
                    {
                        type Output = <<Type as ::std::ops::Index<K>>::Output as ::std::ops::Index<KS>>::Output;

                        fn index(&self, _index: ::inline_config::__private::PathCons<K, KS>) -> &Self::Output {
                            <&'static <<Type as ::std::ops::Index<K>>::Output as ::std::ops::Index<KS>>::Output as Default>::default()
                        }
                    }
                },
            ].into();
            Self {
                ty: syn::parse_quote! {
                    #mod_ident::Type
                },
                expr: syn::parse_quote! {
                    #mod_ident::EXPR
                },
                item_mod: syn::parse_quote! {
                    pub mod #mod_ident {
                        #item_struct
                        #item_static
                        #ref_default_impl
                        #(#index_key_impls)*
                        #(#index_path_impls)*
                        #(#field_mods)*
                        #(#from_impls)*
                        #src_const
                    }
                },
            }
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

    static NIL_IMPLS: &[PrimitiveConvertItem] = &[];

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
            ty_fn: |generic| syn::parse_quote! { ::indexmap::IndexMap<&'static str, #generic> },
            expr_fn: |tags, exprs| syn::parse_quote! { [#((#tags, #exprs)),*].into() },
        },
        #[cfg(feature = "indexmap")]
        ContainerConvertItem {
            ty_fn: |generic| syn::parse_quote! { ::indexmap::IndexMap<String, #generic> },
            expr_fn: |tags, exprs| syn::parse_quote! { [#((#tags.to_string(), #exprs)),*].into() },
        },
    ];
}
