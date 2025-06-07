pub mod board;
mod cell;
mod state;
mod util;
pub use state::State;

use std::{cmp::Ordering, collections::HashMap, time::Duration};

use leptos::{
    ev::{self},
    prelude::*,
};

use self::{board::Board, cell::PuzzleCell, util::rotate_from_north};
use crate::{
    bpz::{Dir, Pos, Puzzle},
    components::button::Button,
    heroicons::solid::{ExclamationTriangle, Trash},
};

#[component]
pub fn PuzzleEditor<'a, T: Board>(
    name: &'a str,
    puzzle: &'a Puzzle,
    state: RwSignal<State<T>>,
) -> impl IntoView {
    let (state, set_state) = state.split();
    let preview = move || state.read().preview();
    let (clearing, set_clearing) = signal(false);

    let clear = move |_| {
        set_clearing.set(true);
        set_timeout(move || set_clearing.set(false), Duration::from_secs(2));
    };

    let confirm_clear = move |_| {
        set_state.write().board.clear();
        set_clearing.set(false);
    };

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
        let marked = Signal::derive(move || state.read().board.marked(pos));

        let lines = Memo::new(move |_| {
            let dirs = state.read().board.dirs_for_cell(pos);
            let preview_dirs = preview().dirs_for_cell(pos);
            dirs.into_iter()
                .map(|dir| (dir, "bg-red-500"))
                .chain(preview_dirs.into_iter().map(|dir| (dir, "bg-black/30")))
                .collect::<HashMap<_, _>>()
        });

        view! { <PuzzleCell puzzle=&puzzle pos=pos marked=marked lines=lines set_state=set_state /> }
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

    let portals = puzzle
        .portals
        .iter()
        .map(|portal| {
            // TODO: Refactor this
            let mid_row = (portal.start.row as f64 + portal.end.row as f64) / 2.0 + 1.0;
            let mid_col = (portal.start.col as f64 + portal.end.col as f64) / 2.0 + 1.0;
            let offsets = (0..portal.nticks).map(|i| i as f64 - (portal.nticks as f64 - 1.0) / 2.0);

            let rotation_dir = match (
                Ord::cmp(&portal.start.row, &portal.end.row),
                Ord::cmp(&portal.start.col, &portal.end.col),
            ) {
                (Ordering::Equal, Ordering::Less) => Dir::East,
                (Ordering::Equal, Ordering::Greater) => Dir::West,
                (Ordering::Less, Ordering::Equal) => Dir::North,
                (Ordering::Greater, Ordering::Equal) => Dir::South,
                _ => return view! {}.into_any(),
            };
            let classes = vec![
                "absolute -translate-x-1/2 translate-y-1/2 portal".to_owned(),
                rotate_from_north(rotation_dir).to_owned(),
                format!("portal-{}-text", portal.nticks),
            ];
            view! {
                <div
                    class=classes.join(" ")
                    style:bottom=format!("{}rem", mid_row * 3.0)
                    style:left=format!("{}rem", mid_col * 3.0)
                >
                    {offsets
                        .map(|offset| {
                            view! {
                                <ExclamationTriangle
                                    {..}
                                    class="w-6 h-6 -scale-y-100"
                                    style:translate=format!("0 {}rem", offset / 4.0)
                                />
                            }
                                .into_any()
                        })
                        .collect::<Vec<_>>()}
                </div>
            }
            .into_any()
        })
        .collect::<Vec<_>>();

    view! {
        <div class="border border-gray-300 rounded-lg p-4">
            <h3 class="text-lg font-semibold">{name.to_owned()}</h3>
            <div class="relative">
                <table class="select-none">
                    <tbody>
                        {(puzzle.bl.row..=puzzle.tr.row).rev().map(render_row).collect::<Vec<_>>()}
                    </tbody>
                </table>
                {portals}
            </div>
            {move || {
                if clearing.get() {
                    view! {
                        <Button {..} type="button" on:click=confirm_clear>
                            <Trash attr:class="w-4 h-4" />
                            "Click again to confirm"
                        </Button>
                    }
                        .into_any()
                } else {
                    view! {
                        <Button {..} type="button" on:click=clear>
                            <Trash attr:class="w-4 h-4" />
                            "Clear"
                        </Button>
                    }
                        .into_any()
                }
            }}
        </div>
    }
}
