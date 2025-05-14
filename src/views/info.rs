use dioxus::prelude::*;

#[component]
pub fn Info() -> Element {
    rsx! {
        div { id: "info-page",
            h1 { "Info page" }
            h2 { "Creators name" }
            h3 { "List of connected peers" }
        }
    }
}
