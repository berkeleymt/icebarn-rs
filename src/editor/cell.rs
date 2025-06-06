use std::collections::HashMap;

use leptos::prelude::*;

use super::state::State;
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
pub fn PuzzleCell<'a>(
    puzzle: &'a Puzzle,
    pos: Pos,
    #[prop(into)] lines: Signal<HashMap<Dir, &'static str>>,
    set_state: WriteSignal<State>,
) -> impl IntoView {
    let mut td_classes = vec!["group w-12 h-12"];
    let mut div_classes = vec!["relative w-12 h-12 flex items-center justify-center"];

    let cell = puzzle.get_cell(pos);

    if cell.shading == Shading::Icebarn {
        td_classes.push("bg-blue-200");
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

    let render_lines = move || -> Vec<_> {
        lines
            .get()
            .iter()
            .map(|(&dir, &class)| {
                let line_classes = [
                    "absolute top-0 bottom-1/2 left-1/2 -translate-x-1/2 w-1 origin-bottom",
                    rotate_from_north(dir),
                    class,
                ];
                let square_classes = ["absolute w-1 h-1", class];
                view! {
                    <div class=line_classes.join(" ") />
                    <div class=square_classes.join(" ") />
                }
            })
            .collect()
    };

    let interactive_overlay = view! {
        <div
            class="absolute inset-0 z-100"
            class=("cursor-pointer group-hover:bg-black/10", cell.interactive)
            on:click=move |_| set_state.write().on_click(pos)
            on:mousedown=move |_| set_state.write().on_mousedown(pos)
            on:mouseenter=move |_| set_state.write().on_mouseenter(pos)
            on:mouseleave=move |_| set_state.write().on_mouseleave(pos)
        />
    };

    let text = match cell.text.as_deref() {
        Some(text) => view! { <span class="z-1">{text}</span> }.into_any(),
        None if cell.interactive => {
            view! { <div class="w-1 h-1 bg-gray-400 group-hover:bg-black rounded-full" /> }
                .into_any()
        }
        None => view!().into_any(),
    };

    view! {
        <td class=td_classes.join(" ")>
            <div class=div_classes
                .join(" ")>{interactive_overlay} {text} {arrows} {render_lines}</div>
        </td>
    }
}
