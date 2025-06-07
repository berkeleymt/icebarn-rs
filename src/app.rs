use leptos::{
    html::{self},
    prelude::*,
};
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::{
    components::{Route, Router, Routes},
    StaticSegment,
};

use crate::{
    components::{button::Button, input::Input},
    editor::{board::singleplayer::SingleplayerBoard, PuzzleEditor, State},
    heroicons::solid::{NoSignal, Signal},
    puzzles::PUZZLES,
    realtime::connect_client,
};

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <link rel="icon" type="image/png" href="/favicon-96x96.png" sizes="96x96" />
                <link rel="icon" type="image/svg+xml" href="/favicon.svg" />
                <link rel="shortcut icon" href="/favicon.ico" />
                <link rel="apple-touch-icon" sizes="180x180" href="/apple-touch-icon.png" />
                <link rel="manifest" href="/site.webmanifest" />
                <AutoReload options=options.clone() />
                <HydrationScripts options />
                <MetaTags />
            </head>
            <body>
                <App />
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    view! {
        // injects a stylesheet into the document <head>
        // id=leptos means cargo-leptos will hot-reload this stylesheet
        <Stylesheet id="leptos" href="/pkg/icebarn-rs.css" />

        // sets the document title
        <Title text="BmMT 2025 Online Puzzle Round" />

        // content for this welcome page
        <Router>
            <main>
                <Routes fallback=|| "Page not found.".into_view()>
                    <Route path=StaticSegment("") view=HomePage />
                </Routes>
            </main>
        </Router>
    }
}

#[derive(Debug, Clone)]
enum Mode {
    Lobby,
    Singleplayer,
    Multiplayer(String),
}

#[component]
fn HomePage() -> impl IntoView {
    let (mode, set_mode) = signal(Mode::Lobby);

    move || match &*mode.read() {
        Mode::Lobby => view! { <Lobby set_mode=set_mode /> }.into_any(),
        Mode::Singleplayer => view! { <Singleplayer set_mode=set_mode /> }.into_any(),
        Mode::Multiplayer(room) => {
            view! { <Multiplayer room=room.clone() set_mode=set_mode /> }.into_any()
        }
    }
}

#[component]
fn Lobby(set_mode: WriteSignal<Mode>) -> impl IntoView {
    let input_element: NodeRef<html::Input> = NodeRef::new();
    let join_team = move |_| {
        let room = input_element.get().unwrap().value();
        set_mode.set(Mode::Multiplayer(room));
    };

    view! {
        <div class="mx-auto flex flex-col w-full min-h-screen max-w-128 justify-center p-8 gap-8">
            <form class="flex gap-4" on:submit=join_team>
                <Input {..} node_ref=input_element placeholder="Enter team password..." />
                <Button {..}>"Join Team"</Button>
            </form>
            <div class="relative">
                <div aria-hidden="true" class="absolute inset-0 flex items-center">
                    <div class="w-full border-t border-gray-300" />
                </div>
                <div class="relative flex justify-center">
                    <span class="bg-white px-2 text-sm text-gray-500">or</span>
                </div>
            </div>
            <Button {..} on:click=move |_| set_mode.set(Mode::Singleplayer)>
                Singleplayer
            </Button>
        </div>
    }
}

