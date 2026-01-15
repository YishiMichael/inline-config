use crate::value::Value;
use std::error::Error;

#[cfg(feature = "json")]
pub mod json;

#[cfg(feature = "toml")]
pub mod toml;

#[cfg(feature = "yaml")]
pub mod yaml;

pub trait Format {
    fn parse(s: &str) -> Result<Value, Box<dyn Error>>;
}
