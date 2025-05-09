use crate::control::Command;
use crate::{
    clock::Clock,
    message::{Message, MessageInfo, NetworkMessageCode},
    state::LOCAL_APP_STATE,
};
use rmp_serde::{decode, encode};
use std::collections::HashMap;
use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::sync::mpsc::Sender;
use tokio::sync::Mutex;

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
    let (local_addr, site_id, clocks) = {
        let state = LOCAL_APP_STATE.lock().await;
        (
            state.get_local_addr().to_string(),
            state.get_site_id().to_string(),
            state.get_clock().clone(),
        )
    };

    let mut handles = Vec::new();

    for port in start_port..=end_port {
        let address = format!("{}:{}", ip, port);
        let local_addr = local_addr.clone();
        let site_id = site_id.clone();
        let clocks = clocks.clone();

        let handle = tokio::spawn(async move {
            {
                // Before sending the message, we need to update the local clock
                let mut state = LOCAL_APP_STATE.lock().await;
                state.increment_lamport();
                state.increment_vector_current();
            }
            let _ = send_message(
                &address,
                MessageInfo::None,
                None,
                NetworkMessageCode::Discovery,
                &local_addr,
                &site_id,
                clocks,
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
                let (local_addr, site_id, clocks) = {
                    let mut state = LOCAL_APP_STATE.lock().await;
                    state.add_peer(message.sender_id.as_str(), message.sender_addr);
                    (
                        state.get_local_addr().to_string(),
                        state.get_site_id().to_string(),
                        state.get_clock().clone(),
                    )
                };
                log::debug!("Sending discovery response to: {}", message.sender_addr);
                let _ = send_message(
                    &message.sender_addr.to_string(),
                    MessageInfo::None,
                    None,
                    NetworkMessageCode::Acknowledgment,
                    &local_addr,
                    &site_id,
                    clocks,
                )
                .await;
            }
            NetworkMessageCode::Transaction => {
                log::debug!("Transaction message received: {:?}", message);
                match message.command {
                    #[allow(unused)]
                    Some(cmd) => {
                        let mut state = LOCAL_APP_STATE.lock().await;
                        state.increment_lamport();
                        state.increment_vector_current();
                        state.update_vector(&message.clock.get_vector());
                        state.update_lamport(message.clock.get_lamport());

                        // TODO: Handle the command
                        //let conn = rusqlite::Connection::open("peillute.db")?;
                        //let _ = crate::control::handle_command(cmd, &conn, &mut state.get_lamport(), &state.get_site_id(), true).await;
                    }
                    None => {
                        log::error!("Command is None for Transaction message");
                    }
                }
            }
            NetworkMessageCode::Acknowledgment => {
                log::debug!("Acknowledgment message received: {:?}", message);
                {
                    let mut state = LOCAL_APP_STATE.lock().await;
                    state.add_peer(message.sender_id.as_str(), message.sender_addr);
                }
            }
            NetworkMessageCode::Error => {
                log::debug!("Error message received: {:?}", message);
            }
            NetworkMessageCode::Disconnect => {
                log::debug!("Disconnect message received: {:?}", message);
                {
                    let mut state = LOCAL_APP_STATE.lock().await;
                    state.remove_peer(message.sender_addr);
                }
            }
            NetworkMessageCode::Sync => {
                log::debug!("Sync message received: {:?}", message);
                on_sync().await;
            }
        }
        {
            let mut state = LOCAL_APP_STATE.lock().await;
            state.increment_lamport();
            state.increment_vector_current();
            state.update_vector(&message.clock.get_vector());
        }
    }
}

pub async fn send_message(
    address: &str,
    info: MessageInfo,
    command: Option<Command>,
    code: crate::message::NetworkMessageCode,
    local_addr: &str,
    local_site: &str,
    clock: Clock,
) -> Result<(), Box<dyn Error>> {
    let addr = address.parse::<SocketAddr>()?;

    /* !!!! DO NOT LOCK APPSTATE HERE, ALREADY LOCKED IN handle_message !!!! */

    if code == NetworkMessageCode::Transaction && command.is_none() {
        log::error!("Command is None for Transaction message");
        return Err("Command is None for Transaction message".into());
    }

    let msg = Message {
        sender_id: local_site.parse().unwrap(),
        sender_addr: local_addr.parse().unwrap(),
        clock: clock.clone(),
        command: command,
        info: info,
        code: code.clone(),
    };

    let buf = encode::to_vec(&msg)?;

    let mut manager = NETWORK_MANAGER.lock().await;

    let sender = match manager.get_sender(&addr) {
        Some(s) => s,
        _none => {
            manager.create_connection(addr).await?;
            manager.get_sender(&addr).unwrap()
        }
    };

    sender.send(buf).await?;

    log::debug!("Sent message {:?} to {}", &msg, address);
    Ok(())
}

pub async fn send_message_to_all(
    command: Option<Command>,
    code: crate::message::NetworkMessageCode,
    info: MessageInfo,
) -> Result<(), Box<dyn Error>> {
    let (local_addr, site_id, peer_addrs, clock) = {
        let state = LOCAL_APP_STATE.lock().await;
        (
            state.get_local_addr().to_string(),
            state.get_site_id().to_string(),
            state.get_peers(),
            state.get_clock().clone(),
        )
    };

    for peer_addr in peer_addrs {
        let peer_addr_str = peer_addr.to_string();
        send_message(
            &peer_addr_str,
            info.clone(),
            command.clone(),
            code.clone(),
            &local_addr,
            &site_id,
            clock.clone(),
        )
        .await?;
    }
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
        let local_addr = "127.0.0.1:8080";
        let local_site = "A";
        let clock = Clock::new();

        let _listener = TcpListener::bind(address).await?;

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let code = crate::message::NetworkMessageCode::Discovery;

        // Send the message
        let send_result = send_message(
            address,
            MessageInfo::None,
            None,
            code,
            local_addr,
            local_site,
            clock,
        )
        .await;
        assert!(send_result.is_ok());
        Ok(())
    }
}
