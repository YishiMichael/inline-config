inline_config::config! {
    pub static MY_CONFIG = "Cargo.toml" + toml!(r#"
        name = "override"
    "#) + json!(r#"
    {
        "inner": {
            "name": {
                "outer": "json_override"
            },
            "name2": "other",
            "l": ["a", "c", "d"]
        }
    }"#) + json5!(r#"
    {
        "name": "json_override",
        "name2": "out_name2",
        "l": ["a", "b"],
    }"#) + toml!(r#"
        [inner.name]
        inner = "5"
    "#);
}

use inline_config::{key, ConfigData, Get};

#[derive(ConfigData, Debug)]
#[allow(unused)]
struct MyName {
    outer: &'static str,
    inner: String,
}

#[derive(ConfigData, Debug)]
#[allow(unused)]
struct MyS {
    name: MyName,
    name2: String,
    l: Vec<&'static str>,
}

fn main() {
    let v: MyS = MY_CONFIG.get(key!("inner"));
    println!("{:?}", v);
    let v: ((&str, &str), &str, Vec<String>) = MY_CONFIG.get(key!("inner"));
    println!("{:?}", v);
    // let v: MyS = MY_CONFIG.get(key!(""));
    // println!("{:?}", v);
    let v: &str = MY_CONFIG.get(key!("inner.name.inner"));
    println!("{:?}", v);
}
