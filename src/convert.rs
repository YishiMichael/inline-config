// use crate::repr::{
//     ReprBoolean, ReprContainer, ReprFloat, ReprNegInt, ReprNil, ReprPosInt, ReprString,
// };

pub trait Convert<T> {
    fn convert() -> T;
}

pub trait ConvertData<R> {
    fn convert_data() -> Self;
}

impl<R, T> Convert<T> for R
where
    T: ConvertData<R>,
{
    fn convert() -> T {
        T::convert_data()
    }
}

// pub trait Convert<T> {
//     fn convert(&'static self) -> T;
// }

// pub trait NonOption {}

// impl<T> ConvertRepr<Option<T>> for ReprNil {
//     fn convert_repr(&self) -> Option<T> {
//         None
//     }
// }

// impl<T> ConvertRepr<T> for ReprNil
// where
//     T: Default + NonOption, // Avoid collision from `Option`.
// {
//     fn convert_repr(&self) -> T {
//         T::default()
//     }
// }

// impl<R, T> ConvertRepr<Option<T>> for R
// where
//     R: ConvertRepr<T> + std::ops::Deref, // Avoid collision from `ReprNil`.
// {
//     fn convert_repr(&'static self) -> Option<T> {
//         Some(self.convert_repr())
//     }
// }

// impl NonOption for bool {}
// impl ConvertRepr<bool> for ReprBoolean {
//     fn convert_repr(&self) -> bool {
//         **self
//     }
// }

// impl NonOption for i8 {}
// impl NonOption for i16 {}
// impl NonOption for i32 {}
// impl NonOption for i64 {}
// impl NonOption for i128 {}
// impl NonOption for isize {}
// impl NonOption for u8 {}
// impl NonOption for u16 {}
// impl NonOption for u32 {}
// impl NonOption for u64 {}
// impl NonOption for u128 {}
// impl NonOption for usize {}
// impl NonOption for f32 {}
// impl NonOption for f64 {}
// macro_rules! numeric_convert {
//     ($source:ty => $($target:ty)*) => {
//         $(
//             impl ConvertRepr<$target> for $source {
//                 fn convert_repr(&self) -> $target {
//                     **self as $target
//                 }
//             }
//         )*
//     };
// }
// numeric_convert!(ReprPosInt => i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize f32 f64);
// numeric_convert!(ReprNegInt => i8 i16 i32 i64 i128 isize f32 f64);
// numeric_convert!(ReprFloat => f32 f64);

// impl ConvertRepr<&'static str> for ReprString {
//     fn convert_repr(&'static self) -> &'static str {
//         self
//     }
// }
// impl ConvertRepr<String> for ReprString {
//     fn convert_repr(&self) -> String {
//         self.to_string()
//     }
// }

// impl<S, T> ConvertRepr<T> for ReprContainer<S>
// where
//     S: Convert<T>,
//     T: NonOption,
// {
//     fn convert_repr(&'static self) -> T {
//         self.0.convert()
//     }
// }

// impl<R, T> Convert<T> for R
// where
//     T: ConvertFrom<R>,
// {
//     fn convert(&'static self) -> T {
//         T::convert_from(self)
//     }
// }

// impl<T> NonOption for Vec<T> {}

// impl<K, T> NonOption for std::collections::BTreeMap<K, T> {}

// #[cfg(feature = "indexmap")]
// impl<K, T> NonOption for indexmap::IndexMap<K, T> {}
