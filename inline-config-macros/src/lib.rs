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

#[cfg(feature = "json")]
#[proc_macro_error::proc_macro_error]
#[proc_macro]
pub fn json_config(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    delegate_macro(
        std::convert::identity::<config_repr::ConfigBlock<format::json::JsonFormat>>,
        input,
    )
}

#[cfg(feature = "yaml")]
#[proc_macro_error::proc_macro_error]
#[proc_macro]
pub fn yaml_config(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    delegate_macro(
        std::convert::identity::<config_repr::ConfigBlock<format::yaml::YamlFormat>>,
        input,
    )
}

#[cfg(feature = "toml")]
#[proc_macro_error::proc_macro_error]
#[proc_macro]
pub fn toml_config(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    delegate_macro(
        std::convert::identity::<config_repr::ConfigBlock<format::toml::TomlFormat>>,
        input,
    )
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
pub fn config_data(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    delegate_macro(config_data::config_data, input)
}
