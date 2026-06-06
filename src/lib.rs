// Raised above the default 128 so cargo-leptos release builds don't hit
// E0275 (overflow evaluating ...: Send) when the trait solver resolves
// leptos/tachys nested view tuples. Compiler directive only; no behavior change.
#![recursion_limit = "256"]

pub mod app;
pub mod bpz;
pub mod components;
pub mod editor;
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
