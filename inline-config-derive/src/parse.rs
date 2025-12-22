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
    type_ts: proc_macro2::TokenStream,
    value_ts: proc_macro2::TokenStream,
    struct_items: Vec<proc_macro2::TokenStream>,
    select_impl_items: Vec<proc_macro2::TokenStream>,
    convert_impl_items: Vec<proc_macro2::TokenStream>,
}

impl ConfigItemCollector {
    fn from_value(value: Value, type_name: &str, vis: &syn::Visibility) -> Self {
        match value.kind {
            ValueKind::Nil => Self::from_nil(),
            ValueKind::Boolean(value) => Self::from_bool(value),
            ValueKind::I64(value) => Self::from_integer(value),
            ValueKind::I128(value) => Self::from_integer(value as i64),
            ValueKind::U64(value) => Self::from_integer(value as i64),
            ValueKind::U128(value) => Self::from_integer(value as i64),
            ValueKind::Float(value) => Self::from_float(value),
            ValueKind::String(value) => Self::from_string(value),
            ValueKind::Table(value) => Self::from_table(value, type_name, vis),
            ValueKind::Array(value) => Self::from_array(value, type_name, vis),
        }
    }

    fn from_nil() -> Self {
        // TODO: Option?
        Self::from_primitive(
            quote::quote! {
                ()
            },
            quote::quote! {
                ()
            },
        )
    }

    fn from_bool(value: bool) -> Self {
        Self::from_primitive(
            quote::quote! {
                bool
            },
            quote::quote! {
                #value
            },
        )
    }

    fn from_integer(value: i64) -> Self {
        Self::from_primitive(
            quote::quote! {
                i64
            },
            quote::quote! {
                #value
            },
        )
    }

    fn from_float(value: f64) -> Self {
        Self::from_primitive(
            quote::quote! {
                f64
            },
            quote::quote! {
                #value
            },
        )
    }

    fn from_string(value: String) -> Self {
        Self::from_primitive(
            quote::quote! {
                &'static str
            },
            quote::quote! {
                #value
            },
        )
    }

    fn from_table(value: Map<String, Value>, type_name: &str, vis: &syn::Visibility) -> Self {
        Self::from_container(
            value.into_iter().map(|(name, value)| {
                let name = name.replace("-", "_"); // TODO: allow -
                (KeySegment::name_type_ts(name.as_str()), name, value)
            }),
            type_name,
            vis,
        )
    }

    fn from_array(value: Vec<Value>, type_name: &str, vis: &syn::Visibility) -> Self {
        Self::from_container(
            value.into_iter().enumerate().map(|(index, value)| {
                (
                    KeySegment::index_type_ts(index),
                    format!("_{index}_"),
                    value,
                )
            }),
            type_name,
            vis,
        )
    }

    fn from_primitive(
        type_ts: proc_macro2::TokenStream,
        value_ts: proc_macro2::TokenStream,
    ) -> Self {
        Self {
            type_ts,
            value_ts,
            struct_items: Vec::new(),
            select_impl_items: Vec::new(),
            convert_impl_items: Vec::new(),
        }
    }

    fn from_container(
        fields: impl Iterator<Item = (proc_macro2::TokenStream, String, Value)>,
        type_name: &str,
        vis: &syn::Visibility,
    ) -> Self {
        let type_ident = syn::Ident::new(type_name, proc_macro2::Span::call_site());
        let type_ts = quote::quote! { #type_ident };
        let mut names = Vec::new();
        let mut fields_type_ts = Vec::new();
        let mut fields_value_ts = Vec::new();
        let mut struct_items = Vec::new();
        let mut select_impl_items = Vec::new();
        let mut convert_impl_items = Vec::new();
        for (key_segment_type_ts, name, value) in fields {
            let ConfigItemCollector {
                type_ts: field_type_ts,
                value_ts: field_value_ts,
                struct_items: field_struct_items,
                select_impl_items: field_select_impl_items,
                convert_impl_items: field_convert_impl_items,
            } = Self::from_value(value, format!("{type_name}__{name}").as_str(), vis);
            let name = syn::Ident::new(name.as_str(), proc_macro2::Span::call_site());
            select_impl_items.push(quote::quote! {
                impl<'c> ::inline_config::__private::Select<'c, #key_segment_type_ts> for #type_ts {
                    type Representation = #field_type_ts;

                    fn select(&'c self, _key_segment: #key_segment_type_ts) -> &'c Self::Representation {
                        &self.#name
                    }
                }
            });
            names.push(name);
            fields_type_ts.push(field_type_ts);
            fields_value_ts.push(field_value_ts);
            struct_items.extend(field_struct_items);
            select_impl_items.extend(field_select_impl_items);
            convert_impl_items.extend(field_convert_impl_items);
        }
        // convert_impl_items.push(quote::quote! {});
        struct_items.push(quote::quote! {
            #vis struct #type_ts {
                #(#names: #fields_type_ts,)*
            }
        });
        let value_ts = quote::quote! {
            #type_ts {
                #(#names: #fields_value_ts,)*
            }
        };
        Self {
            type_ts,
            value_ts,
            struct_items,
            select_impl_items,
            convert_impl_items,
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
        type_ts,
        value_ts,
        struct_items,
        select_impl_items,
        convert_impl_items,
    } = ConfigItemCollector::from_value(parse_value(sources), format!("__{name}").as_str(), &vis);
    quote::quote! {
        #(#attrs)*
        #vis #static_token #name: #type_ts #eq_token #value_ts #semi_token

        #(#struct_items)*
        #(#select_impl_items)*
        #(#convert_impl_items)*
    }
}

pub(crate) fn config_items_ts(config_items: ConfigItems) -> proc_macro2::TokenStream {
    proc_macro2::TokenStream::from_iter(config_items.items.into_iter().map(config_item_ts))
}
