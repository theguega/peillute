use crate::Route;
use dioxus::prelude::*;

#[component]
pub fn Navbar() -> Element {
    rsx! {
        div { id: "navbar",
            Link { to: Route::Home {}, "Home" }
            h1 { "Peillute" }
            Link { to: Route::Info {}, "Debug-Info" }
        }
        Outlet::<Route> {}
    }
}
