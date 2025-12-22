mod traits;

pub use inline_config_derive::{config, key, Config, Key};
pub use traits::Get;

#[doc(hidden)]
pub mod __private {
    pub use super::traits::*;
}
