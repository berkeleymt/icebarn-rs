pub mod editor;
mod parser;

use std::{
    collections::{HashMap, HashSet},
    ops::Add,
    str::FromStr,
};

use chumsky::Parser;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Pos {
    pub row: i32,
    pub col: i32,
}

impl Pos {
    fn is_adjacent_to(&self, other: &Pos) -> bool {
        let row_diff = (other.row - self.row).abs();
        let col_diff = (other.col - self.col).abs();
        row_diff == 1 && col_diff == 0 || row_diff == 0 && col_diff == 1
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Shading {
    #[default]
    Default,
    Icebarn,
    Removed,
}

#[derive(Debug, Clone)]
pub struct Cell {
    shading: Shading,
    text: Option<String>,
    arrows: HashSet<Dir>,
    interactive: bool,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            shading: Shading::default(),
            text: Option::default(),
            arrows: HashSet::default(),
            interactive: true,
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
}

#[derive(Debug, Clone)]
pub struct Puzzle {
    bl: Pos,
    tr: Pos,
    default_cell: Cell,
    cells: HashMap<Pos, Cell>,
}

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("parse error")]
    ParseError,

    #[error("build error")]
    BuildError(#[from] parser::BuildError),
}

impl FromStr for Puzzle {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let instrs = parser::parser()
            .parse(s)
            .into_result()
            .map_err(|_| ParseError::ParseError)?;
        Ok(parser::build(instrs)?)
    }
}

impl Puzzle {
    fn get_cell(&self, pos: Pos) -> &Cell {
        self.cells.get(&pos).unwrap_or(&self.default_cell)
    }
}
