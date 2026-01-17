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