#[component]
fn Multiplayer(room: String, set_mode: WriteSignal<Mode>) -> impl IntoView {
    let client = connect_client(room);

    let ready = {
        let client = client.clone();
        move || client.heartbeat_state.is_connected.get() && client.editor_state.read().is_some()
    };

    let status = {
        let client = client.clone();
        move || {
            if client.heartbeat_state.is_connected.get() {
                view! {
                    <Signal {..} class="w-6 h-6" />
                    "Connected. Your team's updates will sync in real-time."
                }
                .into_any()
            } else {
                view! {
                    <NoSignal {..} class="w-6 h-6 text-red-500" />
                    {client
                        .heartbeat_state
                        .fatal_error
                        .read()
                        .as_deref()
                        .unwrap_or("Disconnected. If this persists, try reloading the page.")}
                }
                .into_any()
            }
        }
    };

    let leave_room = {
        let client = client.clone();
        move |_| {
            let client = client.clone();
            leptos::task::spawn_local(async move {
                client.close().await;
            });
            set_mode.set(Mode::Lobby);
        }
    };

    view! {
        <div class="mx-auto flex flex-col w-min min-w-xl justify-center p-8 gap-8">
            <div class="flex gap-4 items-center sticky top-0 bg-white z-100 p-2 shadow">
                {status} <div class="flex-1" /> <Button {..} on:click=leave_room>
                    Leave Room
                </Button>
            </div>
            <Rules />
            <Show when=ready>
                {if let Some(state) = &*client.editor_state.read() {
                    state
                        .iter()
                        .map(|(key, (puzzle, state))| {
                            view! { <PuzzleEditor name=key puzzle=puzzle state=*state /> }
                        })
                        .collect::<Vec<_>>()
                        .into_any()
                } else {
                    view! { "Something weird occurred. If this persists, try reloading the page." }
                        .into_any()
                }}
            </Show>
        </div>
    }
}

#[component]
fn Singleplayer(set_mode: WriteSignal<Mode>) -> impl IntoView {
    let puzzles: Vec<_> = PUZZLES
        .iter()
        .map(|(key, puzzle)| {
            let state = RwSignal::new(State::new(SingleplayerBoard::default()));
            view! { <PuzzleEditor name=key puzzle=puzzle state=state /> }
        })
        .collect();

    let leave_room = move |_| {
        set_mode.set(Mode::Lobby);
    };

    view! {
        <div class="mx-auto flex flex-col w-min min-w-xl justify-center p-8 gap-8">
            <div class="flex gap-4 items-center sticky top-0 bg-white z-100 p-2 shadow">
                "Singleplayer mode" <div class="flex-1" /> <Button {..} on:click=leave_room>
                    Close and Delete Game
                </Button>
            </div>
            <Rules />
            {puzzles}
        </div>
    }
}

#[component]
fn Rules() -> impl IntoView {
    view! {
        <div class="border border-gray-300 rounded-lg p-4 flex flex-col gap-2">
            <h3 class="text-lg font-semibold">Rules</h3>
            <p>"Welcome to the BmMT 2025 Online Puzzle Round!"</p>
            <p>"Here are the rules for the Puzzle Round. Please read them in detail!"</p>
            <p>
                <a href="/rules.pdf" target="_blank" class="text-blue-500 hover:underline">
                    Puzzle Round Rules
                </a>
            </p>
        </div>

        <div class="border border-gray-300 rounded-lg p-4 flex flex-col gap-2">
            <h3 class="text-lg font-semibold">Website Instructions</h3>
            <p>
                "Here are some brief instructions on how to use this software to enter your answers for the Puzzle Round. Keep in mind that — unless you are in singleplayer mode — your whole team sees the same grids, and any team member's edits are immediately visible to everyone on the team."
            </p>

            <ul class="ml-8 list-disc flex flex-col gap-1">
                <li>
                    "To get credit for solving the puzzle, you will have to draw a single, continuous path starting from the IN arrow (outside the grid) and ending at the OUT square (also outside the grid)."
                </li>
                <li>
                    "To draw lines, you can either left-click and drag from one box to another, or left-click on two different cells to connect them with the straightest line that can go between them."
                </li>
                <li>
                    "To erase lines, you can left-click and drag across lines that are already drawn. Also, while you're dragging to draw a line, you can drag over your most recently drawn segment to erase it."
                </li>
                <li>
                    "You can also click Clear to clear the entire grid. (Be careful! You can't undo a clear.)"
                </li>
                <li>
                    "Don't draw outside the boundary of the puzzle, except for the IN and OUT squares, even if the software lets you!"
                </li>
            </ul>

            <p>
                "Also, for Black Ice puzzles, it may be helpful to shade in potential ice squares. You can do this by right-clicking (or, on a Mac trackpad, clicking with two fingers) on a square to mark/unmark it as an ice square. This will not be graded."
            </p>
        </div>
    }
}
