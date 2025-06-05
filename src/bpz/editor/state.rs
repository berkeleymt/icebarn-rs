use std::{cmp::Ordering, collections::HashSet, hash::Hash};

use itertools::Itertools;
use vec1::{vec1, Vec1};

use crate::bpz::{Dir, Pos};

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
pub enum DrawState {
    Idle,
    Clicked(Pos),
    Held(Pos),
    ClickedAndHeld { clicked: Pos, held: Pos },
    Drawing { visited: Vec1<Pos> },
    Erasing { last: Pos },
}

impl Default for DrawState {
    fn default() -> Self {
        Self::Idle
    }
}

#[derive(Debug, Clone, Default)]
pub struct State {
    lines: HashSet<Line>,
    draw_state: DrawState,
}

impl State {
    pub fn has_line(&self, p1: Pos, p2: Pos) -> bool {
        return self.lines.contains(&Line(p1, p2));
    }

    pub fn for_cell(&self, pos: Pos) -> HashSet<Dir> {
        Dir::iter()
            .filter(|&dir| self.has_line(pos, pos + dir))
            .collect()
    }

    pub fn on_click(&mut self, pos: Pos) {
        use DrawState::*;
        match self.draw_state {
            Idle | Drawing { .. } | Erasing { .. } => {
                self.draw_state = Clicked(pos);
            }
            Held(held) => {
                self.draw_state = ClickedAndHeld { clicked: pos, held };
            }
            Clicked(from) | ClickedAndHeld { clicked: from, .. } => {
                for (p1, p2) in from.line_to(pos).into_iter().tuple_windows() {
                    self.lines.insert(Line(p1, p2));
                }
                self.draw_state = Idle;
            }
        };
    }

    pub fn on_mousedown(&mut self, pos: Pos) {
        use DrawState::*;
        match self.draw_state {
            Clicked(clicked) => {
                self.draw_state = ClickedAndHeld { clicked, held: pos };
            }
            Idle | Held(_) | ClickedAndHeld { .. } | Drawing { .. } | Erasing { .. } => {
                self.draw_state = Held(pos)
            }
        };
    }

    pub fn on_mouseenter(&mut self, pos: Pos) {
        use DrawState::*;
        match self.draw_state {
            Idle | Clicked(_) => {}
            Held(held) | ClickedAndHeld { held, .. } if self.has_line(held, pos) => {
                self.lines.remove(&Line(held, pos));
                self.draw_state = Erasing { last: pos }
            }
            Held(held) | ClickedAndHeld { held, .. } => {
                self.lines.insert(Line(held, pos));
                self.draw_state = Drawing {
                    visited: vec1![held, pos],
                };
            }
            Drawing { ref mut visited } => {
                match Vec1::as_slice(&visited) {
                    [] => unreachable!(),
                    &[.., sl, last] if sl == pos => {
                        self.lines.remove(&Line(last, pos));
                        visited.pop().unwrap();
                    }
                    &[.., last] => {
                        for (p1, p2) in last.line_to(pos).into_iter().tuple_windows() {
                            self.lines.insert(Line(p1, p2));
                            visited.push(p2);
                        }
                    }
                };
            }
            Erasing { ref mut last } => {
                self.lines.remove(&Line(*last, pos));
                *last = pos;
            }
        }
    }

    pub fn on_mouseup(&mut self) {
        use DrawState::*;
        match self.draw_state {
            Idle | Clicked(_) => {}
            Held(_) | Drawing { .. } | Erasing { .. } => self.draw_state = Idle,
            ClickedAndHeld { clicked, .. } => self.draw_state = Clicked(clicked),
        }
    }
}
