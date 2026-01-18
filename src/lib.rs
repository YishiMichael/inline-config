//! Effortlessly embed config as static data and access with any compatible data structures.
//!
//! ## Example
//!
//! Below is a basic example illustrating how to declare a config type and access data from it.
//!
//! ```
//! use inline_config::{config, path};
//!
//! // Declare a config module containing literal sources.
//! // With `export(static = MY_CONFIG)`, a static variable `MY_CONFIG` will be brought into scope.
//! #[config(export(static = MY_CONFIG))]
//! mod my_config {
//!     // When there are multiple sources, latter ones overwrite former ones.
//!     // Including a file from disk is also possible, see `examples/include.rs`.
//!     toml!(
//!     r#"
//!         title = "TOML example"
//!
//!         [server]
//!         owner = "Tom"
//!         timeout = 2000
//!         ports = [ 8000, 8001, 8002 ]
//!     "#
//!     );
//!     toml!(
//!     r#"
//!         [server]
//!         timeout = 5000
//!     "#
//!     );
//! }
//!
//! // Multiple types may implement `From` trait, so type annotations are required.
//! let title: &str = MY_CONFIG[path!(title)].into();
//! assert_eq!("TOML example", title);
//! let title: String = MY_CONFIG[path!(title)].into();
//! assert_eq!("TOML example", title);
//!
//! // A deeper path.
//! let owner: &str = MY_CONFIG[path!(server.owner)].into();
//! assert_eq!("Tom", owner);
//!
//! // Any numerical types.
//! let timeout: u32 = MY_CONFIG[path!(server.timeout)].into();
//! assert_eq!(5000, timeout);
//! let timeout: f32 = MY_CONFIG[path!(server.timeout)].into();
//!
//! // A homogeneous array can be accessed as `Vec<T>`.
//! let ports: Vec<u64> = MY_CONFIG[path!(server.ports)].into();
//! assert_eq!([8000, 8001, 8002].to_vec(), ports);
//! ```
//!
//! See [`config`] and [`path!()`] for specs on those macros.
//!
//! ## Compatible types
//!
//! Internally, data from config sources are parsed into one of the seven variants:
//! booleans, unsigned integers, signed integers, floats, strings, arrays, tables.
//! Each of them has a specific storage representation, and have different compatible types.
//!
//! | Representation variant | Compatible types |
//! |---|---|
//! | Boolean | [`bool`] |
//! | Unsigned Integer | [`i8`], [`i16`], [`i32`], [`i64`], [`i128`], [`isize`],<br>[`u8`], [`u16`], [`u32`], [`u64`], [`u128`], [`usize`],<br>[`f32`], [`f64`] |
//! | Signed Integer | [`i8`], [`i16`], [`i32`], [`i64`], [`i128`], [`isize`],<br>[`f32`], [`f64`] |
//! | Float | [`f32`], [`f64`] |
//! | String | [`&str`], [`String`] |
//! | Array | [`Vec<T>`] if homogeneous,<br>User-defined structs with unnamed fields |
//! | Table | [`std::collections::BTreeMap<&str, T>`] if homogeneous,<br>[`std::collections::BTreeMap<String, T>`] if homogeneous,<br>[`indexmap::IndexMap<&str, T>`] if homogeneous\*,<br>[`indexmap::IndexMap<String, T>`] if homogeneous\*,<br>User-defined structs with named fields |
//!
//! \* Only available when enabling `indexmap` feature flag.
//!
//! ### Container types
//!
//! Arrays and tables are both "containers" in the sense of containing children data, therefore you can use [`path!()`] to access children data.
//! The only difference between the two containers is that arrays have unnamed but ordered fields, while tables have named but unamed fields.
//! This suggests you should use indices when accessing a field of an array, but use names when accessing a field of a table.
//!
//! Note that they are inhomogeneous in general (children are of different types).
//! You need to define custom types and derive [`ConfigData`] if you want to access structured data.
//! Define structs with unnamed fields to model an array, while structs with named fields to model a table.
//! Specially, in the case when they do contain homogeneous data,
//! arrays can be accessed as [`Vec<T>`], and tables can be accessed as [`std::collections::BTreeMap<&str, T>`] or [`std::collections::BTreeMap<String, T>`],
//! as long as the representation of children can be accessed as `T`.
//! For containers, this type compatibility comes with a recursive sense.
//! There's a relevant concept from functional programming, known as [transmogrifying].
//!
//! [transmogrifying]: https://docs.rs/frunk/0.4.4/frunk/#transmogrifying
//!
//! ## Feature flags
//!
//! * `json` - supports JSON file format. Enabled by default.
//! * `yaml` - supports YAML file format. Enabled by default.
//! * `toml` - supports TOML file format. Enabled by default.
//! * `indexmap` - enables preserving orders of tables.

mod key;

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
/// * `type = <TYPE_IDENT>` -> `pub type <TYPE_IDENT> = <IDENT>::Type;`
/// * `const = <CONST_IDENT>` -> `pub const <CONST_IDENT>: <IDENT>::Type = <IDENT>::EXPR;`
/// * `static = <STATIC_IDENT>` -> `pub static <STATIC_IDENT>: <IDENT>::Type = <IDENT>::EXPR;`
///
/// [`proc_macro_expand`]: https://github.com/rust-lang/rust/issues/90765
/// [`macro_string`]: https://docs.rs/macro-string/latest/macro_string/
pub use inline_config_macros::config;

/// Constructs a path with which one accesses a nested-in piece of data from config.
///
/// A path can be constructed by a sequence of keys, separated by `.`.
/// A key can be either an index (access an array field) or a name (access a table field).
/// The name may be quoted if it is not a valid identifier (e.g. contains `-`).
pub use inline_config_macros::path;

/// The type version of [`path!()`]. Used in type bounds.
pub use inline_config_macros::Path;

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
pub use inline_config_macros::ConfigData;

#[doc(hidden)]
pub mod __private {
    pub use crate::key::*;
}
