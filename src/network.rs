use crate::state::GLOBAL_APP_STATE;
use futures::future::join_all;
use rmp_serde::{decode, encode};
use std::collections::HashMap;
use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tokio::sync::mpsc;
use tokio::sync::mpsc::Sender;

use crate::message::Message;

pub struct PeerConnection {
    pub sender: Sender<Vec<u8>>,
}
pub struct NetworkManager {
    pub nb_active_connections: u16,
    pub connection_pool: HashMap<SocketAddr, PeerConnection>,
}

impl NetworkManager {
    pub fn new() -> Self {
        NetworkManager {
            nb_active_connections: 0,
            connection_pool: HashMap::new(),
        }
    }
    pub fn add_connection(&mut self, addr: SocketAddr, sender: Sender<Vec<u8>>) {
        self.connection_pool.insert(addr, PeerConnection { sender });
        self.nb_active_connections += 1;
    }

    pub async fn create_connection(&mut self, addr: SocketAddr) -> Result<(), Box<dyn Error>> {
        let stream = TcpStream::connect(addr).await?;
        let (tx, rx) = mpsc::channel(256);
        spawn_writer_task(stream, rx).await;
        self.add_connection(addr, tx);
        Ok(())
    }

    pub fn get_sender(&self, addr: &SocketAddr) -> Option<Sender<Vec<u8>>> {
        self.connection_pool.get(addr).map(|p| p.sender.clone())
    }
    #[allow(unused)]
    #[allow(dead_code)]
    pub fn get_all_connections(&self) -> Vec<SocketAddr> {
        self.connection_pool.keys().cloned().collect()
    }
}

lazy_static::lazy_static! {
    pub static ref NETWORK_MANAGER: Arc<Mutex<NetworkManager>> = Arc::new(Mutex::new(NetworkManager::new()));
}

// create new async task to handle the writing (unique strema)
// async way to do it
pub async fn spawn_writer_task(
    mut stream: TcpStream,
    mut rx: tokio::sync::mpsc::Receiver<Vec<u8>>,
) {
    tokio::spawn(async move {
        while let Some(data) = rx.recv().await {
            if let Err(e) = stream.write_all(&data).await {
                log::error!("Failed to send message: {}", e);
                break; // connection closed
            }
        }
        log::debug!("Writer task closed.");
    });
}

pub async fn announce(ip: &str, start_port: u16, end_port: u16) {
    let mut tasks = vec![];

    // lock just to get the local address and site id
    let (local_addr, site_id, local_vc) = {
        let state = GLOBAL_APP_STATE.lock().await;
        (
            state.get_local_addr().to_string(),
            state.get_site_id().to_string(),
            state.get_vector_clock().clone(),
        )
    };

    for port in start_port..=end_port {
        let address = format!("{}:{}", ip, port);
        let local_addr_clone = local_addr.clone();
        let site_id_clone = site_id.clone();
        let local_vc = local_vc.clone();

        let task = tokio::spawn(async move {
            let _ = send_message(
                &address,
                "",
                crate::message::NetworkMessageCode::Discovery,
                &local_addr_clone,
                &site_id_clone,
                &local_vc,
            )
            .await;
        });

        tasks.push(task);
    }

    join_all(tasks).await;
}

pub async fn start_listening(address: &str) -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind(address).await?;

    log::debug!("Listening on: {}", address);

    loop {
        let (stream, addr) = listener.accept().await?;
        log::debug!("Accepted connection from: {}", addr);

        // Spawn a new task to handle the connection
        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream, addr).await {
                log::error!("Error handling connection from {}: {}", addr, e);
            }
        });
    }
}

