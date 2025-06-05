//! System information component for the Peillute application
//!
//! This module provides a component for displaying system-wide information,
//! including network details, logical clock states, and peer connections.

use dioxus::prelude::*;

/// Server function to retrieve the local network address
#[server]
async fn get_local_addr() -> Result<String, ServerFnError> {
    use crate::state::LOCAL_APP_STATE;
    let state = LOCAL_APP_STATE.lock().await;
    Ok(state.get_site_addr().to_string())
}

/// Server function to retrieve the current site ID
#[server]
async fn get_site_id() -> Result<String, ServerFnError> {
    use crate::state::LOCAL_APP_STATE;
    let state = LOCAL_APP_STATE.lock().await;
    Ok(state.get_site_id().to_string())
}

/// Server function to retrieve the list of connected peers
#[server]
async fn get_peers() -> Result<Vec<String>, ServerFnError> {
    use crate::state::LOCAL_APP_STATE;
    let state = LOCAL_APP_STATE.lock().await;
    Ok(state.get_peers_addrs_string())
}

/// Server function to retrieve the current Lamport clock value
#[server]
async fn get_lamport() -> Result<i64, ServerFnError> {
    use crate::state::LOCAL_APP_STATE;
    let state = LOCAL_APP_STATE.lock().await;
    Ok(state.get_clock().get_lamport())
}

/// Server function to retrieve the current vector clock state
#[server]
async fn get_vector_clock() -> Result<String, ServerFnError> {
    use crate::state::LOCAL_APP_STATE;
    let state = LOCAL_APP_STATE.lock().await;
    let vector_clock = state.get_clock().get_vector_clock();
    let vector_clock_string = vector_clock
        .iter()
        .map(|x| x.to_string())
        .collect::<Vec<String>>()
        .join(", ");
    Ok(vector_clock_string)
}

/// Server function to retrieve the database path
#[server]
async fn get_db_path() -> Result<String, ServerFnError> {
    let conn = crate::db::DB_CONN.lock().unwrap();
    let path = conn.path().unwrap();
    //keep only the name of the file (after the last "/")
    Ok(path.to_string().split("/").last().unwrap().to_string())
}

/// Server function to retrieve the number of sites in the network
#[server]
async fn get_nb_sites() -> Result<i64, ServerFnError> {
    use crate::state::LOCAL_APP_STATE;
    let state = LOCAL_APP_STATE.lock().await;
    Ok(state.nb_connected_neighbours as i64)
}

/// Ask for a snapshot
#[server]
async fn ask_for_snapshot() -> Result<(), ServerFnError> {
    let _ = crate::snapshot::start_snapshot().await;
    Ok(())
}

/// System information component
///
/// Displays real-time information about the distributed system, including:
/// - Database info
/// - Local network address
/// - Site ID
/// - Lamport timestamp
/// - Vector clock state
/// - Number of connected sites
/// - List of connected peers
/// - Snapshot button
#[component]
pub fn Info() -> Element {
    let mut local_addr = use_signal(|| "".to_string());
    let mut site_id = use_signal(|| "".to_string());
    let mut peers = use_signal(|| Vec::new());
    let mut lamport = use_signal(|| 0i64);
    let mut vector_clock = use_signal(|| "".to_string());
    let mut nb_sites = use_signal(|| 0i64);
    let mut db_path = use_signal(|| "".to_string());

    use_future(move || async move {
        // Fetch local address
        if let Ok(data) = get_local_addr().await {
            local_addr.set(data);
        } else {
            // Optional: Handle error, e.g., log or set a default error message
            local_addr.set("Error fetching local address".to_string());
        }

        // Fetch site ID
        if let Ok(data) = get_site_id().await {
            site_id.set(data);
        } else {
            site_id.set("Error fetching site ID".to_string());
        }

        // Fetch peers
        if let Ok(data) = get_peers().await {
            peers.set(data);
        } // else: peers remains empty or you could set an error state if needed

        // Fetch Lamport clock
        if let Ok(data) = get_lamport().await {
            lamport.set(data);
        } // else: lamport remains 0 or handle error

        // Fetch vector clock (example value)
        if let Ok(data) = get_vector_clock().await {
            vector_clock.set(data);
        } // else: vector_clock remains 0 or handle error

        // Fetch number of sites
        if let Ok(data) = get_nb_sites().await {
            nb_sites.set(data);
        } // else: nb_sites remains 0 or handle error

        // Fetch database path
        if let Ok(data) = get_db_path().await {
            db_path.set(data);
        } // else: db_path remains "" or handle error
    });

    rsx! {
        div { class: "info-panel", // You can style this class with CSS
            h2 { "System Information" }

            div { class: "info-item",
                strong { "💾 Database : " }
                span { "{db_path}" }
            }

            div { class: "info-item",
                strong { "🌐 Local Address: " }
                span { "{local_addr}" }
            }
            div { class: "info-item",
                strong { "🆔 Site ID: " }
                span { "{site_id}" }
            }
            div { class: "info-item",
                strong { "⏰ Lamport Timestamp: " }
                span { "{lamport}" }
            }
            div { class: "info-item",
                strong { "⏱️ Vector Clock : " }
                span { "{vector_clock}" }
            }
            div { class: "info-item",
                strong { "🌍 Number of Sites in Network: " }
                span { "{nb_sites}" }
            }

            div { class: "info-item",
                strong { "🤝 Connected Peers: " }
                if peers.read().is_empty() {
                    span { "No peers currently connected." }
                } else {
                    ul { class: "peer-list",
                        for peer_address in peers.read().iter() {
                            li { key: "{peer_address}", "{peer_address}" }
                        }
                    }
                }
            }

            div {
                class: "info-item",
                style: "display: flex; justify-content: center;",
                button {
                    class: "snapshot",
                    r#type: "submit",
                    onclick: move |_| {
                        async move {
                            if let Err(e) = ask_for_snapshot().await {
                                log::error!("Error taking snapshot: {e}");
                            }
                        }
                    },
                    "Take a snapshot"
                }
            }
        }
    }
}
