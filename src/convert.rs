use super::key::{KeyIndex, KeyName};
use super::repr::{Bool, Field, Float, HCons, HNil, Integer, Nil, StaticStr};
use indexmap::IndexMap;
use std::collections::HashMap;

pub struct PrimitivePhantom;
pub struct NilOptionPhantom;

pub trait Convert<Source, Phantom> {
    fn convert(source: &Source) -> Self;
}

impl<Target, Source, Phantom> Convert<Source, Option<Phantom>> for Option<Target>
where
    Target: Convert<Source, Phantom>,
{
    fn convert(source: &Source) -> Self {
        Some(Convert::convert(source))
    }
}

impl<Target> Convert<Nil, NilOptionPhantom> for Option<Target> {
    fn convert(_source: &Nil) -> Self {
        None
    }
}

impl Convert<Bool, PrimitivePhantom> for bool {
    fn convert(source: &Bool) -> Self {
        source.0
    }
}

macro_rules! transmogrify_numeric {
    ($($target:ty)*) => {
        $(
            impl Convert<Integer, PrimitivePhantom> for $target {
                fn convert(source: &Integer) -> Self {
                    source.0 as $target
                }
            }
            impl Convert<Float, PrimitivePhantom> for $target {
                fn convert(source: &Float) -> Self {
                    source.0 as $target
                }
            }
        )*
    };
}
transmogrify_numeric!(i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize f32 f64);

impl Convert<StaticStr, PrimitivePhantom> for &'static str {
    fn convert(source: &StaticStr) -> Self {
        source.0
    }
}

impl<Target> Convert<HNil, HNil> for Vec<Target> {
    fn convert(_source: &HNil) -> Self {
        Vec::new()
    }
}
impl<Target, Index, Repr, Tail, HeadPhantom, TailPhantom>
    Convert<HCons<Field<KeyIndex<Index>, Repr>, Tail>, HCons<HeadPhantom, TailPhantom>>
    for Vec<Target>
where
    Target: Convert<Repr, HeadPhantom>,
    Vec<Target>: Convert<Tail, TailPhantom>,
{
    fn convert(source: &HCons<Field<KeyIndex<Index>, Repr>, Tail>) -> Self {
        let mut output: Self = Convert::convert(&source.tail);
        output.insert(0, Convert::convert(&source.head.value));
        output
    }
}

impl<Target> Convert<HNil, HNil> for HashMap<&'static str, Target> {
    fn convert(_source: &HNil) -> Self {
        HashMap::new()
    }
}
impl<Target, Name, Repr, Tail, HeadPhantom, TailPhantom>
    Convert<HCons<Field<KeyName<Name>, Repr>, Tail>, HCons<HeadPhantom, TailPhantom>>
    for HashMap<&'static str, Target>
where
    Target: Convert<Repr, HeadPhantom>,
    HashMap<&'static str, Target>: Convert<Tail, TailPhantom>,
{
    fn convert(source: &HCons<Field<KeyName<Name>, Repr>, Tail>) -> Self {
        let mut output: Self = Convert::convert(&source.tail);
        output.insert(source.head.key.name, Convert::convert(&source.head.value));
        output
    }
}

impl<Target> Convert<HNil, HNil> for IndexMap<&'static str, Target> {
    fn convert(_source: &HNil) -> Self {
        IndexMap::new()
    }
}
impl<Target, Name, Repr, Tail, HeadPhantom, TailPhantom>
    Convert<HCons<Field<KeyName<Name>, Repr>, Tail>, HCons<HeadPhantom, TailPhantom>>
    for IndexMap<&'static str, Target>
where
    Target: Convert<Repr, HeadPhantom>,
    IndexMap<&'static str, Target>: Convert<Tail, TailPhantom>,
{
    fn convert(source: &HCons<Field<KeyName<Name>, Repr>, Tail>) -> Self {
        let mut output: Self = Convert::convert(&source.tail);
        output.shift_insert(
            0,
            source.head.key.name,
            Convert::convert(&source.head.value),
        );
        output
    }
}
