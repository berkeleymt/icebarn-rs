pub mod app;
pub mod auth;
pub mod bpz;
pub mod components;
pub mod editor;
pub mod examples;
pub mod heroicons;
pub mod puzzles;
pub mod realtime;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use crate::app::*;
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
}
