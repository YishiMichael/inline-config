use inline_config::{Get, config, path};

// Include from a config file from disk.
#[config]
pub type MyConfig = toml!(include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/examples/example_config.toml"
)));

// Included configs and inline configs can be arbitrarily composed.
#[config]
pub type ChainedConfig = toml!(
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/examples/example_config.toml"
    )),
    r#"
    [owner]
        name = "Tom"
        dob = "1979-05-27"
    "#
);

fn main() {
    let name: &str = MyConfig.get(path!(owner.name));
    println!("{name}");
    let name: &str = ChainedConfig.get(path!(owner.name));
    println!("{name}");
}
