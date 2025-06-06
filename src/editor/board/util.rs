use serde::{Deserialize, Serialize};

use crate::bpz::Pos;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub struct PosOrd {
    pub row: i32,
    pub col: i32,
}

impl From<Pos> for PosOrd {
    fn from(Pos { row, col }: Pos) -> Self {
        Self { row, col }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub struct UnorderedPair<T: Ord>(T, T);

impl<T: Ord> UnorderedPair<T> {
    pub fn new(a: impl Into<T>, b: impl Into<T>) -> Self {
        let mut a = a.into();
        let mut b = b.into();
        if b < a {
            std::mem::swap(&mut a, &mut b);
        }
        Self(a, b)
    }
}
