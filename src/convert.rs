pub trait ConvertInto<'r, T> {
    fn convert_into(&'r self) -> T;
}

pub trait ConvertFrom<'r, R> {
    fn convert_from(repr: &'r R) -> Self;
}

pub trait NonNilRepr {}
pub trait NonNil {}

impl<'r, R, T> ConvertInto<'r, T> for R
where
    T: ConvertFrom<'r, R>,
{
    fn convert_into(&'r self) -> T {
        <T as ConvertFrom<'r, R>>::convert_from(self)
    }
}

impl<T> ConvertFrom<'_, ()> for T
where
    T: Default + NonNil,
{
    fn convert_from(_repr: &()) -> T {
        T::default()
    }
}

impl<'r, T, R> ConvertFrom<'r, R> for Option<T>
where
    R: ConvertInto<'r, T> + NonNilRepr,
{
    fn convert_from(repr: &'r R) -> Option<T> {
        Some(repr.convert_into())
    }
}

impl<T> ConvertFrom<'_, ()> for Option<T> {
    fn convert_from(_repr: &()) -> Option<T> {
        None
    }
}

impl ConvertFrom<'_, bool> for bool {
    fn convert_from(repr: &bool) -> Self {
        *repr
    }
}

impl NonNilRepr for bool {}
impl NonNil for bool {}

impl<'r> ConvertFrom<'_, &'r str> for &'r str {
    fn convert_from(repr: &&'r str) -> Self {
        *repr
    }
}

impl ConvertFrom<'_, &str> for String {
    fn convert_from(repr: &&str) -> Self {
        repr.to_string()
    }
}

impl NonNilRepr for &str {}
impl NonNil for &str {}
impl NonNil for String {}

macro_rules! numeric_convert {
    ($($target:ty)*) => {
        $(
            impl NonNil for $target {}

            impl ConvertFrom<'_, i64> for $target {
                fn convert_from(repr: &i64) -> Self {
                    *repr as $target
                }
            }
            impl ConvertFrom<'_, f64> for $target {
                fn convert_from(repr: &f64) -> Self {
                    *repr as $target
                }
            }
        )*
    };
}
numeric_convert!(i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize f32 f64);

impl NonNilRepr for i64 {}
impl NonNilRepr for f64 {}

impl<T> NonNil for Vec<T> {}
impl<T> NonNil for std::collections::BTreeMap<&str, T> {}
impl<T> NonNil for std::collections::BTreeMap<String, T> {}

#[cfg(feature = "indexmap")]
impl<T> NonNil for indexmap::IndexMap<&str, T> {}
#[cfg(feature = "indexmap")]
impl<T> NonNil for indexmap::IndexMap<String, T> {}
