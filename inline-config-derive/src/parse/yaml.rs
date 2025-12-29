use crate::value::Value;
use std::error::Error;
use yaml_rust2 as yaml;

pub fn parse(s: &str) -> Result<Value, Box<dyn Error>> {
    let values = yaml::YamlLoader::load_from_str(s)?;
    let value = values.into_iter().map(morph).sum();
    Ok(value)
}

fn morph(value: yaml::Yaml) -> Value {
    match value {
        yaml::Yaml::Real(value) => value.parse().map(Value::Float).unwrap_or_else(|e| {
            proc_macro_error::emit_call_site_error!("yaml parsing error: {}", e);
            Value::Nil
        }),
        yaml::Yaml::Integer(value) => Value::Integer(value),
        yaml::Yaml::String(value) => Value::String(value),
        yaml::Yaml::Boolean(value) => Value::Boolean(value),
        yaml::Yaml::Array(value) => Value::Array(value.into_iter().map(morph).collect()),
        yaml::Yaml::Hash(value) => Value::Table(
            value
                .into_iter()
                .filter_map(|(key, value)| {
                    match key {
                        yaml::Yaml::Real(value) => Some(value),
                        yaml::Yaml::Integer(value) => Some(value.to_string()),
                        yaml::Yaml::String(value) => Some(value),
                        value => {
                            proc_macro_error::emit_call_site_error!(
                                "yaml parsing error: unsupported hash key: {:?}",
                                value
                            );
                            None
                        }
                    }
                    .map(|key| (key, morph(value)))
                })
                .collect(),
        ),
        yaml::Yaml::Alias(_) => Value::Nil,
        yaml::Yaml::Null => Value::Nil,
        yaml::Yaml::BadValue => Value::Nil,
    }
}
