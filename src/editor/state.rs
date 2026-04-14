use itertools::Itertools;
use vec1::{vec1, Vec1};

use crate::{
    bpz::{Pos, PuzzleType},
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
    AqreShading { shade: bool },
}

impl Default for DrawState {
    fn default() -> Self {
        Self::Idle
    }
}

#[derive(Debug, Clone)]
pub struct State<T: Board> {
    pub board: T,
    draw_state: DrawState,
    hovered: Option<Pos>,
    pub puzzle_type: PuzzleType,
}

impl<T: Board> State<T> {
    pub fn new(board: T, puzzle_type: PuzzleType) -> Self {
        Self {
            board,
            draw_state: DrawState::default(),
            hovered: None,
            puzzle_type,
        }
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
        if self.puzzle_type == PuzzleType::Aqre {
            // For AQRE, mousedown already handles toggling
            return;
        }

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
                    .board
                    .contains_all(from.line_to(pos).into_iter().tuple_windows());

                for (p1, p2) in from.line_to(pos).into_iter().tuple_windows() {
                    if erasing {
                        self.board.erase(p1, p2);
                    } else {
                        self.board.draw(p1, p2);
                    };
                }

                self.draw_state = Idle;
            }
            AqreShading { .. } => {}
        };
    }

    pub fn on_contextmenu(&mut self, pos: Pos) {
        if self.puzzle_type == PuzzleType::Aqre {
            // For AQRE, right-click is not used
            return;
        }
        self.board.toggle_mark(pos);
    }

    pub fn on_mousedown(&mut self, pos: Pos) {
        if self.puzzle_type == PuzzleType::Aqre {
            self.board.toggle_mark(pos);
            let shade = self.board.marked(pos);
            self.draw_state = DrawState::AqreShading { shade };
            return;
        }

        use DrawState::*;
        match self.draw_state {
            Clicked(clicked) => {
                self.draw_state = ClickedAndHeld { clicked, held: pos };
            }
            Idle | Held(_) | ClickedAndHeld { .. } | Drawing { .. } | Erasing { .. }
            | AqreShading { .. } => self.draw_state = Held(pos),
        };
    }

    pub fn on_mouseenter(&mut self, pos: Pos) {
        use DrawState::*;
        match self.draw_state {
            AqreShading { shade } => {
                if shade && !self.board.marked(pos) {
                    self.board.toggle_mark(pos);
                } else if !shade && self.board.marked(pos) {
                    self.board.toggle_mark(pos);
                }
            }
            Idle | Clicked(_) => {}
            Held(held) | ClickedAndHeld { held, .. } if self.board.contains(held, pos) => {
                // TODO: pos and held might not be adjacent here
                self.board.erase(held, pos);
                self.draw_state = Erasing { last: pos }
            }
            Held(held) | ClickedAndHeld { held, .. } => {
                self.board.draw(held, pos);
                self.draw_state = Drawing {
                    visited: vec1![held, pos],
                };
            }
            Drawing { ref mut visited } => {
                for pos in visited.last().line_to(pos).into_iter().skip(1) {
                    match Vec1::as_slice(&visited) {
                        [] => unreachable!(),
                        &[.., sl, last] if sl == pos => {
                            self.board.erase(last, pos);
                            visited.pop().unwrap();
                        }
                        &[.., last] => {
                            self.board.draw(last, pos);
                            visited.push(pos);
                        }
                    };
                }
            }
            Erasing { ref mut last } => {
                self.board.erase(*last, pos);
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
            Held(_) | Drawing { .. } | Erasing { .. } | AqreShading { .. } => {
                self.draw_state = Idle
            }
            ClickedAndHeld { clicked, .. } => self.draw_state = Clicked(clicked),
        }
    }

    pub fn on_escape(&mut self) {
        if let DrawState::Clicked(_) = self.draw_state {
            self.draw_state = DrawState::Idle;
        }
    }
}
