use inline_config::{ConfigData, Get, Path, config, path};

config! {
    /// Edited from TOML official example.
    pub static TOML_EXAMPLE: TomlExample = #[toml] r#"
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
    "#;
}

fn primitive_types() {
    // Get a string at field `title`.
    let title: String = TOML_EXAMPLE.get(path!(title));
    println!("{title}");

    // String references are also compatible.
    let title: &'static str = TOML_EXAMPLE.get(path!(title));
    println!("{title}");

    // Incompatible types will cause compile error.
    // let title: u32 = MY_CONFIG.get(path!(title));

    // Missing keys will cause compile error.
    // let _: u32 = MY_CONFIG.get(path!(unknown));

    // Nested paths chained by `.`.
    let owner_name: &str = TOML_EXAMPLE.get(path!(owner.name));
    println!("{owner_name}");
    let server: &str = TOML_EXAMPLE.get(path!(database.server));
    println!("{server}");

    // Non-identifier key can be wrapped in quotes.
    let date_of_birth: &str = TOML_EXAMPLE.get(path!(owner."date-of-birth"));
    println!("{date_of_birth}");

    // Any numeric types are compatible for numbers.
    let connection_max: u32 = TOML_EXAMPLE.get(path!(database.connection_max));
    println!("{connection_max}");
    let connection_max: u64 = TOML_EXAMPLE.get(path!(database.connection_max));
    println!("{connection_max}");

    // Index into an array using `.0`.
    let port: u32 = TOML_EXAMPLE.get(path!(database.ports.0));
    println!("{port}");
}

fn container_types() {
    // Collect all items from a homogeneous array into a `Vec`.
    let ports: Vec<u32> = TOML_EXAMPLE.get(path!(database.ports));
    println!("{ports:?}");

    // Collect all items from a homogeneous table into a `BTreeMap`.
    // See `order.rs` if the order of entries needs to be preserved.
    let languages: std::collections::BTreeMap<&str, u32> = TOML_EXAMPLE.get(path!(languages));
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

    let server: Server = TOML_EXAMPLE.get(path!(servers.alpha));
    println!("{server:?}");

    // We can even compose with other containers!
    let servers: std::collections::BTreeMap<String, Server> = TOML_EXAMPLE.get(path!(servers));
    println!("{servers:?}");

    // Fields do not need to fully match. We only require all keys show up in the source data.
    // Generics supported.
    #[derive(ConfigData, Debug)]
    struct PartialServer<'a> {
        ip: &'a str,
    }
    let partial_server: PartialServer<'_> = TOML_EXAMPLE.get(path!(servers.alpha));
    println!("{partial_server:?}");

    // Field renaming supported. Needed if the key is not a valid rust identifier.
    #[derive(ConfigData, Debug)]
    struct Owner<S> {
        name: S, // matches "name"
        #[config_data(rename = "date-of-birth")]
        date_of_birth: S, // matches "date-of-birth"
        r#mod: S, // matches "mod"
    }
    let owner: Owner<String> = TOML_EXAMPLE.get(path!(owner));
    println!("{owner:?}");

    // Nesting supported.
    #[derive(ConfigData, Debug)]
    struct Root {
        title: String,
        owner: Owner<String>,
    }
    // An empty path fetches data at the root.
    let root: Root = TOML_EXAMPLE.get(path!());
    println!("{root:?}");

    // Unnamed structs corresponds to arrays.
    #[derive(ConfigData, Debug)]
    struct Hosts(String, String);
    let hosts: Hosts = TOML_EXAMPLE.get(path!(clients.hosts));
    println!("{hosts:?}");
}

fn optional_types() {
    config! {
        // Note, some formats like toml do not have null types.
        static JSON_CONFIG: _ = #[json] r#"
        {
            "name": "Tom Preston-Werner",
            "preferred-name": null,
            "servers": null
        }
        "#;
    }

    // Any non-null `T` can be converted into `Some(T)` for free.
    let name: String = JSON_CONFIG.get(path!(name));
    println!("{name}");

    // `null` can be converted into `None` as any `Option<T>`.
    let preferred_name: Option<&str> = JSON_CONFIG.get(path!("preferred-name"));
    println!("{preferred_name:?}");

    // `null` can be converted into `T` if it implements `Default`.
    let servers_fallback: u32 = JSON_CONFIG.get(path!("servers"));
    println!("{servers_fallback}");
}

fn overwrite() {
    config! {
        // Use `+` to chain multiple config sources. The latter overwrites the former.
        static CHAINED_CONFIG: _ = #[json] r#"
        {
            "name": "Tom Preston-Werner",
            "preferred-name": null
        }
        "# + #[json] r#"
        {
            "preferred-name": "Tom",
            "year-of-birth": 1979
        }
        "#;
    }

    // `preferred-name` is overwritten by the latter config source.
    let preferred_name: Option<&str> = CHAINED_CONFIG.get(path!("preferred-name"));
    println!("{preferred_name:?}");

    // `year-of-birth` is newly added by the latter config source.
    let year_of_birth: u32 = CHAINED_CONFIG.get(path!("year-of-birth"));
    println!("{year_of_birth}");
}

fn get_trait() {
    config! {
        static PRIMARY_CONFIG: _ = #[json] r#"
        {
            "name": "Tom Preston-Werner",
            "preferred-name": null
        }
        "#;

        static CHAINED_CONFIG: _ = #[json] r#"
        {
            "name": "Tom Preston-Werner",
            "preferred-name": null
        }
        "# + #[json] r#"
        {
            "preferred-name": "Tom",
            "year-of-birth": 1979
        }
        "#;
    }

    // After overwriting, the two configs have different types.
    // The `Get` trait modeled their shared data-getting behavior.
    fn get_names<C>(config: &'static C) -> (String, Option<String>)
    where
        C: Get<Path!(name), String>,
        C: Get<Path!("preferred-name"), Option<String>>,
    {
        (config.get(path!(name)), config.get(path!("preferred-name")))
    }

    let names = get_names(&PRIMARY_CONFIG);
    println!("{names:?}");
    let names = get_names(&CHAINED_CONFIG);
    println!("{names:?}");
}

fn main() {
    primitive_types();
    container_types();
    user_types();
    optional_types();
    overwrite();
    get_trait();
}
