mod config;
mod config_data;
mod format;
mod path;
mod value;

fn delegate_macro<I, T>(
    f: fn(I) -> syn::Result<T>,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream
where
    I: syn::parse::Parse,
    T: quote::ToTokens,
{
    match syn::parse(input).and_then(f) {
        Ok(output) => output.into_token_stream().into(),
        Err(e) => proc_macro_error::abort!(e.span(), e),
    }
}

#[proc_macro_error::proc_macro_error]
#[proc_macro_attribute]
pub fn config(
    _input: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    delegate_macro(config::config, item)
}

#[proc_macro_error::proc_macro_error]
#[proc_macro_derive(ConfigData, attributes(config_data))]
pub fn config_data(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    delegate_macro(config_data::config_data, item)
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
