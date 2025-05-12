use dioxus::prelude::*;

#[component]
pub fn User(id: String) -> Element {
    rsx! {
        h1 { "User" }
        h2 { "{id}" }
    }
}
