use std::sync::Arc;

use leptos::prelude::*;

use crate::{
    heroicons::solid::{NoSignal, Signal},
    realtime::Client,
};

#[component]
pub fn Status() -> impl IntoView {
    let client: Arc<Client> = use_context().expect("Missing realtime client");

    return move || {
        if client.state.is_connected.get() {
            view! { <Signal {..} class="w-6 h-6" /> }.into_any()
        } else {
            view! { <NoSignal {..} class="w-6 h-6 text-red-500" /> }.into_any()
        }
    };
}
