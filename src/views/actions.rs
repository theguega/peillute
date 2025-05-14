use dioxus::prelude::*;

// show all transactions as vertical card list
#[component]
pub fn History(name: String) -> Element {
    let name = std::rc::Rc::new(name);
    let name_for_future = name.clone();

    let transactions_resource = use_resource(move || {
        let name_clone = name_for_future.clone();
        async move { get_transactions_for_user_server(name_clone.to_string()).await }
    });

    rsx! {
        div { id: "history-page",
            h1 { "History page for {name}" }

            match &*transactions_resource.read() {
                None => rsx! {
                    p { "Loading history..." }
                },
                Some(Ok(transactions)) => {
                    if transactions.is_empty() {
                        rsx! {
                            p { "No transactions found for {name}." }
                        }
                    } else {
                        rsx! {
                            ul { class: "transactions-list",
                                for transaction in transactions.iter() {
                                    li {
                                        key: "{transaction.lamport_time}-{transaction.source_node}",
                                        class: "transaction-card",
                                        p {
                                            strong { "From:" }
                                            " {transaction.from_user}"
                                        }
                                        p {
                                            strong { "To:" }
                                            " {transaction.to_user}"
                                        }
                                        p {
                                            strong { "Amount:" }
                                            " {transaction.amount:.2}"
                                        }
                                        p {
                                            strong { "Timestamp:" }
                                            " {transaction.lamport_time}"
                                        }
                                        p {
                                            strong { "Source Node:" }
                                            " {transaction.source_node}"
                                        }
                                        if let Some(msg) = &transaction.optional_msg {
                                            if !msg.is_empty() {
                                                p {
                                                    strong { "Message:" }
                                                    " {msg}"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Some(Err(e)) => rsx! {
                    p { class: "error-message", "Error loading history: {e}" }
                },
            }
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
                            if withdraw_for_user_server(name.to_string(), *withdraw_amount.read())
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

const PRODUCTS: &[(&str, f64, &str)] = &[
    ("Coca", 1.50, "/assets/images/coca.png"),
    ("Chips", 2.00, "/assets/images/chips.png"),
    ("Sandwich", 4.50, "/assets/images/sandwich.png"),
    ("Coffee", 1.20, "/assets/images/coffee.png"),
];

// take the username and collect the an amount (float from form) to make a payment
// to-do : create some products to buy as a list of product cards
// (product card : name, price, quantity, total price)
#[component]
pub fn Pay(name: String) -> Element {
    let mut product_quantities = use_signal(|| vec![0u32; PRODUCTS.len()]);
    let name_for_payment = std::rc::Rc::new(name.clone());

    let handle_pay = move |_| {
        let current_quantities = product_quantities.read().clone();
        let name_clone = name_for_payment.clone();

        let mut total_amount = 0.0;
        for (i, &(_, price, _)) in PRODUCTS.iter().enumerate() {
            if let Some(&quantity) = current_quantities.get(i) {
                total_amount += price * quantity as f64;
            }
        }

        if total_amount > 0.0 {
            spawn(async move {
                match pay_for_user_server(name_clone.to_string(), total_amount).await {
                    Ok(_) => {
                        log::info!("Payment successful toast/notification would show here.");
                        product_quantities.set(vec![0u32; PRODUCTS.len()]);
                    }
                    Err(e) => {
                        log::error!("Payment failed: {}", e);
                    }
                }
            });
        } else {
            log::warn!("Attempted to pay with a total of 0.0. No action taken.");
        }
    };

    let current_total_display = use_memo(move || {
        let mut total = 0.0;
        let quantities_read = product_quantities.read();
        for (i, &(_, price, _)) in PRODUCTS.iter().enumerate() {
            if let Some(&quantity) = quantities_read.get(i) {
                total += price * quantity as f64;
            }
        }
        total
    });

    rsx! {
        div { id: "pay-page",
            h1 { "Welcome {name}, select your products" }

            // Products section
            div {
                for (index , (product_name , price , image_path)) in PRODUCTS.iter().enumerate() {
                    div { key: "{product_name}-{index}",
                        img { src: "{image_path}", alt: "{product_name}" }
                        div { class: "product-info",
                            h3 { "{product_name}" }
                            p { "€{price:.2}" }
                            div {
                                label { r#for: "qty-{index}", "Quantity:" }
                                input {
                                    r#type: "number",
                                    id: "qty-{index}",
                                    min: "0",
                                    value: "{product_quantities.read()[index]}",
                                    oninput: move |event| {
                                        let mut pq_signal_for_input = product_quantities.clone();
                                        if let Ok(new_quantity) = event.value().parse::<u32>() {
                                            let mut quantities_writer = pq_signal_for_input.write();
                                            if index < quantities_writer.len() {
                                                quantities_writer[index] = new_quantity;
                                            }
                                        } else if event.value().is_empty() {
                                            let mut quantities_writer = pq_signal_for_input.write();
                                            if index < quantities_writer.len() {
                                                quantities_writer[index] = 0;
                                            }
                                        }
                                    },
                                }
                            }
                        }
                    }
                }
            }

            div { class: "cart-summary",
                h2 { "Order Summary" }
                h3 { "Total: €{current_total_display():.2}" }
                button {
                    disabled: current_total_display() == 0.0,
                    onclick: handle_pay,
                    "Pay Now"
                }
            }
        }
    }
}

// show all transactions as vertical card list
// allow the user to select a transaction to refund it
#[component]
pub fn Refund(name: String) -> Element {
    let name = std::rc::Rc::new(name);
    let name_for_future = name.clone();

    let transactions_resource = use_resource(move || {
        let name_clone = name_for_future.clone();
        async move { get_transactions_for_user_server(name_clone.to_string()).await }
    });

    rsx! {
        div { id: "history-page",
            h1 { "History page for {name}" }

            match &*transactions_resource.read() {
                None => rsx! {
                    p { "Loading history..." }
                },
                Some(Ok(transactions)) => {
                    if transactions.is_empty() {
                        rsx! {
                            p { "No transactions found for {name}." }
                        }
                    } else {
                        rsx! {
                            ul { class: "transactions-list",
                                for transaction in transactions.iter() {
                                    li {
                                        key: "{transaction.lamport_time}-{transaction.source_node}",
                                        class: "transaction-card",
                                        p {
                                            strong { "From:" }
                                            " {transaction.from_user}"
                                        }
                                        p {
                                            strong { "To:" }
                                            " {transaction.to_user}"
                                        }
                                        p {
                                            strong { "Amount:" }
                                            " {transaction.amount:.2}"
                                        }
                                        p {
                                            strong { "Timestamp:" }
                                            " {transaction.lamport_time}"
                                        }
                                        p {
                                            strong { "Source Node:" }
                                            " {transaction.source_node}"
                                        }
                                        if let Some(msg) = &transaction.optional_msg {
                                            if !msg.is_empty() {
                                                p {
                                                    strong { "Message:" }
                                                    " {msg}"
                                                }
                                            }
                                        }
                                        button { "Refund" }
                                    }
                                }
                            }
                        }
                    }
                }
                Some(Err(e)) => rsx! {
                    p { class: "error-message", "Error loading history: {e}" }
                },
            }
        }
    }
}

// allow to select a user between all users (except the current one)
// and allow user to transfer money with an amout (float from form) to another user
// allow user to add a message to the transaction
#[component]
pub fn Transfer(name: String) -> Element {
    let mut transfer_amount = use_signal(|| 0f64);
    let mut transfer_message = use_signal(String::new);
    let mut selected_user = use_signal(String::new);
    let name = std::rc::Rc::new(name);
    let name_for_future = name.clone();

    let users_resource = use_resource({
        move || {
            let current_user = name_for_future.clone();
            async move {
                let all_users = get_users_server().await.unwrap_or_default();
                all_users
                    .into_iter()
                    .filter(|u| u != current_user.as_ref())
                    .collect::<Vec<_>>()
            }
        }
    });

    rsx! {
        div { id: "transfer-page",
            h1 { "Transfer page for {name}" }

            match &*users_resource.read() {
                None => rsx! {
                    p { "Loading users..." }
                },
                Some(users) => rsx! {
                    form {
                        label { r#for: "user-select", "Select user to transfer to:" }
                        select {
                            id: "user-select",
                            onchange: move |evt| {
                                selected_user.set(evt.value());
                            },
                            option {
                                value: "",
                                disabled: true,
                                selected: selected_user.read().is_empty(),
                                "Choose a user"
                            }
                            for user in users {
                                option { key: "{user}", value: "{user}", "{user}" }
                            }
                        }
                        label { r#for: "transfer-amount", "Amount to transfer:" }
                        input {
                            r#type: "number",
                            id: "transfer-amount",
                            step: 0.01,
                            value: "{transfer_amount}",
                            oninput: move |evt| {
                                if let Ok(val) = evt.value().parse::<f64>() {
                                    transfer_amount.set(val);
                                }
                            },
                        }
                        label { r#for: "transfer-message", "Message (optional):" }
                        input {
                            r#type: "text",
                            id: "transfer-message",
                            value: "{transfer_message}",
                            oninput: move |evt| {
                                transfer_message.set(evt.value());
                            },
                        }
                        button {
                            r#type: "submit",
                            onclick: move |_| {
                                let to_user = selected_user.read().clone();
                                let amount = *transfer_amount.read();
                                let message = transfer_message.read().clone();
                                let from_user = name.clone();
                                async move {
                                    if !to_user.is_empty() && amount > 0.0 {
                                        if transfer_from_user_to_user_server(
                                                from_user.to_string(),
                                                to_user,
                                                amount,
                                                message,
                                            )
                                            .await
                                            .is_ok()
                                        {
                                            transfer_amount.set(0.0);
                                            transfer_message.set(String::new());
                                            selected_user.set(String::new());
                                        }
                                    }
                                }
                            },
                            "Transfer"
                        }
                    }
                },
            }
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
    lamport_time: i64,
    transac_node: String,
) -> Result<(), ServerFnError> {
    use crate::db;

    //FIXME: clock are wrong here
    let mut lamport_clock = 21934;
    let node_name: &str = "E";
    if let Err(e) = db::refund_transaction(
        lamport_time,
        transac_node.as_str(),
        &mut lamport_clock,
        node_name,
    ) {
        return Err(ServerFnError::new(e.to_string()));
    }
    Ok(())
}
