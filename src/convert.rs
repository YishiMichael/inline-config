// use super::key::{KeyIndex, KeyName};
// use super::repr::{Bool, Field, Float, HCons, HNil, Integer, Nil, StaticStr};
// use indexmap::IndexMap;
// use std::collections::HashMap;

// pub trait Convert<Source> {
//     fn convert(source: &Source) -> Self;
// }

// impl<Target, Source, Phantom> Convert<Source, Option<Phantom>> for Option<Target>
// where
//     Target: Convert<Source, Phantom>,
// {
//     fn convert(source: &Source) -> Self {
//         Some(Convert::convert(source))
//     }
// }

// // TODO: default fill-in

// impl<Target> Convert<Nil, NilOptionPhantom> for Option<Target> {
//     fn convert(_source: &Nil) -> Self {
//         None
//     }
// }

// TODO: () Nil
pub trait ConvertInto<'r, T> {
    fn convert_into(&'r self) -> T;
}

pub trait ConvertFrom<'r, R> {
    fn convert_from(repr: &'r R) -> Self;
}

impl<'r, R, T> ConvertInto<'r, T> for R
where
    T: ConvertFrom<'r, R>,
{
    fn convert_into(&'r self) -> T {
        <T as ConvertFrom<'r, R>>::convert_from(self)
    }
}

impl ConvertFrom<'_, bool> for bool {
    fn convert_from(repr: &bool) -> Self {
        *repr
    }
}

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

macro_rules! numeric_convert {
    ($($target:ty)*) => {
        $(
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
