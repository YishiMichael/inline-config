use crate::path::Key;
use crate::value::Value;
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
            syn::Item::Macro(syn::ItemMacro {
                mac: syn::Macro { path, tokens, .. },
                ..
            }) => {
                let format = Format::from_str(&path.require_ident()?.to_string())
                    .ok_or(syn::Error::new_spanned(&path, "format not supported"))?;
                let source: macro_string::MacroString = syn::parse2(tokens)?;
                format.parse(&source.eval()?).map_err(|e| source.error(e))
            }
            item => Err(syn::Error::new_spanned(&item, "expecting macro"))?,
        })
        .sum::<syn::Result<_>>()?;

    let (ty, item_mod) = ModItems::from_value(&value).encapsulate(ident);
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
                pub const #ident: #ty = #ty;
            }
        }),
        export_item_static: args.export.r#static.map(|ident| {
            syn::parse_quote! {
                pub static #ident: #ty = #ty;
            }
        }),
    })
}

struct PrimitiveConvertItem<V> {
    ty_fn: fn() -> syn::Type,
    expr_fn: fn(&V) -> syn::Expr,
}

struct ContainerConvertItem<T> {
    ty_fn: fn(&syn::Ident) -> syn::Type,
    expr_fn: fn(&[T], &[syn::Expr]) -> syn::Expr,
}

struct ModItems {
    item_struct: syn::ItemStruct,
    item_impl_default: syn::ItemImpl,
    item_impls_index_key: Vec<syn::ItemImpl>,
    item_impls_index_path_nil: syn::ItemImpl,
    item_impls_index_path_cons: syn::ItemImpl,
    item_impls_from_primitive: Vec<syn::ItemImpl>,
    item_impls_from_container: Vec<syn::ItemImpl>,
    item_mods: Vec<syn::ItemMod>,
}

struct __A<T> {
    t: T,
}

static A: __A<()> = __A { t: () };

type A = __A<()>;

macro_rules! toml {
    ($($_:tt)*) => {
        i32
    };
}

pub struct Abc(
    toml!(
        r#"
        title = "TOML Example"

        [owner]
        name = "Tom Preston-Werner"
        dob = "1979-05-27"
        date-of-birth = "1979-05-27"
        mod = "toml"

        [database]
        server = "192.168.1.1"
        ports = [ 8000, 8001, 8002 ]
        connection_max = 5000
        enabled = true

        [servers.alpha]
        ip = "10.0.0.1"
        dc = "eqdc10"

        [servers.beta]
        ip = "10.0.0.2"
        dc = "eqdc10"

        [clients]
        data = [ ["gamma", "delta"], [1, 2] ]
        hosts = [
          "alpha",
          "omega"
        ]

        [languages]
        json = 2001
        yaml = 2001
        toml = 2013
        "#
    ),
);

impl ModItems {
    fn from_nil() -> Self {
        Self::from_general(
            &(),
            |_| Vec::new(),
            |_: &std::convert::Infallible| unreachable!(),
            [],
            [],
        )
    }

    fn from_primitive<V, const PN: usize>(
        value: &V,
        primitive_convert_items: [PrimitiveConvertItem<V>; PN],
    ) -> Self {
        Self::from_general(
            value,
            |_| Vec::new(),
            |_: &std::convert::Infallible| unreachable!(),
            primitive_convert_items,
            [],
        )
    }

    fn from_container<'v, V, T: 'v, const CN: usize>(
        value: &'v V,
        field_iter: fn(&'v V) -> Vec<(T, &'v Value)>,
        key_ty_fn: fn(&T) -> syn::Type,
        container_convert_items: [ContainerConvertItem<T>; CN],
    ) -> Self {
        Self::from_general(value, field_iter, key_ty_fn, [], container_convert_items)
    }

