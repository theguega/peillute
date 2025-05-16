//! Web interface components for the Peillute application
//!
//! This module contains the Dioxus components that make up the web interface,
//! including navigation, home page, user management, and transaction actions.

/// Navigation bar component
mod navbar;
pub use navbar::Navbar;

/// Home page component
mod home;
pub use home::Home;

/// System information component
mod info;
pub use info::Info;

/// User management component
mod user;
pub use user::User;

/// Transaction action components
mod actions;
pub use actions::{Deposit, History, Pay, Refund, Transfer, Withdraw};
