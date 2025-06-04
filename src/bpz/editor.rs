use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet, VecDeque},
    hash::Hash,
};

use leptos::{
    ev::{self, MouseEvent},
    prelude::*,
};

use crate::{
    bpz::{Dir, Pos, Puzzle, Shading},
    heroicons::solid::{ArrowLongUp, Trash},
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
fn PuzzleCell<'a>(
    puzzle: &'a Puzzle,
    pos: Pos,
    lines: Signal<HashSet<Dir>>,
    on_mousedown: impl FnMut(MouseEvent) -> () + 'static,
    on_mouseenter: impl FnMut(MouseEvent) -> () + 'static,
) -> impl IntoView {
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

    let render_lines = move || -> Vec<_> {
        lines
            .get()
            .iter()
            .map(|&dir| {
                let classes = [
                    "absolute top-0 bottom-1/2 left-1/2 -translate-x-1/2 w-1 bg-red-500 origin-bottom",
                    rotate_from_north(dir)
                ];
                view! { <div class=classes.join(" ") /> }
            })
            .collect()
    };

    view! {
        <td class=td_classes.join(" ")>
            <div class=div_classes.join(" ")>
                <div
                    class="absolute inset-0 z-100"
                    class=("cursor-pointer", cell.interactive)
                    on:mousedown=on_mousedown
                    on:mouseenter=on_mouseenter
                />
                {match cell.text.as_deref() {
                    Some(text) => view!(<span class="z-1">{text}</span>).into_any(),
                    None if cell.interactive => {
                        view!(<div class="w-1 h-1 bg-gray-400 group-hover:bg-black rounded-full" />)
                            .into_any()
                    }
                    None => view!().into_any(),
                }}
                {arrows}
                {render_lines}
            </div>
        </td>
    }
}

#[derive(Debug, Clone, Copy, Eq)]
struct Line(pub Pos, pub Pos);

impl Hash for Line {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match Ord::cmp(&self.0.row, &self.1.row).then(Ord::cmp(&self.0.col, &self.1.col)) {
            Ordering::Greater => {
                self.1.hash(state);
                self.0.hash(state);
            }
            _ => {
                self.0.hash(state);
                self.1.hash(state);
            }
        }
    }
}

impl PartialEq for Line {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0 && self.1 == other.1 || self.0 == other.1 && self.1 == other.0
    }
}

#[derive(Debug, Clone)]
enum DragMode {
    Add,
    Remove,
}

#[derive(Debug, Clone)]
struct DragState {
    last_pos: Pos,
    mode: Option<DragMode>,
    drawn_lines: VecDeque<Line>,
}

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
