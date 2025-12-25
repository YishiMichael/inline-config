use super::key::{KeyIndex, KeyName};
use indexmap::IndexMap;
use std::collections::HashMap;

pub struct Nil;

pub struct Bool(pub bool);

pub struct Integer(pub i64);

pub struct Float(pub f64);

pub struct StaticStr(pub &'static str);

pub struct HNil;

pub struct HCons<Key, Type, Tail> {
    // index: usize,
    // key: Key,
    // value: Type,
    // head: Head, // FieldUnnamed || FieldNamed
    pub key: Key,
    pub value: Type,
    pub tail: Tail, // HNil || HCons
}

// pub struct FieldUnnamed<Key>, Type {
//     key: PhantomData<Key>,
//     index: usize,
//     value: Type,
// }

// pub struct FieldNamed<Key>, Type {
//     key: PhantomData<Key>,
//     name: &'static str,
//     value: Type,
// }

// pub struct TNil;

// pub struct TCons<Key, Type, Tail> {
//     name: &'static str,
//     key: Key,
//     value: Type,
//     tail: Tail,
// }

trait NonNil {}
impl NonNil for Bool {}
impl NonNil for Integer {}
impl NonNil for Float {}
impl NonNil for StaticStr {}
impl NonNil for HNil {}
impl<Key, Type, Tail> NonNil for HCons<Key, Type, Tail> {}

pub trait NonOption {}

pub trait Convert<Source> {
    fn convert(source: &'static Source) -> Self;
}

impl<Target, Source> Convert<Source> for Option<Target>
where
    Source: NonNil,
    Target: Convert<Source>,
{
    fn convert(source: &'static Source) -> Self {
        Some(Convert::convert(source))
    }
}

impl<Target> Convert<Nil> for Target
where
    Target: Default + NonOption,
{
    fn convert(_source: &'static Nil) -> Self {
        Default::default()
    }
}

impl<Target> Convert<Nil> for Option<Target>
where
    Target: Convert<Nil>,
{
    fn convert(_source: &'static Nil) -> Self {
        None
    }
}

impl NonOption for bool {}
impl Convert<Bool> for bool {
    fn convert(source: &'static Bool) -> Self {
        source.0
    }
}

macro_rules! transmogrify_numeric {
    ($($target:ty)*) => {
        $(
            impl NonOption for $target {}
            impl Convert<Integer> for $target {
                fn convert(source: &'static Integer) -> Self {
                    source.0 as $target
                }
            }
            impl Convert<Float> for $target {
                fn convert(source: &'static Float) -> Self {
                    source.0 as $target
                }
            }
        )*
    };
}
transmogrify_numeric!(i32 i64 i128 isize u32 u64 u128 usize f32 f64);

impl NonOption for &'static str {}
impl Convert<StaticStr> for &'static str {
    fn convert(source: &'static StaticStr) -> Self {
        source.0
    }
}

impl<Target> NonOption for Vec<Target> {}
impl<Target> Convert<HNil> for Vec<Target> {
    fn convert(_source: &'static HNil) -> Self {
        Vec::new()
    }
}
impl<Target, Type, Tail, const INDEX: usize> Convert<HCons<KeyIndex<INDEX>, Type, Tail>>
    for Vec<Target>
where
    Target: Convert<Type>,
    Vec<Target>: Convert<Tail>,
{
    fn convert(source: &'static HCons<KeyIndex<INDEX>, Type, Tail>) -> Self {
        let mut output: Self = Convert::convert(&source.tail);
        output.insert(0, Convert::convert(&source.value));
        output
    }
}

impl<Target> NonOption for HashMap<&'static str, Target> {}
impl<Target> Convert<HNil> for HashMap<&'static str, Target> {
    fn convert(_source: &'static HNil) -> Self {
        HashMap::new()
    }
}
impl<Target, Type, Tail, KEY> Convert<HCons<KeyName<KEY>, Type, Tail>>
    for HashMap<&'static str, Target>
where
    Target: Convert<Type>,
    HashMap<&'static str, Target>: Convert<Tail>,
{
    fn convert(source: &'static HCons<KeyName<KEY>, Type, Tail>) -> Self {
        let mut output: Self = Convert::convert(&source.tail);
        output.insert(source.key.name, Convert::convert(&source.value));
        output
    }
}

impl<Target> NonOption for IndexMap<&'static str, Target> {}
impl<Target> Convert<HNil> for IndexMap<&'static str, Target> {
    fn convert(_source: &'static HNil) -> Self {
        IndexMap::new()
    }
}
impl<Target, Key, Type, Tail> Convert<HCons<KeyName<Key>, Type, Tail>>
    for IndexMap<&'static str, Target>
where
    Target: Convert<Type>,
    IndexMap<&'static str, Target>: Convert<Tail>,
{
    fn convert(source: &'static HCons<KeyName<Key>, Type, Tail>) -> Self {
        let mut output: Self = Convert::convert(&source.tail);
        output.shift_insert(0, source.key.name, Convert::convert(&source.value));
        output
    }
}

// impl Convert<&'static Nil> for bool {
//     fn convert(_source: &'static Nil) -> Self {
//         false
//     }
// }

// impl Convert<&'static Nil> for Option<bool> {
//     fn convert(_source: &'static Nil) -> Self {
//         None
//     }
// }

// frunk::labelled::Transmogrifier
// pub trait Transmogrify<Target> {
//     fn transmogrify(self) -> Target;
// }

// // Bool -> bool
// impl Transmogrify<bool> for &'static Bool {
//     fn transmogrify(self) -> bool {
//         self.0
//     }
// }

// impl Transmogrify<Option<bool>> for &'static Bool {
//     fn transmogrify(self) -> Option<bool> {
//         Some(self.transmogrify())
//     }
// }

