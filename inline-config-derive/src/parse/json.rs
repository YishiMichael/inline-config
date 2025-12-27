use crate::value::Value;
use serde_json as json;
use std::error::Error;

pub fn parse(s: &str) -> Result<Value, Box<dyn Error>> {
    let value = json::from_str(s)?;
    Ok(morph(value))
}

fn morph(value: json::Value) -> Value {
    match value {
        json::Value::Null => Value::Nil,
        json::Value::Bool(value) => Value::Boolean(value),
        json::Value::Number(value) => value
            .as_i64()
            .map(Value::Integer)
            .unwrap_or_else(|| value.as_f64().map(Value::Float).unwrap_or(Value::Nil)),
        json::Value::String(value) => Value::String(value),
        json::Value::Array(value) => Value::Array(value.into_iter().map(morph).collect()),
        json::Value::Object(value) => Value::Table(
            value
                .into_iter()
                .map(|(key, value)| (key, morph(value)))
                .collect(),
        ),
    }
}
