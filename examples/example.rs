#![allow(unused)]

use inline_config::{config, path, ConfigData, Get, Path};

config! {
    pub static MY_CONFIG
        = toml!(r#"
            name = "Peter"
            age = 18
        "#)
        + json!(r#"
            {
                "preferred-name": null
            }
        "#);
}

#[derive(ConfigData, Debug)]
struct MyName {
    name: &'static str,
    age: u16,
    #[config_data(rename = "preferred-name")]
    preferred_name: Option<&'static str>,
}

fn get_name<'c, C>(config: &'c C) -> MyName
where
    C: Get<'c, Path!("name"), MyName>,
{
    config.get(path!("name"))
}

fn main() {
    let v: &str = MY_CONFIG.get(path!("name"));
    println!("{:?}", v);
    let v: u32 = MY_CONFIG.get(path!("age"));
    println!("{:?}", v);
    let v: Option<&str> = MY_CONFIG.get(path!("preferred-name"));
    println!("{:?}", v);
    let v: MyName = MY_CONFIG.get(path!(""));
    println!("{:?}", v);
}
