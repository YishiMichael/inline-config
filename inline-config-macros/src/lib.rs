mod config;
mod config_data;
mod format;
mod path;
mod value;

fn emit_impl_or_error<T: quote::ToTokens>(result: syn::Result<T>) -> proc_macro::TokenStream {
    match result {
        Ok(output) => output.into_token_stream().into(),
        Err(e) => proc_macro_error::abort!(e.span(), e),
    }
}

#[proc_macro_error::proc_macro_error]
#[proc_macro_attribute]
pub fn config(
    input: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    emit_impl_or_error(
        syn::parse(input)
            .and_then(|input| syn::parse(item).and_then(|item| config::config(input, item))),
    )
}

#[proc_macro_error::proc_macro_error]
#[proc_macro_derive(ConfigData, attributes(config_data))]
pub fn config_data(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    emit_impl_or_error(syn::parse(item).and_then(config_data::config_data))
}

#[proc_macro_error::proc_macro_error]
#[proc_macro]
pub fn path(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    emit_impl_or_error(syn::parse(input).map(path::Path::expr))
}

#[proc_macro_error::proc_macro_error]
#[proc_macro]
#[allow(non_snake_case)]
pub fn Path(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    emit_impl_or_error(syn::parse(input).map(path::Path::ty))
}
