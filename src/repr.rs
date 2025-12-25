pub struct Nil;

pub struct Bool(pub bool);

pub struct Integer(pub i64);

pub struct Float(pub f64);

pub struct StaticStr(pub &'static str);

pub struct HNil;

pub struct HCons<Head, Tail> {
    pub head: Head,
    pub tail: Tail,
}

pub struct Field<Key, Repr> {
    pub key: Key,
    pub value: Repr,
}
