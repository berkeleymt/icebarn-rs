use std::sync::LazyLock;

use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::{
    components::{Route, Router, Routes},
    StaticSegment,
};

use crate::bpz::{editor::PuzzleEditor, Puzzle};

static PUZZLES: LazyLock<Vec<(&'static str, Puzzle)>> = LazyLock::new(|| {
    [
        ("Basic 1", include_str!("../puzzles/basic-1.bpz")),
        ("Basic 2", include_str!("../puzzles/basic-2.bpz")),
        ("Basic 3", include_str!("../puzzles/basic-3.bpz")),
    ]
    .into_iter()
    .map(|(name, src)| (name, src.parse().unwrap()))
    .collect()
});

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

/// Renders the home page of your application.
#[component]
fn HomePage() -> impl IntoView {
    view! {
        <div class="flex flex-col items-center justify-center p-8 gap-8">
            {PUZZLES
                .iter()
                .map(|(_, puzzle)| view! { <PuzzleEditor puzzle=puzzle /> })
                .collect::<Vec<_>>()}
        </div>
    }
}
