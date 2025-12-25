mod convert;
mod key;
mod repr;

pub trait Get<'r, Path, AccessPhantom, ConvertPhantom, Type> {
    fn get(&'r self, path: Path) -> Type;
}

impl<'r, Repr, Path: 'r, AccessPhantom: 'r, ConvertPhantom, Type>
    Get<'r, Path, AccessPhantom, ConvertPhantom, Type> for Repr
where
    Repr: key::AccessPath<'r, Path, AccessPhantom>,
    Type: convert::Convert<Repr::Repr, ConvertPhantom>,
{
    fn get(&'r self, _path: Path) -> Type {
        convert::Convert::convert(self.access_path())
    }
}

pub use inline_config_derive::{config, path, ConfigData, Path};

#[doc(hidden)]
pub mod __private {
    pub mod convert {
        pub use crate::convert::*;
    }
    pub mod key {
        pub use crate::key::*;
    }
    pub mod repr {
        pub use crate::repr::*;
    }
}
