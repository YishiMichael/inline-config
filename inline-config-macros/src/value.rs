pub enum Value {
    Nil,
    Boolean(bool),
    PosInt(u64),
    NegInt(i64),
    Float(f64),
    String(String),
    Array(Vec<Self>),
    Table(indexmap::IndexMap<String, Self>),
}

impl std::ops::AddAssign for Value {
    fn add_assign(&mut self, rhs: Self) {
        match (self, rhs) {
            (Self::Table(old), Self::Table(new)) => {
                for (key, new_value) in new {
                    old.entry(key)
                        .or_insert(Value::Table(indexmap::IndexMap::new()))
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
        iter.fold(Value::Table(indexmap::IndexMap::new()), std::ops::Add::add)
    }
}
