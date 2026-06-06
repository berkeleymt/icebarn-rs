use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::{
    components::{Route, Router, Routes},
    StaticSegment,
};

use crate::{
    auth::current_team,
    components::button::{Button, ButtonColor},
    editor::{board::singleplayer::SingleplayerBoard, PuzzleEditor, State},
    examples::Examples,
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
        <Title text="BmMT 2026 Online Puzzle Round" />

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

/// Anchor styled like a primary [`Button`] (for navigations such as the OAuth
/// sign-in redirect, which must be a real link, not a button click).
const LINK_BUTTON_CLASS: &str = "cursor-pointer flex items-center gap-1.5 justify-center rounded-md border border-transparent px-4 py-2 text-sm font-medium shadow-sm transition-colors text-white bg-blue-600 hover:bg-blue-700";

#[component]
fn Lobby(set_mode: WriteSignal<Mode>) -> impl IntoView {
    let team = OnceResource::new(current_team());

    view! {
        <div class="mx-auto flex flex-col w-full min-h-screen max-w-sm justify-center p-6 gap-6">
            <div class="flex flex-col gap-1 text-center">
                <h1 class="text-2xl font-bold">"BmMT 2026 Puzzle Round"</h1>
                <p class="text-sm text-gray-500">
                    "Sign in with ContestDojo to join your team, or solve on your own."
                </p>
            </div>
            <Suspense fallback=|| {
                view! { <p class="text-center text-sm text-gray-500">"Loading…"</p> }
            }>
                {move || Suspend::new(async move {
                    match team.await {
                        Ok(Some(team)) => {
                            let team_id = team.team_id.clone();
                            view! {
                                <div class="flex flex-col gap-1 text-center">
                                    <p class="text-sm text-gray-500">"Signed in — your team:"</p>
                                    <p class="text-lg font-semibold">{team.team_name.clone()}</p>
                                </div>
                                <Button {..} on:click=move |_| {
                                    set_mode.set(Mode::Multiplayer(team_id.clone()))
                                }>"Join your team"</Button>
                                <a
                                    href="/auth/logout"
                                    rel="external"
                                    class="text-center text-sm text-gray-500 hover:underline"
                                >
                                    "Sign out"
                                </a>
                            }
                                .into_any()
                        }
                        _ => {
                            view! {
                                <a href="/auth/login" rel="external" class=LINK_BUTTON_CLASS>
                                    "Sign in with ContestDojo"
                                </a>
                            }
                                .into_any()
                        }
                    }
                })}
            </Suspense>
            <div class="relative">
                <div aria-hidden="true" class="absolute inset-0 flex items-center">
                    <div class="w-full border-t border-gray-300" />
                </div>
                <div class="relative flex justify-center">
                    <span class="bg-white px-2 text-sm text-gray-500">or</span>
                </div>
            </div>
            <Button color={ButtonColor::Ghost} {..} on:click=move |_| set_mode.set(Mode::Singleplayer)>
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
                    <span class="inline-flex items-center gap-1.5 rounded-full bg-green-100 text-green-800 px-3 py-1 text-sm font-medium">
                        <Signal {..} class="w-4 h-4" />
                        "Connected — your team's updates sync in real-time."
                    </span>
                }
                .into_any()
            } else {
                view! {
                    <span class="inline-flex items-center gap-1.5 rounded-full bg-red-100 text-red-800 px-3 py-1 text-sm font-medium">
                        <NoSignal {..} class="w-4 h-4" />
                        {client
                            .heartbeat_state
                            .fatal_error
                            .read()
                            .as_deref()
                            .unwrap_or("Disconnected. If this persists, try reloading the page.")}
                    </span>
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
            <Show when=ready>
                <Rules />
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
            let state =
                RwSignal::new(State::new(SingleplayerBoard::default(), puzzle.puzzle_type));
            view! { <PuzzleEditor name=key puzzle=puzzle state=state /> }
        })
        .collect();

    let leave_room = move |_| {
        set_mode.set(Mode::Lobby);
    };

    view! {
        <div class="mx-auto flex flex-col w-min min-w-xl justify-center p-8 gap-8">
            <div class="flex gap-4 items-center sticky top-0 bg-white z-100 p-2 shadow">
                <span class="inline-flex items-center rounded-full bg-gray-100 text-gray-700 px-3 py-1 text-sm font-medium">
                    "Singleplayer mode"
                </span> <div class="flex-1" />
                <Button color={ButtonColor::Danger} {..} on:click=leave_room>
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
            <p>"Welcome to the BmMT 2026 Online Puzzle Round!"</p>
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

            <p>
                "To solve an Aqre puzzle, shade some of the cells in the grid so that all of the following rules are satisfied:"
            </p>

            <ul class="ml-8 list-disc flex flex-col gap-1">
                <li>"Each cell is either completely shaded or completely unshaded."</li>
                <li>
                    "All shaded cells are orthogonally connected — you can travel from any shaded cell to any other shaded cell through shaded cells that share a side (no diagonals)."
                </li>
                <li>
                    "There is no run of four or more consecutive shaded cells, or four or more consecutive unshaded cells, in any row or column."
                </li>
                <li>
                    "If an outlined region (a group of cells outlined in bold) contains a number, then exactly that many cells in that region are shaded."
                </li>
            </ul>

            <p>"Some variants add an extra rule on top of the Basic rules above:"</p>

            <ul class="ml-8 list-disc flex flex-col gap-1">
                <li>
                    <span class="font-semibold">"Paint: "</span>
                    "each outlined region must be either fully shaded or fully unshaded."
                </li>
                <li>
                    <span class="font-semibold">"Spiral: "</span>
                    "the shaded cells in each outlined region must have 180° rotational symmetry about the region's center."
                </li>
                <li>
                    <span class="font-semibold">"Binario: "</span>
                    "each row (but not necessarily each column) must have the same number of shaded and unshaded cells."
                </li>
            </ul>

            <p>
                "To shade a cell, left-click it; click and drag to shade or unshade several cells at once. Click a shaded cell again to unshade it. Only shaded cells count — leave unshaded cells unmarked."
            </p>
            <p>"You can also click Clear to clear the entire grid. (Be careful! You can't undo a clear.)"</p>
        </div>

        <Examples />
    }
}
