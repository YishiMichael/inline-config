use inline_config::{config, path};

// Include from a config file from disk.
#[config(export(static = MY_CONFIG))]
mod my_config {
    toml!(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/examples/example_config.toml"
    )));
}

// Included configs and inline configs can be arbitrarily composed.
#[config(export(static = CHAINED_CONFIG))]
mod chained_config {
    toml!(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/examples/example_config.toml"
    )));
    toml!(
        r#"
        [owner]
            name = "Tom"
            dob = "1979-05-27"
        "#
    );
}

fn main() {
    let name: &str = MY_CONFIG[path!(owner.name)].into();
    println!("{name:?}");
    let name: &str = CHAINED_CONFIG[path!(owner.name)].into();
    println!("{name:?}");
}
