use crate::value::Value;
use serde_json as json;
use std::error::Error;

pub fn parse(s: &str) -> Result<Value, Box<dyn Error>> {
    let value = json::from_str(s)?;
    morph(value)
}

fn morph(value: json::Value) -> Result<Value, Box<dyn Error>> {
    Ok(match value {
        json::Value::Null => Value::Nil,
        json::Value::Bool(value) => Value::Boolean(value),
        json::Value::Number(value) => value.as_u64().map(Value::PosInt).unwrap_or_else(|| {
            value.as_i64().map(Value::NegInt).unwrap_or_else(|| {
                Value::Float(
                    value.as_f64().unwrap(), // Never fails.
                )
            })
        }),
        json::Value::String(value) => Value::String(value),
        json::Value::Array(value) => {
            Value::Array(value.into_iter().map(morph).collect::<Result<_, _>>()?)
        }
        json::Value::Object(value) => Value::Table(
            value
                .into_iter()
                .map(|(key, value)| morph(value).map(|value| (key, value)))
                .collect::<Result<_, _>>()?,
        ),
    })
}
