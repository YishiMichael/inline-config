# inline-config

Effortlessly embed config as static data and access with any compatible data structures.

[![Version](https://img.shields.io/crates/v/inline-config?style=for-the-badge)](https://crates.io/crates/inline-config)
[![License](https://img.shields.io/crates/l/inline-config?style=for-the-badge)](https://github.com/YishiMichael/inline-config/blob/main/LICENSE-MIT)
[![Docs](https://img.shields.io/docsrs/inline-config?style=for-the-badge&logo=docs.rs)](https://docs.rs/inline-config)
[![CI](https://img.shields.io/github/actions/workflow/status/YishiMichael/inline-config/rust.yml?style=for-the-badge&logo=github&label=CI)](https://github.com/YishiMichael/inline-config)

A procedual macro [`config`](https://docs.rs/inline-config/latest/inline_config/macro.config.html) is provided to parse sources at compile time, generate static data structures, from which we can access values via the [`Index`](https://doc.rust-lang.org/std/ops/trait.Index.html) trait and the [`Into`](https://doc.rust-lang.org/std/convert/trait.Into.html) trait.

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

In your source file, declare a module using [`config`](https://docs.rs/inline-config/latest/inline_config/macro.config.html) holding the config data

```rust
use inline_config::config;

// Declare a config module containing literal sources.
// With `export(static = MY_CONFIG)`, a static variable `MY_CONFIG` will be brought into scope.
#[config(export(static = MY_CONFIG))]
mod my_config {
    // When there are multiple sources, latter ones overwrite former ones.
    // Including a file from disk is also possible, see `examples/include.rs`.
    toml!(
    r#"
        title = "TOML example"

        [server]
        owner = "Tom"
        timeout = 2000
        ports = [ 8000, 8001, 8002 ]
    "#
    );
    toml!(
    r#"
        [server]
        timeout = 5000
    "#
    );
}
```

Then, access the data inside using the [`path!()`](https://docs.rs/inline-config/latest/inline_config/macro.path.html) macro

```rust
use inline_config::path;

// Multiple types may implement `From` trait, so type annotations are required.
let title: &str = MY_CONFIG[path!(title)].into();
assert_eq!("TOML example", title);
let title: String = MY_CONFIG[path!(title)].into();
assert_eq!("TOML example", title);

// A deeper path.
let owner: &str = MY_CONFIG[path!(server.owner)].into();
assert_eq!("Tom", owner);

// Any numerical types.
let timeout: u32 = MY_CONFIG[path!(server.timeout)].into();
assert_eq!(5000, timeout);
let timeout: f32 = MY_CONFIG[path!(server.timeout)].into();

// A homogeneous array can be accessed as `Vec<T>`.
let ports: Vec<u64> = MY_CONFIG[path!(server.ports)].into();
assert_eq!([8000, 8001, 8002].to_vec(), ports);
```

Check out [more examples](examples) for more details about usage.
