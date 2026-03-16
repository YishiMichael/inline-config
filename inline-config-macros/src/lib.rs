//! Proc macro implementations for [`inline_config`].
//!
//! [`inline_config`]: https://docs.rs/inline-config/latest/inline_config/

mod config;
mod format;
mod from_config;
mod path;
mod value;

fn emit_tokens_or_error<T: quote::ToTokens>(result: syn::Result<T>) -> proc_macro::TokenStream {
    match result {
        Ok(output) => output.into_token_stream(),
        Err(e) => e.to_compile_error(),
    }
    .into()
}

/// Attaches config data to a unit struct.
///
/// ### Attribute `format`
///
/// ```ignore
/// #[config(format = "toml")]
/// ```
///
/// All sources within a config type can only share the same format.
/// The format may be omitted if it is clear from extensions of included paths.
///
/// Note, Every format has a corresponding feature gate.
///
/// ### Attribute `src`
///
/// Config sources come in three flavors:
///
/// ```ignore
/// #[config(src = "<SRC_LITERAL>")]
/// #[config(src = include!("<PATH_LITERAL>"))]
/// #[config(src = include_env!("<PATH_LITERAL>"))]
/// ```
///
/// There can be an arbitrary number of sources, combined in arbitrary order, as long as they agree on the same format.
/// When there are multiple sources, they got merged recursively per field, with latter ones overwriting former ones.
///
/// When including files, the paths are resolved relative to the call site file.
/// `include_env!` specially supports environment variable interpolation -
/// environment variables of form `$ENV_VAR` are interpolated. Escape `$` with `$$`.
/// The support of environment variable interpolation is to aid any code analyzer to locate files,
/// as environment variables like `$CARGO_MANIFEST_DIR` and `$OUT_DIR` resolve to absolute paths.
/// This is mostly inspired by [include_dir](https://docs.rs/include_dir/latest/include_dir/) crate.
#[proc_macro_derive(Config, attributes(config))]
pub fn config(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    emit_tokens_or_error(syn::parse(item).and_then(config::config))
}

/// Defines a data structure that can be converted directly from a compatible container.
///
/// One needs to ensure all field types, if containing custom types, shall inherit [`FromConfig`] as well.
/// Use structs with unnamed fields to access from arrays; use structs with named fields to access from tables.
/// The fields do not necessarily need to be "full" - it may only contain a subset of fields in source data.
///
/// To avoid non-identifier key names occurred in source config (e.g. contains `-`), use `#[config(name = "...")]` on certain named fields.
/// Symmetrically, on unnamed fields you may want to use `#[config(index = ...)]` to remap array indices.
///
/// ```
/// use inline_config::FromConfig;
///
/// #[derive(FromConfig)]
/// struct MyStruct {
///     name: String, // matches "name"
///     #[config(name = "date-of-birth")]
///     date_of_birth: String, // matches "date-of-birth"
///     r#mod: String, // matches "mod"
/// }
/// ```
#[proc_macro_derive(FromConfig, attributes(config))]
pub fn from_config(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    emit_tokens_or_error(syn::parse(item).and_then(from_config::from_config))
}

/// Constructs a path with which one accesses a nested-in piece of data from config.
///
/// A path can be constructed by a sequence of keys, separated by `.`, i.e.
///
/// ```ignore
/// path!(<KEY>.<KEY>.<KEY>)
/// ```
///
/// Every `<KEY>` can be either an index (access an array field) or a name (access a table field).
/// The name may be quoted if it is not a valid identifier (e.g. contains `-`).
/// The following are all valid keys:
///
/// * `key`
/// * `"key"`
/// * `"key-in-kebab-case"`
/// * `0`
#[proc_macro]
pub fn path(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    emit_tokens_or_error(syn::parse(input).map(path::Path::expr))
}

/// The type version of [`path!()`]. Used in type bounds.
#[proc_macro]
#[allow(non_snake_case)]
pub fn Path(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    emit_tokens_or_error(syn::parse(input).map(path::Path::ty))
}
