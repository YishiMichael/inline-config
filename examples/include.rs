use inline_config::{Get, config, path};

// Include from a config file from disk.
#[config(toml)]
pub static MY_CONFIG: MyConfigEnv = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/examples/example_config.toml",
));

// Included configs and inline configs can be arbitrarily composed.
#[config(toml)]
pub static CHAINED_CONFIG: ChainedConfig = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/examples/example_config.toml"
)) + r#"
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
