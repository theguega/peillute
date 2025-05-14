#![allow(non_snake_case)]

mod clock;
mod control;
mod db;
mod message;
mod network;
mod state;

#[derive(clap::Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, default_value_t = std::process::id().to_string())]
    site_id: String,
    #[arg(long, default_value_t = 0)]
    port: u16,
    #[arg(long, value_delimiter = ',')]
    peers: Vec<String>,
    #[arg(long, default_value_t = String::from("127.0.0.1"))]
    ip: String,
}

#[cfg(feature = "server")]
#[tokio::main]
async fn main() -> rusqlite::Result<(), Box<dyn std::error::Error>> {
    use crate::state::LOCAL_APP_STATE;
    use clap::Parser;
    use std::io::{self as std_io, Write};
    use std::net::SocketAddr;
    use tokio::io::{self as tokio_io, AsyncBufReadExt, BufReader};
    use tokio::net::TcpListener;

    const LOW_PORT: u16 = 10000;
    const HIGH_PORT: u16 = 11000;
    const PORT_OFFSET: u16 = HIGH_PORT - LOW_PORT + 1;

    env_logger::init();

    let args = Args::parse();

    let port_range = LOW_PORT..=HIGH_PORT;
    let mut selected_port = args.port;
    if selected_port == 0 {
        for port in port_range {
            if let Ok(listener) = std::net::TcpListener::bind(("127.0.0.1", port)) {
                selected_port = port;
                drop(listener);
                break;
            }
        }
    }

    let site_ip: &str = &args.ip;
    let peer_interaction_addr: SocketAddr = format!("{}:{}", site_ip, selected_port).parse()?;

    let client_server_interaction_addr: SocketAddr =
        format!("{}:{}", site_ip, selected_port + PORT_OFFSET).parse()?;

    {
        let mut state = LOCAL_APP_STATE.lock().await;
        state.site_id = args.site_id;
        state.local_addr = peer_interaction_addr;
        state.nb_sites_on_network = args.peers.len();
        let site_id = state.site_id.clone();
        state.clocks.set_site_id(&site_id);
    }

    let network_listener_local_addr = peer_interaction_addr.clone();
    let listener: TcpListener = TcpListener::bind(network_listener_local_addr).await?;
    log::debug!("Listening on: {}", network_listener_local_addr);

    let router = axum::Router::new().serve_dioxus_application(ServeConfigBuilder::default(), App);
    let router = router.into_make_service();
    let backend_listener = tokio::net::TcpListener::bind(client_server_interaction_addr)
        .await
        .unwrap();

    if !db::is_database_initialized()? {
        let _ = db::init_db();
    }

    let (mut local_lamport_time, node_name) = {
        let state = LOCAL_APP_STATE.lock().await;
        let lamport_time = state.get_lamport().clone();
        let site_id = state.get_site_id().to_string();
        (lamport_time, site_id)
    };

    let stdin: tokio_io::Stdin = tokio_io::stdin();
    let reader: BufReader<tokio_io::Stdin> = BufReader::new(stdin);
    let mut lines: tokio_io::Lines<_> = reader.lines();

    network::announce(site_ip, LOW_PORT, HIGH_PORT, selected_port).await;

    println!(
        "Welcome on peillute, write /help to get the command list, access the web interface at {}",
        format! {"http://{}", client_server_interaction_addr}
    );
    print!("> ");
    std_io::stdout().flush().unwrap();

    let main_loop_app_state = LOCAL_APP_STATE.clone();

    // Spawn the web server
    let server_task = tokio::spawn(async move {
        axum::serve(backend_listener, router).await.unwrap();
    });

    main_loop(
        main_loop_app_state,
        &mut lines,
        &mut local_lamport_time,
        node_name.as_str(),
        listener,
    )
    .await;

    // Ensure the server task finishes cleanly if ever reached
    server_task.await?;

    Ok(())
}

