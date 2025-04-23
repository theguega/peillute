use clap::Parser;
use control::run_cli;
use log::info;
use rusqlite::{Connection, Result};
use std::io::{self as std_io, Write};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{self as tokio_io, AsyncBufReadExt, BufReader};
use tokio::net::TcpListener;
use tokio::select;
use tokio::sync::Mutex;

mod control;
mod clock;
mod db;
mod message;
mod network;
mod state;

const LOW_PORT: u16 = 8000;
const HIGH_PORT: u16 = 9000;

use crate::state::{AppState, GLOBAL_APP_STATE};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, default_value_t = std::process::id() as u64)]
    site_id: u64,
    #[arg(long, default_value_t = 0)]
    port: u16,
    #[arg(long, value_delimiter = ',')]
    peers: Vec<String>,
    #[arg(long, default_value_t = String::from("127.0.0.1"))]
    ip: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let args = Args::parse();

    // if none port was provided, try to find a free port in the range 8000-9000 and use it
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
    let local_addr: SocketAddr = format!("{}:{}", site_ip, selected_port).parse()?;

    {
        let mut state = GLOBAL_APP_STATE.lock().await;
        state.site_id = args.site_id;
        state.local_addr = local_addr;
        state.nb_sites_on_network = args.peers.len();
        state.vector_clock = (0..args.peers.len())
            .map(|_| std::sync::atomic::AtomicU64::new(0))
            .collect();
        state.lamport_clock = std::sync::atomic::AtomicU64::new(args.site_id);
    }

    network::announce(site_ip, LOW_PORT, HIGH_PORT).await;

    let network_listener_local_addr = local_addr.clone();
    let listener: TcpListener = TcpListener::bind(network_listener_local_addr).await?;
    log::debug!("Listening on: {}", network_listener_local_addr);

    let conn: Connection = Connection::open("peillute.db").unwrap();
    let _ = db::drop_table(&conn);
    let _ = db::init_db(&conn);
    let node_name = "A";
    let mut local_lamport_time: i64 = 0;

    let stdin: tokio_io::Stdin = tokio_io::stdin();
    let reader: BufReader<tokio_io::Stdin> = BufReader::new(stdin);
    let mut lines: tokio_io::Lines<BufReader<tokio_io::Stdin>> = reader.lines();

    log::info!("Welcome on peillute, write /help to get the command list.");
    print!("> ");
    std_io::stdout().flush().unwrap();

    let main_loop_app_state = GLOBAL_APP_STATE.clone();
    let _ = main_loop(
        main_loop_app_state,
        &mut lines,
        &conn,
        &mut local_lamport_time,
        node_name,
        listener,
    )
    .await;

    Ok(())
}

//TODO : should not take local_lamport_time -> refer to app state instead, same for node_name
async fn main_loop(
    _state: Arc<Mutex<AppState>>,
    lines: &mut tokio_io::Lines<BufReader<tokio_io::Stdin>>,
    conn: &Connection,
    local_lamport_time: &mut i64,
    node_name: &str,
    listener: TcpListener,
) {
    loop {
        select! {
            line = lines.next_line() => {
                let _ = run_cli(line, &conn,local_lamport_time, node_name);
            }
            Ok((stream, addr)) = listener.accept() => {
                let _ = network::start_listening(stream, addr).await;
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
    // lock just to get the local address and site id
    let (local_addr, site_id, peer_addrs, local_vc) = {
        let state = GLOBAL_APP_STATE.lock().await;
        (
            state.get_local_addr().to_string(),
            state.get_site_id().to_string(),
            state.get_peers(),
            state.get_vector_clock().clone(),
        )
    };

    info!("Shutting down site {}.", site_id);
    for peer_addr in peer_addrs {
        let peer_addr_str = peer_addr.to_string();
        if let Err(e) = network::send_message(
            &peer_addr_str,
            "",
            message::NetworkMessageCode::Disconnect,
            &local_addr,
            &site_id,
            &local_vc,
        )
        .await
        {
            log::error!("Error sending message to {}: {}", peer_addr_str, e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_args_parsing() {
        let args = Args::parse_from(vec![
            "my_program",
            "--site-id",
            "1",
            "--port",
            "8080",
            "--peers",
            "127.0.0.1:8081,127.0.0.1:8082",
        ]);
        assert_eq!(args.site_id, 1);
        assert_eq!(args.port, 8080);
        assert_eq!(args.peers.len(), 2);
        assert_eq!(args.peers[0], "127.0.0.1:8081");
        assert_eq!(args.peers[1], "127.0.0.1:8082");
    }

    #[test]
    fn test_args_parsing_no_peers() {
        let args = Args::parse_from(vec!["my_program", "--site-id", "1", "--port", "8080"]);
        assert_eq!(args.site_id, 1);
        assert_eq!(args.port, 8080);
        assert_eq!(args.peers.len(), 0);
    }
}
