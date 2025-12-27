use crate::impls::ConfigReprStructure;

use super::impls::{ArraySlot, ContainerStructure, TableSlot};
use config::{File, FileFormat, Map, Source, Value, ValueKind};

pub(crate) struct ConfigItems {
    items: Vec<ConfigItem>,
}

struct ConfigItem {
    attrs: Vec<syn::Attribute>,
    vis: syn::Visibility,
    static_token: syn::Token![static],
    ident: syn::Ident,
    eq_token: syn::Token![=],
    sources: syn::punctuated::Punctuated<ConfigSource, syn::Token![+]>,
    semi_token: syn::Token![;],
}

struct ConfigSource(Box<dyn Source>);

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
            sources: syn::punctuated::Punctuated::parse_separated_nonempty(input)?,
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
            sources,
            semi_token,
        } = self;
        let value = match sources.into_iter().try_fold(
            Value::from(Map::<String, Value>::new()),
            |mut value, source| {
                source.0.collect_to(&mut value)?;
                Ok::<_, config::ConfigError>(value)
            },
        ) {
            Ok(value) => value,
            Err(e) => proc_macro_error::abort_call_site!(e),
        };

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

impl syn::parse::Parse for ConfigSource {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if let Ok(file_name) = input.parse::<syn::LitStr>() {
            return Ok(Self(Box::new(File::with_name(file_name.value().as_str()))));
        }
        if let Ok(syn::Macro {
            path: format,
            tokens: content_tokens,
            ..
        }) = input.parse()
        {
            let format = match format.require_ident()?.clone().to_string().as_str() {
                "toml" => FileFormat::Toml,
                "json" => FileFormat::Json,
                "yaml" => FileFormat::Yaml,
                "ini" => FileFormat::Ini,
                "ron" => FileFormat::Ron,
                "json5" => FileFormat::Json5,
                _ => {
                    Err(input.error("supported config formats: toml, json, yaml, ini, ron, json5"))?
                }
            };
            let content: syn::LitStr = syn::parse2(content_tokens)?;
            return Ok(Self(Box::new(File::from_str(
                content.value().as_str(),
                format,
            ))));
        }
        Err(input.error("expected a string literal or a macro invocation"))
        // TODO: include! ?
    }
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
    fn from_value(value: &config::Value, ident: &syn::Ident, vis: &syn::Visibility) -> Self {
        match &value.kind {
            ValueKind::Nil => Self::primitive(syn::parse_quote! { () }, syn::parse_quote! { () }),
            ValueKind::Boolean(value) => {
                Self::primitive(syn::parse_quote! { bool }, syn::parse_quote! { #value })
            }
            ValueKind::I64(value) => {
                Self::primitive(syn::parse_quote! { i64 }, syn::parse_quote! { #value })
            }
            ValueKind::I128(value) => {
                let value = &(*value as i64);
                Self::primitive(syn::parse_quote! { i64 }, syn::parse_quote! { #value })
            }
            ValueKind::U64(value) => {
                let value = &(*value as i64);
                Self::primitive(syn::parse_quote! { i64 }, syn::parse_quote! { #value })
            }
            ValueKind::U128(value) => {
                let value = &(*value as i64);
                Self::primitive(syn::parse_quote! { i64 }, syn::parse_quote! { #value })
            }
            ValueKind::Float(value) => {
                Self::primitive(syn::parse_quote! { f64 }, syn::parse_quote! { #value })
            }
            ValueKind::String(value) => Self::primitive(
                syn::parse_quote! { &'static str },
                syn::parse_quote! { #value },
            ),
            ValueKind::Array(value) => {
                let (slots, values): (Vec<_>, Vec<_>) = value
                    .iter()
                    .enumerate()
                    .map(|(index, value)| (ArraySlot { index }, value))
                    .unzip();
                Self::dispatch(ContainerStructure { slots }, values.as_slice(), ident, vis)
            }
            ValueKind::Table(value) => {
                let (slots, values): (Vec<_>, Vec<_>) = value
                    .iter()
                    .enumerate()
                    .map(|(index, (name, value))| {
                        (
                            TableSlot {
                                name: name.to_owned(),
                                ident: syn::parse_str::<syn::Ident>(name)
                                    .ok()
                                    .filter(|_| !name.chars().all(|c| matches!(c, '0'..'9' | '_')))
                                    .unwrap_or_else(|| quote::format_ident!("_{index}")),
                            },
                            value,
                        )
                    })
                    .unzip();
                Self::dispatch(ContainerStructure { slots }, values.as_slice(), ident, vis)
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
        values: &[&config::Value],
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
            impl ::inline_config::__private::convert::NonNilRepr for #ident {}
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
