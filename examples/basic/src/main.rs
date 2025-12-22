inline_config::config! {
    pub static MY_CONFIG = "Cargo.toml" + toml!(r#"
        name = "override"
    "#) + json!(r#"
    {
        "inner": {
            "name": "json_override",
            "name2": "other",
            "l": ["a", "c", "d"]
        }
    }"#) + json5!(r#"
    {
        "name": "json_override",
        "name2": "out_name2",
        "l": ["a", "b"],
    }"#);
}

use inline_config::{key, Config, Get};

#[derive(Config, Debug)]
#[allow(unused)]
struct MyS {
    name: &'static str,
    name2: String,
    l: Vec<&'static str>,
}

fn main() {
    let v: MyS = MY_CONFIG.get(key!("inner"));
    println!("{:?}", v);
    let v: (&str, &str, Vec<String>) = MY_CONFIG.get(key!("inner"));
    println!("{:?}", v);
    let v: MyS = MY_CONFIG.get(key!(""));
    println!("{:?}", v);
}
