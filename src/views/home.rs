//! Home page component for the Peillute application
//!
//! This component provides the main user interface for managing users in the system,
//! including listing existing users, adding new users, and deleting users.

use crate::Route;
use dioxus::prelude::*;

/// Home page component
///
/// Renders the main user management interface with the following features:
/// - List of existing users with links to their transaction history
/// - Form for adding new users
/// - Delete buttons for removing users
#[component]
pub fn Home() -> Element {
    let mut user_input = use_signal(|| "".to_string());
    let mut users = use_signal(|| Vec::new());

    use_future(move || async move {
        if let Ok(data) = get_users().await {
            users.set(data);
        }
    });

    rsx! {
        div { id: "users-list",
            for item in users.iter() {
                div { class: "user-card",
                    div { class: "user-content",
                        Link {
                            to: Route::History {
                                name: item.to_string(),
                            },
                            span { class: "user-name", "{item}" }
                        }
                    }
                    {
                        let item_for_delete = item.clone();
                        rsx! {
                            button {
                                r#type: "button",
                                class: "delete-btn",
                                onclick: move |_| {
                                    let username = item_for_delete.clone();
                                    spawn(async move {
                                        if let Ok(_) = delete_user(username).await {
                                            if let Ok(data) = get_users().await {
                                                users.set(data);
                                            }
                                        }
                                    });
                                },
                                "X"
                            }
                        }
                    }
                }
            }
        }
        div { id: "add-user-form",
            form {
                label { r#for: "fusername", "Enter a new user:" }
                input {
                    r#type: "text",
                    id: "form-username",
                    r#name: "fusername",
                    placeholder: "New user name",
                    value: user_input,
                    oninput: move |event| user_input.set(event.value()),
                }
                button {
                    id: "submit",
                    r#type: "submit",
                    onclick: move |_| async move {
                        if let Ok(_) = add_user(user_input.to_string()).await {
                            user_input.set("".to_string());
                        }
                        if let Ok(data) = get_users().await {
                            users.set(data);
                        }
                    },
                    "Submit"
                }
            }
        }
    }
}

/// Server function to retrieve the list of users
#[server]
async fn get_users() -> Result<Vec<String>, ServerFnError> {
    use crate::db;
    let users = db::get_users()?;
    Ok(users)
}

/// Server function to add a new user
///
/// Creates a user in the local database and broadcasts the creation
/// to all nodes in the network.
#[server]
async fn add_user(name: String) -> Result<(), ServerFnError> {
    if name == "" {
        return Err(ServerFnError::new("User name cannot be empty."));
    }

    if let Err(e) = crate::control::enqueue_critical(crate::control::CriticalCommands::CreateUser {
        name: name,
    })
    .await
    {
        return Err(ServerFnError::new(format!(
            "Failed to diffuse the create user message: {e}"
        )));
    }

    Ok(())
}

/// Server function to delete a user
///
/// Removes a user from the local database.
#[server]
async fn delete_user(name: String) -> Result<(), ServerFnError> {
    use crate::db;
    db::delete_user(&name)?;
    Ok(())
}
