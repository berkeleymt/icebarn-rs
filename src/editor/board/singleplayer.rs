use std::collections::HashSet;

use super::Board;
use crate::{
    bpz::Pos,
    editor::board::util::{PosOrd, UnorderedPair},
};

#[derive(Debug, Clone, Default)]
pub struct SingleplayerBoard(HashSet<UnorderedPair<PosOrd>>);

impl Board for SingleplayerBoard {
    fn contains(&self, p1: Pos, p2: Pos) -> bool {
        self.0.contains(&UnorderedPair::new(p1, p2))
    }

    fn draw(&mut self, p1: Pos, p2: Pos) {
        self.0.insert(UnorderedPair::new(p1, p2));
    }

    fn erase(&mut self, p1: Pos, p2: Pos) {
        self.0.remove(&UnorderedPair::new(p1, p2));
    }

    fn clear(&mut self) {
        self.0.clear();
    }
}
