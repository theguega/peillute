mod clock;
mod control;
mod db;
mod message;
mod network;
mod snapshot;
mod state;

const LOW_PORT: u16 = 10000;
const HIGH_PORT: u16 = 11000;
const PORT_OFFSET: u16 = HIGH_PORT - LOW_PORT + 1;

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
    #[arg(long, default_value_t = 1)]
    db_id: u16,
}

#[tokio::main]
async fn main() -> rusqlite::Result<(), Box<dyn std::error::Error>> {
    use crate::state::LOCAL_APP_STATE;
    use clap::Parser;
    use std::io::{self as std_io, Write};
    use std::net::SocketAddr;
    use tokio::io::{self as tokio_io, AsyncBufReadExt, BufReader};
    use tokio::net::TcpListener;

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
        state.nb_neighbors = args.peers.len() as i64;
        let site_id = state.site_id.clone();
        state.clocks.set_site_id(&site_id);

        let mut peers_addrs = Vec::new();
        for peer in args.peers {
            let peer_addr = peer.parse::<SocketAddr>()?;
            peers_addrs.push(peer_addr);
            state.nb_sites_on_network+=1;
        }
        state.peer_addrs = peers_addrs;


    }

    let network_listener_local_addr = peer_interaction_addr.clone();
    let listener: TcpListener = TcpListener::bind(network_listener_local_addr).await?;
    log::debug!("Listening on: {}", network_listener_local_addr);

    if !db::is_database_initialized()? {
        let _ = db::init_db();
    }

    let node_name = {
        let state = LOCAL_APP_STATE.lock().await;
        let site_id = state.get_site_id().to_string();
        site_id
    };

    let stdin: tokio_io::Stdin = tokio_io::stdin();
    let reader: BufReader<tokio_io::Stdin> = BufReader::new(stdin);
    let mut lines: tokio_io::Lines<_> = reader.lines();

    log::info!(
        "Welcome on peillute, write /help to get the command list, access the web interface at {}",
        format! {"http://{}", client_server_interaction_addr}
    );
    print!("> ");
    std_io::stdout().flush().unwrap();

    network::announce(site_ip, LOW_PORT, HIGH_PORT, selected_port).await;

    let main_loop_app_state = LOCAL_APP_STATE.clone();
    let _ = main_loop(
        main_loop_app_state,
        &mut lines,
        node_name.as_str(),
        listener,
    )
    .await;

    Ok(())
}

async fn main_loop(
    _state: std::sync::Arc<tokio::sync::Mutex<crate::state::AppState>>,
    lines: &mut tokio::io::Lines<tokio::io::BufReader<tokio::io::Stdin>>,
    node_name: &str,
    listener: tokio::net::TcpListener,
) {
    use crate::control::{handle_command_from_cli, run_cli};
    use std::io::{self as std_io, Write};
    use tokio::select;

    loop {
        select! {
            line = lines.next_line() => {
                let command = run_cli(line);
                if let Err(e) = handle_command_from_cli(command, node_name).await{
                    log::error!("Error handling command:\n{}", e);
                }
                print!("> ");
                std_io::stdout().flush().unwrap();
            }
            Ok((stream, addr)) = listener.accept() => {
                let _ = crate::network::start_listening(stream, addr).await;
            }
            _ = tokio::signal::ctrl_c() => {
                disconnect().await;
                log::info!("👋 Bye !");
                std::process::exit(0);
            }
        }
    }
}

async fn disconnect() {
    use crate::message::{MessageInfo, NetworkMessageCode};
    use crate::state::LOCAL_APP_STATE;
    use log::{error, info};

    let (local_addr, site_id, peer_addrs,clock) = {
        let mut state = LOCAL_APP_STATE.lock().await;
        state.increment_lamport();
        state.increment_vector_current();
        (
            state.get_local_addr(),
            state.get_site_id().to_string(),
            state.get_peers(),
            state.get_clock().clone(),
        )

    };

    info!("Shutting down site {}.", site_id);
    for peer_addr in peer_addrs {
        {
            let mut state = LOCAL_APP_STATE.lock().await;
            state.increment_lamport();
            state.increment_vector_current();
        }
        if let Err(e) = crate::network::send_message(
            peer_addr,
            MessageInfo::None,
            None,
            NetworkMessageCode::Disconnect,
            local_addr.parse().unwrap(),
            &site_id,
            &site_id,
            local_addr.parse().unwrap(),
            clock.clone(),
        )
        .await
        {
            error!("Error sending message to {}: {}", peer_addr, e);
        }
    }
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
