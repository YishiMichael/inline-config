//! Effortlessly embed config as static data and access with any compatible data structures.
//!
//! ## Example
//!
//! Below is a basic example illustrating how to declare a config type and access data from it.
//!
//! ```
//! use inline_config::{Get, config, path};
//!
//! // The syntax mostly follows from type alias declaration, but generates a unit struct `MyConfig`.
//! // When there are multiple sources, latter ones overwrite former ones.
//! // Including a file from disk is also possible, see `examples/include.rs`.
//! #[config]
//! pub type MyConfig = toml!(
//!     r#"
//!         title = "TOML example"
//!
//!         [server]
//!         owner = "Tom"
//!         timeout = 2000
//!         ports = [ 8000, 8001, 8002 ]
//!     "#,
//!     r#"
//!         [server]
//!         timeout = 5000
//!     "#
//! );
//!
//! // Multiple types may be compatible. As a cost, type annotation is always required.
//! let title: &str = MyConfig.get(path!(title));
//! assert_eq!("TOML example", title);
//! let title: String = MyConfig.get(path!(title));
//! assert_eq!("TOML example", title);
//!
//! // A deeper path.
//! let owner: &str = MyConfig.get(path!(server.owner));
//! assert_eq!("Tom", owner);
//!
//! // Any numerical types.
//! let timeout: u32 = MyConfig.get(path!(server.timeout));
//! assert_eq!(5000, timeout);
//! let timeout: f32 = MyConfig.get(path!(server.timeout));
//!
//! // A homogeneous array can be accessed as `Vec<T>`.
//! let ports: Vec<u64> = MyConfig.get(path!(server.ports));
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

mod convert;
mod key;

/// Declares a config type.
///
/// A config type is declared using the type alias syntax, in the form illustrated below:
///
/// ```ignore
/// #[config]
/// <VIS> type <TYPE> = <FORMAT>!(<SRC>);
///
/// #[config(<FORMAT>)]
/// <VIS> type <TYPE> = <FORMAT>!(<SRC>, <SRC>, <SRC>);
/// ```
///
/// The `<TYPE>` shall be a new identifier (not already defined).
/// After expansion, for each `<TYPE>` the following symbols are brought into scope:
///
/// * A unit struct `<TYPE>`;
/// * A module `__<TYPE>` for internal usage.
///
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
/// [`proc_macro_expand`]: https://github.com/rust-lang/rust/issues/90765
/// [`macro_string`]: https://docs.rs/macro-string/latest/macro_string/
pub use inline_config_macros::config;

/// Constructs a path with which one accesses a nested-in piece of data from config.
///
/// A path can be constructed by a sequence of keys, separated by `.`.
/// A key can be either an index (access an array field) or a name (access a table field).
/// The name may be quoted if it is not a valid identifier (e.g. contains `-`).
pub use inline_config_macros::path;

/// The type version of [`path!()`]. Used in type parameters of [`Get`].
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

/// A trait modeling type compatibility.
///
/// A type bound `C: Get<P, T>` means the data at path `P` from config `C` is compatible with and can be converted into `T`.
///
/// This trait is not meant to be custom implemented; all implementations are induced from [`config`] macro.
pub trait Get<P, T> {
    fn get(&self, path: P) -> T;
}

#[doc(hidden)]
pub mod __private {
    pub use crate::convert::*;
    pub use crate::key::*;
}
