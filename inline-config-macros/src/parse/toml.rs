use crate::value::Value;
use std::error::Error;

pub fn parse(s: &str) -> Result<Value, Box<dyn Error>> {
    let value = toml::from_str(s)?;
    Ok(morph(value))
}

fn morph(value: toml::Value) -> Value {
    match value {
        toml::Value::String(value) => Value::String(value),
        toml::Value::Integer(value) => Value::Integer(value),
        toml::Value::Float(value) => Value::Float(value),
        toml::Value::Boolean(value) => Value::Boolean(value),
        toml::Value::Datetime(value) => Value::String(value.to_string()),
        toml::Value::Array(value) => Value::Array(value.into_iter().map(morph).collect()),
        toml::Value::Table(value) => Value::Table(
            value
                .into_iter()
                .map(|(key, value)| (key, morph(value)))
                .collect(),
        ),
    }
}
