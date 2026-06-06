use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::{
    components::{Route, Router, Routes},
    StaticSegment,
};
use web_time::{Duration, SystemTime};

use crate::{
    auth::current_team,
    components::button::Button,
    editor::PuzzleEditor,
    examples::Examples,
    heroicons::solid::{NoSignal, Signal},
    realtime::connect_client,
    round::{get_round, set_round, RoundAction, RoundPhase, RoundState},
};

/// Milliseconds since Unix epoch, portable across server and wasm.
fn epoch_ms() -> i64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

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
    provide_meta_context();

    view! {
        <Stylesheet id="leptos" href="/pkg/icebarn-rs.css" />
        <Title text="BmMT 2026 Online Puzzle Round" />

        <Router>
            <main>
                <Routes fallback=|| "Page not found.".into_view()>
                    <Route path=StaticSegment("") view=HomePage />
                    <Route path=StaticSegment("admin") view=AdminPage />
                </Routes>
            </main>
        </Router>
    }
}

#[derive(Debug, Clone)]
enum Mode {
    Lobby,
    Multiplayer(String),
}

#[component]
fn HomePage() -> impl IntoView {
    let (mode, set_mode) = signal(Mode::Lobby);

    move || match &*mode.read() {
        Mode::Lobby => view! { <Lobby set_mode=set_mode /> }.into_any(),
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
                    "Sign in with ContestDojo to join your team."
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
        </div>
    }
}

// ---------------------------------------------------------------------------
// Multiplayer + round timer
// ---------------------------------------------------------------------------

