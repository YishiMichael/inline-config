use crate::value::Value;
use std::error::Error;

#[cfg(feature = "json")]
pub mod json;

#[cfg(feature = "toml")]
pub mod toml;

#[cfg(feature = "yaml")]
pub mod yaml;

pub enum Format {
    #[cfg(feature = "json")]
    Json,

    #[cfg(feature = "toml")]
    Toml,

    #[cfg(feature = "yaml")]
    Yaml,
}

impl Format {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            #[cfg(feature = "json")]
            "json" => Some(Self::Json),

            #[cfg(feature = "toml")]
            "toml" => Some(Self::Toml),

            #[cfg(feature = "yaml")]
            "yaml" => Some(Self::Yaml),

            _ => None,
        }
    }

    pub fn parse(&self, s: &str) -> Result<Value, Box<dyn Error>> {
        match self {
            #[cfg(feature = "json")]
            Self::Json => json::parse(s),

            #[cfg(feature = "toml")]
            Self::Toml => toml::parse(s),

            #[cfg(feature = "yaml")]
            Self::Yaml => yaml::parse(s),
        }
    }
}
