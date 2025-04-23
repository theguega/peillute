use clap::Parser;
use cli::run_cli;
use log::info;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::io::{self as tokio_io, AsyncBufReadExt, BufReader};
use tokio::net::TcpListener;
use tokio::select;
use rusqlite::{Connection, Result};
use std::io::{self as std_io, Write};

mod cli;
mod clock;
mod db;
mod message;
mod network;
mod state;

// singleton
use crate::state::{AppState, GLOBAL_APP_STATE};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    // Site ID
    #[arg(long, default_value_t = 0)]
    id: usize,

    // Port number for this site to listen on
    #[arg(long, default_value_t = 0)]
    port: u16,

    // Comma-separated list of peer addresses (ip:port)
    #[arg(long, value_delimiter = ',')]
    peers: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let args = Args::parse();

    // if none is provided, use the process id
    let site_id = match args.id {
        0 => std::process::id() as usize,
        id => id,
    };

    // if none port was provided, try to find a free port in the range 8000-9000
    let port_range = 8000..=9000;
    let mut selected_port = args.port;
    if selected_port == 0 {
        for port in port_range {
            if let Ok(listener) =
                std::net::TcpListener::bind(("127.0.0.1", port))
            {
                selected_port = port;
                drop(listener);
                break;
            }
        }
    }

    let local_addr: SocketAddr =
        format!("127.0.0.1:{}", selected_port).parse()?;
    let num_sites = 1; //1 for self then it will be managed by communications between peers

    {
        let mut state = GLOBAL_APP_STATE.lock().await;
        state.site_id = site_id;
        state.local_addr = local_addr;
        state.num_sites = num_sites;
        state.vector_clock = (0..num_sites)
            .map(|_| std::sync::atomic::AtomicU64::new(0))
            .collect();
    }

    network::announce("127.0.0.1", 8000, 9000).await;

    // start listening for incoming connections
    let network_listener_local_addr = local_addr.clone();
    //task::spawn(async move {
    //    if let Err(e) = network::start_listening(&network_listener_local_addr.to_string()).await {
    //        log::error!("Error starting listener: {}", e);
    //    }
    //});

    let listener: TcpListener = TcpListener::bind(network_listener_local_addr).await?;
    log::debug!("Listening on: {}", network_listener_local_addr);

    let conn: Connection = Connection::open("peillute.db").unwrap();
    let _ = db::drop_table(&conn);
    let _ = db::init_db(&conn);
    let noeud = "A";
    let mut local_lamport_time: i64 = 0;

    let stdin: tokio_io::Stdin = tokio_io::stdin();
    let reader: BufReader<tokio_io::Stdin> = BufReader::new(stdin);
    let mut lines: tokio_io::Lines<BufReader<tokio_io::Stdin>> = reader.lines();

    log::info!("Welcome on peillute, write /help to get the command list.");
    print!("> ");
    std_io::stdout().flush().unwrap();


    let main_loop_app_state = GLOBAL_APP_STATE.clone();
    tokio::select! {
        _ = main_loop(main_loop_app_state, &mut lines, &conn, &mut local_lamport_time, noeud, listener) => {},
        _ = tokio::signal::ctrl_c() => {
            disconnect().await;
        }
    }

    Ok(())
}

async fn main_loop(_state: Arc<Mutex<AppState>>, lines : &mut tokio_io::Lines<BufReader<tokio_io::Stdin>>, conn: &Connection, local_lamport_time: &mut i64, noeud: &str,listener: TcpListener) {

    loop {
        select! {
            line = lines.next_line() => {
                let _ = run_cli(line, &conn,local_lamport_time, noeud);
            }
            Ok((stream, addr)) = listener.accept() => {
                let _ = network::start_listening(stream, addr).await;
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
            log::error!(
                "Error sending message to {}: {}",
                peer_addr_str,
                e
            );
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
            "--id",
            "1",
            "--port",
            "8080",
            "--peers",
            "127.0.0.1:8081,127.0.0.1:8082",
        ]);
        assert_eq!(args.id, 1);
        assert_eq!(args.port, 8080);
        assert_eq!(args.peers.len(), 2);
        assert_eq!(args.peers[0], "127.0.0.1:8081");
        assert_eq!(args.peers[1], "127.0.0.1:8082");
    }

    #[test]
    fn test_args_parsing_no_peers() {
        let args = Args::parse_from(vec![
            "my_program",
            "--id",
            "1",
            "--port",
            "8080",
        ]);
        assert_eq!(args.id, 1);
        assert_eq!(args.port, 8080);
        assert_eq!(args.peers.len(), 0);
    }
}
