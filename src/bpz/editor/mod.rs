mod cell;
mod state;

use std::collections::{HashSet, VecDeque};

use leptos::{
    ev::{self},
    prelude::*,
};

use crate::{
    bpz::{
        editor::{
            cell::PuzzleCell,
            state::{DragMode, DragState, Line},
        },
        Dir, Pos, Puzzle,
    },
    heroicons::solid::Trash,
};

#[component]
pub fn PuzzleEditor<'a>(puzzle: &'a Puzzle) -> impl IntoView {
    let (_drag_state, set_drag_state) = signal(None);
    let (all_lines, set_all_lines) = signal(HashSet::<Line>::new());

    let render_cell = |pos| {
        let lines = Memo::new(move |_| {
            let all_lines = all_lines.get();
            Dir::iter()
                .filter(|&dir| all_lines.contains(&Line(pos, pos + dir)))
                .collect()
        });

        let on_mousedown = move |_| {
            set_drag_state.set(Some(DragState {
                last_pos: pos,
                mode: None,
                drawn_lines: VecDeque::new(),
            }));
        };

        let on_mouseenter = move |_| {
            let all_lines = all_lines.get();
            set_drag_state.update(|state| {
                if let Some(inner @ DragState { mode: None, .. }) = state {
                    if all_lines.contains(&Line(inner.last_pos, pos)) {
                        inner.mode = Some(DragMode::Remove);
                    } else {
                        inner.mode = Some(DragMode::Add);
                    }
                };
                if let Some(inner) = state {
                    match inner {
                        DragState {
                            last_pos,
                            mode: Some(DragMode::Add),
                            drawn_lines,
                        } => {
                            if last_pos.is_adjacent_to(&pos) {
                                let line = Line(*last_pos, pos);
                                if drawn_lines.back() == Some(&line) {
                                    drawn_lines.pop_back();
                                    set_all_lines.write().remove(&line);
                                } else {
                                    drawn_lines.push_back(line);
                                    set_all_lines.write().insert(line);
                                }
                            };
                        }
                        DragState {
                            last_pos,
                            mode: Some(DragMode::Remove),
                            ..
                        } => {
                            if last_pos.is_adjacent_to(&pos) {
                                set_all_lines.write().remove(&Line(*last_pos, pos));
                            };
                        }
                        _ => {}
                    };
                    inner.last_pos = pos;
                };
            });
        };

        let handle = window_event_listener(ev::mouseup, move |_| {
            set_drag_state.set(None);
        });
        on_cleanup(move || handle.remove());

        view! {
            <PuzzleCell
                puzzle=&puzzle
                pos=pos
                lines=lines.into()
                on_mousedown=on_mousedown
                on_mouseenter=on_mouseenter
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
        <button
            class="inline-flex items-center gap-x-1.5 rounded-md bg-red-600 px-3 py-2 text-sm font-semibold text-white shadow-xs hover:bg-red-500 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-red-600"
            type="button"
            on:click=move |_| set_all_lines.write().clear()
        >
            <Trash attr:class="w-4 h-4" />
            "Clear"
        </button>
    }
}
