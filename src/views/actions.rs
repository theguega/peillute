use dioxus::prelude::*;

#[component]
pub fn History(name: String) -> Element {
    rsx! {
        div { id: "history-page",
            h1 { "History page for {name}" }
        }
    }
}

#[component]
pub fn Withdraw(name: String) -> Element {
    rsx! {
        div { id: "withdraw-page",
            h1 { "Withdraw page for {name}" }
        }
    }
}

#[component]
pub fn Pay(name: String) -> Element {
    rsx! {
        div { id: "pay-page",
            h1 { "Pay page for {name}" }
        }
    }
}

#[component]
pub fn Refund(name: String) -> Element {
    rsx! {
        div { id: "refund-page",
            h1 { "Refund page for {name}" }
        }
    }
}

#[component]
pub fn Transfer(name: String) -> Element {
    rsx! {
        div { id: "transfer-page",
            h1 { "Transfer page for {name}" }
        }
    }
}

#[component]
pub fn Deposit(name: String) -> Element {
    let mut deposit_amount = use_signal(|| 0f64);
    let name = std::rc::Rc::new(name);

    let name_for_future = name.clone();

    rsx! {
        div { id: "deposit-form",
            h1 { "Deposit page for {name}" }
            form {
                label { r#for: "fdeposit", "Deposit amount :" }
                input {
                    r#type: "number",
                    id: "form-deposit",
                    r#name: "fdeposit",
                    step: 0.01,
                    value: "{deposit_amount}",
                    oninput: move |event| {
                        if let Ok(as_number) = event.value().parse::<f64>() {
                            deposit_amount.set(as_number);
                        }
                    },
                }
                button {
                    id: "submit",
                    r#type: "submit",
                    onclick: move |_| {
                        let name = name_for_future.clone();
                        async move {
                            if deposit_for_user(name.to_string(), *deposit_amount.read()).await.is_ok() {
                                deposit_amount.set(0.0);
                            }
                        }
                    },
                    "Submit"
                }
            }
        }
    }
}

#[server]
async fn deposit_for_user(user: String, amount: f64) -> Result<(), ServerFnError> {
    use crate::db;
    if amount < 0.0 {
        return Err(ServerFnError::new("Amount cannot be negative."));
    }
    //FIXME: clock are wrong here
    let mut lamport_clock = 21934;
    let node_name: &str = "E";
    if let Err(e) = db::deposit(&user, amount, &mut lamport_clock, node_name) {
        return Err(ServerFnError::new(e.to_string()));
    }
    Ok(())
}
