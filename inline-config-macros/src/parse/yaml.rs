use crate::value::Value;
use serde_yaml as yaml;
use std::error::Error;

pub fn parse(s: &str) -> Result<Value, Box<dyn Error>> {
    let value = yaml::from_str(s)?;
    morph(value)
}

fn morph(value: yaml::Value) -> Result<Value, Box<dyn Error>> {
    Ok(match value {
        yaml::Value::Null => Value::Nil,
        yaml::Value::Bool(value) => Value::Boolean(value),
        yaml::Value::Number(value) => value.as_i64().map(Value::Integer).unwrap_or_else(|| {
            Value::Float(
                value.as_f64().unwrap(), // Never fails.
            )
        }),
        yaml::Value::String(value) => Value::String(value),
        yaml::Value::Sequence(value) => {
            Value::Array(value.into_iter().map(morph).collect::<Result<_, _>>()?)
        }
        yaml::Value::Mapping(value) => Value::Table(
            value
                .into_iter()
                .map(|(key, value)| {
                    yaml::to_string(&key)
                        .map_err(|e| Box::new(e) as Box<dyn Error>)
                        .and_then(|key| morph(value).map(|value| (key, value)))
                })
                .collect::<Result<_, _>>()?,
        ),
        yaml::Value::Tagged(value) => morph(value.value)?,
    })
}
