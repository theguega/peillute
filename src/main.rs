use clap::Parser;
use log::info;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;

mod clock;
mod message;
mod state;

use state::AppState;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    // Site ID
    #[arg(long)]
    id: usize,

    // Port number for this site to listen on
    #[arg(long)]
    port: u16,

    // Comma-separated list of peer addresses (ip:port)
    #[arg(long, value_delimiter = ',')]
    peers: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let args = Args::parse();

    let site_id = match args.id {
        0 => return Err("Site ID must be non-zero".into()),
        id => id,
    };

    let local_addr: SocketAddr = format!("0.0.0.0:{}", args.port).parse()?;

    let peer_addrs: Vec<SocketAddr> = args
        .peers
        .iter()
        .filter_map(|s| s.parse().ok())
        // Ensure we don't list ourselves as a peer
        .filter(|addr| *addr != local_addr)
        .collect();

    let num_sites = peer_addrs.len() + 1; // +1 for self
    info!(
        "Starting site {}/{num_sites} on {} with peers: {:?}",
        site_id, local_addr, peer_addrs
    );

    // Shared state initialization
    #[allow(unused_variables)]
    let shared_state = Arc::new(Mutex::new(AppState::new(
        site_id,
        num_sites,
        peer_addrs.clone(),
    )));

    info!("Shutting down site {}.", site_id);
    Ok(())
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
        let args = Args::parse_from(vec!["my_program", "--id", "1", "--port", "8080"]);
        assert_eq!(args.id, 1);
        assert_eq!(args.port, 8080);
        assert_eq!(args.peers.len(), 0);
    }
}
