use super::convert;
use super::key::KeySegment;
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

enum ConfigSource {
    Include {
        file_name: syn::LitStr,
    },
    Literal {
        format: syn::Ident,
        content: syn::LitStr,
    },
}

impl syn::parse::Parse for ConfigItems {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut items = vec![];
        while !input.is_empty() {
            items.push(input.parse()?);
        }
        Ok(Self { items })
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

impl syn::parse::Parse for ConfigSource {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if let Ok(file_name) = input.parse() {
            return Ok(Self::Include { file_name });
        }
        if let Ok(syn::Macro {
            path: format,
            tokens: content_tokens,
            ..
        }) = input.parse()
        {
            let format = format.require_ident()?.clone();
            let content = syn::parse2(content_tokens)?;
            return Ok(Self::Literal { format, content });
        }
        Err(input.error("expected a string literal or a macro invocation"))
    }
}

fn parse_value(sources: impl IntoIterator<Item = ConfigSource>) -> Value {
    let mut value: Value = Map::<String, Value>::new().into();
    for source in sources {
        match source {
            ConfigSource::Include { file_name } => {
                if let Err(e) = File::with_name(file_name.value().as_str()).collect_to(&mut value) {
                    proc_macro_error::emit_error!(file_name, e);
                }
            }
            ConfigSource::Literal { format, content } => {
                let format = match format.to_string().as_str() {
                    "toml" => FileFormat::Toml,
                    "json" => FileFormat::Json,
                    "yaml" => FileFormat::Yaml,
                    "ini" => FileFormat::Ini,
                    "ron" => FileFormat::Ron,
                    "json5" => FileFormat::Json5,
                    _ => {
                        proc_macro_error::emit_error!(
                            format,
                            "supported config formats: toml, json, yaml, ini, ron, json5",
                        );
                        continue;
                    }
                };
                if let Err(e) =
                    File::from_str(content.value().as_str(), format).collect_to(&mut value)
                {
                    proc_macro_error::emit_error!(content, e);
                }
            }
        }
    }
    value
}

struct ConfigItemCollector {
    ty: syn::Type,
    expr: syn::Expr,
    structs: Vec<syn::ItemStruct>,
    select_impls: Vec<syn::ItemImpl>,
    convert_impls: Vec<syn::ItemImpl>,
}

impl ConfigItemCollector {
    fn from_value(value: Value, ident: &syn::Ident, vis: &syn::Visibility) -> Self {
        match value.kind {
            ValueKind::Nil => Self::from_nil(),
            ValueKind::Boolean(value) => Self::from_bool(value),
            ValueKind::I64(value) => Self::from_integer(value),
            ValueKind::I128(value) => Self::from_integer(value as i64),
            ValueKind::U64(value) => Self::from_integer(value as i64),
            ValueKind::U128(value) => Self::from_integer(value as i64),
            ValueKind::Float(value) => Self::from_float(value),
            ValueKind::String(value) => Self::from_string(value),
            ValueKind::Table(value) => Self::from_table(value, ident, vis),
            ValueKind::Array(value) => Self::from_array(value, ident, vis),
        }
    }

    fn from_nil() -> Self {
        // TODO: Option?
        Self::from_primitive(syn::parse_quote! { () }, syn::parse_quote! { () })
    }

