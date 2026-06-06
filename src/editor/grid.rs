use leptos::prelude::*;

use super::cell::cell_border_classes;
use crate::bpz::{Pos, Puzzle};

/// Read-only render of a puzzle grid: region borders, clue numbers, and
/// shaded / "definitely unshaded" (✕) cells. Shares border + shading styling
/// with the interactive [`PuzzleCell`](super::cell::PuzzleCell) via
/// [`cell_border_classes`], so worked examples stay visually in sync with the
/// real puzzles.
#[component]
pub fn PuzzleGrid<'a>(
    puzzle: &'a Puzzle,
    /// Display coordinates `(row, col)` of shaded cells; row `0` is the top.
    #[prop(optional)]
    shaded: &'static [(usize, usize)],
    /// Display coordinates `(row, col)` marked as definitely unshaded (✕).
    #[prop(optional)]
    xmark: &'static [(usize, usize)],
) -> impl IntoView {
    let rows = (puzzle.bl.row..=puzzle.tr.row)
        .rev()
        .map(|row| {
            let cells = (puzzle.bl.col..=puzzle.tr.col)
                .map(|col| {
                    let pos = Pos { row, col };
                    let display = (
                        (puzzle.tr.row - row) as usize,
                        (col - puzzle.bl.col) as usize,
                    );

                    let (border_td, border_div) = cell_border_classes(puzzle, pos);
                    let is_shaded = shaded.contains(&display);

                    let mut td_classes = vec!["w-9 h-9".to_owned()];
                    td_classes.extend(border_td);
                    if is_shaded {
                        td_classes.push("aqre-shaded".to_owned());
                    }

                    let mut div_classes =
                        vec!["relative w-9 h-9 flex items-center justify-center"];
                    div_classes.extend(border_div);

                    let cell = puzzle.get_cell(pos);
                    let content = if let Some(text) = cell.text.clone() {
                        view! { <span class="text-sm font-medium">{text}</span> }.into_any()
                    } else if !is_shaded && xmark.contains(&display) {
                        view! { <span class="text-xs text-gray-500">"✕"</span> }.into_any()
                    } else {
                        ().into_any()
                    };

                    view! {
                        <td class=td_classes.join(" ")>
                            <div class=div_classes.join(" ")>{content}</div>
                        </td>
                    }
                })
                .collect::<Vec<_>>();
            view! { <tr>{cells}</tr> }
        })
        .collect::<Vec<_>>();

    view! {
        <table class="select-none">
            <tbody>{rows}</tbody>
        </table>
    }
}
