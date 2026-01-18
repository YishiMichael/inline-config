use inline_config::{ConfigData, Path, config, path};

/// Edited from TOML official example.
#[config(export(static = TOML_EXAMPLE))]
mod toml_example {
    toml!(
        r#"
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
        "#
    );
}

fn primitive_types() {
    // Get a string at field `title`.
    let title: String = TOML_EXAMPLE[path!(title)].into();
    println!("{title}");

    // String references are also compatible.
    let title: &'static str = TOML_EXAMPLE[path!(title)].into();
    println!("{title}");

    // Incompatible types will cause compile error.
    // let title: u32 = TOML_EXAMPLE[path!(title)].into();

    // Missing keys will cause compile error.
    // let _: u32 = TOML_EXAMPLE[path!(unknown)].into();

    // Nested paths chained by `.`.
    let owner_name: &str = TOML_EXAMPLE[path!(owner.name)].into();
    println!("{owner_name}");
    let server: &str = TOML_EXAMPLE[path!(database.server)].into();
    println!("{server}");

    // Non-identifier key can be wrapped in quotes.
    let date_of_birth: &str = TOML_EXAMPLE[path!(owner."date-of-birth")].into();
    println!("{date_of_birth}");

    // Any numeric types are compatible for numbers.
    let connection_max: u32 = TOML_EXAMPLE[path!(database.connection_max)].into();
    println!("{connection_max}");
    let connection_max: u64 = TOML_EXAMPLE[path!(database.connection_max)].into();
    println!("{connection_max}");

    // Index into an array using `.0`.
    let port: u32 = TOML_EXAMPLE[path!(database.ports.0)].into();
    println!("{port}");
}

fn container_types() {
    // Collect all items from a homogeneous array into a `Vec`.
    let ports: Vec<u32> = TOML_EXAMPLE[path!(database.ports)].into();
    println!("{ports:?}");

    // Collect all items from a homogeneous table into a `BTreeMap`.
    // See `order.rs` if the order of entries needs to be preserved.
    let languages: std::collections::BTreeMap<&str, u32> = TOML_EXAMPLE[path!(languages)].into();
    println!("{languages:?}");
}

fn user_types() {
    #![allow(unused)]

    // Define a struct to match structured data from config.
    // Named structs corresponds to tables.
    #[derive(ConfigData, Debug)]
    struct Server {
        ip: String,
        dc: String,
    }

    let server: Server = TOML_EXAMPLE[path!(servers.alpha)].into();
    println!("{server:?}");

    // We can even compose with other containers!
    let servers: std::collections::BTreeMap<String, Server> = TOML_EXAMPLE[path!(servers)].into();
    println!("{servers:?}");

    // Fields do not need to fully match. We only require all keys show up in the source data.
    // Generics supported.
    #[derive(ConfigData, Debug)]
    struct PartialServer<'a> {
        ip: &'a str,
    }
    let partial_server: PartialServer<'_> = TOML_EXAMPLE[path!(servers.alpha)].into();
    println!("{partial_server:?}");

    // Field renaming supported. Needed if the key is not a valid rust identifier.
    #[derive(ConfigData, Debug)]
    struct Owner<S> {
        name: S, // matches "name"
        #[config_data(rename = "date-of-birth")]
        date_of_birth: S, // matches "date-of-birth"
        r#mod: S, // matches "mod"
    }
    let owner: Owner<String> = TOML_EXAMPLE[path!(owner)].into();
    println!("{owner:?}");

    // Nesting supported.
    #[derive(ConfigData, Debug)]
    struct Root {
        title: String,
        owner: Owner<String>,
    }
    // An empty path fetches data at the root.
    let root: Root = TOML_EXAMPLE[path!()].into();
    println!("{root:?}");

    // Unnamed structs corresponds to arrays.
    #[derive(ConfigData, Debug)]
    struct Hosts(String, String);
    let hosts: Hosts = TOML_EXAMPLE[path!(clients.hosts)].into();
    println!("{hosts:?}");
}

fn overwrite() {
    // Some formats like json have null values. They need to be resolved eventually.
    // Include multiple sources in the mod to perform overwriting. The latter overwrites the former.
    #[config(export(static = CHAINED_CONFIG))]
    mod chained_config {
        json!(
            r#"
            {
                "name": "Tom Preston-Werner",
                "preferred-name": null
            }
            "#
        );
        json!(
            r#"
            {
                "preferred-name": "Tom",
                "year-of-birth": 1979
            }
            "#
        );
    }

    // `preferred-name` is overwritten by the latter config source.
    let preferred_name: &str = CHAINED_CONFIG[path!("preferred-name")].into();
    println!("{preferred_name:?}");

    // `year-of-birth` is newly added by the latter config source.
    let year_of_birth: u32 = CHAINED_CONFIG[path!("year-of-birth")].into();
    println!("{year_of_birth}");
}

fn generic() {
    #[config(export(static = PRIMARY_CONFIG))]
    mod primary_config {
        json!(
            r#"
            {
                "name": "Tom Preston-Werner",
                "preferred-name": ""
            }
            "#
        );
    }

    #[config(export(static = CHAINED_CONFIG))]
    mod chained_config {
        json!(
            r#"
            {
                "name": "Tom Preston-Werner",
                "preferred-name": ""
            }
            "#
        );
        json!(
            r#"
            {
                "preferred-name": "Tom",
                "year-of-birth": 1979
            }
            "#
        );
    }

    // After overwriting, the two configs have different types.
    fn get_names<C>(config: &C) -> (String, String)
    where
        C: std::ops::Index<Path!(name), Output: Copy + Into<String>>,
        C: std::ops::Index<Path!("preferred-name"), Output: Copy + Into<String>>,
    {
        (
            config[path!(name)].into(),
            config[path!("preferred-name")].into(),
        )
    }

    let names = get_names(&PRIMARY_CONFIG);
    println!("{names:?}");
    let names = get_names(&CHAINED_CONFIG);
    println!("{names:?}");
}

fn main() {
    println!("\n* primitive_types\n");
    primitive_types();
    println!("\n* container_types\n");
    container_types();
    println!("\n* user_types\n");
    user_types();
    println!("\n* overwrite\n");
    overwrite();
    println!("\n* generic\n");
    generic();
}
