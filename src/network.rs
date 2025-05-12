pub struct PeerConnection {
    pub sender: tokio::sync::mpsc::Sender<Vec<u8>>,
}

pub struct NetworkManager {
    pub nb_active_connections: u16,
    pub connection_pool: std::collections::HashMap<std::net::SocketAddr, PeerConnection>,
}

impl NetworkManager {
    pub fn new() -> Self {
        Self {
            nb_active_connections: 0,
            connection_pool: std::collections::HashMap::new(),
        }
    }

    pub fn add_connection(
        &mut self,
        addr: std::net::SocketAddr,
        sender: tokio::sync::mpsc::Sender<Vec<u8>>,
    ) {
        self.connection_pool.insert(addr, PeerConnection { sender });
        self.nb_active_connections += 1;
    }

    pub async fn create_connection(
        &mut self,
        addr: std::net::SocketAddr,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use tokio::net::TcpStream;
        use tokio::sync::mpsc;

        let stream = TcpStream::connect(addr).await?;
        let (tx, rx) = mpsc::channel(256);
        spawn_writer_task(stream, rx).await;
        self.add_connection(addr, tx);
        Ok(())
    }

    pub fn get_sender(
        &self,
        addr: &std::net::SocketAddr,
    ) -> Option<tokio::sync::mpsc::Sender<Vec<u8>>> {
        self.connection_pool.get(addr).map(|p| p.sender.clone())
    }

    #[allow(unused)]
    pub fn get_all_connections(&self) -> Vec<std::net::SocketAddr> {
        self.connection_pool.keys().cloned().collect()
    }
}

lazy_static::lazy_static! {
    pub static ref NETWORK_MANAGER: std::sync::Arc<tokio::sync::Mutex<NetworkManager>> =
        std::sync::Arc::new(tokio::sync::Mutex::new(NetworkManager::new()));
}

pub async fn spawn_writer_task(
    stream: tokio::net::TcpStream,
    mut rx: tokio::sync::mpsc::Receiver<Vec<u8>>,
) {
    use tokio::io::AsyncWriteExt;

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

pub async fn announce(ip: &str, start_port: u16, end_port: u16, selected_port: u16) {
    use crate::message::{MessageInfo, NetworkMessageCode};
    use crate::state::LOCAL_APP_STATE;

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
        if port == selected_port {
            continue;
        }
        let address = format!("{}:{}", ip, port);
        let local_addr = local_addr.clone();
        let site_id = site_id.clone();
        let clocks = clocks.clone();

        let handle = tokio::spawn(async move {
            let mut state = crate::state::LOCAL_APP_STATE.lock().await;
            state.increment_lamport();
            state.increment_vector_current();

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

pub async fn start_listening(stream: tokio::net::TcpStream, addr: std::net::SocketAddr) {
    log::debug!("Accepted connection from: {}", addr);

    tokio::spawn(async move {
        if let Err(e) = handle_message(stream, addr).await {
            log::error!("Error handling connection from {}: {}", addr, e);
        }
    });
}

pub async fn handle_message(
    mut stream: tokio::net::TcpStream,
    addr: std::net::SocketAddr,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::message::{Message, MessageInfo, NetworkMessageCode};
    use crate::state::LOCAL_APP_STATE;
    use rmp_serde::decode;
    use tokio::io::AsyncReadExt;

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

        // tokio::time::sleep(std::time::Duration::from_millis(100)).await;

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
                #[allow(unused)]
                if let Some(cmd) = message.command {
                    let mut state = LOCAL_APP_STATE.lock().await;
                    state.increment_lamport();
                    state.increment_vector_current();
                    state.update_vector(&message.clock.get_vector());
                    state.update_lamport(message.clock.get_lamport());

                    // handle_command(cmd).await;
                } else {
                    log::error!("Command is None for Transaction message");
                }
            }
            NetworkMessageCode::Acknowledgment => {
                log::debug!("Acknowledgment message received: {:?}", message);
                let mut state = LOCAL_APP_STATE.lock().await;
                state.add_peer(message.sender_id.as_str(), message.sender_addr);
            }
            NetworkMessageCode::Error => {
                log::debug!("Error message received: {:?}", message);
            }
            NetworkMessageCode::Disconnect => {
                log::debug!("Disconnect message received: {:?}", message);
                let mut state = LOCAL_APP_STATE.lock().await;
                state.remove_peer(message.sender_addr);
            }
            NetworkMessageCode::Sync => {
                log::debug!("Sync message received: {:?}", message);
                on_sync().await;
            }
        }

        let mut state = LOCAL_APP_STATE.lock().await;
        state.increment_lamport();
        state.increment_vector_current();
        state.update_vector(&message.clock.get_vector());
    }
}

pub async fn send_message(
    address: &str,
    info: crate::message::MessageInfo,
    command: Option<crate::control::Command>,
    code: crate::message::NetworkMessageCode,
    local_addr: &str,
    local_site: &str,
    clock: crate::clock::Clock,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::message::Message;
    use rmp_serde::encode;
    use std::net::SocketAddr;

    let addr = address.parse::<SocketAddr>()?;

    if code == crate::message::NetworkMessageCode::Transaction && command.is_none() {
        log::error!("Command is None for Transaction message");
        return Err("Command is None for Transaction message".into());
    }

    let msg = Message {
        sender_id: local_site.parse().unwrap(),
        sender_addr: local_addr.parse().unwrap(),
        clock: clock.clone(),
        command,
        info,
        code,
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

pub async fn send_message_to_all(
    command: Option<crate::control::Command>,
    code: crate::message::NetworkMessageCode,
    info: crate::message::MessageInfo,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::state::LOCAL_APP_STATE;

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
    // TODO: implement sync
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn test_send_message() -> Result<(), Box<dyn std::error::Error>> {
        use crate::clock::Clock;
        use crate::message::{MessageInfo, NetworkMessageCode};

        let address = "127.0.0.1:8081";
        let local_addr = "127.0.0.1:8080";
        let local_site = "A";
        let clock = Clock::new();

        let _listener = TcpListener::bind(address).await?;
        // tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let code = NetworkMessageCode::Discovery;

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