    fn from_general<'v, V, T: 'v, const PN: usize, const CN: usize>(
        value: &'v V,
        field_iter: fn(&'v V) -> Vec<(T, &'v Value)>,
        key_ty_fn: fn(&T) -> syn::Type,
        primitive_convert_items: [PrimitiveConvertItem<V>; PN],
        container_convert_items: [ContainerConvertItem<T>; CN],
    ) -> ModItems {
        let (tags, values): (Vec<_>, Vec<_>) = field_iter(value).into_iter().unzip();
        let (field_tys, item_mods): (Vec<_>, Vec<_>) = values
            .into_iter()
            .enumerate()
            .map(|(index, value)| {
                Self::from_value(value).encapsulate(quote::format_ident!("_{index}"))
            })
            .unzip();
        let item_impls_index_key = tags
            .iter()
            .map(key_ty_fn)
            .zip(field_tys.iter())
            .map(|(key_ty, field_ty)| {
                syn::parse_quote! {
                    impl ::std::ops::Index<#key_ty> for Type {
                        type Output = #field_ty;

                        fn index(&self, _index: #key_ty) -> &Self::Output {
                            &#field_ty
                        }
                    }
                }
            })
            .collect();
        let item_impls_index_path_nil = syn::parse_quote! {
            impl ::std::ops::Index<::inline_config::__private::PathNil> for Type {
                type Output = Type;

                fn index(&self, _index: ::inline_config::__private::PathNil) -> &Self::Output {
                    <&Type as Default>::default()
                }
            }
        };
        let item_impls_index_path_cons = syn::parse_quote! {
            impl<K, KS> ::std::ops::Index<::inline_config::__private::PathCons<K, KS>> for Type
            where
                Type: ::std::ops::Index<K, Output: ::std::ops::Index<KS, Output: 'static>>,
                &'static <<Type as ::std::ops::Index<K>>::Output as ::std::ops::Index<KS>>::Output: Default,
            {
                type Output = <<Type as ::std::ops::Index<K>>::Output as ::std::ops::Index<KS>>::Output;

                fn index(&self, _index: ::inline_config::__private::PathCons<K, KS>) -> &Self::Output {
                    <&<<Type as ::std::ops::Index<K>>::Output as ::std::ops::Index<KS>>::Output as Default>::default()
                }
            }
        };
        let item_impls_from_primitive = primitive_convert_items
            .iter()
            .map(|convert_item| {
                let ty = (convert_item.ty_fn)();
                let expr = (convert_item.expr_fn)(value);
                syn::parse_quote! {
                    impl From<Type> for #ty {
                        fn from(_value: Type) -> Self {
                            #expr
                        }
                    }
                }
            })
            .collect();
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
        let item_impls_from_container = container_convert_items
            .iter()
            .map(|convert_item| {
                let ty = (convert_item.ty_fn)(&generic);
                let expr = (convert_item.expr_fn)(&tags, &exprs);
                syn::parse_quote! {
                    impl<#generic> From<Type> for #ty
                    where
                        #(#where_predicates,)*
                    {
                        fn from(_value: Type) -> Self {
                            #expr
                        }
                    }
                }
            })
            .collect();
        Self {
            item_struct: syn::parse_quote! {
                #[derive(Clone, Copy, Default)]
                pub struct Type;
            },
            item_impl_default: syn::parse_quote! {
                impl Default for &Type {
                    fn default() -> Self {
                        &Type
                    }
                }
            },
            item_impls_index_key,
            item_impls_index_path_nil,
            item_impls_index_path_cons,
            item_impls_from_primitive,
            item_impls_from_container,
            item_mods,
        }
    }

    fn from_value(value: &Value) -> Self {
        macro_rules! numeric_convert_item {
            ($ty:ty) => {
                PrimitiveConvertItem {
                    ty_fn: || syn::parse_quote! { $ty },
                    expr_fn: |expr| syn::parse_quote! { #expr as $ty },
                }
            };
        }
        match value {
            Value::Nil => Self::from_nil(),
            Value::Boolean(value) => Self::from_primitive(
                value,
                [PrimitiveConvertItem {
                    ty_fn: || syn::parse_quote! { bool },
                    expr_fn: |expr| syn::parse_quote! { #expr },
                }],
            ),
            Value::PosInt(value) => Self::from_primitive(
                value,
                [
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
                ],
            ),
            Value::NegInt(value) => Self::from_primitive(
                value,
                [
                    numeric_convert_item!(i8),
                    numeric_convert_item!(i16),
                    numeric_convert_item!(i32),
                    numeric_convert_item!(i64),
                    numeric_convert_item!(i128),
                    numeric_convert_item!(isize),
                    numeric_convert_item!(f32),
                    numeric_convert_item!(f64),
                ],
            ),
            Value::Float(value) => Self::from_primitive(
                value,
                [numeric_convert_item!(f32), numeric_convert_item!(f64)],
            ),
            Value::String(value) => Self::from_primitive(
                value,
                [
                    PrimitiveConvertItem {
                        ty_fn: || syn::parse_quote! { &'static str },
                        expr_fn: |expr| syn::parse_quote! { #expr },
                    },
                    PrimitiveConvertItem {
                        ty_fn: || syn::parse_quote! { String },
                        expr_fn: |expr| syn::parse_quote! { #expr.to_string() },
                    },
                ],
            ),
            Value::Array(value) => Self::from_container(
                value,
                |value| value.iter().enumerate().collect(),
                |index| Key::index_ty(*index),
                [ContainerConvertItem {
                    ty_fn: |generic| syn::parse_quote! { Vec<#generic> },
                    expr_fn: |_tags, exprs| syn::parse_quote! { [#(#exprs),*].into() },
                }],
            ),
            Value::Table(value) => Self::from_container(
                value,
                |value| value.iter().collect(),
                |name| Key::name_ty(name),
                [
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
                ],
            ),
        }
    }

    fn encapsulate(&self, mod_ident: syn::Ident) -> (syn::Type, syn::ItemMod) {
        let Self {
            item_struct,
            item_impl_default,
            item_impls_index_key,
            item_impls_index_path_nil,
            item_impls_index_path_cons,
            item_impls_from_primitive,
            item_impls_from_container,
            item_mods,
        } = self;
        (
            syn::parse_quote! {
                #mod_ident::Type
            },
            syn::parse_quote! {
                pub mod #mod_ident {
                    #item_struct
                    #item_impl_default
                    #(#item_impls_index_key)*
                    #item_impls_index_path_nil
                    #item_impls_index_path_cons
                    #(#item_impls_from_primitive)*
                    #(#item_impls_from_container)*
                    #(#item_mods)*
                }
            },
        )
    }
}
