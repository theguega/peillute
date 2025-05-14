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
    let products = vec![
        ("Coca", 1.50, asset!("/assets/images/coca.png")),
        ("Chips", 2.00, asset!("/assets/images/chips.png")),
        ("Sandwich", 4.50, asset!("/assets/images/sandwich.png")),
        ("Coffee", 1.20, asset!("/assets/images/coffee.png")),
    ];

    // State for cart
    let mut cart = use_signal(Vec::<(String, f64, u32)>::new);
    let mut total_amount = use_signal(|| 0f64);
    let name = std::rc::Rc::new(name);
    let name_for_payment = name.clone();

    // Function to add product to cart
    let mut add_to_cart = move |product_name: String, price: f64| {
        let mut updated_cart = cart.read().clone();

        // Check if product is already in cart
        if let Some(index) = updated_cart
            .iter()
            .position(|(name, _, _)| name == &product_name)
        {
            // Update quantity
            let (name, price, quantity) = updated_cart[index].clone();
            updated_cart[index] = (name, price, quantity + 1);
        } else {
            // Add new product to cart
            updated_cart.push((product_name, price, 1));
        }

        cart.set(updated_cart);

        // Update total amount
        let new_total = cart.read().iter().fold(0.0, |acc, (_, price, quantity)| {
            acc + (price * *quantity as f64)
        });
        total_amount.set(new_total);
    };

    // Function to remove product from cart
    let mut remove_from_cart = move |product_name: String| {
        let mut updated_cart = cart.read().clone();

        if let Some(index) = updated_cart
            .iter()
            .position(|(name, _, _)| name == &product_name)
        {
            let (name, price, quantity) = updated_cart[index].clone();

            if quantity > 1 {
                // Decrease quantity
                updated_cart[index] = (name, price, quantity - 1);
            } else {
                // Remove product
                updated_cart.remove(index);
            }
        }

        cart.set(updated_cart);

        // Update total amount
        let new_total = cart.read().iter().fold(0.0, |acc, (_, price, quantity)| {
            acc + (price * *quantity as f64)
        });
        total_amount.set(new_total);
    };

    // Function to checkout
    let checkout = move |_| {
        let total = *total_amount.read();
        let username = name_for_payment.clone();

        // Clear cart after payment
        cart.set(Vec::new());
        total_amount.set(0.0);

        // Make payment
        async move {
            if total > 0.0 {
                if pay_for_user_server(username.to_string(), total)
                    .await
                    .is_ok()
                {
                    // Payment successful
                }
            }
        }
    };

    rsx! {
        div { id: "pay-page",
            h1 { "Welcome {name}, buy products" }
            // Products section
            div { class: "products-container",
                for (index , (product_name , price , image_path)) in products.iter().enumerate() {
                    div { key: "{index}", class: "product-card",
                        img {
                            src: "{image_path}",
                            alt: "{product_name}",
                            class: "product-image",
                        }
                        div { class: "product-info",
                            h3 { "{product_name}" }
                            p { class: "product-price", "€{price:.2}" }
                            button {
                                class: "add-to-cart-btn",
                                onclick: move |_| {
                                    add_to_cart(product_name.to_string(), *price);
                                },
                                "Add to Cart"
                            }
                        }
                    }
                }
            }
            // Cart section
            div { class: "cart-container",
                h2 { "Your Cart" }
                if cart.read().is_empty() {
                    p { class: "empty-cart", "Your cart is empty" }
                } else {
                    ul { class: "cart-items",
                        for (index , (item_name , item_price , quantity)) in cart.read().iter().enumerate() {
                            li { key: "{index}", class: "cart-item",
                                div { class: "item-details",
                                    span { class: "item-name", "{item_name}" }
                                    span { class: "item-price", "€{item_price:.2} x {quantity}" }
                                    span { class: "item-total",
                                        "= €{item_price * *quantity as f64:.2}"
                                    }
                                }
                                div { class: "item-actions",
                                    button {
                                        class: "remove-item-btn",
                                        onclick: move |_| {
                                            remove_from_cart(item_name.to_string());
                                        },
                                        "−"
                                    }
                                    span { class: "item-quantity", "{quantity}" }
                                    button {
                                        class: "add-item-btn",
                                        onclick: move |_| {
                                            add_to_cart(item_name.to_string(), *item_price);
                                        },
                                        "+"
                                    }
                                }
                            }
                        }
                    }
                    div { class: "cart-summary",
                        h3 { "Total: €{total_amount:.2}" }
                        button {
                            class: "checkout-btn",
                            disabled: cart.read().is_empty(),
                            onclick: checkout,
                            "Pay Now"
                        }
                    }
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
