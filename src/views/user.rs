use crate::Route;
use dioxus::prelude::*;

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

    rsx! {
        div { id: "user-info",
            h1 { "Welcome {name} !" }
            h2 { "{solde} €" }
        }
        div { id: "user-page",
            Link {
                to: Route::History {
                    name: name.to_string(),
                },
                "History"
            }
            Link {
                to: Route::Withdraw {
                    name: name.to_string(),
                },
                "Withdraw"
            }
            Link {
                to: Route::Pay {
                    name: name.to_string(),
                },
                "Pay"
            }
            Link {
                to: Route::Refund {
                    name: name.to_string(),
                },
                "Refund"
            }
            Link {
                to: Route::Transfer {
                    name: name.to_string(),
                },
                "Transfer"
            }
            Link {
                to: Route::Deposit {
                    name: name.to_string(),
                },
                "Deposit"
            }
        }
        Outlet::<Route> {}
    }
}

#[server]
async fn get_solde(name: String) -> Result<f64, ServerFnError> {
    use crate::db;
    let solde = db::calculate_solde(&name)?;
    Ok(solde)
}
