use crate::value::Value;
use std::error::Error;

pub fn parse(s: &str) -> Result<Value, Box<dyn Error>> {
    let value = toml::from_str(s)?;
    morph(value)
}

fn morph(value: toml::Value) -> Result<Value, Box<dyn Error>> {
    Ok(match value {
        toml::Value::String(value) => Value::String(value),
        toml::Value::Integer(value) => {
            if value.is_negative() {
                Value::NegInt(value)
            } else {
                Value::PosInt(value as u64)
            }
        }
        toml::Value::Float(value) => Value::Float(value),
        toml::Value::Boolean(value) => Value::Boolean(value),
        toml::Value::Datetime(value) => Value::String(value.to_string()),
        toml::Value::Array(value) => {
            Value::Array(value.into_iter().map(morph).collect::<Result<_, _>>()?)
        }
        toml::Value::Table(value) => Value::Table(
            value
                .into_iter()
                .map(|(key, value)| morph(value).map(|value| (key, value)))
                .collect::<Result<_, _>>()?,
        ),
    })
}
