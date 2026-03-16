#![allow(unused)]

use inline_config::{path, Config};

// Edited from TOML official example.
#[derive(Config)]
#[config(format = "toml")]
#[config(src = r#"
    title = "TOML Example"

    [owner]
    name = "Tom Preston-Werner"
    dob = "1979-05-27"
    date-of-birth = "1979-05-27"
    mod = "toml"

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

    [clients]
    data = [ ["gamma", "delta"], [1, 2] ]
    hosts = [
      "alpha",
      "omega"
    ]

    [languages]
    json = 2001
    yaml = 2001
    toml = 2013
"#)]
pub struct TomlExample;

fn primitive_types() {
    // Get a string at field `title`.
    let title: String = TomlExample[path!(title)].into();
    dbg!(title);

    // String references are also compatible.
    let title: &'static str = TomlExample[path!(title)].into();
    dbg!(title);

    // Incompatible types will cause compile error.
    // let title: u32 = TomlExample[path!(title)].into();

    // Missing keys will cause compile error.
    // let _: u32 = TomlExample[path!(unknown)].into();

    // Nested paths chained by `.`.
    let owner_name: &str = TomlExample[path!(owner.name)].into();
    dbg!(owner_name);
    let server: &str = TomlExample[path!(database.server)].into();
    dbg!(server);

    // Non-identifier key can be wrapped in quotes.
    let date_of_birth: &str = TomlExample[path!(owner."date-of-birth")].into();
    dbg!(date_of_birth);

    // Any numeric types are compatible for numbers.
    let connection_max: u32 = TomlExample[path!(database.connection_max)].into();
    dbg!(connection_max);
    let connection_max: u64 = TomlExample[path!(database.connection_max)].into();
    dbg!(connection_max);

    // Index into an array using `.0`.
    let port: u32 = TomlExample[path!(database.ports.0)].into();
    dbg!(port);

    // Array index out-of-bound will cause compile error.
    // let port: u32 = TomlExample[path!(database.ports.9)].into();
}

fn container_types() {
    // Collect all items from a homogeneous array into a `Vec`.
    let ports: Vec<u32> = TomlExample[path!(database.ports)].into();
    dbg!(ports);

    // Collect all items from a homogeneous table into a `BTreeMap`.
    // See `order.rs` if the order of entries needs to be preserved.
    let languages: std::collections::BTreeMap<&str, u32> = TomlExample[path!(languages)].into();
    dbg!(languages);
}

fn user_types() {
    use inline_config::FromConfig;

    // Define a struct to match structured data from config.
    // Named structs corresponds to tables.
    #[derive(FromConfig, Debug)]
    struct Server {
        ip: String,
        dc: String,
    }

    let server: Server = TomlExample[path!(servers.alpha)].into();
    dbg!(server);

    // We can even compose with other containers!
    let servers: std::collections::BTreeMap<String, Server> = TomlExample[path!(servers)].into();
    dbg!(servers);

    // Fields do not need to fully match. We only require all keys show up in the source data.
    // Generics supported.
    #[derive(FromConfig, Debug)]
    struct PartialServer<'a> {
        ip: &'a str,
    }
    let partial_server: PartialServer<'_> = TomlExample[path!(servers.alpha)].into();
    dbg!(partial_server);

    // Field renaming supported. Needed if the key is not a valid rust identifier.
    #[derive(FromConfig, Debug)]
    struct Owner<S> {
        name: S, // matches "name"
        #[config(name = "date-of-birth")]
        date_of_birth: S, // matches "date-of-birth"
        r#mod: S, // matches "mod"
    }
    let owner: Owner<String> = TomlExample[path!(owner)].into();
    dbg!(owner);

    // Nesting supported.
    #[derive(FromConfig, Debug)]
    struct Root {
        title: String,
        owner: Owner<String>,
    }
    // An empty path fetches data at the root.
    let root: Root = TomlExample[path!()].into();
    dbg!(root);

    // Unnamed structs corresponds to arrays.
    #[derive(FromConfig, Debug)]
    struct Hosts(String, String);
    let hosts: Hosts = TomlExample[path!(clients.hosts)].into();
    dbg!(hosts);
}

fn yaml_example() {
    // Formats in addition to TOML are also supported.
    #[derive(Config)]
    #[config(format = "yaml")]
    #[config(src = r#"
        info:
            name: Tom Preston-Werner
            preferred-name: ""
    "#)]
    struct YamlConfig;

    let preferred_name: &str = YamlConfig[path!(info."preferred-name")].into();
    dbg!(preferred_name);
}

fn overwrite() {
    // Some formats like json have null values. They need to be resolved eventually.
    // Include multiple sources in the mod to perform overwriting. The latter overwrites the former.
    #[derive(Config)]
    #[config(format = "json")]
    #[config(src = r#"
        {
            "name": "Tom Preston-Werner",
            "preferred-name": null
        }
    "#)]
    #[config(src = r#"
        {
            "preferred-name": "Tom",
            "year-of-birth": 1979
        }
    "#)]
    struct ChainedConfig;

    // `preferred-name` is overwritten by the latter config source.
    let preferred_name: &str = ChainedConfig[path!("preferred-name")].into();
    dbg!(preferred_name);

    // `year-of-birth` is newly added by the latter config source.
    let year_of_birth: u32 = ChainedConfig[path!("year-of-birth")].into();
    dbg!(year_of_birth);
}

fn generic() {
    use inline_config::Path;

    #[derive(Config)]
    #[config(format = "json")]
    #[config(src = r#"
        {
            "name": "Tom Preston-Werner",
            "preferred-name": ""
        }
    "#)]
    struct PrimaryConfig;

    #[derive(Config)]
    #[config(format = "json")]
    #[config(src = r#"
        {
            "name": "Tom Preston-Werner",
            "preferred-name": ""
        }
    "#)]
    #[config(src = r#"
        {
            "preferred-name": "Tom",
            "year-of-birth": 1979
        }
    "#)]
    struct ChainedConfig;

    // After overwriting, the two configs have different types.
    // Use trait bounds to model the intersection of these types.
    fn get_names<C>(config: C) -> (String, String)
    where
        C: std::ops::Index<Path!(name), Output: Copy + Into<String>>,
        C: std::ops::Index<Path!("preferred-name"), Output: Copy + Into<String>>,
    {
        (
            config[path!(name)].into(),
            config[path!("preferred-name")].into(),
        )
    }

    let names = get_names(PrimaryConfig);
    dbg!(names);
    let names = get_names(ChainedConfig);
    dbg!(names);
}

fn conditioned_src() {
    // Use `cfg_attr` to conditionally load sources.
    #[derive(Config)]
    #[config(format = "toml")]
    #[config(src = r#"
        platform = "unknown"
    "#)]
    #[cfg_attr(
        target_os = "windows",
        config(src = r#"
            platform = "windows"
        "#)
    )]
    #[cfg_attr(
        target_os = "macos",
        config(src = r#"
            platform = "macos"
        "#)
    )]
    #[cfg_attr(
        target_os = "linux",
        config(src = r#"
            platform = "linux"
        "#)
    )]
    struct PlatformConfig;

    let platform: String = PlatformConfig[path!(platform)].into();
    dbg!(platform);
}

fn main() {
    println!("\n* primitive_types\n");
    primitive_types();
    println!("\n* container_types\n");
    container_types();
    println!("\n* user_types\n");
    user_types();
    println!("\n* yaml_example\n");
    yaml_example();
    println!("\n* overwrite\n");
    overwrite();
    println!("\n* generic\n");
    generic();
    println!("\n* conditioned_src\n");
    conditioned_src();
}
