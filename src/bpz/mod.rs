mod parser;

use std::{
    collections::{HashMap, HashSet},
    ops::Add,
    str::FromStr,
};

use chumsky::Parser;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct Pos {
    pub row: i32,
    pub col: i32,
}

impl Pos {
    pub fn is_adjacent_to(&self, other: &Pos) -> bool {
        let row_diff = (other.row - self.row).abs();
        let col_diff = (other.col - self.col).abs();
        row_diff == 1 && col_diff == 0 || row_diff == 0 && col_diff == 1
    }

    pub fn line_to(mut self, other: Pos) -> Vec<Pos> {
        if other.row < self.row {
            return other.line_to(self).into_iter().rev().collect();
        }

        let row_dist = (other.row - self.row).abs();
        let row_step = (other.row - self.row).clamp(-1, 1);
        let col_dist = -(other.col - self.col).abs();
        let col_step = (other.col - self.col).clamp(-1, 1);

        let mut error = row_dist + col_dist;
        let mut result = vec![self];

        while self != other {
            if row_dist + col_dist < 4 * error {
                error += col_dist;
                self.row += row_step;
            } else {
                error += row_dist;
                self.col += col_step;
            }
            result.push(self);
        }

        return result;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum Dir {
    North,
    South,
    East,
    West,
}

impl Dir {
    pub fn iter() -> impl Iterator<Item = Self> {
        [Self::North, Self::South, Self::East, Self::West].into_iter()
    }
}

impl Add<Dir> for Pos {
    type Output = Self;

    fn add(self, rhs: Dir) -> Self::Output {
        let Self { row, col } = self;

        match rhs {
            Dir::North => Pos { row: row + 1, col },
            Dir::South => Pos { row: row - 1, col },
            Dir::East => Pos { row, col: col + 1 },
            Dir::West => Pos { row, col: col - 1 },
        }
    }
}

impl Pos {
    pub fn rect(bl: Self, tr: Self) -> impl Iterator<Item = Self> {
        (bl.row..=tr.row).flat_map(move |row| (bl.col..=tr.col).map(move |col| Pos { row, col }))
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
pub enum Shading {
    #[default]
    Default,
    Icebarn,
    Removed,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Portal {
    pub start: Pos,
    pub end: Pos,
    pub nticks: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Cell {
    pub shading: Shading,
    pub text: Option<String>,
    pub arrows: HashSet<Dir>,
    pub portals: HashMap<Dir, u32>,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            shading: Shading::default(),
            text: Option::default(),
            arrows: HashSet::default(),
            portals: HashMap::default(),
        }
    }
}

impl Cell {
    pub fn set_shading(&mut self, shading: Shading) -> &mut Self {
        self.shading = shading;
        self
    }

    pub fn set_text(&mut self, text: String) -> &mut Self {
        self.text = Some(text);
        self
    }

    pub fn insert_arrow(&mut self, dir: Dir) -> &mut Self {
        self.arrows.insert(dir);
        self
    }

    pub fn insert_portal(&mut self, dir: Dir, nticks: u32) -> &mut Self {
        // TODO: It's weird to store only nticks
        self.portals.insert(dir, nticks);
        self
    }

    pub fn interactive(&self) -> bool {
        return self.text.is_some() || self.shading != Shading::Removed || !self.portals.is_empty();
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Puzzle {
    pub bl: Pos,
    pub tr: Pos,
    pub portals: Vec<Portal>,
    default_cell: Cell,
    cells: HashMap<Pos, Cell>,
}

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("parse error")]
    ParseError(String),

    #[error("build error")]
    BuildError(#[from] parser::BuildError),
}

impl FromStr for Puzzle {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let instrs = parser::parser().parse(s).into_result().map_err(|err| {
            ParseError::ParseError(
                err.iter()
                    .map(|err| format!("{}: {}", err.span(), err.reason()))
                    .join("; "),
            )
        })?;
        Ok(parser::build(instrs)?)
    }
}

impl Puzzle {
    pub fn get_cell(&self, pos: Pos) -> &Cell {
        self.cells.get(&pos).unwrap_or(&self.default_cell)
    }
}
