use inline_config::{Get, config, path};

#[config]
pub type MyConfig = toml!(
    r#"
        [fruits]
        apple = "red"
        orange = "orange"
        grape = "purple"
    "#
);

fn main() {
    // `BTreeMap` yields key-value pairs in lexicographical order of keys.
    let v: std::collections::BTreeMap<&str, &str> = MyConfig.get(path!(fruits));
    println!("{:?}", v);

    // If the order of array and tables in sources needs to be preserved,
    // add `indexmap` as a dependency and enable the `indexmap` feature.
    let v: indexmap::IndexMap<&str, &str> = MyConfig.get(path!(fruits));
    println!("{:?}", v);
}
