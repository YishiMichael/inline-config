use super::repr::{Field, HCons};
use std::marker::PhantomData;

// Borrowed from frunk_core::labelled::chars
pub mod chars {
    macro_rules! create_enums_for {
        ($($c:tt)*) => {
            $(
                #[allow(non_camel_case_types)]
                #[derive(PartialEq, Debug, Eq, Clone, Copy, PartialOrd, Ord, Hash)]
                pub enum $c {}
            )*
        };
    }

    create_enums_for!(
        A B C D E F G H I J K L M N O P Q R S T U V W X Y Z
        a b c d e f g h i j k l m n o p q r s t u v w x y z
        _0 _1 _2 _3 _4 _5 _6 _7 _8 _9 __
    );

    // For unicode chars
    pub enum UC<const CODEPOINT: u32> {}
}

pub struct KeyIndex<Index> {
    pub phantom: PhantomData<Index>,
    pub index: usize,
}

pub struct KeyName<Name> {
    pub phantom: PhantomData<Name>,
    pub name: &'static str,
}

// The phantom types are used to avoid overlapping impls for some traits.
// See frunk_core::indices, frunk_core::labelled::ByNameFieldPlucker
pub struct HerePhantom;

pub struct TherePhantom<Phantom>(Phantom);

pub struct PathNil;

pub struct PathCons<Head, Tail>(pub Head, pub Tail);

pub trait AccessKey<Key, Phantom> {
    type Repr;

    fn access_key(&self) -> &Self::Repr;
}

pub trait AccessPath<'r, Path, Phantom> {
    type Repr;

    fn access_path(&'r self) -> &'r Self::Repr;
}

impl<Key, Repr, Tail> AccessKey<Key, HerePhantom> for HCons<Field<Key, Repr>, Tail> {
    type Repr = Repr;

    fn access_key(&self) -> &Self::Repr {
        &self.head.value
    }
}

impl<Head, Tail, Key, Phantom> AccessKey<Key, TherePhantom<Phantom>> for HCons<Head, Tail>
where
    Tail: AccessKey<Key, Phantom>,
{
    type Repr = Tail::Repr;

    fn access_key(&self) -> &Self::Repr {
        self.tail.access_key()
    }
}

impl<Repr> AccessPath<'_, PathNil, PathNil> for Repr {
    type Repr = Repr;

    fn access_path(&self) -> &Self::Repr {
        self
    }
}

impl<'r, Head, Tail, PathHead, PathTail, PhantomHead, PhantomTail>
    AccessPath<'r, PathCons<PathHead, PathTail>, PathCons<PhantomHead, PhantomTail>>
    for HCons<Head, Tail>
where
    HCons<Head, Tail>: AccessKey<PathHead, PhantomHead>,
    <HCons<Head, Tail> as AccessKey<PathHead, PhantomHead>>::Repr:
        'r + AccessPath<'r, PathTail, PhantomTail>,
{
    type Repr = <<HCons<Head, Tail> as AccessKey<PathHead, PhantomHead>>::Repr as AccessPath<
        'r,
        PathTail,
        PhantomTail,
    >>::Repr;

    fn access_path(&'r self) -> &'r Self::Repr {
        self.access_key().access_path()
    }
}
