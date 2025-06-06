use leptos::prelude::*;

use crate::{
    heroicons::solid::{NoSignal, Signal},
    realtime::use_client,
};

#[component]
pub fn Status() -> impl IntoView {
    let client = use_client().expect("Missing realtime client");

    return move || {
        if client.heartbeat_state.is_connected.get() {
            view! { <Signal {..} class="w-6 h-6" /> }.into_any()
        } else {
            view! { <NoSignal {..} class="w-6 h-6 text-red-500" /> }.into_any()
        }
    };
}
