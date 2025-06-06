pub mod board;
mod cell;
mod state;
pub use state::State;

use std::collections::HashMap;

use leptos::{
    ev::{self},
    prelude::*,
};

use self::cell::PuzzleCell;
use crate::{
    bpz::{Pos, Puzzle},
    components::button::Button,
    editor::board::Board,
    heroicons::solid::Trash,
};

#[component]
pub fn PuzzleEditor<'a, T: Board>(
    name: &'a str,
    puzzle: &'a Puzzle,
    state: RwSignal<State<T>>,
) -> impl IntoView {
    let (state, set_state) = state.split();
    let preview = move || state.read().preview();

    let handles = [
        window_event_listener(ev::mouseup, move |_| set_state.write().on_mouseup()),
        window_event_listener(ev::keydown, move |evt| {
            if evt.key() == "Escape" {
                set_state.write().on_escape();
            }
        }),
    ];

    on_cleanup(move || {
        for handle in handles {
            handle.remove()
        }
    });

    let render_cell = |pos| {
        let lines = Memo::new(move |_| {
            let dirs = state.read().board.dirs_for_cell(pos);
            let preview_dirs = preview().dirs_for_cell(pos);
            dirs.into_iter()
                .map(|dir| (dir, "bg-red-500"))
                .chain(preview_dirs.into_iter().map(|dir| (dir, "bg-black/30")))
                .collect::<HashMap<_, _>>()
        });

        view! { <PuzzleCell puzzle=&puzzle pos=pos lines=lines set_state=set_state /> }
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
        <div class="border border-gray-300 rounded-lg p-4">
            <h3 class="text-lg font-semibold">{name.to_owned()}</h3>
            <table class="select-none">
                <tbody>
                    {(puzzle.bl.row..=puzzle.tr.row).rev().map(render_row).collect::<Vec<_>>()}
                </tbody>
            </table>
            <Button {..} type="button" on:click=move |_| todo!()>
                <Trash attr:class="w-4 h-4" />
                "Clear"
            </Button>
        </div>
    }
}
