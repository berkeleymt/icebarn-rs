use leptos::prelude::*;

#[component]
pub fn Input() -> impl IntoView {
    view! {
        <input class="block w-full rounded-md border-gray-300 placeholder-gray-400 shadow-sm invalid:border-red-300 invalid:text-red-900 invalid:placeholder-red-300 focus:border-blue-500 focus:ring-blue-500 invalid:focus:border-red-500 invalid:focus:ring-red-500 sm:text-sm" />
    }
}
