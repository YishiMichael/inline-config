#[cfg(not(feature = "indexmap"))]
type Map<K, V> = std::collections::BTreeMap<K, V>;
#[cfg(feature = "indexmap")]
type Map<K, V> = indexmap::IndexMap<K, V>;

#[derive(Clone)]
pub enum Value {
    Nil,
    Boolean(bool),
    Integer(i64),
    Float(f64),
    String(String),
    Array(Vec<Self>),
    Table(Map<String, Self>),
}

impl<'v> std::iter::Sum for Value {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        let mut summand = Value::Table(Map::new());
        iter.for_each(|value| summand.update(value));
        summand
    }
}

impl Value {
    fn update(&mut self, new: Self) {
        match (self, new) {
            (Self::Table(old), Self::Table(new)) => {
                for (key, new_value) in new {
                    old.entry(key)
                        .or_insert(Value::Table(Map::new()))
                        .update(new_value);
                }
            }
            (old, new) => {
                let _ = std::mem::replace(old, new);
            }
        }
    }
}
