pub trait Get<'c, K, T> {
    fn get(&'c self, key: K) -> T;
}

impl<'c, C, K, T> Get<'c, K, T> for C
where
    K: 'c,
    C: Access<'c, K>,
    C::Representation: ConvertInto<'c, T>,
{
    fn get(&'c self, key: K) -> T {
        <C::Representation as ConvertInto<'c, T>>::into(<C as Access<'c, K>>::access(self, key))
    }
}

pub trait Access<'c, K> {
    type Representation;

    fn access(&'c self, key: K) -> &'c Self::Representation;
}

pub trait Select<'c, KS> {
    type Representation;

    fn select(&'c self, key_segment: KS) -> &'c Self::Representation;
}

#[derive(Default)]
pub struct KeySegmentName<const H: u64>;

#[derive(Default)]
pub struct KeySegmentIndex<const I: isize>;

#[derive(Default)]
pub struct KeyNil;

#[derive(Default)]
pub struct KeyCons<KS, K>(KS, K);

impl<'c, C> Access<'c, KeyNil> for C {
    type Representation = C;

    fn access(&'c self, _key: KeyNil) -> &'c Self::Representation {
        self
    }
}

impl<'c, C, KS: 'c, K> Access<'c, KeyCons<KS, K>> for C
where
    C: Select<'c, KS>,
    C::Representation: Access<'c, K>,
{
    type Representation = <C::Representation as Access<'c, K>>::Representation;

    fn access(&'c self, key: KeyCons<KS, K>) -> &'c Self::Representation {
        <C::Representation as Access<'c, K>>::access(
            <C as Select<'c, KS>>::select(self, key.0),
            key.1,
        )
    }
}

// impl<'c, C, KSS, const I: usize> Access<'c, Cons<KeySegmentIndex<I>, KSS>> for C
// where
//     C: Access<'c, KeySegmentIndex<I>>,
//     C::Representation: Access<'c, KSS>,
// {
//     type Representation = <C::Representation as Access<'c, KSS>>::Representation;

//     fn access(&'c self, key: Cons<KeySegmentIndex<I>, KSS>) -> &'c Self::Representation {
//         <C::Representation as Access<'c, KSS>>::access(
//             <C as Access<'c, KeySegmentIndex<I>>>::access(self, key.0),
//             key.1,
//         )
//     }
// }

// #[doc(hidden)]
// pub mod __representation_types {
//     pub struct Nil;

//     pub struct Bool(pub bool);

//     pub struct StaticStr(pub &'static str);

//     pub enum Number {
//         Integer(i64),
//         Float(f64),
//     }
// }

// #[doc(hidden)]
// pub const MAX_TUPLE_ARITY: usize = 16;

// struct MyC {
//     a: i64,
//     b: String,
// }

// impl<'c, R> crate::__private::ConvertInto<'c, MyC> for R
// where
//     R: crate::__private::Access<'c, usize>,
//     <R as crate::__private::Access<'c, usize>>::Representation: crate::__private::ConvertInto<'c, i64>,
//     R: crate::__private::Access<'c, ()>,
//     <R as crate::__private::Access<'c, ()>>::Representation: crate::__private::ConvertInto<'c, String>,
// {
//     fn into(&'c self) -> MyC {
//         MyC {
//             a: self.access(<usize>::default()).into(),
//             b: self.access(<()>::default()).into(),
//         }
//     }
// }

pub trait ConvertInto<'r, T> {
    fn into(&'r self) -> T;
}

pub trait ConvertFrom<'r, R> {
    fn from(representation: &'r R) -> Self;
}

// pub struct Nil;

// pub struct Bool(pub bool);

// pub struct StaticStr(pub &'static str);

// pub enum Number {
//     Integer(i64),
//     Float(f64),
// }

// use super::access::__representation_types::{Bool, Nil, Number, StaticStr};

// impl<'c, R, P, C> Access<'c, Cons<R, P>> for C
// where
//     C: Access<'c, R>,
//     C::Representation: Access<'c, P>,
// {
//     type Representation = <C::Representation as Access<'c, P>>::Representation;

//     fn access(&'c self, key: Cons<R, P>) -> &'c Self::Representation {
//         self.access(key.root).access(key.postfix)
//     }
// }

// impl<T> ConvertInto<'_, T> for ()
// // TODO
// where
//     T: Default,
// {
//     fn into(&self) -> T {
//         T::default()
//     }
// }

impl ConvertInto<'_, bool> for bool {
    fn into(&self) -> bool {
        *self
    }
}

impl<'r> ConvertInto<'r, &'r str> for &'static str {
    fn into(&'r self) -> &'r str {
        *self
    }
}

impl ConvertInto<'_, String> for &'static str {
    fn into(&self) -> String {
        self.to_string()
    }
}

macro_rules! numeric_convert {
        ($($target:ty),*) => {
            $(
                impl ConvertInto<'_, $target> for i64 {
                    fn into(&self) -> $target {
                        *self as $target
                    }
                }

                impl ConvertInto<'_, $target> for f64 {
                    fn into(&self) -> $target {
                        *self as $target
                    }
                }
            )*
        };
    }
numeric_convert!(i32, i64, i128, isize, u32, u64, u128, usize, f32, f64);

impl<'r, R, T> ConvertInto<'r, T> for R
where
    T: ConvertFrom<'r, R>,
{
    fn into(&'r self) -> T {
        <T as ConvertFrom<'r, R>>::from(self)
    }
}

// include!(concat!(env!("OUT_DIR"), "/representation_tuples_impl.rs"));

// impl ConvertInto<RepresentationEmptyArray> for () {
//     fn into(_representation: &RepresentationEmptyArray) -> Self {
//         ()
//     }
// }

// impl<T> ConvertInto<RepresentationEmptyArray> for [T; 0] {
//     fn into(_representation: &RepresentationEmptyArray) -> Self {
//         []
//     }
// }

// impl<T> ConvertInto<RepresentationEmptyArray> for Vec<T> {
//     fn into(_representation: &RepresentationEmptyArray) -> Self {
//         Vec::new()
//     }
// }

// impl<R, T, const N: usize> ConvertInto<RepresentationArray<R, N>> for [T; N]
// where
//     T: ConvertInto<R>,
// {
//     fn into(representation: &RepresentationArray<R, N>) -> Self {
//         std::array::from_fn(|i| T::into(&representation.0[i]))
//     }
// }

// impl<R, T, const N: usize> ConvertInto<RepresentationArray<R, N>> for Vec<T>
// where
//     T: ConvertInto<R>,
// {
//     fn into(representation: &RepresentationArray<R, N>) -> Self {
//         <[T; N]>::into(representation).into()
//     }
// }
