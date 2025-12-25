use super::path::Key;
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
        let (ty, expr) = value_ty_expr(value);
        let static_item: syn::ItemStatic = syn::parse_quote! {
            #(#attrs)*
            #vis #static_token #name: #ty #eq_token #expr #semi_token
        };
        static_item.to_tokens(tokens);
    }
}

fn value_ty_expr(value: Value) -> (syn::Type, syn::Expr) {
    match value.kind {
        ValueKind::Nil => (
            syn::parse_quote! { ::inline_config::__private::repr::Nil },
            syn::parse_quote! { ::inline_config::__private::repr::Nil },
        ),
        ValueKind::Boolean(value) => (
            syn::parse_quote! { ::inline_config::__private::repr::Bool },
            syn::parse_quote! { ::inline_config::__private::repr::Bool(#value) },
        ),
        ValueKind::I64(value) => (
            syn::parse_quote! { ::inline_config::__private::repr::Integer },
            syn::parse_quote! { ::inline_config::__private::repr::Integer(#value) },
        ),
        ValueKind::I128(value) => {
            let value = value as i64;
            (
                syn::parse_quote! { ::inline_config::__private::repr::Integer },
                syn::parse_quote! { ::inline_config::__private::repr::Integer(#value) },
            )
        }
        ValueKind::U64(value) => {
            let value = value as i64;
            (
                syn::parse_quote! { ::inline_config::__private::repr::Integer },
                syn::parse_quote! { ::inline_config::__private::repr::Integer(#value) },
            )
        }
        ValueKind::U128(value) => {
            let value = value as i64;
            (
                syn::parse_quote! { ::inline_config::__private::repr::Integer },
                syn::parse_quote! { ::inline_config::__private::repr::Integer(#value) },
            )
        }
        ValueKind::Float(value) => (
            syn::parse_quote! { ::inline_config::__private::repr::Float },
            syn::parse_quote! { ::inline_config::__private::repr::Float(#value) },
        ),
        ValueKind::String(value) => (
            syn::parse_quote! { ::inline_config::__private::repr::StaticStr },
            syn::parse_quote! { ::inline_config::__private::repr::StaticStr(#value) },
        ),
        ValueKind::Array(value) => value.into_iter().enumerate().rfold((
            syn::parse_quote! { ::inline_config::__private::repr::HNil },
            syn::parse_quote! { ::inline_config::__private::repr::HNil },
        ), |(tail_ty, tail_expr), (index, value)| {
            let key_ty = Key::index_ty(index);
            let key_expr = Key::index_expr(index);
            let (value_ty, value_expr) = value_ty_expr(value);
            (
                syn::parse_quote! {
                    ::inline_config::__private::repr::HCons<::inline_config::__private::repr::Field<#key_ty, #value_ty>, #tail_ty>
                },
                syn::parse_quote! {
                    ::inline_config::__private::repr::HCons {
                        head: ::inline_config::__private::repr::Field {
                            key: #key_expr,
                            value: #value_expr,
                        },
                        tail: #tail_expr,
                    }
                },
            )
        }),
        ValueKind::Table(value) => value.into_iter().rfold((
            syn::parse_quote! { ::inline_config::__private::repr::HNil },
            syn::parse_quote! { ::inline_config::__private::repr::HNil },
        ), |(tail_ty, tail_expr), (name, value)| {
            let key_ty = Key::name_ty(name.as_str());
            let key_expr = Key::name_expr(name.as_str());
            let (value_ty, value_expr) = value_ty_expr(value);
            (
                syn::parse_quote! {
                    ::inline_config::__private::repr::HCons<::inline_config::__private::repr::Field<#key_ty, #value_ty>, #tail_ty>
                },
                syn::parse_quote! {
                    ::inline_config::__private::repr::HCons {
                        head: ::inline_config::__private::repr::Field {
                            key: #key_expr,
                            value: #value_expr,
                        },
                        tail: #tail_expr,
                    }
                },
            )
        }),
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
