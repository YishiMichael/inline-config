# inline-config

Effortlessly embed config as static data and access with any compatible data structures.

[![Version](https://img.shields.io/crates/v/inline-config?style=for-the-badge)](https://crates.io/crates/inline-config)
[![License](https://img.shields.io/crates/l/inline-config?style=for-the-badge)](https://github.com/cptpiepmatz/inline-config/blob/main/LICENSE)
[![Docs](https://img.shields.io/docsrs/inline-config?style=for-the-badge&logo=docs.rs)](https://docs.rs/inline-config)
[![CI](https://img.shields.io/github/actions/workflow/status/YishiMichael/inline-config/rust.yml?style=for-the-badge&logo=github&label=CI)](https://docs.rs/inline-config)

A procedual macro [`config!()`](https://docs.rs/inline-config/latest/inline_config/macro.config.html) is provided to parse sources at compile time, generate static data structures, from which we can access values via the [`Get`](https://docs.rs/inline-config/latest/inline_config/trait.Get.html) trait. The output types of accessed values can be almost "at will", as long as they are compatible.

## Features

* JSON, YAML, TOML formats are supported.
* Both inline literal configs and file inclusions are supported; overwriting is supported.
* Compile-time source validation. Errors are clearly reported for easier debugging.
* Infallible data access. Path existence and type compatibility are both checked at compile time.
* Define custom data structures to access data.
* The feature flag `indexmap` enables preserving orders of tables. Check [this example](examples/order.rs) for details.

## Usage

Add `inline-config` to your dependencies

```cmd
cargo add inline-config
```

In your source file, declare a static variable using [`config!()`](https://docs.rs/inline-config/latest/inline_config/macro.config.html) holding the config data

```rust
use inline_config::config;

config! {
    // Note, this looks like a typical static item declaration, but the type is omitted.
    // `#[toml]` is needed to specify the format of this source.
    // Including a file from disk is also possible, see `examples/include.rs`
    pub static MY_CONFIG = #[toml] r#"
        title = "TOML example"

        [server]
        owner = "Tom"
        timeout = 2000
        ports = [ 8000, 8001, 8002 ]
    "# + #[toml] r#"
        [server]
        timeout = 5000
    "#;
}
```

Then, access the data inside using the [`Get`](https://docs.rs/inline-config/latest/inline_config/trait.Get.html) trait in combination with the [`path!`](https://docs.rs/inline-config/latest/inline_config/macro.path.html) macro

```rust
use inline_config::{Get, path};

// Multiple types may be compatible. As a cost, type annotation is always required.
let title: &str = MY_CONFIG.get(path!(title));
assert_eq!("TOML example", title);
let title: String = MY_CONFIG.get(path!(title));
assert_eq!("TOML example", title);

// A deeper path.
let owner: &str = MY_CONFIG.get(path!(server.owner));
assert_eq!("Tom", owner);

// Any numerical types.
let timeout: u32 = MY_CONFIG.get(path!(server.timeout));
assert_eq!(5000, timeout);
let timeout: f32 = MY_CONFIG.get(path!(server.timeout));

// A homogeneous array can be accessed as `Vec<T>`.
let ports: Vec<u64> = MY_CONFIG.get(path!(server.ports));
assert_eq!([8000, 8001, 8002].to_vec(), ports);
```

Check out [more examples](examples) for more details about usage.
