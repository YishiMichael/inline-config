#![allow(unused)]

use inline_config::{config, path, ConfigData, Get};

config! {
    pub static MY_CONFIG = toml!(r#"
        name = "Peter"
        age = 18
    "#) + json!(r#"
        {
            "preferred-name": null
        }
    "#);
    // pub static MY_CONFIG = toml!(r#"
    //     name = "override"
    // "#) + json!(r#"
    // {
    //     "inner": {
    //         "name": {
    //             "outer": "json_override"
    //         },
    //         "name2": "other",
    //         "l": ["a", "c", "d"]
    //     }
    // }"#) + json5!(r#"
    // {
    //     "name": "json_override",
    //     "name2": "out_name2",
    //     "l": ["a", "b"],
    // }"#) + toml!(r#"
    //     [inner.name]
    //     inner = "5"
    // "#);
}

#[derive(ConfigData, Debug)]
struct MyName {
    name: &'static str,
    age: u16,
    #[config_data(rename = "preferred-name")]
    preferred_name: Option<&'static str>,
}

// #[derive(ConfigData, Debug)]
// #[allow(unused)]
// struct MyS {
//     name: MyName,
//     name2: String,
//     l: Vec<&'static str>,
// }

fn main() {
    let v: &str = MY_CONFIG.get(path!("name"));
    println!("{:?}", v);
    let v: u32 = MY_CONFIG.get(path!("age"));
    println!("{:?}", v);
    let v: Option<&str> = MY_CONFIG.get(path!("preferred-name"));
    println!("{:?}", v);
    let v: MyName = MY_CONFIG.get(path!(""));
    println!("{:?}", v);
    // let v: Option<&str> = MY_CONFIG.get(path!("value"));
    // let v: ((&str, &str), &str, Vec<String>) = MY_CONFIG.get(key!("inner"));
    // println!("{:?}", v);
    // let v: MyS = MY_CONFIG.get(key!(""));
    // println!("{:?}", v);
    // let v: &str = MY_CONFIG.get(key!("inner.name.inner"));
    // println!("{:?}", v);
}
