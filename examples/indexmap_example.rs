use inline_config::{config, path, Get};

config! {
    pub static MY_CONFIG = #[toml] r#"
        [fruits]
        apple = "red"
        orange = "orange"
        grape = "purple"
    "#;
}

fn main() {
    let v: std::collections::BTreeMap<&str, &str> = MY_CONFIG.get(path!(fruits));
    println!("{:?}", v);
    let v: indexmap::IndexMap<&str, &str> = MY_CONFIG.get(path!(fruits));
    println!("{:?}", v);
}
