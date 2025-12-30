use inline_config::{config, path, Get};

config! {
    // Include from a config file adjacent to this file, similar to `include_str!()`.
    pub static MY_CONFIG = #[toml] include_config!("example_config.toml");

    // The format attribute may be omitted if it's clear from file extension.
    pub static MY_CONFIG_2 = include_config!("example_config.toml");

    // Enable environment variable expansion by using `include_config_env`.
    // All `$ENV_VAR` will be replaced by corresponding environment varialbes.
    // Escape `$` by `$$`.
    pub static MY_CONFIG_ENV = include_config_env!("$CARGO_MANIFEST_DIR/examples/example_config.toml");

    // Included configs and inline configs can be arbitrarily composed.
    pub static CHAINED_CONFIG
        = include_config_env!("$CARGO_MANIFEST_DIR/examples/example_config.toml")
        + #[toml] r#"
            [owner]
            name = "Tom"
            dob = 1979-05-27
        "#;
}

fn main() {
    let name: &str = MY_CONFIG.get(path!(owner.name));
    println!("{name}");
    let name: &str = CHAINED_CONFIG.get(path!(owner.name));
    println!("{name}");
}
