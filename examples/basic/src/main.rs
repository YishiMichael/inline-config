inline_config::config! {
    pub static C = "Cargo.toml" + toml!(r#"
        name = "override"
    "#) + json!(r#"
    {
        "inner": {
            "name": "json_override",
            "name2": "other"
        }
    }"#) + json!(r#"
    {
        "name": "json_override",
        "name2": "out_name2"
    }"#);
}

use inline_config::{key, ConfigData, Get};

#[derive(ConfigData, Debug)]
#[allow(unused)]
struct MyS {
    name: &'static str,
    name2: String,
}

fn main() {
    let w: MyS = C.get(key!(inner));
    println!("{:?}", w);
    let w: MyS = C.get(key!());
    println!("{:?}", w);
}
