//! Navigation bar component for the Peillute application
//!
//! This component provides the main navigation interface, including links to
//! the home page and debug information, along with the application title.

use crate::Route;
use dioxus::prelude::*;

/// Navigation bar component
///
/// Renders a navigation bar with links to different sections of the application
/// and displays the application title. The component also includes an outlet
/// for rendering child routes.
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
