use crate::value::Value;
use std::error::Error;

#[cfg(feature = "json")]
mod json;

#[cfg(feature = "toml")]
mod toml;

#[cfg(feature = "yaml")]
mod yaml;

pub enum Format {
    #[cfg(feature = "json")]
    Json,

    #[cfg(feature = "toml")]
    Toml,

    #[cfg(feature = "yaml")]
    Yaml,
}

impl Format {
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

impl std::str::FromStr for Format {
    type Err = FormatError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            #[cfg(feature = "json")]
            "json" => Ok(Self::Json),

            #[cfg(feature = "toml")]
            "toml" => Ok(Self::Toml),

            #[cfg(feature = "yaml")]
            "yaml" => Ok(Self::Yaml),

            _ => Err(FormatError),
        }
    }
}

pub struct FormatError;

impl std::fmt::Display for FormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("unsupported format")
    }
}
