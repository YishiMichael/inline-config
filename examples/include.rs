use inline_config::{Config, path};

// Include from a config file from disk.
// The format is clear from path extension, so no need to specify.
#[derive(Config)]
#[config(src = include_env!("$CARGO_MANIFEST_DIR/examples/example_config.toml"))]
struct MyConfig;

// Included sources and inline sources can be arbitrarily composed.
#[derive(Config)]
#[config(src = include_env!("$CARGO_MANIFEST_DIR/examples/example_config.toml"))]
#[config(src = r#"
    [owner]
    name = "Tom"
    dob = "1979-05-27"
"#)]
struct ChainedConfig;

fn main() {
    let name: &str = MyConfig[path!(owner.name)].into();
    dbg!(name);
    let name: &str = ChainedConfig[path!(owner.name)].into();
    dbg!(name);
}