    fn from_bool(value: bool) -> Self {
        Self::from_primitive(syn::parse_quote! { bool }, syn::parse_quote! { #value })
    }

    fn from_integer(value: i64) -> Self {
        Self::from_primitive(syn::parse_quote! { i64 }, syn::parse_quote! { #value })
    }

    fn from_float(value: f64) -> Self {
        Self::from_primitive(syn::parse_quote! { f64 }, syn::parse_quote! { #value })
    }

    fn from_string(value: String) -> Self {
        Self::from_primitive(
            syn::parse_quote! { &'static str },
            syn::parse_quote! { #value },
        )
    }

    fn from_table(value: Map<String, Value>, ident: &syn::Ident, vis: &syn::Visibility) -> Self {
        Self::from_container(
            value.into_iter().map(|(name, value)| {
                let name = quote::format_ident!("{}", name.replace("-", "_")); // TODO: allow -
                (KeySegment::name_ty(name.to_string().as_str()), name, value)
            }),
            ident,
            vis,
        )
    }

    fn from_array(value: Vec<Value>, ident: &syn::Ident, vis: &syn::Visibility) -> Self {
        Self::from_container(
            value.into_iter().enumerate().map(|(index, value)| {
                (
                    KeySegment::index_ty(index),
                    quote::format_ident!("_{index}_"),
                    value,
                )
            }),
            ident,
            vis,
        )
    }

    fn from_primitive(ty: syn::Type, expr: syn::Expr) -> Self {
        Self {
            ty,
            expr,
            structs: Vec::new(),
            select_impls: Vec::new(),
            convert_impls: Vec::new(),
        }
    }

    fn from_container(
        fields: impl Iterator<Item = (syn::Type, syn::Ident, Value)>,
        ident: &syn::Ident,
        vis: &syn::Visibility,
    ) -> Self {
        let mut fields_name = Vec::new();
        let mut fields_ty = Vec::new();
        let mut fields_expr = Vec::new();
        let mut structs = Vec::new();
        let mut select_impls = Vec::new();
        let mut convert_impls = Vec::new();
        for (key_segment_ty, field_name, value) in fields {
            let ConfigItemCollector {
                ty: field_ty,
                expr: field_expr,
                structs: field_structs,
                select_impls: field_select_impls,
                convert_impls: field_convert_impls,
            } = Self::from_value(value, &quote::format_ident!("{ident}__{field_name}"), vis);
            select_impls.push(syn::parse_quote! {
                impl<'c> ::inline_config::__private::Select<'c, #key_segment_ty> for #ident {
                    type Representation = #field_ty;

                    fn select(&'c self, _key_segment: #key_segment_ty) -> &'c Self::Representation {
                        &self.#field_name
                    }
                }
            });
            fields_name.push(field_name);
            fields_ty.push(field_ty);
            fields_expr.push(field_expr);
            structs.extend(field_structs);
            select_impls.extend(field_select_impls);
            convert_impls.extend(field_convert_impls);
        }
        convert_impls.extend(convert::representation_into_container(
            ident,
            &fields_name,
            &fields_ty,
        ));
        structs.push(syn::parse_quote! {
            #vis struct #ident {
                #(#fields_name: #fields_ty,)*
            }
        });
        Self {
            ty: syn::parse_quote! { #ident },
            expr: syn::parse_quote! {
                #ident {
                    #(#fields_name: #fields_expr,)*
                }
            },
            structs,
            select_impls,
            convert_impls,
        }
    }
}

fn config_item_ts(config_item: ConfigItem) -> proc_macro2::TokenStream {
    let ConfigItem {
        attrs,
        vis,
        static_token,
        name,
        eq_token,
        sources,
        semi_token,
    } = config_item;
    let ConfigItemCollector {
        ty,
        expr,
        structs,
        select_impls,
        convert_impls,
    } = ConfigItemCollector::from_value(
        parse_value(sources),
        &quote::format_ident!("__{name}"),
        &vis,
    );
    let static_item: syn::ItemStatic = syn::parse_quote! {
        #(#attrs)*
        #vis #static_token #name: #ty #eq_token #expr #semi_token
    };
    quote::quote! {
        #static_item
        #(#structs)*
        #(#select_impls)*
        #(#convert_impls)*
    }
}

pub(crate) fn config_items_ts(config_items: ConfigItems) -> proc_macro2::TokenStream {
    proc_macro2::TokenStream::from_iter(config_items.items.into_iter().map(config_item_ts))
}
