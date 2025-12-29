use inline_config::{config, path, ConfigData, Get, Path};

config! {
    pub static MY_CONFIG
        = include_config!("example_config.toml")
        + #[toml] r#"
            name = "Peter"
            age = 18
        "#
        + #[json] r#"
            {
                "preferred-name": null
            }
        "#;
}

#[allow(unused)]
#[derive(ConfigData, Debug)]
struct MyName {
    name: String,
    age: u16,
    #[config_data(rename = "preferred-name")]
    preferred_name: Option<&'static str>,
}

fn get_name<'c, C>(config: &'c C) -> MyName
where
    C: Get<'c, Path!(), MyName>,
{
    config.get(path!())
}

fn main() {
    let v = MY_CONFIG.name;
    println!("{:?}", v);
    let v: &str = MY_CONFIG.get(path!(name));
    println!("{:?}", v);
    let v: u32 = MY_CONFIG.get(path!(age));
    println!("{:?}", v);
    let v: Option<&str> = MY_CONFIG.get(path!("preferred-name"));
    println!("{:?}", v);
    let v: MyName = MY_CONFIG.get(path!());
    println!("{:?}", v);
    let v = get_name(&MY_CONFIG);
    println!("{:?}", v);
}
