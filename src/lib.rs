mod convert;
mod key;

pub use inline_config_derive::{config, path, ConfigData, Path};

pub trait Get<'c, P, T> {
    fn get(&'c self, path: P) -> T;
}

impl<'c, C, P, T> Get<'c, P, T> for C
where
    C: key::AccessPath<'c, P>,
    C::Repr: 'c + convert::ConvertInto<'c, T>,
{
    fn get(&'c self, _path: P) -> T {
        convert::ConvertInto::convert_into(self.access_path())
    }
}

#[doc(hidden)]
pub mod __private {
    pub use crate::convert::*;
    pub use crate::key::*;
}
