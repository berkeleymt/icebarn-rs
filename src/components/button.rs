use leptos::prelude::*;

#[component]
pub fn Button(#[prop(optional)] children: Option<Children>) -> impl IntoView {
    view! {
        <button class="cursor-pointer flex items-center gap-1.5 justify-center rounded-md border border-transparent bg-blue-600 px-4 py-2 text-sm font-medium text-white shadow-sm hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 disabled:opacity-50 disabled:hover:bg-blue-600">
            {children.map(|c| c())}
        </button>
    }
}
