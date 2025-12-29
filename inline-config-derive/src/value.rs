#[cfg(not(feature = "indexmap"))]
type Map<K, V> = std::collections::BTreeMap<K, V>;
#[cfg(feature = "indexmap")]
type Map<K, V> = indexmap::IndexMap<K, V>;

pub enum Value {
    Nil,
    Boolean(bool),
    Integer(i64),
    Float(f64),
    String(String),
    Array(Vec<Self>),
    Table(Map<String, Self>),
}

impl std::ops::AddAssign for Value {
    fn add_assign(&mut self, rhs: Self) {
        match (self, rhs) {
            (Self::Table(old), Self::Table(new)) => {
                for (key, new_value) in new {
                    old.entry(key)
                        .or_insert(Value::Table(Map::new()))
                        .add_assign(new_value);
                }
            }
            (old, new) => {
                let _ = std::mem::replace(old, new);
            }
        }
    }
}

impl std::ops::Add for Value {
    type Output = Self;

    fn add(mut self, rhs: Self) -> Self::Output {
        self += rhs;
        self
    }
}

impl std::iter::Sum for Value {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Value::Table(Map::new()), std::ops::Add::add)
    }
}
