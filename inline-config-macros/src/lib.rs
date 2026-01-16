mod config_data;
mod config_repr;
mod format;
mod path;
mod value;

fn delegate_macro<I, T>(f: fn(I) -> T, input: proc_macro::TokenStream) -> proc_macro::TokenStream
where
    I: syn::parse::Parse,
    T: quote::ToTokens,
{
    match syn::parse(input) {
        Ok(input) => f(input).into_token_stream().into(),
        Err(e) => proc_macro_error::abort!(e.span(), e),
    }
}

#[proc_macro_error::proc_macro_error]
#[proc_macro_attribute]
pub fn config(
    input: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    match syn::parse::<syn::Ident>(input) {
        Ok(ident) => match ident.to_string().as_str() {
            #[cfg(feature = "json")]
            "json" => delegate_macro(
                std::convert::identity::<config_repr::ConfigItem<format::json::JsonFormat>>,
                item,
            ),

            #[cfg(feature = "yaml")]
            "yaml" => delegate_macro(
                std::convert::identity::<config_repr::ConfigItem<format::yaml::YamlFormat>>,
                item,
            ),

            #[cfg(feature = "toml")]
            "toml" => delegate_macro(
                std::convert::identity::<config_repr::ConfigItem<format::toml::TomlFormat>>,
                item,
            ),

            _ => proc_macro_error::abort!(ident, "unsupported format"),
        },
        Err(e) => proc_macro_error::abort!(e.span(), e),
    }
}

#[proc_macro_error::proc_macro_error]
#[proc_macro]
pub fn path(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    delegate_macro(path::Path::expr, input)
}

#[proc_macro_error::proc_macro_error]
#[proc_macro]
#[allow(non_snake_case)]
pub fn Path(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    delegate_macro(path::Path::ty, input)
}

#[proc_macro_error::proc_macro_error]
#[proc_macro_derive(ConfigData, attributes(config_data))]
pub fn config_data(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    delegate_macro(config_data::config_data, item)
}
