mod cell;
mod state;

use std::collections::HashMap;

use leptos::{
    ev::{self},
    prelude::*,
};

use self::{cell::PuzzleCell, state::State};
use crate::{
    bpz::{Pos, Puzzle},
    heroicons::solid::Trash,
};

#[component]
pub fn PuzzleEditor<'a>(puzzle: &'a Puzzle) -> impl IntoView {
    let (state, set_state) = signal(State::default());

    let preview = move || state.get().preview();

    let render_cell = |pos| {
        let lines = Memo::new(move |_| {
            let dirs = state.get().lines().dirs_for_cell(pos);
            let preview_dirs = preview().dirs_for_cell(pos);
            dirs.into_iter()
                .map(|dir| (dir, "bg-red-500"))
                .chain(preview_dirs.into_iter().map(|dir| (dir, "bg-gray-500")))
                .collect::<HashMap<_, _>>()
        });

        let handle = window_event_listener(ev::mouseup, move |_| set_state.write().on_mouseup());
        on_cleanup(move || handle.remove());

        view! {
            <PuzzleCell
                puzzle=&puzzle
                pos=pos
                lines=lines
                set_state=set_state
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
            on:click=move |_| set_state.set(State::default())
        >
            <Trash attr:class="w-4 h-4" />
            "Clear"
        </button>
    }
}
