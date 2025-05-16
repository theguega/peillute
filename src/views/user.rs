//! User management component for the Peillute application
//!
//! This module provides a component for displaying user information and
//! managing user-specific actions, including viewing balance and accessing
//! various transaction operations.

use crate::Route;
use dioxus::prelude::*;

/// User management component
///
/// Displays user information and provides navigation to various transaction
/// operations, including:
/// - Viewing transaction history
/// - Making withdrawals
/// - Making payments
/// - Processing refunds
/// - Transferring money
/// - Making deposits
#[component]
pub fn User(name: String) -> Element {
    let mut solde = use_signal(|| 0f64);

    let name = std::rc::Rc::new(name);
    let name_for_future = name.clone();

    {
        use_future(move || {
            let name = name_for_future.clone();
            async move {
                if let Ok(data) = get_solde(name.to_string()).await {
                    solde.set(data);
                }
            }
        });
    }

    let history_route = Route::History {
        name: name.to_string(),
    };
    let withdraw_route = Route::Withdraw {
        name: name.to_string(),
    };
    let pay_route = Route::Pay {
        name: name.to_string(),
    };
    let refund_route = Route::Refund {
        name: name.to_string(),
    };
    let transfer_route = Route::Transfer {
        name: name.to_string(),
    };
    let deposit_route = Route::Deposit {
        name: name.to_string(),
    };

    rsx! {
        div { id: "user-info",
            h1 { "Welcome {name}!" }
            h2 { "{solde()} €" }
        }
        div { id: "user-page",
            Link { to: history_route, "History" }
            Link { to: withdraw_route, "Withdraw" }
            Link { to: pay_route, "Pay" }
            Link { to: refund_route, "Refund" }
            Link { to: transfer_route, "Transfer" }
            Link { to: deposit_route, "Deposit" }
        }
        Outlet::<Route> {}
    }
}

/// Server function to retrieve a user's current balance
#[server]
async fn get_solde(name: String) -> Result<f64, ServerFnError> {
    use crate::db;
    let solde = db::calculate_solde(&name)?;
    Ok(solde)
}
