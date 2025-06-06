use std::sync::Arc;

use leptos::prelude::*;

use crate::heroicons::solid::{NoSignal, Signal};
use crate::realtime::RealtimeClient;

#[component]
pub fn Status() -> impl IntoView {
    let client: Arc<RealtimeClient> = use_context().expect("Missing RealtimeClient");

    return move || {
        if client.is_connected.get() {
            view! { <Signal {..} class="w-6 h-6" /> }.into_any()
        } else {
            view! { <NoSignal {..} class="w-6 h-6 text-red-500" /> }.into_any()
        }
    };
}
