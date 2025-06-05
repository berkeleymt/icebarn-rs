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

#[derive(Debug, Clone, Default)]
pub struct Lines(HashSet<Line>);

impl Lines {
    fn contains(&self, p1: Pos, p2: Pos) -> bool {
        self.0.contains(&Line(p1, p2))
    }

    fn contains_all(&self, mut pairs: impl Iterator<Item = (Pos, Pos)>) -> bool {
        pairs.all(|(p1, p2)| self.contains(p1, p2))
    }

    fn draw(&mut self, p1: Pos, p2: Pos) {
        self.0.insert(Line(p1, p2));
    }

    fn erase(&mut self, p1: Pos, p2: Pos) {
        self.0.remove(&Line(p1, p2));
    }

    pub fn dirs_for_cell(&self, pos: Pos) -> HashSet<Dir> {
        Dir::iter()
            .filter(|&dir| self.contains(pos, pos + dir))
            .collect()
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
    lines: Lines,
    draw_state: DrawState,
    hovered: Option<Pos>,
}

impl State {
    pub fn lines(&self) -> &Lines {
        &self.lines
    }

    pub fn preview(&self) -> Lines {
        if let (
            DrawState::Clicked(from) | DrawState::ClickedAndHeld { clicked: from, .. },
            Some(to),
        ) = (&self.draw_state, self.hovered)
        {
            let mut lines = Lines::default();
            for (p1, p2) in from.line_to(to).into_iter().tuple_windows() {
                lines.draw(p1, p2);
            }
            lines
        } else {
            Lines::default()
        }
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
                let erasing = self
                    .lines
                    .contains_all(from.line_to(pos).into_iter().tuple_windows());

                for (p1, p2) in from.line_to(pos).into_iter().tuple_windows() {
                    if erasing {
                        self.lines.erase(p1, p2);
                    } else {
                        self.lines.draw(p1, p2);
                    };
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
            Held(held) | ClickedAndHeld { held, .. } if self.lines.contains(held, pos) => {
                self.lines.erase(held, pos);
                self.draw_state = Erasing { last: pos }
            }
            Held(held) | ClickedAndHeld { held, .. } => {
                self.lines.draw(held, pos);
                self.draw_state = Drawing {
                    visited: vec1![held, pos],
                };
            }
            Drawing { ref mut visited } => {
                match Vec1::as_slice(&visited) {
                    [] => unreachable!(),
                    &[.., sl, last] if sl == pos => {
                        self.lines.erase(last, pos);
                        visited.pop().unwrap();
                    }
                    &[.., last] => {
                        for (p1, p2) in last.line_to(pos).into_iter().tuple_windows() {
                            self.lines.draw(p1, p2);
                            visited.push(p2);
                        }
                    }
                };
            }
            Erasing { ref mut last } => {
                self.lines.erase(*last, pos);
                *last = pos;
            }
        }
        self.hovered = Some(pos);
    }

    pub fn on_mouseleave(&mut self, pos: Pos) {
        self.hovered.take_if(|p| *p == pos);
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
