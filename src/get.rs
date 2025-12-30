use crate::convert::ConvertInto;
use crate::key::AccessPath;

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
