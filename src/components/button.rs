use leptos::prelude::*;

#[derive(Debug, Clone, Copy, Default)]
pub enum ButtonColor {
    #[default]
    Primary,
    Danger,
    Warning,
    Ghost,
}

impl ButtonColor {
    fn class(&self) -> &'static str {
        match self {
            Self::Primary => {
                "border-transparent text-white bg-blue-600 hover:bg-blue-700 focus:ring-blue-500 disabled:hover:bg-blue-600"
            }
            Self::Danger => {
                "border-transparent text-white bg-red-600 hover:bg-red-700 focus:ring-red-500 disabled:hover:bg-red-600"
            }
            Self::Warning => {
                "border-transparent text-white bg-yellow-600 hover:bg-yellow-700 focus:ring-yellow-500 disabled:hover:bg-yellow-600"
            }
            Self::Ghost => {
                "border-gray-300 text-gray-700 bg-white hover:bg-gray-50 focus:ring-gray-400"
            }
        }
    }
}

#[component]
pub fn Button(
    #[prop(optional)] color: ButtonColor,
    #[prop(optional)] children: Option<Children>,
) -> impl IntoView {
    let class = "cursor-pointer flex items-center gap-1.5 justify-center rounded-md border px-4 py-2 text-sm font-medium shadow-sm transition-colors focus:outline-none focus:ring-2 focus:ring-offset-2 disabled:opacity-50 ".to_owned() + color.class();

    view! {
        <button class=class>
            {children.map(|c| c())}
        </button>
    }
}
