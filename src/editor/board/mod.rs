use std::collections::HashSet;

use crate::bpz::{Dir, Pos};

mod singleplayer;
pub use singleplayer::*;

pub trait Board: Send + Sync + 'static {
    fn contains(&self, p1: Pos, p2: Pos) -> bool;
    fn draw(&mut self, p1: Pos, p2: Pos);
    fn erase(&mut self, p1: Pos, p2: Pos);

    fn contains_all(&self, mut pairs: impl Iterator<Item = (Pos, Pos)>) -> bool {
        pairs.all(|(p1, p2)| self.contains(p1, p2))
    }

    fn dirs_for_cell(&self, pos: Pos) -> HashSet<Dir> {
        Dir::iter()
            .filter(|&dir| self.contains(pos, pos + dir))
            .collect()
    }
}
