//! Proc macro implementations for [`inline_config`].
//!
//! [`inline_config`]: https://docs.rs/inline-config/latest/inline_config/

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

/// Declares a config module.
///
/// ```ignore
/// #[config]
/// <VIS> mod <IDENT> {
///     <FORMAT>!(<SRC>);
///     <FORMAT>!(<SRC>);
///     <FORMAT>!(<SRC>);
/// }
/// ```
/// When there are multiple sources, they got merged recursively per field, with latter ones overwriting former ones.
/// All null values need to be overwritten eventually.
///
/// Every `<SRC>` shall be a literal string, or a macro invocation expanding into a literal string.
/// The full support of eager expansion is impossible without nightly-only feature [`proc_macro_expand`].
/// A subset of eager expansion for built-in macros is handled by [`macro_string`] crate, which identifies both the following as valid sources:
///
/// * `r#"name = "Tom""#`
/// * `include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/example_config.toml"))`
///
/// After expansion, a type `Type` and a static variable `EXPR` holding config data will become available inside the module, i.e.
///
/// ```ignore
/// <VIS> mod <IDENT> {
///     pub struct Type;
///     pub static EXPR: Type = Type;
///     // Other internal implementation details...
/// }
/// ```
///
/// Users now can freely use `<IDENT>::Type` and `<IDENT>::EXPR`.
/// Optionally, for convenience, you can export these items directly into the scope via `export` arguments:
///
/// ```ignore
/// #[config(export(type = <TYPE_IDENT>, const = <CONST_IDENT>, static = <STATIC_IDENT>))]
/// ```
///
/// Each will yield
///
/// | Argument | Generated item |
/// | --- | --- |
/// | `type = <TYPE_IDENT>` | `pub type <TYPE_IDENT> = <IDENT>::Type;` |
/// | `const = <CONST_IDENT>` | `pub const <CONST_IDENT>: <IDENT>::Type = <IDENT>::EXPR;` |
/// | `static = <STATIC_IDENT>` | `pub static <STATIC_IDENT>: <IDENT>::Type = <IDENT>::EXPR;` |
///
/// [`proc_macro_expand`]: https://github.com/rust-lang/rust/issues/90765
/// [`macro_string`]: https://docs.rs/macro-string/latest/macro_string/
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

/// Defines a data structure that can be converted directly from a compatible container.
///
/// One needs to ensure all field types, if containing custom types, shall inherit [`ConfigData`] as well.
/// Use structs with unnamed fields to access from arrays; use structs with named fields to access from tables.
/// The fields do not necessarily need to be "full" - it may only contain a subset of fields in source data.
///
/// To avoid non-identifier key names occurred in source config (e.g. contains `-`), use `#[config_data(rename = "...")]` on certain fields.
///
/// ```
/// use inline_config::ConfigData;
///
/// #[derive(ConfigData)]
/// struct MyStruct {
///     name: String, // matches "name"
///     #[config_data(rename = "date-of-birth")]
///     date_of_birth: String, // matches "date-of-birth"
///     r#mod: String, // matches "mod"
/// }
/// ```
#[proc_macro_error::proc_macro_error]
#[proc_macro_derive(ConfigData, attributes(config_data))]
pub fn config_data(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    emit_impl_or_error(syn::parse(item).and_then(config_data::config_data))
}

/// Constructs a path with which one accesses a nested-in piece of data from config.
///
/// A path can be constructed by a sequence of keys, separated by `.`.
/// A key can be either an index (access an array field) or a name (access a table field).
/// The name may be quoted if it is not a valid identifier (e.g. contains `-`).
#[proc_macro_error::proc_macro_error]
#[proc_macro]
pub fn path(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    emit_impl_or_error(syn::parse(input).map(path::Path::expr))
}

/// The type version of [`path!()`]. Used in type bounds.
#[proc_macro_error::proc_macro_error]
#[proc_macro]
#[allow(non_snake_case)]
pub fn Path(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    emit_impl_or_error(syn::parse(input).map(path::Path::ty))
}
