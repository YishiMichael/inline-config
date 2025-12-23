use super::convert;
use config::{File, FileFormat, Map, Source, Value, ValueKind};

pub(crate) struct ConfigItems {
    items: Vec<ConfigItem>,
}

struct ConfigItem {
    attrs: Vec<syn::Attribute>,
    vis: syn::Visibility,
    static_token: syn::Token![static],
    name: syn::Ident,
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
            vis: input.parse::<syn::Visibility>()?,
            static_token: input.parse::<syn::Token![static]>()?,
            name: input.parse::<syn::Ident>()?,
            eq_token: input.parse::<syn::Token![=]>()?,
            sources: syn::punctuated::Punctuated::parse_separated_nonempty(input)?,
            semi_token: input.parse::<syn::Token![;]>()?,
        })
    }
}

impl quote::ToTokens for ConfigItem {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let ConfigItem {
            attrs,
            vis,
            static_token,
            name,
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
        let ConfigItemCollector {
            ty,
            expr,
            struct_items,
            select_impls,
            convert_impls,
        } = ConfigItemCollector::from_value(value, &quote::format_ident!("__{name}_"), &vis);
        let static_item: syn::ItemStatic = syn::parse_quote! {
            #(#attrs)*
            #vis #static_token #name: #ty #eq_token #expr #semi_token
        };
        static_item.to_tokens(tokens);
        struct_items
            .iter()
            .for_each(|struct_item| struct_item.to_tokens(tokens));
        select_impls
            .iter()
            .for_each(|select_impl| select_impl.to_tokens(tokens));
        convert_impls
            .iter()
            .for_each(|convert_impl| convert_impl.to_tokens(tokens));
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
    }
}

struct ConfigItemCollector {
    ty: syn::Type,
    expr: syn::Expr,
    struct_items: Vec<syn::ItemStruct>,
    select_impls: Vec<syn::ItemImpl>,
    convert_impls: Vec<syn::ItemImpl>,
}

impl ConfigItemCollector {
    fn from_value(value: Value, ident: &syn::Ident, vis: &syn::Visibility) -> Self {
        match value.kind {
            ValueKind::Nil => {
                proc_macro_error::abort_call_site!("cannot handle Nil type");
                // Self::from_primitive(syn::parse_quote! { () }, syn::parse_quote! { () })
            }
            ValueKind::Boolean(value) => {
                Self::from_primitive(syn::parse_quote! { bool }, syn::parse_quote! { #value })
            }
            ValueKind::I64(value) => {
                Self::from_primitive(syn::parse_quote! { i64 }, syn::parse_quote! { #value })
            }
            ValueKind::I128(value) => {
                Self::from_primitive(syn::parse_quote! { i128 }, syn::parse_quote! { #value })
            }
            ValueKind::U64(value) => {
                Self::from_primitive(syn::parse_quote! { u64 }, syn::parse_quote! { #value })
            }
            ValueKind::U128(value) => {
                Self::from_primitive(syn::parse_quote! { u128 }, syn::parse_quote! { #value })
            }
            ValueKind::Float(value) => {
                Self::from_primitive(syn::parse_quote! { f64 }, syn::parse_quote! { #value })
            }
            ValueKind::String(value) => Self::from_primitive(
                syn::parse_quote! { &'static str },
                syn::parse_quote! { #value },
            ),
            ValueKind::Table(value) => {
                let (names, values): (Vec<_>, Vec<_>) = value.into_iter().unzip();
                Self::from_container(Some(names), values, ident, vis)
            }
            ValueKind::Array(value) => Self::from_container(None, value, ident, vis),
        }
    }

    fn from_primitive(ty: syn::Type, expr: syn::Expr) -> Self {
        Self {
            ty,
            expr,
            struct_items: Vec::new(),
            select_impls: Vec::new(),
            convert_impls: Vec::new(),
        }
    }

    fn from_container(
        names: Option<Vec<String>>,
        values: Vec<Value>,
        ident: &syn::Ident,
        vis: &syn::Visibility,
    ) -> Self {
        let mut fields_ty = Vec::new();
        let mut fields_expr = Vec::new();
        let mut struct_items = Vec::new();
        let mut select_impls = Vec::new();
        let mut convert_impls = Vec::new();
        for (index, value) in values.into_iter().enumerate() {
            let ConfigItemCollector {
                ty: field_ty,
                expr: field_expr,
                struct_items: field_struct_items,
                select_impls: field_select_impls,
                convert_impls: field_convert_impls,
            } = Self::from_value(value, &quote::format_ident!("{ident}_{index}"), vis);
            fields_ty.push(field_ty);
            fields_expr.push(field_expr);
            struct_items.extend(field_struct_items);
            select_impls.extend(field_select_impls);
            convert_impls.extend(field_convert_impls);
        }
        select_impls.extend(convert::select_representation(ident, &names, &fields_ty));
        convert_impls.extend(convert::representation_into_containers(
            ident, &names, &fields_ty,
        ));
        struct_items.push(syn::parse_quote! {
            #vis struct #ident(#(#fields_ty),*);
        });
        Self {
            ty: syn::parse_quote! { #ident },
            expr: syn::parse_quote! {
                #ident(#(#fields_expr,)*)
            },
            struct_items,
            select_impls,
            convert_impls,
        }
    }
}
