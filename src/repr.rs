#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ReprNil;

impl std::fmt::Debug for ReprNil {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "NIL")
    }
}

macro_rules! transparent_repr {
    ($(
        $repr_vis:vis struct $ident:ident$(<$generic:ident>)?($vis:vis $ty:ty) as $deref_ty:ty;
    )*) => {$(
        #[repr(transparent)]
        #[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
        $repr_vis struct $ident$(<$generic>)?($vis $ty);

        impl$(<$generic>)? std::ops::Deref for $ident$(<$generic>)? {
            type Target = $deref_ty;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl$(<$generic>)? std::fmt::Debug for $ident$(<$generic>)? $(where $generic: std::fmt::Debug)? {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }
    )*}
}

transparent_repr! {
    pub struct ReprBoolean(pub bool) as bool;
    pub struct ReprPosInt(pub u64) as u64;
    pub struct ReprNegInt(pub i64) as i64;
    pub struct ReprFloat(pub ordered_float::OrderedFloat<f64>) as f64;
    pub struct ReprString(pub &'static str) as str;
    pub struct ReprContainer<S>(pub S) as S;
}