pub async fn handle_connection(
    mut stream: TcpStream,
    addr: SocketAddr,
) -> Result<(), Box<dyn Error>> {
    let mut buf = [0; 1024];

    loop {
        let n = stream.read(&mut buf).await?;
        if n == 0 {
            log::debug!("Connection closed by: {}", addr);
            break;
        }

        log::debug!("Received {} bytes from {}", n, addr);

        let message: Message = decode::from_slice(&buf).expect("Error decoding message");

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        match message.code {
            crate::message::NetworkMessageCode::Discovery => {
                {
                    let mut state = GLOBAL_APP_STATE.lock().await;
                    // envoyer une réponse de découverte
                    log::debug!("Sending discovery response to: {}", message.sender_addr);
                    let ack_code = crate::message::NetworkMessageCode::Acknowledgment;
                    let _ = send_message(
                        &message.sender_addr.to_string(),
                        "",
                        ack_code,
                        &state.get_local_addr(),
                        &state.get_site_id().to_string(),
                        &state.get_vector_clock(),
                    )
                    .await;

                    // add to list of peers
                    state.add_peer(&message.sender_addr.to_string());
                }
            }
            crate::message::NetworkMessageCode::Transaction => {
                log::debug!("Transaction message received: {:?}", message);
                // handle transaction
            }
            crate::message::NetworkMessageCode::Acknowledgment => {
                log::debug!("Acknowledgment message received: {:?}", message);
                {
                    let mut state = GLOBAL_APP_STATE.lock().await;
                    state.add_peer(&message.sender_addr.to_string());
                }
            }
            crate::message::NetworkMessageCode::Error => {
                log::debug!("Error message received: {:?}", message);
                // handle error
            }
            crate::message::NetworkMessageCode::Disconnect => {
                log::debug!("Disconnect message received: {:?}", message);
                {
                    let mut state = GLOBAL_APP_STATE.lock().await;
                    state.remove_peer(&message.sender_addr.to_string());
                }
            }
            crate::message::NetworkMessageCode::Sync => {
                log::debug!("Sync message received: {:?}", message);
                on_sync().await;
            }
        }
    }

    let state = GLOBAL_APP_STATE.lock().await;
    let peer_addrs: Vec<SocketAddr> = state.get_peers();
    for peer in &peer_addrs {
        log::debug!("{}", peer);
    }

    Ok(())
}

pub async fn send_message(
    address: &str,
    message: &str,
    code: crate::message::NetworkMessageCode,
    local_addr: &str,
    local_site: &str,
    local_vc: &Vec<u64>,
) -> Result<(), Box<dyn Error>> {
    let addr = address.parse::<SocketAddr>()?;

    /* !!!! DO NOT LOCK APPSTATE HERE, ALREADY LOCKED IN handle_connection !!!! */

    let msg = Message {
        sender_id: local_site.parse().unwrap(),
        sender_addr: local_addr.parse().unwrap(),
        sender_vc: local_vc.clone(),
        message: message.to_string(),
        code: code.clone(),
    };

    let buf = encode::to_vec(&msg)?;

    let mut manager = NETWORK_MANAGER.lock().await;

    let sender = match manager.get_sender(&addr) {
        Some(s) => s,
        None => {
            manager.create_connection(addr).await?;
            manager.get_sender(&addr).unwrap()
        }
    };

    sender.send(buf).await?;

    log::debug!("Sent message {:?} to {}", &msg, address);
    Ok(())
}

pub async fn on_sync() {
    // TODO : implement sync
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::test;

    #[test]
    async fn test_send_message() {
        let address = "127.0.0.1:8081";
        let message = "hello";
        let local_addr = "127.0.0.1:8080";
        let local_site = "1";
        let local_vc: Vec<u64> = vec![1, 2, 3];

        // Start a listener in a separate task
        tokio::spawn(async move {
            let listener_result = start_listening(address).await;
            assert!(listener_result.is_ok());
        });

        // Give the listener some time to start
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let code = crate::message::NetworkMessageCode::Discovery;

        // Send the message
        let send_result =
            send_message(address, message, code, local_addr, local_site, &local_vc).await;
        assert!(send_result.is_ok());
    }
}
