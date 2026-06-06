use leptos::prelude::*;

use super::cell::cell_border_classes;
use crate::bpz::{Pos, Puzzle};

/// Read-only render of a puzzle grid: region borders, clue numbers, and the
/// shaded / "definitely unshaded" (✕) solution overlay.
///
/// The overlay is read straight from the puzzle's own cells (`cell.shaded` /
/// `cell.xmark`, set by the `SHADE` / `XMARK` bpz instructions), so a worked
/// example is authored as a plain `.bpz` file just like the real puzzles.
/// Border + shading styling is shared with the interactive
/// [`PuzzleCell`](super::cell::PuzzleCell) via [`cell_border_classes`].
#[component]
pub fn PuzzleGrid<'a>(puzzle: &'a Puzzle) -> impl IntoView {
    let rows = (puzzle.bl.row..=puzzle.tr.row)
        .rev()
        .map(|row| {
            let cells = (puzzle.bl.col..=puzzle.tr.col)
                .map(|col| {
                    let pos = Pos { row, col };
                    let cell = puzzle.get_cell(pos);

                    let (border_td, border_div) = cell_border_classes(puzzle, pos);
                    let mut td_classes = vec!["w-9 h-9".to_owned()];
                    td_classes.extend(border_td);
                    if cell.shaded {
                        td_classes.push("aqre-shaded".to_owned());
                    }

                    let mut div_classes = vec!["relative w-9 h-9 flex items-center justify-center"];
                    div_classes.extend(border_div);

                    let content = if let Some(text) = cell.text.clone() {
                        view! { <span class="text-sm font-medium">{text}</span> }.into_any()
                    } else if cell.xmark && !cell.shaded {
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
