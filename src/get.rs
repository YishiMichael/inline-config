use crate::convert::ConvertInto;
use crate::key::AccessPath;

/// A trait modeling the type compatibility.
///
/// A type bound `C: Get<'_, P, T>` means the data at path `P` from config `C` is compatible with and can be converted into `T`.
///
/// This trait is not meant to be custom implemented.
/// The trait bound is satisfied by macro-generated implementations of internal helper traits.
pub trait Get<'c, P, T> {
    fn get(&'c self, path: P) -> T;
}

impl<'c, C, P, T> Get<'c, P, T> for C
where
    C: AccessPath<'c, P>,
    C::Repr: 'c + ConvertInto<'c, T>,
{
    fn get(&'c self, _path: P) -> T {
        self.access_path().convert_into()
    }
}
