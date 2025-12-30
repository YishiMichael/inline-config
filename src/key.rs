use std::marker::PhantomData;

// Borrowed from `frunk_core::labelled::chars`.
pub mod chars {
    macro_rules! create_enums_for {
        ($($c:tt)*) => {
            $(
                #[allow(non_camel_case_types)]
                pub struct $c;
            )*
        };
    }

    create_enums_for!(
        A B C D E F G H I J K L M N O P Q R S T U V W X Y Z
        a b c d e f g h i j k l m n o p q r s t u v w x y z
        _0 _1 _2 _3 _4 _5 _6 _7 _8 _9 __
    );

    // For unicode chars.
    pub struct UC<const CODEPOINT: u32>;
}

pub struct KeyIndex<Index>(PhantomData<Index>);

pub struct KeyName<Name>(PhantomData<Name>);

impl<Index> Default for KeyIndex<Index> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<Name> Default for KeyName<Name> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

#[derive(Default)]
pub struct PathNil;

#[derive(Default)]
pub struct PathCons<K, KS>(K, KS);

pub trait AccessKey<K> {
    type Repr;

    fn access_key(&self) -> &Self::Repr;
}

pub trait AccessPath<'c, P> {
    type Repr;

    fn access_path(&'c self) -> &'c Self::Repr;
}

impl<'c, R> AccessPath<'c, PathNil> for R {
    type Repr = R;

    fn access_path(&'c self) -> &'c Self::Repr {
        self
    }
}

impl<'c, R, K, KS> AccessPath<'c, PathCons<K, KS>> for R
where
    R: AccessKey<K>,
    R::Repr: 'c + AccessPath<'c, KS>,
{
    type Repr = <R::Repr as AccessPath<'c, KS>>::Repr;

    fn access_path(&'c self) -> &'c Self::Repr {
        self.access_key().access_path()
    }
}
