use dioxus::prelude::*;

// show all transactions as vertical card list
#[component]
pub fn History(name: String) -> Element {
    rsx! {
        div { id: "history-page",
            h1 { "History page for {name}" }
        }
    }
}

// take the username and collect the an amount (float from form) to make a withdrawal
#[component]
pub fn Withdraw(name: String) -> Element {
    let mut withdraw_amount = use_signal(|| 0f64);
    let name = std::rc::Rc::new(name);

    let name_for_future = name.clone();

    rsx! {
        div { id: "withdraw-form",
            h1 { "Pay page for {name}" }
            form {
                label { r#for: "fwithdraw", "Withdraw amount :" }
                input {
                    r#type: "number",
                    id: "form-withdraw",
                    r#name: "fwithdraw",
                    step: 0.01,
                    value: "{withdraw_amount}",
                    oninput: move |event| {
                        if let Ok(as_number) = event.value().parse::<f64>() {
                            withdraw_amount.set(as_number);
                        }
                    },
                }
                button {
                    id: "submit",
                    r#type: "submit",
                    onclick: move |_| {
                        let name = name_for_future.clone();
                        async move {
                            if deposit_for_user_server(name.to_string(), *withdraw_amount.read())
                                .await
                                .is_ok()
                            {
                                withdraw_amount.set(0.0);
                            }
                        }
                    },
                    "Submit"
                }
            }
        }
    }
}

// take the username and collect the an amount (float from form) to make a payment
// to-do : create some products to buy as a list of product cards
// (product card : name, price, quantity, total price)
#[component]
pub fn Pay(name: String) -> Element {
    let mut pay_amount = use_signal(|| 0f64);
    let name = std::rc::Rc::new(name);

    let name_for_future = name.clone();

    rsx! {
        div { id: "pay-form",
            h1 { "Pay page for {name}" }
            form {
                label { r#for: "fpay", "Paiement amount :" }
                input {
                    r#type: "number",
                    id: "form-pay",
                    r#name: "fpay",
                    step: 0.01,
                    value: "{pay_amount}",
                    oninput: move |event| {
                        if let Ok(as_number) = event.value().parse::<f64>() {
                            pay_amount.set(as_number);
                        }
                    },
                }
                button {
                    id: "submit",
                    r#type: "submit",
                    onclick: move |_| {
                        let name = name_for_future.clone();
                        async move {
                            if deposit_for_user_server(name.to_string(), *pay_amount.read())
                                .await
                                .is_ok()
                            {
                                pay_amount.set(0.0);
                            }
                        }
                    },
                    "Submit"
                }
            }
        }
    }
}

// show all transactions as vertical card list
// allow the user to select a transaction to refund it
#[component]
pub fn Refund(name: String) -> Element {
    rsx! {
        div { id: "refund-page",
            h1 { "Refund page for {name}" }
        }
    }
}

// allow to select a user between all users (except the current one)
// and allow user to transfer money with an amout (float from form) to another user
// allow user to add a message to the transaction
#[component]
pub fn Transfer(name: String) -> Element {
    rsx! {
        div { id: "transfer-page",
            h1 { "Transfer page for {name}" }
        }
    }
}

// take the username and collect the an amount (float from form) to make a deposit
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
                            if deposit_for_user_server(name.to_string(), *deposit_amount.read())
                                .await
                                .is_ok()
                            {
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
async fn get_users_server() -> Result<Vec<String>, ServerFnError> {
    use crate::db;
    let users = db::get_users()?;
    Ok(users)
}

#[server]
async fn deposit_for_user_server(user: String, amount: f64) -> Result<(), ServerFnError> {
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

#[server]
async fn withdraw_for_user_server(user: String, amount: f64) -> Result<(), ServerFnError> {
    use crate::db;
    if amount < 0.0 {
        return Err(ServerFnError::new("Amount cannot be negative."));
    }
    //FIXME: clock are wrong here
    let mut lamport_clock = 21934;
    let node_name: &str = "E";
    if let Err(e) = db::withdraw(&user, amount, &mut lamport_clock, node_name) {
        return Err(ServerFnError::new(e.to_string()));
    }
    Ok(())
}

#[server]
async fn pay_for_user_server(user: String, amount: f64) -> Result<(), ServerFnError> {
    use crate::db;
    if amount < 0.0 {
        return Err(ServerFnError::new("Amount cannot be negative."));
    }
    //FIXME: clock are wrong here
    let mut lamport_clock = 21934;
    let node_name: &str = "E";
    if let Err(e) = db::create_transaction(&user, "NULL", amount, &mut lamport_clock, node_name, "")
    {
        return Err(ServerFnError::new(e.to_string()));
    }
    Ok(())
}

#[server]
async fn transfer_from_user_to_user_server(
    from_user: String,
    to_user: String,
    amount: f64,
    optional_message: String,
) -> Result<(), ServerFnError> {
    use crate::db;
    if amount < 0.0 {
        return Err(ServerFnError::new("Amount cannot be negative."));
    }
    //FIXME: clock are wrong here
    let mut lamport_clock = 21934;
    let node_name: &str = "E";
    if let Err(e) = db::create_transaction(
        &from_user,
        &to_user,
        amount,
        &mut lamport_clock,
        node_name,
        &optional_message,
    ) {
        return Err(ServerFnError::new(e.to_string()));
    }
    Ok(())
}

#[server]
async fn get_transactions_for_user_server(
    name: String,
) -> Result<Vec<crate::db::Transaction>, ServerFnError> {
    use crate::db;

    if let Ok(data) = db::get_transactions_for_user(&name) {
        Ok(data)
    } else {
        Err(ServerFnError::new("User not found."))
    }
}

#[server]
async fn refund_transaction_server(
    transac_time: i64,
    transac_node: String,
) -> Result<(), ServerFnError> {
    use crate::db;

    //FIXME: clock are wrong here
    let mut lamport_clock = 21934;
    let node_name: &str = "E";
    if let Err(e) = db::refund_transaction(
        transac_time,
        transac_node.as_str(),
        &mut lamport_clock,
        node_name,
    ) {
        return Err(ServerFnError::new(e.to_string()));
    }
    Ok(())
}
