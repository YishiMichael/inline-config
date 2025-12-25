mod config;
mod config_data;
mod path;

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
#[proc_macro]
pub fn config(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    delegate_macro(std::convert::identity::<config::ConfigItems>, input)
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
