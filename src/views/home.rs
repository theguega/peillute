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
        h1 { "Users" }
        div { id: "users-list",
            for item in users.iter() {
                Link {
                    to: Route::History {
                        name: item.to_string(),
                    },
                    "{item}"
                }
            }
        }
        div { id: "add-user-form",
            form {
                label { r#for: "fusername", "User name :" }
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
                    text { "Submit" }
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
    use crate::db;
    if name == "" {
        return Err(ServerFnError::new("User name cannot be empty."));
    }
    db::create_user(&name)?;
    Ok(())
}
