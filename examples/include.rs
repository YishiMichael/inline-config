use inline_config::{Get, config, path};

// Include from a config file adjacent to this file, similar to `include_str!()`.
#[config(toml)]
pub static MY_CONFIG: MyConfig = include_config!("example_config.toml");

// Enable environment variable expansion by using `include_config_env`.
// All `$ENV_VAR` will be replaced by corresponding environment varialbes.
// Escape `$` by `$$`.
// This yields an absolute path which may help IDE better locate the file.
#[config(toml)]
pub static MY_CONFIG_ENV: MyConfigEnv =
    include_config_env!("$CARGO_MANIFEST_DIR/examples/example_config.toml");

// Included configs and inline configs can be arbitrarily composed.
#[config(toml)]
pub static CHAINED_CONFIG: ChainedConfig =
    include_config_env!("$CARGO_MANIFEST_DIR/examples/example_config.toml")
        + r#"
        [owner]
        name = "Tom"
        dob = "1979-05-27"
    "#;

fn main() {
    let name: &str = MY_CONFIG.get(path!(owner.name));
    println!("{name}");
    let name: &str = CHAINED_CONFIG.get(path!(owner.name));
    println!("{name}");
}
