use super::structures::{
    ArraySlot, ArrayTypedSlot, ConfigDataStructure, ConfigReprStructure, ContainerStructure,
    TableSlot, TableTypedSlot, UnitStructure,
};
use config::{File, FileFormat, Map, Source, Value, ValueKind};
use darling::FromField;

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

        let ConfigItemTokens {
            ty,
            expr,
            struct_items,
            access_key_impls,
            convert_into_impls,
        } = ConfigItemTokens::from_value(&value, &quote::format_ident!("__{name}_"), &vis);

        let static_item: syn::ItemStatic = syn::parse_quote! {
            #(#attrs)*
            #vis #static_token #name: #ty #eq_token #expr #semi_token
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

struct ConfigItemTokens {
    ty: syn::Type,
    expr: syn::Expr,
    struct_items: Vec<syn::ItemStruct>,
    access_key_impls: Vec<syn::ItemImpl>,
    convert_into_impls: Vec<syn::ItemImpl>,
}

impl ConfigItemTokens {
    fn from_value(value: &Value, ident: &syn::Ident, vis: &syn::Visibility) -> Self {
        match &value.kind {
            ValueKind::Nil => {
                // proc_macro_error::abort_call_site!("cannot handle Nil type");
                Self::from_primitive(syn::parse_quote! { () }, syn::parse_quote! { () })
            }
            ValueKind::Boolean(value) => {
                Self::from_primitive(syn::parse_quote! { bool }, syn::parse_quote! { #value })
            }
            ValueKind::I64(value) => {
                Self::from_primitive(syn::parse_quote! { i64 }, syn::parse_quote! { #value })
            }
            ValueKind::I128(value) => {
                let value = &(*value as i64);
                Self::from_primitive(syn::parse_quote! { i64 }, syn::parse_quote! { #value })
            }
            ValueKind::U64(value) => {
                let value = &(*value as i64);
                Self::from_primitive(syn::parse_quote! { i64 }, syn::parse_quote! { #value })
            }
            ValueKind::U128(value) => {
                let value = &(*value as i64);
                Self::from_primitive(syn::parse_quote! { i64 }, syn::parse_quote! { #value })
            }
            ValueKind::Float(value) => {
                Self::from_primitive(syn::parse_quote! { f64 }, syn::parse_quote! { #value })
            }
            ValueKind::String(value) => Self::from_primitive(
                syn::parse_quote! { &'static str },
                syn::parse_quote! { #value },
            ),
            ValueKind::Array(value) => {
                let (slots, values): (Vec<_>, Vec<_>) = value
                    .iter()
                    .enumerate()
                    .map(|(index, value)| (ArraySlot { index }, value))
                    .unzip();
                Self::from_config_repr_structure(
                    ContainerStructure { slots },
                    values.as_slice(),
                    ident,
                    vis,
                )
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
                Self::from_config_repr_structure(
                    ContainerStructure { slots },
                    values.as_slice(),
                    ident,
                    vis,
                )
            }
        }
    }

    fn from_primitive(ty: syn::Type, expr: syn::Expr) -> Self {
        Self {
            ty,
            expr,
            struct_items: Vec::new(),
            access_key_impls: Vec::new(),
            convert_into_impls: Vec::new(),
        }
    }

    fn from_config_repr_structure<S>(
        config_repr_structure: S,
        values: &[&Value],
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
        for (index, value) in values.iter().enumerate() {
            let ConfigItemTokens {
                ty: field_ty,
                expr: field_expr,
                struct_items: field_struct_items,
                access_key_impls: field_access_key_impls,
                convert_into_impls: field_convert_into_impls,
            } = Self::from_value(value, &quote::format_ident!("{ident}_{index}"), vis);
            tys.push(field_ty);
            exprs.push(field_expr);
            struct_items.extend(field_struct_items);
            access_key_impls.extend(field_access_key_impls);
            convert_into_impls.extend(field_convert_into_impls);
        }
        struct_items.push(config_repr_structure.struct_item(ident, tys.as_slice(), vis));
        access_key_impls.extend(config_repr_structure.access_key_impls(ident, tys.as_slice()));
        convert_into_impls.extend(config_repr_structure.convert_into_impls(ident, tys.as_slice()));
        Self {
            ty: syn::parse_quote! { #ident },
            expr: config_repr_structure.expr(ident, exprs.as_slice()),
            struct_items,
            access_key_impls,
            convert_into_impls,
        }
    }
}

#[derive(FromField)]
#[darling(attributes(config_data))]
struct ConfigDataFieldAttrs {
    rename: Option<String>,
}

pub(crate) fn config_data(input: syn::ItemStruct) -> syn::ItemImpl {
    match &input.fields {
        syn::Fields::Unit => UnitStructure.convert_from_impl(&input.ident, &input.generics),
        syn::Fields::Unnamed(fields_unnamed) => ContainerStructure {
            slots: fields_unnamed
                .unnamed
                .iter()
                .enumerate()
                .map(|(index, field)| ArrayTypedSlot {
                    index,
                    ty: &field.ty,
                })
                .collect(),
        }
        .convert_from_impl(&input.ident, &input.generics),
        syn::Fields::Named(fields_named) => ContainerStructure {
            slots: fields_named
                .named
                .iter()
                .map(|field| {
                    let ident = field.ident.as_ref().unwrap();
                    let attrs = ConfigDataFieldAttrs::from_field(field)
                        .unwrap_or_else(|e| proc_macro_error::abort_call_site!(e));
                    TableTypedSlot {
                        name: attrs.rename.unwrap_or(ident.to_string()),
                        ident,
                        ty: &field.ty,
                    }
                })
                .collect(),
        }
        .convert_from_impl(&input.ident, &input.generics),
    }
}
