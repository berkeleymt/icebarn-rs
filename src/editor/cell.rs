use std::collections::HashMap;

use leptos::prelude::*;

use super::{board::Board, state::State, util::rotate_from_north};
use crate::{
    bpz::{Dir, Pos, Puzzle, PuzzleType, Shading},
    heroicons::solid::ArrowLongUp,
};

fn boundary_border(dir: Dir) -> (String, &'static str) {
    match dir {
        Dir::North => ("border-t-3".to_owned(), "-mt-[1.5px]"),
        Dir::South => ("border-b-3".to_owned(), "-mb-[1.5px]"),
        Dir::East => ("border-r-3".to_owned(), "-mr-[1.5px]"),
        Dir::West => ("border-l-3".to_owned(), "-ml-[1.5px]"),
    }
}

fn icebarn_border(dir: Dir) -> (String, &'static str) {
    match dir {
        Dir::North => ("border-t-2".to_owned(), "-mt-[1px]"),
        Dir::South => ("border-b-2".to_owned(), "-mb-[1px]"),
        Dir::East => ("border-r-2".to_owned(), "-mr-[1px]"),
        Dir::West => ("border-l-2".to_owned(), "-ml-[1px]"),
    }
}

fn portal_border(dir: Dir, nticks: u32) -> (String, &'static str) {
    match dir {
        Dir::North => (format!("border-t-6 portal-t-{}", nticks), "-mt-[3px]"),
        Dir::South => (format!("border-b-6 portal-b-{}", nticks), "-mb-[3px]"),
        Dir::East => (format!("border-r-6 portal-r-{}", nticks), "-mr-[3px]"),
        Dir::West => (format!("border-l-6 portal-l-{}", nticks), "-ml-[3px]"),
    }
}

fn default_border(dir: Dir) -> (String, &'static str) {
    match dir {
        Dir::North => ("border-t border-t-gray-400".to_owned(), "-mt-[0.5px]"),
        Dir::South => ("border-b border-b-gray-400".to_owned(), "-mb-[0.5px]"),
        Dir::East => ("border-r border-r-gray-400".to_owned(), "-mr-[0.5px]"),
        Dir::West => ("border-l border-l-gray-400".to_owned(), "-ml-[0.5px]"),
    }
}

#[component]
pub fn PuzzleCell<'a, T: Board>(
    puzzle: &'a Puzzle,
    pos: Pos,
    #[prop(into)] lines: Signal<HashMap<Dir, &'static str>>,
    marked: Signal<bool>,
    set_state: WriteSignal<State<T>>,
) -> impl IntoView {
    let mut td_classes = vec!["group w-12 h-12".to_owned()];
    let mut div_classes = vec!["relative w-12 h-12 flex items-center justify-center"];

    let cell = puzzle.get_cell(pos).clone();
    let is_aqre = puzzle.puzzle_type == PuzzleType::Aqre;

    if cell.shading == Shading::Icebarn {
        td_classes.push("bg-blue-200".to_owned());
    }

    for dir in Dir::iter() {
        use Shading::*;
        let (td_class, div_class) = if is_aqre {
            let neighbor = puzzle.get_cell(pos + dir);
            match (cell.shading, neighbor.shading) {
                (Removed, Removed) => ("".to_owned(), ""),
                (_, Removed) | (Removed, _) => boundary_border(dir),
                _ if cell.region != neighbor.region => boundary_border(dir),
                _ => default_border(dir),
            }
        } else {
            match cell.portals.get(&dir) {
                Some(&portal) => portal_border(dir, portal),
                None => match (cell.shading, puzzle.get_cell(pos + dir).shading) {
                    (Icebarn, Default) | (Default, Icebarn) => icebarn_border(dir),
                    (Default | Icebarn, Removed) | (Removed, Default | Icebarn) => {
                        boundary_border(dir)
                    }
                    (Removed, Removed) => ("".to_owned(), ""),
                    _ => default_border(dir),
                },
            }
        };
        td_classes.push(td_class);
        div_classes.push(div_class);
    }

    let arrows: Vec<_> = cell.arrows.iter()
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

    let mut overlay_classes = vec!["absolute inset-0 z-100"];

    if cell.interactive() {
        overlay_classes.push("cursor-pointer group-hover:bg-black/10");
    }

    let interactive_overlay = view! {
        <div
            class=overlay_classes.join(" ")
            class:marked=move || !is_aqre && marked.get()
            on:click=move |evt| {
                if evt.button() == 0 {
                    set_state.write().on_click(pos)
                };
            }
            on:contextmenu=move |evt| {
                set_state.write().on_contextmenu(pos);
                evt.prevent_default();
            }
            on:mousedown=move |evt| {
                if evt.button() == 0 {
                    set_state.write().on_mousedown(pos)
                };
            }
            on:mouseenter=move |_| set_state.write().on_mouseenter(pos)
            on:mouseleave=move |_| set_state.write().on_mouseleave(pos)
        />
    };

    let text = match cell.text.as_deref() {
        Some(text) if is_aqre => {
            view! { <span class="z-1 text-sm font-medium">{text}</span> }.into_any()
        }
        Some(text) => view! { <span class="z-1">{text}</span> }.into_any(),
        None if !is_aqre && cell.shading != Shading::Removed => {
            view! { <div class="w-1 h-1 bg-gray-400 group-hover:bg-black rounded-full" /> }
                .into_any()
        }
        None => view!().into_any(),
    };

    let render_lines = move || -> Vec<_> {
        lines
            .read()
            .iter()
            .map(|(&dir, &class)| {
                // TODO: Maybe a better way to do this
                let mut line_classes = vec![
                    "absolute top-0 bottom-1/2 left-1/2 -translate-x-1/2 w-1 origin-bottom",
                    rotate_from_north(dir),
                    class,
                ];
                let mut square_classes = vec!["absolute w-1 h-1", class];
                if cell.portals.contains_key(&dir) && cell.shading == Shading::Removed {
                    line_classes.push("hidden");
                    square_classes.push("hidden");
                };
                view! {
                    <div class=line_classes.join(" ") />
                    <div class=square_classes.join(" ") />
                }
            })
            .collect()
    };

    view! {
        <td class=td_classes.join(" ") class:aqre-shaded=move || is_aqre && marked.get()>
            <div class=div_classes
                .join(" ")>{interactive_overlay} {text} {arrows} {render_lines}</div>
        </td>
    }
}
