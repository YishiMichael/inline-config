# inline-config
Effortlessly embed config as static data and access with any compatible data structures.
<!-- TODO: badges here -->

A procedual macro `config!` is provided to parse sources at compile time, generate static data structures, from which we can access values via the `Get` trait. The output types of accessed values can be almost "at will", as long as they are compatible.
<!-- TODO: link to doc.rs items -->

## Usage
Add `inline-config` to your dependencies:
```cmd
cargo add inline-config
```

In your source file, declare a static variable holding the config data
```rust
use inline_config::config;

config! {
    // Note, this looks like a typical static item declaration, but the type is omitted.
    // `#[toml]` is needed to specify the format of this source.
    pub static MY_CONFIG = #[toml] r#"
        title = "TOML example"

        [profile]
        name = "Tom"
        languages = 3
    "#;
}
```
Then, access the data inside using the `Get` trait in combination with the `path!` macro
```rust
use inline_config::{Get, path};

// Multiple types may be compatible. As a cost, type annotation is always required.
let title: &str = MY_CONFIG.get(path!(title));
assert_eq!("TOML example", title);
let title: String = MY_CONFIG.get(path!(title));
assert_eq!("TOML example", title);

// A deeper path.
let name: &str = MY_CONFIG.get(path!(profile.name));
assert_eq!("Tom", name);

// Any numerical types.
let languages: u32 = MY_CONFIG.get(path!(profile.languages));
assert_eq!(3, languages);
```

For a more advanced example, say you have an external toml file
```toml
# example_config.toml
title = "TOML Example"

[owner]
name = "Tom Preston-Werner"
dob = "1979-05-27"

[database]
server = "192.168.1.1"
ports = [ 8000, 8001, 8002 ]
connection_max = 5000
enabled = true

[servers.alpha]
ip = "10.0.0.1"
dc = "eqdc10"

[servers.beta]
ip = "10.0.0.2"
dc = "eqdc10"
```
In a rust file it's adjacent to (in the same directory), you can include it as
```rust
use inline_config::config;

// Declare multiple configs in a `config!` block.
config! {
    pub static INCLUDED_CONFIG = include_config!("example_config.toml");

    // Use `+` to overwrite.
    pub static FULL_CONFIG = include_config!("example_config.toml") + #[toml] r#"
        [owner]
        dob = "1979-05-27T07:32:00-08:00"
        preferred-name = "Tom"
    "#;
}
```
and access data with fancy types
```rust
use inline_config::{ConfigData, Get, Path, path};

// A homogeneous array can be accessed as `Vec<T>`.
let ports: Vec<u64> = INCLUDED_CONFIG.get(path!(database.ports));
assert_eq!([8000, 8001, 8002].to_vec(), ports);

// Define custom types for accessing
#[derive(ConfigData, Debug, Eq, PartialEq)]
struct Server<'a> {
    ip: &'a str,
    dc: &'a str,
}
// A homogeneous table can be accessed as `BTreeMap<&str, T>`.
let servers: std::collections::BTreeMap<&str, Server<'_>> = INCLUDED_CONFIG.get(path!(servers));
assert_eq!(
    vec![("alpha", Server { ip: "10.0.0.1", dc: "eqdc10" }), ("beta", Server { ip: "10.0.0.2", dc: "eqdc10" })],
    servers.into_iter().collect::<Vec<_>>()
);

// Use the `Get` trait to represent "any config with a field `owner.dob` with string type".
fn get_title<'c, C>(config: &'c C) -> String
where
    C: Get<'c, Path!(owner.dob), String>
{
    config.get(path!(owner.dob))
}
let dob = get_title(&INCLUDED_CONFIG);
assert_eq!("1979-05-27", dob);
let dob = get_title(&FULL_CONFIG);
assert_eq!("1979-05-27T07:32:00-08:00", dob);
```

Check out [more examples](examples) for more details about usage.

## Features
* `json`, `yaml`, `toml` formats are supported. The support of each config format is supported via a feature flag (all enabled by default).
* Both inline literal configs and file inclusions are supported; overwriting is supported.
* Compile-time source validation. Errors are clearly reported for easier debugging.
* Infallible data access. Path existence and type compatibility are both checked at compile time.
* Define custom data structures to access data.
* The feature flag `indexmap` enables preserving orders of tables. Check [this example](examples/order.rs) for details.
