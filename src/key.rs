use std::marker::PhantomData;

// Borrowed from frunk_core/labelled.rs
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
        _0 _1 _2 _3 _4 _5 _6 _7 _8 _9 __ _uc uc_
    );
}

pub struct KeyName<NAME> {
    pub phantom: PhantomData<NAME>,
    pub name: &'static str,
}

pub struct KeyIndex<const INDEX: usize> {
    pub index: usize,
}

pub struct KeyFallible;

pub struct PathNil;

pub struct PathCons<Head, Tail>(Head, Tail);
