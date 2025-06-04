use std::collections::HashMap;

use chumsky::prelude::*;
use chumsky::{text::inline_whitespace, Parser};
use itertools::Itertools;
use thiserror::Error;

use crate::bpz::{Cell, Dir, Pos, Puzzle, Shading};

#[derive(Debug, Clone)]
pub enum Instr {
    Heading(#[allow(dead_code)] String),
    SetPuzzleType(String),
    SetWidth(u32),
    SetHeight(u32),
    SetIn(Pos, Dir),
    SetOut(Pos, Dir),
    SetShading(Pos, Shading),
    RectSetShading {
        bl: Pos,
        tr: Pos,
        shading: Shading,
    },
    SetText(Pos, String),
    AddArrow(Pos, Dir),
    #[allow(dead_code)]
    AddPortal {
        start: Pos,
        end: Pos,
        nticks: u32,
    },
    Path(#[allow(dead_code)] Vec<Pos>),
    Noop,
}

fn int<'a>() -> impl Parser<'a, &'a str, i32> {
    just('-')
        .or_not()
        .then(text::int(10))
        .to_slice()
        .map(|s: &str| s.parse().unwrap())
}

fn uint<'a>() -> impl Parser<'a, &'a str, u32> {
    text::int(10).to_slice().map(|s: &str| s.parse().unwrap())
}

fn pos<'a>() -> impl Parser<'a, &'a str, Pos> {
    int()
        .then_ignore(just(','))
        .then(int())
        .map(|(col, row)| Pos { row, col })
}

fn dir<'a>() -> impl Parser<'a, &'a str, Dir> {
    choice((
        just("NORTH").or(just("UP")).to(Dir::North),
        just("SOUTH").or(just("DOWN")).to(Dir::South),
        just("EAST").or(just("RIGHT")).to(Dir::East),
        just("WEST").or(just("LEFT")).to(Dir::West),
    ))
}

fn shading<'a>() -> impl Parser<'a, &'a str, Shading> {
    choice((
        just("ICEBARN").to(Shading::Icebarn),
        just("REMOVED").to(Shading::Removed),
    ))
}

pub fn parser<'a>() -> impl Parser<'a, &'a str, Vec<Instr>> {
    let recovery = none_of('\n').repeated();

    choice((
        just("#")
            .then(inline_whitespace())
            .ignore_then(text::ident().map(ToString::to_string).map(Instr::Heading)),
        just("WIDTH")
            .then(inline_whitespace())
            .ignore_then(uint().map(Instr::SetWidth)),
        just("HEIGHT")
            .then(inline_whitespace())
            .ignore_then(uint().map(Instr::SetHeight)),
        just("PUZZLE-TYPE")
            .then(inline_whitespace())
            .ignore_then(text::ident())
            .map(ToString::to_string)
            .map(Instr::SetPuzzleType),
        just("IN")
            .or(just("OUT"))
            .then_ignore(inline_whitespace())
            .then(pos())
            .then_ignore(inline_whitespace())
            .then(dir())
            .map(|((kw, pos), dir)| match kw {
                "IN" => Instr::SetIn(pos, dir),
                "OUT" => Instr::SetOut(pos, dir),
                _ => panic!("expected IN or OUT"),
            }),
        just("RECT")
            .then_ignore(inline_whitespace())
            .ignore_then(pos())
            .then_ignore(inline_whitespace())
            .then(pos())
            .then_ignore(inline_whitespace())
            .then(shading())
            .map(|((bl, tr), shading)| Instr::RectSetShading { bl, tr, shading }),
        pos()
            .then_ignore(inline_whitespace())
            .then(shading())
            .map(|(pos, shading)| Instr::SetShading(pos, shading)),
        pos()
            .then_ignore(inline_whitespace())
            .then_ignore(just("HOLE"))
            .then(text::int(10))
            .map(|(pos, text)| Instr::SetText(pos, text.to_owned())),
        pos()
            .then_ignore(inline_whitespace())
            .then_ignore(just("ARROW"))
            .then_ignore(inline_whitespace())
            .then(dir())
            .map(|(pos, dir)| Instr::AddArrow(pos, dir)),
        just("PATH")
            .then(inline_whitespace())
            .ignore_then(pos().separated_by(inline_whitespace()).collect())
            .map(Instr::Path),
        just("").to(Instr::Noop),
    ))
    .padded_by(inline_whitespace())
    .recover_with(via_parser(recovery.to(Instr::Noop)))
    .separated_by(just('\n'))
    .collect()
}

#[derive(Debug, Error)]
pub enum BuildError {
    #[error("missing width")]
    MissingWidth,
    #[error("missing height")]
    MissingHeight,
    #[error("unknown puzzle type")]
    UnknownPuzzleType,
}

pub fn build(instrs: Vec<Instr>) -> Result<Puzzle, BuildError> {
    let mut width = None;
    let mut height = None;
    let mut cells: HashMap<Pos, Cell> = HashMap::new();

    for instr in instrs {
        match instr {
            Instr::Heading(_) => {}
            Instr::SetPuzzleType(puzzle_type) => match puzzle_type.as_str() {
                "icebarn" => {}
                _ => return Err(BuildError::UnknownPuzzleType),
            },
            Instr::SetWidth(w) => {
                width = Some(w as i32);
            }
            Instr::SetHeight(h) => {
                height = Some(h as i32);
            }
            Instr::SetIn(pos, dir) => {
                cells
                    .entry(pos)
                    .or_default()
                    .set_text("IN".to_owned())
                    .insert_arrow(dir);
            }
            Instr::SetOut(pos, dir) => {
                cells.entry(pos).or_default().insert_arrow(dir);
                cells
                    .entry(pos + dir)
                    .or_default()
                    .set_text("OUT".to_owned());
            }
            Instr::SetShading(pos, shading) => {
                cells.entry(pos).or_default().set_shading(shading);
            }
            Instr::RectSetShading { bl, tr, shading } => {
                for pos in Pos::rect(bl, tr) {
                    cells.entry(pos).or_default().set_shading(shading);
                }
            }
            Instr::SetText(pos, text) => {
                cells.entry(pos).or_default().set_text(text);
            }
            Instr::AddArrow(pos, dir) => {
                cells.entry(pos).or_default().insert_arrow(dir);
            }
            Instr::AddPortal { .. } => todo!(),
            Instr::Path(_) => {}
            Instr::Noop => {}
        }
    }

    let Some(width) = width else {
        return Err(BuildError::MissingWidth);
    };
    let Some(height) = height else {
        return Err(BuildError::MissingHeight);
    };

    for (row, col) in Iterator::chain(
        (-2..=height + 1).cartesian_product([-2, -1, width, width + 1]),
        [-2, -1, height, height + 1]
            .into_iter()
            .cartesian_product(-2..=width + 1),
    ) {
        let entry = cells
            .entry(Pos { row, col })
            .or_default()
            .set_shading(Shading::Removed);
        entry.interactive = matches!(entry.text, Some(_));
    }

    Ok(Puzzle {
        bl: Pos { row: -1, col: -1 },
        tr: Pos {
            row: height,
            col: width,
        },
        default_cell: Default::default(),
        cells,
    })
}
