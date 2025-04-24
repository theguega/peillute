use crate::{
    message::{Message, NetworkMessageCode},
    state::GLOBAL_APP_STATE,
};
use rmp_serde::{decode, encode};
use std::collections::HashMap;
use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio::sync::mpsc;
use tokio::sync::mpsc::Sender;

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
    pub fn get_all_connections(&self) -> Vec<SocketAddr> {
        self.connection_pool.keys().cloned().collect()
    }
}

lazy_static::lazy_static! {
    pub static ref NETWORK_MANAGER: Arc<Mutex<NetworkManager>> = Arc::new(Mutex::new(NetworkManager::new()));
}

pub async fn spawn_writer_task(stream: TcpStream, mut rx: mpsc::Receiver<Vec<u8>>) {
    tokio::spawn(async move {
        let mut stream = stream;
        while let Some(data) = rx.recv().await {
            if stream.write_all(&data).await.is_err() {
                log::error!("Failed to send message");
                break;
            }
        }
        log::debug!("Writer task closed.");
    });
}

pub async fn announce(ip: &str, start_port: u16, end_port: u16) {
    let (local_addr, site_id, local_vc) = {
        let state = GLOBAL_APP_STATE.lock().await;
        (
            state.get_local_addr().to_string(),
            state.get_site_id().to_string(),
            state.get_vector_clock().clone(),
        )
    };

    let mut handles = Vec::new();

    for port in start_port..=end_port {
        let address = format!("{}:{}", ip, port);
        let local_addr = local_addr.clone();
        let site_id = site_id.clone();
        let vc = local_vc.clone();

        let handle = tokio::spawn(async move {
            let _ = send_message(
                &address,
                "",
                NetworkMessageCode::Discovery,
                &local_addr,
                &site_id,
                &vc,
            )
            .await;
        });

        handles.push(handle);
    }

    for handle in handles {
        let _ = handle.await;
    }
}

pub async fn start_listening(stream: TcpStream, addr: SocketAddr) {
    log::debug!("Accepted connection from: {}", addr);

    tokio::spawn(async move {
        if let Err(e) = handle_message(stream, addr).await {
            log::error!("Error handling connection from {}: {}", addr, e);
        }
    });
}

pub async fn handle_message(mut stream: TcpStream, addr: SocketAddr) -> Result<(), Box<dyn Error>> {
    let mut buf = vec![0; 1024];

    loop {
        let n = stream.read(&mut buf).await?;

        if n == 0 {
            log::debug!("Connection closed by: {}", addr);
            return Ok(());
        }

        log::debug!("Received {} bytes from {}", n, addr);

        let message: Message = match decode::from_slice(&buf[..n]) {
            Ok(msg) => msg,
            Err(e) => {
                log::error!("Error decoding message: {}", e);
                continue;
            }
        };

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        match message.code {
            NetworkMessageCode::Discovery => {
                let mut state = GLOBAL_APP_STATE.lock().await;
                log::debug!("Sending discovery response to: {}", message.sender_addr);
                let _ = send_message(
                    &message.sender_addr.to_string(),
                    "",
                    NetworkMessageCode::Acknowledgment,
                    &state.get_local_addr(),
                    &state.get_site_id().to_string(),
                    &state.get_vector_clock(),
                )
                .await;

                state.add_peer(message.sender_addr);
            }
            NetworkMessageCode::Transaction => {
                log::debug!("Transaction message received: {:?}", message);
            }
            NetworkMessageCode::Acknowledgment => {
                log::debug!("Acknowledgment message received: {:?}", message);
                GLOBAL_APP_STATE.lock().await.add_peer(message.sender_addr);
            }
            NetworkMessageCode::Error => {
                log::debug!("Error message received: {:?}", message);
            }
            NetworkMessageCode::Disconnect => {
                log::debug!("Disconnect message received: {:?}", message);
                GLOBAL_APP_STATE
                    .lock()
                    .await
                    .remove_peer(message.sender_addr);
            }
            NetworkMessageCode::Sync => {
                log::debug!("Sync message received: {:?}", message);
                on_sync().await;
            }
        }
    }
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

    /* !!!! DO NOT LOCK APPSTATE HERE, ALREADY LOCKED IN handle_message !!!! */

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
    use tokio::{net::TcpListener, test};

    #[test]
    async fn test_send_message() -> Result<(), Box<dyn Error>> {
        let address = "127.0.0.1:8081";
        let message = "hello";
        let local_addr = "127.0.0.1:8080";
        let local_site = "1";
        let local_vc: Vec<u64> = vec![1, 2, 3];

        let _listener = TcpListener::bind(address).await?;

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let code = crate::message::NetworkMessageCode::Discovery;

        // Send the message
        let send_result =
            send_message(address, message, code, local_addr, local_site, &local_vc).await;
        assert!(send_result.is_ok());
        Ok(())
    }
}
