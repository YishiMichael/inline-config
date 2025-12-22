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
pub struct KeySegmentIndex<const I: usize>;

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

pub trait ConvertInto<'r, T> {
    fn into(&'r self) -> T;
}

pub trait ConvertFrom<'r, R> {
    fn from(representation: &'r R) -> Self;
}

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
