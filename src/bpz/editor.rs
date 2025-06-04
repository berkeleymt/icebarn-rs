use leptos::prelude::*;

use crate::{
    bpz::{Dir, Pos, Puzzle, Shading},
    heroicons::solid::ArrowLongUp,
};

fn boundary_border(dir: Dir) -> (&'static str, &'static str) {
    match dir {
        Dir::North => ("border-t-3", "-mt-[1.5px]"),
        Dir::South => ("border-b-3", "-mb-[1.5px]"),
        Dir::East => ("border-r-3", "-mr-[1.5px]"),
        Dir::West => ("border-l-3", "-ml-[1.5px]"),
    }
}

fn icebarn_border(dir: Dir) -> (&'static str, &'static str) {
    match dir {
        Dir::North => ("border-t-2", "-mt-[1px]"),
        Dir::South => ("border-b-2", "-mb-[1px]"),
        Dir::East => ("border-r-2", "-mr-[1px]"),
        Dir::West => ("border-l-2", "-ml-[1px]"),
    }
}

fn default_border(dir: Dir) -> (&'static str, &'static str) {
    match dir {
        Dir::North => ("border-t border-t-gray-400", "-mt-[0.5px]"),
        Dir::South => ("border-b border-b-gray-400", "-mb-[0.5px]"),
        Dir::East => ("border-r border-r-gray-400", "-mr-[0.5px]"),
        Dir::West => ("border-l border-l-gray-400", "-ml-[0.5px]"),
    }
}

fn rotate_from_north(dir: Dir) -> &'static str {
    match dir {
        Dir::North => "",
        Dir::South => "rotate-180",
        Dir::East => "rotate-90",
        Dir::West => "rotate-270",
    }
}

#[component]
fn PuzzleCell<'a>(puzzle: &'a Puzzle, pos: Pos) -> impl IntoView {
    let mut td_classes = vec!["group w-12 h-12"];
    let mut div_classes = vec!["relative w-12 h-12 flex items-center justify-center"];

    let cell = puzzle.get_cell(pos);

    match (cell.shading, &cell.text) {
        (Shading::Default, _) => td_classes.push("hover:bg-gray-200"),
        (Shading::Icebarn, _) => td_classes.push("bg-blue-200 hover:bg-blue-300"),
        (Shading::Removed, Some(_)) => td_classes.push("hover:bg-gray-200"),
        (Shading::Removed, None) => {}
    }

    for dir in Dir::iter() {
        use Shading::*;
        let (td_class, div_class) = match (cell.shading, puzzle.get_cell(pos + dir).shading) {
            (Icebarn, Default) | (Default, Icebarn) => icebarn_border(dir),
            (Default | Icebarn, Removed) | (Removed, Default | Icebarn) => boundary_border(dir),
            (Removed, Removed) => ("", ""),
            _ => default_border(dir),
        };
        td_classes.push(td_class);
        div_classes.push(div_class);
    }

    let arrows: Vec<_> = cell
        .arrows
        .iter()
        .map(|&dir| {
            let classes = [
                "z-1 flex items-center justify-center absolute inset-0 -translate-y-1/2 origin-bottom",
                rotate_from_north(dir)
            ];
            view! {
                <div class=classes.join(" ")>
                    <ArrowLongUp attr:class="w-6 h-6" />
                </div>
            }
        })
        .collect();

    view! {
        <td class=td_classes.join(" ")>
            <div class=div_classes.join(" ")>
                {match cell.text.as_deref() {
                    Some(text) => view!(<span class="z-1">{text}</span>).into_any(),
                    None if cell.interactive => {
                        view!(<div class="w-1 h-1 bg-gray-400 group-hover:bg-black rounded-full" />)
                            .into_any()
                    }
                    None => view!().into_any(),
                }}
                {arrows}
            </div>
        </td>
    }
}

#[component]
pub fn PuzzleEditor<'a>(puzzle: &'a Puzzle) -> impl IntoView {
    let render_cell = |pos| {
        view! {
            <PuzzleCell
                puzzle=&puzzle
                pos=pos
            />
        }
    };

    let render_row = |row| {
        view! {
            <tr>
                {(puzzle.bl.col..=puzzle.tr.col)
                    .map(|col| render_cell(Pos { row, col }))
                    .collect::<Vec<_>>()}
            </tr>
        }
    };

    view! {
        <table class="select-none">
            <tbody>
                {(puzzle.bl.row..=puzzle.tr.row).rev().map(render_row).collect::<Vec<_>>()}
            </tbody>
        </table>
    }
}
