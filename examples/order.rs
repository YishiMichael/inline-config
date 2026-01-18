use inline_config::{config, path};

#[config(export(static = MY_CONFIG))]
mod my_config {
    toml!(
        r#"
        [fruits]
        apple = "red"
        orange = "orange"
        grape = "purple"
        "#
    );
}

fn main() {
    // `BTreeMap` yields key-value pairs in lexicographical order of keys.
    let v: std::collections::BTreeMap<&str, &str> = MY_CONFIG[path!(fruits)].into();
    println!("{:?}", v);

    // If the order of array and tables in sources needs to be preserved,
    // add `indexmap` as a dependency and enable the `indexmap` feature.
    let v: indexmap::IndexMap<&str, &str> = MY_CONFIG[path!(fruits)].into();
    println!("{:?}", v);
}