#[cfg(feature = "server")]
async fn main_loop(
    _state: std::sync::Arc<tokio::sync::Mutex<crate::state::AppState>>,
    lines: &mut tokio::io::Lines<tokio::io::BufReader<tokio::io::Stdin>>,
    local_lamport_time: &mut i64,
    node_name: &str,
    listener: tokio::net::TcpListener,
) {
    use crate::control::{handle_command, run_cli};
    use crate::state::LOCAL_APP_STATE;
    use std::io::{self as std_io, Write};
    use tokio::select;

    loop {
        select! {
            line = lines.next_line() => {
                {
                    let mut state = LOCAL_APP_STATE.lock().await;
                    state.increment_vector_current();
                    state.increment_lamport();
                }
                let command = run_cli(line);
                let _ = handle_command(command, local_lamport_time, node_name, false).await;
                print!("> ");
                std_io::stdout().flush().unwrap();
            }
            Ok((stream, addr)) = listener.accept() => {
                let _ = crate::network::start_listening(stream, addr).await;
            }
            _ = tokio::signal::ctrl_c() => {
                disconnect().await;
                println!("👋 Bye !");
                std::process::exit(0);
            }
        }
    }
}

#[cfg(feature = "server")]
async fn disconnect() {
    use crate::message::{MessageInfo, NetworkMessageCode};
    use crate::state::LOCAL_APP_STATE;
    use log::{error, info};

    let (local_addr, site_id, peer_addrs, clock) = {
        let state = LOCAL_APP_STATE.lock().await;
        (
            state.get_local_addr().to_string(),
            state.get_site_id().to_string(),
            state.get_peers(),
            state.get_clock().clone(),
        )
    };

    {
        let mut state = LOCAL_APP_STATE.lock().await;
        state.increment_lamport();
        state.increment_vector_current();
    }

    info!("Shutting down site {}.", site_id);
    for peer_addr in peer_addrs {
        let peer_addr_str = peer_addr.to_string();
        {
            let mut state = LOCAL_APP_STATE.lock().await;
            state.increment_lamport();
            state.increment_vector_current();
        }
        if let Err(e) = crate::network::send_message(
            &peer_addr_str,
            MessageInfo::None,
            None,
            NetworkMessageCode::Disconnect,
            &local_addr,
            &site_id,
            clock.clone(),
        )
        .await
        {
            error!("Error sending message to {}: {}", peer_addr_str, e);
        }
    }
}

use dioxus::prelude::*;

#[cfg(not(feature = "server"))]
fn main() {
    dioxus::launch(App);
}

mod views;
use views::*;

const FAVICON: Asset = asset!("/assets/icon.png");
const MAIN_CSS: Asset = asset!("/assets/styling/main.css");

#[component]
fn App() -> Element {
    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }

        Router::<Route> {}
    }
}

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[layout(Navbar)]
        #[route("/")]
        Home {},
        #[route("/info")]
        Info {},
        #[nest("/:name")]
        #[layout(User)]
            #[route("/history")]
            History {
                name: String,
            },
            #[route("/withdraw")]
            Withdraw {
                name: String,
            },
            #[route("/pay")]
            Pay {
                name: String,
            },
            #[route("/refund")]
            Refund {
                name: String,
            },
            #[route("/transfer")]
            Transfer {
                name: String,
            },
            #[route("/deposit")]
            Deposit {
                name: String,
            },
}

#[cfg(test)]
mod tests {
    use clap::Parser;
    #[test]
    fn test_args_parsing() {
        use super::Args;
        let args = Args::parse_from(vec![
            "my_program",
            "--site-id",
            "A",
            "--port",
            "8080",
            "--peers",
            "127.0.0.1:8081,127.0.0.1:8082",
        ]);
        assert_eq!(args.site_id, "A");
        assert_eq!(args.port, 8080);
        assert_eq!(args.peers.len(), 2);
        assert_eq!(args.peers[0], "127.0.0.1:8081");
        assert_eq!(args.peers[1], "127.0.0.1:8082");
    }

    #[test]
    fn test_args_parsing_no_peers() {
        use super::Args;
        let args = Args::parse_from(vec!["my_program", "--site-id", "A", "--port", "8080"]);
        assert_eq!(args.site_id, "A");
        assert_eq!(args.port, 8080);
        assert_eq!(args.peers.len(), 0);
    }
}
