use leptos::{html, prelude::*};
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
        <Title text="Welcome to Leptos" />

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
        Mode::Lobby => view! { <Lobby set_mode=set_mode />}.into_any(),
        Mode::Singleplayer => view! { <Singleplayer set_mode=set_mode />}.into_any(),
        Mode::Multiplayer(room) => {
            view! { <Multiplayer room=room.clone() set_mode=set_mode />}.into_any()
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

    let render_puzzles = {
        let client = client.clone();
        move || match (
            client.heartbeat_state.is_connected.get(),
            &*client.editor_state.read(),
        ) {
            (true, Some(state)) => state
                .iter()
                .map(|(key, (puzzle, state))| {
                    view! { <PuzzleEditor name=key puzzle=puzzle state=*state /> }
                })
                .collect::<Vec<_>>()
                .into_any(),
            (true, None) => view! { "Loading puzzles..." }.into_any(),
            _ => view! {}.into_any(),
        }
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
                    {client.heartbeat_state.fatal_error.read().as_deref().unwrap_or("Disconnected. If this persists, try reloading the page.")}
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
                {status}
                <div class="flex-1" />
                <Button {..} on:click=leave_room>Leave Room</Button>
            </div>
            {render_puzzles}
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
                "Singleplayer mode"
                <div class="flex-1" />
                <Button {..} on:click=leave_room>Close and Delete Game</Button>
            </div>
            {puzzles}
        </div>
    }
}
