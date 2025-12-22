inline_config::config! {
    pub static CA = "Cargo.toml" + toml!(r#"
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

use inline_config::{key, ConfigData, Get};

#[derive(ConfigData, Debug)]
#[allow(unused)]
struct MyS {
    name: &'static str,
    name2: String,
    l: Vec<&'static str>,
}

fn main() {
    let w: MyS = CA.get(key!("inner"));
    println!("{:?}", w);
    let w: MyS = CA.get(key!(""));
    println!("{:?}", w);
}
