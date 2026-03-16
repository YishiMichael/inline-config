use inline_config::{Config, path};

#[derive(Config)]
#[config(format = "toml")]
#[config(src = r#"
    [fruits]
    apple = "red"
    orange = "orange"
    grape = "purple"
"#)]
struct MyConfig;

fn main() {
    // `BTreeMap` yields key-value pairs in lexicographical order of keys.
    let map: std::collections::BTreeMap<&str, &str> = MyConfig[path!(fruits)].into();
    dbg!(map);

    // If the order of array and tables in sources needs to be preserved,
    // add `indexmap` as a dependency and enable the `indexmap` feature.
    let map: indexmap::IndexMap<&str, &str> = MyConfig[path!(fruits)].into();
    dbg!(map);
}
