use std::{cmp::Ordering, collections::VecDeque, hash::Hash};

use crate::bpz::Pos;

#[derive(Debug, Clone, Copy, Eq)]
pub struct Line(pub Pos, pub Pos);

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

#[derive(Debug, Clone)]
pub enum DragMode {
    Add,
    Remove,
}

#[derive(Debug, Clone)]
pub struct DragState {
    pub last_pos: Pos,
    pub mode: Option<DragMode>,
    pub drawn_lines: VecDeque<Line>,
}