// impl Transmogrify<Option<bool>> for &'static Nil {
//     fn transmogrify(self) -> Option<bool> {
//         None
//     }
// }

// // StaticStr -> &'static str
// impl Transmogrify<&'static str> for &'static StaticStr {
//     fn transmogrify(self) -> &'static str {
//         self.0
//     }
// }

// impl Transmogrify<Option<&'static str>> for &'static StaticStr {
//     fn transmogrify(self) -> Option<&'static str> {
//         Some(self.transmogrify())
//     }
// }

// impl Transmogrify<Option<&'static str>> for &'static Nil {
//     fn transmogrify(self) -> Option<&'static str> {
//         None
//     }
// }

// // HList -> Vec<Target>
// impl<Target> Transmogrify<Vec<Target>> for &'static HNil {
//     fn transmogrify(self) -> Vec<Target> {
//         Vec::new()
//     }
// }

// impl<Key, Type, Tail, Target> Transmogrify<Vec<Target>>
//     for &'static HCons<FieldUnnamed<Key>, Type, Tail>
// where
//     &'static Type: Transmogrify<Target>,
//     &'static Tail: Transmogrify<Vec<Target>>,
// {
//     fn transmogrify(self) -> Vec<Target> {
//         let mut output = self.tail.transmogrify();
//         output.insert(0, self.head.value.transmogrify());
//         output
//     }
// }

// impl<Target> Transmogrify<Option<Vec<Target>>> for &'static HNil {
//     fn transmogrify(self) -> Option<Vec<Target>> {
//         Some(self.transmogrify())
//     }
// }

// impl<Key, Type, Tail, Target> Transmogrify<Option<Vec<Target>>>
//     for &'static HCons<FieldUnnamed<Key>, Type, Tail>
// where
//     &'static Type: Transmogrify<Target>,
//     &'static Tail: Transmogrify<Vec<Target>>,
// {
//     fn transmogrify(self) -> Option<Vec<Target>> {
//         Some(self.transmogrify())
//     }
// }

// impl<Target> Transmogrify<Option<Vec<Target>>> for &'static Nil {
//     fn transmogrify(self) -> Option<Vec<Target>> {
//         None
//     }
// }

// // HList -> HashMap<&'static str, Target>
// impl<Target> Transmogrify<HashMap<&'static str, Target>> for &'static HNil {
//     fn transmogrify(self) -> HashMap<&'static str, Target> {
//         HashMap::new()
//     }
// }

// impl<Key, Type, Tail, Target> Transmogrify<HashMap<&'static str, Target>>
//     for &'static HCons<FieldNamed<Key>, Type, Tail>
// where
//     &'static Type: Transmogrify<Target>,
//     &'static Tail: Transmogrify<HashMap<&'static str, Target>>,
// {
//     fn transmogrify(self) -> HashMap<&'static str, Target> {
//         let mut output = self.tail.transmogrify();
//         output.insert(self.head.name, self.head.value.transmogrify());
//         output
//     }
// }

// impl<Target> Transmogrify<Option<HashMap<&'static str, Target>>> for &'static HNil {
//     fn transmogrify(self) -> Option<HashMap<&'static str, Target>> {
//         Some(self.transmogrify())
//     }
// }

// impl<Key, Type, Tail, Target> Transmogrify<Option<HashMap<&'static str, Target>>>
//     for &'static HCons<FieldNamed<Key>, Type, Tail>
// where
//     &'static Type: Transmogrify<Target>,
//     &'static Tail: Transmogrify<HashMap<&'static str, Target>>,
// {
//     fn transmogrify(self) -> Option<HashMap<&'static str, Target>> {
//         Some(self.transmogrify())
//     }
// }

// impl<Target> Transmogrify<Option<HashMap<&'static str, Target>>> for &'static Nil {
//     fn transmogrify(self) -> Option<HashMap<&'static str, Target>> {
//         None
//     }
// }

// // HList -> IndexMap<&'static str, Target>
// impl<Target> Transmogrify<IndexMap<&'static str, Target>> for &'static HNil {
//     fn transmogrify(self) -> IndexMap<&'static str, Target> {
//         IndexMap::new()
//     }
// }

// impl<Key, Type, Tail, Target> Transmogrify<IndexMap<&'static str, Target>>
//     for &'static HCons<FieldNamed<Key>, Type, Tail>
// where
//     &'static Type: Transmogrify<Target>,
//     &'static Tail: Transmogrify<IndexMap<&'static str, Target>>,
// {
//     fn transmogrify(self) -> IndexMap<&'static str, Target> {
//         let mut output = self.tail.transmogrify();
//         output.shift_insert(0, self.head.name, self.head.value.transmogrify());
//         output
//     }
// }

// impl<Target> Transmogrify<Option<IndexMap<&'static str, Target>>> for &'static HNil {
//     fn transmogrify(self) -> Option<IndexMap<&'static str, Target>> {
//         Some(self.transmogrify())
//     }
// }

// impl<Key, Type, Tail, Target> Transmogrify<Option<IndexMap<&'static str, Target>>>
//     for &'static HCons<FieldNamed<Key>, Type, Tail>
// where
//     &'static Type: Transmogrify<Target>,
//     &'static Tail: Transmogrify<IndexMap<&'static str, Target>>,
// {
//     fn transmogrify(self) -> Option<IndexMap<&'static str, Target>> {
//         Some(self.transmogrify())
//     }
// }

// impl<Target> Transmogrify<Option<IndexMap<&'static str, Target>>> for &'static Nil {
//     fn transmogrify(self) -> Option<IndexMap<&'static str, Target>> {
//         None
//     }
// }
