//! Effortlessly embed config modules and access with any compatible types.
//!
//! ## Example
//!
//! Below is a basic example illustrating how to declare a config module and access data from it.
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
//!         "#
//!     );
//!     toml!(
//!         r#"
//!         [server]
//!         timeout = 5000
//!         "#
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
//! | --- | --- |
//! | Boolean | [`bool`] |
//! | Unsigned Integer | [`i8`], [`i16`], [`i32`], [`i64`], [`i128`], [`isize`],<br>[`u8`], [`u16`], [`u32`], [`u64`], [`u128`], [`usize`],<br>[`f32`], [`f64`] |
//! | Signed Integer | [`i8`], [`i16`], [`i32`], [`i64`], [`i128`], [`isize`],<br>[`f32`], [`f64`] |
//! | Float | [`f32`], [`f64`] |
//! | String | [`&str`], [`String`] |
//! | Array | [`Vec<T>`] if homogeneous,<br>User-defined structs deriving [`ConfigData`] with unnamed fields |
//! | Table | [`std::collections::BTreeMap<&str, T>`] if homogeneous,<br>[`std::collections::BTreeMap<String, T>`] if homogeneous,<br>[`indexmap::IndexMap<&str, T>`] if homogeneous\*,<br>[`indexmap::IndexMap<String, T>`] if homogeneous\*,<br>User-defined structs deriving [`ConfigData`] with named fields |
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

pub use inline_config_macros::*;

#[doc(hidden)]
pub mod __private {
    use std::marker::PhantomData;

    // Borrowed from `frunk_core::labelled::chars`.
    pub mod chars {
        macro_rules! create_enums_for {
        ($($c:tt)*) => {
            $(
                #[allow(non_camel_case_types)]
                pub struct $c;
            )*
        };
    }

        create_enums_for!(
            A B C D E F G H I J K L M N O P Q R S T U V W X Y Z
            a b c d e f g h i j k l m n o p q r s t u v w x y z
            _0 _1 _2 _3 _4 _5 _6 _7 _8 _9 __
        );

        // For unicode chars.
        pub struct UC<const CODEPOINT: u32>;
    }

    pub struct KeyIndex<Index>(PhantomData<Index>);

    pub struct KeyName<Name>(PhantomData<Name>);

    impl<Index> Default for KeyIndex<Index> {
        fn default() -> Self {
            Self(PhantomData)
        }
    }

    impl<Name> Default for KeyName<Name> {
        fn default() -> Self {
            Self(PhantomData)
        }
    }

    #[derive(Default)]
    pub struct PathNil;

    #[derive(Default)]
    pub struct PathCons<K, KS>(pub K, pub KS);
}
