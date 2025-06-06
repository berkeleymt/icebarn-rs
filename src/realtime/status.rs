use leptos::prelude::*;

use crate::{
    heroicons::solid::{NoSignal, Signal},
    realtime::use_client,
};

#[component]
pub fn Status() -> impl IntoView {
    let client = use_client().expect("Missing realtime client");

    let status = move || {
        if client.heartbeat_state.is_connected.get() {
            view! {
                <Signal {..} class="w-6 h-6" />
                "Connected. Your team's updates will sync in real-time."
            }
            .into_any()
        } else {
            view! {
                <NoSignal {..} class="w-6 h-6 text-red-500" />
                "Disconnected. If this persists, try reloading the page."
            }
            .into_any()
        }
    };

    view! { <div class="flex gap-4 items-center sticky top-0 bg-white z-100 p-2 shadow">{status}</div> }
}
