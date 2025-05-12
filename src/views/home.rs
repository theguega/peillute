use dioxus::prelude::*;

#[component]
pub fn Home() -> Element {
    let mut users = use_signal(|| Vec::new());
    let mut user_input = use_signal(|| "".to_string());

    use_future(move || async move {
        if let Ok(data) = get_users().await {
            users.set(data);
        }
    });

    rsx! {
        document::Meta {
            name: "viewport",
            content: "width=device-width, initial-scale=1.0",
        }
        body {
            div { class: "app-container",
                div { id: "home-page", class: "page active",
                    header { class: "app-header",
                        a { class: "logo-link", href: "#",
                            img {
                                class: "logo-img",
                                src: asset!("/assets/logo.png"),
                                alt: "Logo peillute",
                            }
                        }
                        h1 { "Peillute" }
                        button { id: "theme-toggle", class: "theme-button",
                            text { "🌙" }
                        }
                    }
                    main {
                        h2 { "Users" }
                        div { class: "user-list-container",
                            ul { id: "user-list", class: "user-list",
                                for item in users.iter() {
                                    li { "{item}" }
                                }
                            }
                        }
                    }
                    footer { class: "add-user-footer",
                        input {
                            r#type: "text",
                            id: "new-user-name",
                            placeholder: "New user name",
                            value: user_input,
                            oninput: move |event| user_input.set(event.value()),
                        }
                        button {
                            id: "add-user-button",
                            onclick: move |_| async move {
                                if let Ok(_) = add_user(user_input.to_string()).await {
                                    user_input.set("".to_string());
                                }
                                if let Ok(data) = get_users().await {
                                    users.set(data);
                                }
                            },
                            text { "✔️" }
                        }
                    }
                }
            }
        }
    }
}

#[server]
async fn get_users() -> Result<Vec<String>, ServerFnError> {
    let users = crate::db::get_users()?;
    Ok(users)
}

#[server]
async fn add_user(name: String) -> Result<(), ServerFnError> {
    crate::db::create_user(&name)?;
    Ok(())
}