#[component]
fn Multiplayer(room: String, set_mode: WriteSignal<Mode>) -> impl IntoView {
    let client = connect_client(room);

    // --- Round state polling -------------------------------------------------
    let round_state: RwSignal<Option<RoundState>> = RwSignal::new(None);
    let skew: RwSignal<i64> = RwSignal::new(0);
    let now_ms: RwSignal<i64> = RwSignal::new(epoch_ms());

    if cfg!(feature = "hydrate") {
        // Initial fetch
        leptos::task::spawn_local(async move {
            if let Ok(rs) = get_round().await {
                skew.set(rs.server_now_ms - epoch_ms());
                round_state.set(Some(rs));
            }
        });
        // Poll every 2 seconds
        let _ = set_interval_with_handle(
            move || {
                leptos::task::spawn_local(async move {
                    if let Ok(rs) = get_round().await {
                        skew.set(rs.server_now_ms - epoch_ms());
                        round_state.set(Some(rs));
                    }
                });
            },
            Duration::from_secs(2),
        );
        // Tick every second for countdown
        let _ = set_interval_with_handle(
            move || {
                now_ms.set(epoch_ms());
            },
            Duration::from_secs(1),
        );
    }

    // Derived reactive signals
    let show_puzzles = Signal::derive(move || match round_state.get() {
        Some(rs) => rs.show_puzzles(),
        None => false,
    });

    let locked = Signal::derive(move || match round_state.get() {
        Some(rs) => match rs.phase {
            RoundPhase::Running => match rs.end_at_ms {
                Some(end) => (now_ms.get() + skew.get()) >= end,
                None => false,
            },
            _ => true,
        },
        None => true,
    });

    // --- Connection status ---------------------------------------------------
    let ready = {
        let client = client.clone();
        move || {
            client.heartbeat_state.is_connected.get()
                && client.editor_state.read().is_some()
        }
    };

    let status = {
        let client = client.clone();
        move || {
            if client.heartbeat_state.is_connected.get() {
                view! {
                    <span class="inline-flex items-center gap-1.5 rounded-full bg-green-100 text-green-800 px-3 py-1 text-sm font-medium">
                        <Signal {..} class="w-4 h-4" />
                        "Connected"
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
                    "Leave Room"
                </Button>
            </div>
            <RoundBanner round_state=round_state now_ms=now_ms skew=skew />
            <Show when=ready>
                <Rules />
                <Show when=move || show_puzzles.get()>
                    {if let Some(state) = &*client.editor_state.read() {
                        state
                            .iter()
                            .map(|(key, (puzzle, state))| {
                                view! { <PuzzleEditor name=key puzzle=puzzle state=*state locked=locked /> }
                            })
                            .collect::<Vec<_>>()
                            .into_any()
                    } else {
                        view! { "Something weird occurred. If this persists, try reloading the page." }
                            .into_any()
                    }}
                </Show>
            </Show>
        </div>
    }
}

// ---------------------------------------------------------------------------
// Round banner / countdown
// ---------------------------------------------------------------------------

fn format_remaining(ms: i64) -> String {
    let total_secs = (ms / 1000).max(0);
    let hours = total_secs / 3600;
    let mins = (total_secs % 3600) / 60;
    let secs = total_secs % 60;
    if hours > 0 {
        format!("{hours}:{mins:02}:{secs:02}")
    } else {
        format!("{mins:02}:{secs:02}")
    }
}

#[component]
fn RoundBanner(
    round_state: RwSignal<Option<RoundState>>,
    now_ms: RwSignal<i64>,
    skew: RwSignal<i64>,
) -> impl IntoView {
    move || {
        let rs = round_state.get();
        match rs {
            None => {
                view! {
                    <div class="rounded-lg bg-gray-100 border border-gray-300 px-4 py-3 text-sm text-gray-600 text-center">
                        "Checking round status…"
                    </div>
                }
                .into_any()
            }
            Some(rs) => match rs.phase {
                RoundPhase::NotStarted => {
                    view! {
                        <div class="rounded-lg bg-yellow-50 border border-yellow-300 px-4 py-3 text-sm text-yellow-800 text-center">
                            "The round has not started yet. Please wait for staff to begin."
                        </div>
                    }
                    .into_any()
                }
                RoundPhase::Running => {
                    let remaining = match rs.end_at_ms {
                        Some(end) => end - (now_ms.get() + skew.get()),
                        None => 0,
                    };
                    if remaining <= 0 {
                        view! {
                            <div class="rounded-lg bg-red-50 border border-red-300 px-4 py-3 text-sm text-red-800 text-center font-medium">
                                "Time is up! The round has ended."
                            </div>
                        }
                        .into_any()
                    } else {
                        let text = format_remaining(remaining);
                        view! {
                            <div class="rounded-lg bg-green-50 border border-green-300 px-4 py-3 text-center">
                                <span class="text-sm text-green-800">"Time remaining: "</span>
                                <span class="font-mono font-bold text-green-900">{text}</span>
                            </div>
                        }
                        .into_any()
                    }
                }
                RoundPhase::Ended => {
                    view! {
                        <div class="rounded-lg bg-red-50 border border-red-300 px-4 py-3 text-sm text-red-800 text-center font-medium">
                            "The round has ended."
                        </div>
                    }
                    .into_any()
                }
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Admin page
// ---------------------------------------------------------------------------

#[component]
fn AdminPage() -> impl IntoView {
    let (password, set_password) = signal(String::new());
    let (duration_mins, set_duration_mins) = signal(String::from("60"));
    let (status_msg, set_status_msg) = signal(String::new());
    let (round, set_round_state) = signal(None::<RoundState>);

    // Poll round state every 2s on admin page too
    if cfg!(feature = "hydrate") {
        leptos::task::spawn_local(async move {
            if let Ok(rs) = get_round().await {
                set_round_state.set(Some(rs));
            }
        });
        let _ = set_interval_with_handle(
            move || {
                leptos::task::spawn_local(async move {
                    if let Ok(rs) = get_round().await {
                        set_round_state.set(Some(rs));
                    }
                });
            },
            Duration::from_secs(2),
        );
    }

    let do_action = move |action: RoundAction| {
        let pw = password.get_untracked();
        leptos::task::spawn_local(async move {
            match set_round(pw, action).await {
                Ok(rs) => {
                    set_round_state.set(Some(rs));
                    set_status_msg.set(String::new());
                }
                Err(e) => set_status_msg.set(format!("Error: {e}")),
            }
        });
    };

    let start = move |_| {
        let mins: i64 = duration_mins
            .get_untracked()
            .parse()
            .unwrap_or(60);
        do_action(RoundAction::Start {
            duration_secs: mins * 60,
        });
    };

    let stop = move |_| {
        do_action(RoundAction::Stop);
    };

    let reset = move |_| {
        do_action(RoundAction::Reset);
    };

    let phase_display = move || match round.get() {
        None => "Loading…".to_owned(),
        Some(rs) => match rs.phase {
            RoundPhase::NotStarted => "Not Started".to_owned(),
            RoundPhase::Running => {
                let remaining = match rs.end_at_ms {
                    Some(end) => format_remaining(end - rs.server_now_ms),
                    None => "∞".to_owned(),
                };
                format!("Running — {} left", remaining)
            }
            RoundPhase::Ended => "Ended".to_owned(),
        },
    };

    view! {
        <div class="mx-auto flex flex-col w-full max-w-md min-h-screen justify-center p-6 gap-6">
            <h1 class="text-2xl font-bold text-center">"Round Admin"</h1>

            <div class="rounded-lg border border-gray-300 p-4 text-center">
                <p class="text-sm text-gray-500">"Current phase:"</p>
                <p class="text-lg font-semibold">{phase_display}</p>
            </div>

            <div class="flex flex-col gap-2">
                <label class="text-sm font-medium">"Admin Password"</label>
                <input
                    type="password"
                    class="block w-full rounded-md border border-gray-300 px-3 py-2 text-sm shadow-sm focus:border-blue-500 focus:ring-blue-500"
                    prop:value=move || password.get()
                    on:input=move |ev| set_password.set(event_target_value(&ev))
                />
            </div>

            <div class="flex flex-col gap-2">
                <label class="text-sm font-medium">"Duration (minutes)"</label>
                <input
                    type="number"
                    min="1"
                    class="block w-full rounded-md border border-gray-300 px-3 py-2 text-sm shadow-sm focus:border-blue-500 focus:ring-blue-500"
                    prop:value=move || duration_mins.get()
                    on:input=move |ev| set_duration_mins.set(event_target_value(&ev))
                />
            </div>

            <div class="flex gap-2">
                <button
                    class="flex-1 rounded-md bg-green-600 text-white px-4 py-2 text-sm font-medium hover:bg-green-700"
                    on:click=start
                >"Start"</button>
                <button
                    class="flex-1 rounded-md bg-red-600 text-white px-4 py-2 text-sm font-medium hover:bg-red-700"
                    on:click=stop
                >"Stop"</button>
                <button
                    class="flex-1 rounded-md bg-gray-600 text-white px-4 py-2 text-sm font-medium hover:bg-gray-700"
                    on:click=reset
                >"Reset"</button>
            </div>

            <Show when=move || !status_msg.get().is_empty()>
                <p class="text-sm text-red-600 text-center">{move || status_msg.get()}</p>
            </Show>
        </div>
    }
}

// ---------------------------------------------------------------------------
// Rules & examples
// ---------------------------------------------------------------------------

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
                "Here are some brief instructions on how to use this software to enter your answers for the Puzzle Round. Keep in mind that your whole team sees the same grids, and any team member's edits are immediately visible to everyone on the team."
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
