mod convert;
mod get;
mod key;

#[doc(inline)]
pub use inline_config_derive::{ConfigData, Path, config, path};

#[doc(inline)]
pub use get::Get;

#[doc(hidden)]
pub mod __private {
    pub use crate::convert::*;
    pub use crate::key::*;
}
