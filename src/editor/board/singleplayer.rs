use std::{cmp::Ordering, collections::HashSet, hash::Hash};

use super::Board;
use crate::bpz::Pos;

#[derive(Debug, Clone, Copy, Eq)]
struct Line(Pos, Pos);

impl Hash for Line {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match Ord::cmp(&self.0.row, &self.1.row).then(Ord::cmp(&self.0.col, &self.1.col)) {
            Ordering::Greater => {
                self.1.hash(state);
                self.0.hash(state);
            }
            _ => {
                self.0.hash(state);
                self.1.hash(state);
            }
        }
    }
}

impl PartialEq for Line {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0 && self.1 == other.1 || self.0 == other.1 && self.1 == other.0
    }
}

#[derive(Debug, Clone, Default)]
pub struct SingleplayerBoard(HashSet<Line>);

impl Board for SingleplayerBoard {
    fn contains(&self, p1: Pos, p2: Pos) -> bool {
        self.0.contains(&Line(p1, p2))
    }

    fn draw(&mut self, p1: Pos, p2: Pos) {
        self.0.insert(Line(p1, p2));
    }

    fn erase(&mut self, p1: Pos, p2: Pos) {
        self.0.remove(&Line(p1, p2));
    }
}
