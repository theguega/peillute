use crate::Route;
use dioxus::prelude::*;

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
                    onkeydown: move |event: dioxus::events::KeyboardEvent| {
                        if let dioxus::events::Key::Enter = event.key() {
                            let user_input_clone = user_input.clone();
                            spawn(async move {
                                if let Ok(_) = add_user(user_input_clone.to_string()).await {
                                    user_input.set("".to_string());
                                }
                                if let Ok(data) = get_users().await {
                                    users.set(data);
                                }
                            });
                        }
                    },
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

#[server]
async fn get_users() -> Result<Vec<String>, ServerFnError> {
    use crate::db;
    let users = db::get_users()?;
    Ok(users)
}

#[server]
async fn add_user(name: String) -> Result<(), ServerFnError> {
    use crate::control::Command;
    use crate::db;
    use crate::message::{CreateUser, MessageInfo, NetworkMessageCode};
    use crate::network::send_message_to_all;

    if name == "" {
        return Err(ServerFnError::new("User name cannot be empty."));
    }

    db::create_user(&name)?;

    if let Err(e) = send_message_to_all(
        Some(Command::CreateUser),
        NetworkMessageCode::Transaction,
        MessageInfo::CreateUser(CreateUser::new(name.clone())),
    )
    .await
    {
        return Err(ServerFnError::new(format!(
            "Failed to send message to all nodes: {e}"
        )));
    }
    Ok(())
}

#[server]
async fn delete_user(name: String) -> Result<(), ServerFnError> {
    use crate::db;
    db::delete_user(&name)?;
    Ok(())
}
