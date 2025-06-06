use itertools::Itertools;
use vec1::{vec1, Vec1};

use crate::{
    bpz::Pos,
    editor::board::{singleplayer::SingleplayerBoard, Board},
};

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
pub struct State<T: Board> {
    lines: T,
    draw_state: DrawState,
    hovered: Option<Pos>,
}

impl<T: Board> State<T> {
    pub fn new(board: T) -> Self {
        Self {
            lines: board,
            draw_state: DrawState::default(),
            hovered: None,
        }
    }

    pub fn lines(&self) -> &impl Board {
        &self.lines
    }

    pub fn preview(&self) -> SingleplayerBoard {
        if let (
            DrawState::Clicked(from) | DrawState::ClickedAndHeld { clicked: from, .. },
            Some(to),
        ) = (&self.draw_state, self.hovered)
        {
            let mut lines = SingleplayerBoard::default();
            for (p1, p2) in from.line_to(to).into_iter().tuple_windows() {
                lines.draw(p1, p2);
            }
            lines
        } else {
            SingleplayerBoard::default()
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
                for pos in visited.last().line_to(pos).into_iter().skip(1) {
                    match Vec1::as_slice(&visited) {
                        [] => unreachable!(),
                        &[.., sl, last] if sl == pos => {
                            self.lines.erase(last, pos);
                            visited.pop().unwrap();
                        }
                        &[.., last] => {
                            self.lines.draw(last, pos);
                            visited.push(pos);
                        }
                    };
                }
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

    pub fn on_escape(&mut self) {
        if let DrawState::Clicked(_) = self.draw_state {
            self.draw_state = DrawState::Idle;
        }
    }
}
