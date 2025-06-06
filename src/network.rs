//! Network communication and peer management
//!
//! This module handles all network-related functionality, including peer discovery,
//! message sending/receiving, and connection management in the distributed system.

#[cfg(feature = "server")]
/// Represents a connection to a peer node
pub struct PeerConnection {
    /// Channel sender for sending messages to the peer
    pub sender: tokio::sync::mpsc::Sender<Vec<u8>>,
}

#[cfg(feature = "server")]
/// Manages network connections and peer communication
pub struct NetworkManager {
    /// Number of currently active peer connections
    pub nb_active_connections: u16,
    /// Pool of active peer connections
    pub connection_pool: std::collections::HashMap<std::net::SocketAddr, PeerConnection>,
}

#[cfg(feature = "server")]
impl NetworkManager {
    /// Creates a new NetworkManager instance
    pub fn new() -> Self {
        Self {
            nb_active_connections: 0,
            connection_pool: std::collections::HashMap::new(),
        }
    }

    /// Adds a new peer connection to the connection pool
    fn add_connection(
        &mut self,
        site_addr: std::net::SocketAddr,
        sender: tokio::sync::mpsc::Sender<Vec<u8>>,
    ) {
        self.connection_pool
            .insert(site_addr, PeerConnection { sender });
        self.nb_active_connections += 1;
    }

    /// Establishes a new connection to a peer
    pub async fn create_connection(
        &mut self,
        site_addr: std::net::SocketAddr,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use tokio::net::TcpStream;
        use tokio::sync::mpsc;

        let stream = TcpStream::connect(site_addr).await?;
        let (tx, rx) = mpsc::channel(256);
        spawn_writer_task(stream, rx).await;
        self.add_connection(site_addr, tx);
        Ok(())
    }

    /// Remove and destroy a connection
    pub fn remove_connection(&mut self, site_addr: &std::net::SocketAddr) {
        self.connection_pool.remove(site_addr);
    }

    /// Returns the message sender for a specific peer address
    pub fn get_sender(
        &self,
        addr: &std::net::SocketAddr,
    ) -> Option<tokio::sync::mpsc::Sender<Vec<u8>>> {
        self.connection_pool.get(addr).map(|p| p.sender.clone())
    }
}

#[cfg(feature = "server")]
lazy_static::lazy_static! {
    pub static ref NETWORK_MANAGER: std::sync::Arc<tokio::sync::Mutex<NetworkManager>> =
        std::sync::Arc::new(tokio::sync::Mutex::new(NetworkManager::new()));
}

#[cfg(feature = "server")]
/// Spawns a task to handle writing messages to a peer connection
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

#[cfg(feature = "server")]
/// Announces this node's presence to potential peers in the network
/// If the user gave peers in args, we will only connect to those peers
/// If not, we will scan the port range and try connecting to all sockets
pub async fn announce(ip: &str, start_port: u16, end_port: u16, selected_port: u16) {
    use crate::message::{MessageInfo, NetworkMessageCode};
    use crate::state::LOCAL_APP_STATE;

    let (local_addr, site_id, clocks, cli_peer) = {
        let state = LOCAL_APP_STATE.lock().await;
        (
            state.get_site_addr(),
            state.get_site_id(),
            state.get_clock(),
            state.get_cli_peers_addrs(),
        )
    };

    let mut handles = Vec::new();

    if cli_peer.len() > 0 {
        log::debug!("Manually connecting to peers based on args");
        for peer in cli_peer {
            let site_id = site_id.clone();
            let clocks = clocks.clone();

            let handle = tokio::spawn(async move {
                let _ = send_message(
                    peer,
                    MessageInfo::None,
                    None,
                    NetworkMessageCode::Discovery,
                    local_addr,
                    &site_id,
                    &site_id,
                    local_addr,
                    clocks,
                )
                .await;
            });

            handles.push(handle);
        }
    } else {
        log::debug!("Looking for all ports to find potential peers");
        for port in start_port..=end_port {
            if port == selected_port {
                continue;
            }
            let address: std::net::SocketAddr = format!("{}:{}", ip, port).parse().unwrap();
            let site_id = site_id.clone();
            let clocks = clocks.clone();

            let handle = tokio::spawn(async move {
                let _ = send_message(
                    address,
                    MessageInfo::None,
                    None,
                    NetworkMessageCode::Discovery,
                    local_addr,
                    &site_id,
                    &site_id,
                    local_addr,
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
}

#[cfg(feature = "server")]
/// Starts listening for messages from a new peer
pub async fn start_listening(stream: tokio::net::TcpStream, addr: std::net::SocketAddr) {
    log::debug!("Accepted connection from: {}", addr);

    tokio::spawn(async move {
        if let Err(e) = handle_network_message(stream, addr).await {
            log::error!("Error handling connection from {}: {}", addr, e);
        }
    });
}

#[cfg(feature = "server")]
/// Handles incoming messages from a peer
/// Implement our wave diffusion protocol
pub async fn handle_network_message(
    mut stream: tokio::net::TcpStream,
    socket_of_the_sender: std::net::SocketAddr,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::message::{Message, MessageInfo, NetworkMessageCode};
    use crate::state::LOCAL_APP_STATE;
    use rmp_serde::decode;
    use tokio::io::AsyncReadExt;

    let mut buf = vec![0; 1024];
    loop {
        let n = stream.read(&mut buf).await?;

        if n == 0 {
            log::warn!("Connection closed by: {}", socket_of_the_sender);
            // Here we should remove the site from the network in the app state
            {
                log::debug!("Removing {} from the peers", socket_of_the_sender);
                let mut state = LOCAL_APP_STATE.lock().await;
                state
                    .remove_peer_from_socket_closed(socket_of_the_sender)
                    .await;
            }
            return Ok(());
        }

        log::debug!("Received {} bytes from {}", n, socket_of_the_sender);

        let message: Message = match decode::from_slice(&buf[..n]) {
            Ok(msg) => msg,
            Err(e) => {
                log::error!("Error decoding message: {}", e);
                continue;
            }
        };

        log::debug!("Message received: {:?}", message.clone());

        match message.code {
            NetworkMessageCode::Discovery => {
                let mut state = LOCAL_APP_STATE.lock().await;

                // Try to add this new site as a new peer
                state.add_incomming_peer(
                    message.message_initiator_addr,
                    socket_of_the_sender,
                    message.clock.clone(),
                );

                // Return ack message if this we are connected to the site
                if state
                    .get_connected_neighbours_addrs()
                    .iter()
                    .find(|addr| addr == &&message.sender_addr)
                    .is_some()
                {
                    if state
                        .get_connected_neighbours_addrs()
                        .iter()
                        .find(|addr| addr == &&message.sender_addr)
                        .is_none()
                    {
                        state.add_connected_neighbour(message.sender_addr);
                    }
                    send_message(
                        message.sender_addr,
                        MessageInfo::None,
                        None,
                        NetworkMessageCode::Acknowledgment,
                        state.get_site_addr(),
                        state.get_site_id().as_str(),
                        &message.message_initiator_id.clone(),
                        message.message_initiator_addr,
                        state.get_clock(),
                    )
                    .await?;
                }
            }

            NetworkMessageCode::Acknowledgment => {
                let mut state = LOCAL_APP_STATE.lock().await;
                state.add_incomming_peer(
                    message.sender_addr,
                    socket_of_the_sender,
                    message.clock.clone(),
                );
                if message.message_initiator_addr == state.get_site_addr() {
                    for (site_id, nb_a_i) in state
                        .get_attended_neighbours_nb_for_transaction_wave()
                        .iter()
                    {
                        state
                            .attended_neighbours_nb_for_transaction_wave
                            .insert(site_id.clone(), *nb_a_i + 1);
                    }
                }
            }

            NetworkMessageCode::Transaction => {
                // messages bleus
                #[allow(unused)]
                if message.command.is_some() {
                    let site_id = {
                        let mut state = LOCAL_APP_STATE.lock().await;
                        (state.get_site_id().to_string())
                    };

                    use crate::control::process_network_command;
                    if let Err(e) = process_network_command(
                        message.info.clone(),
                        message.clock.clone(),
                        &site_id,
                    )
                    .await
                    {
                        log::error!("Error handling command:\n{}", e);
                    }
                    // wave diffusion
                    let mut diffuse = false;
                    let (local_site_id, local_site_addr) = {
                        let mut state = LOCAL_APP_STATE.lock().await;
                        let parent_id = state
                            .parent_addr_for_transaction_wave
                            .get(&message.message_initiator_id.clone())
                            .unwrap_or(&"0.0.0.0:0".parse().unwrap())
                            .to_string();
                        if parent_id == "0.0.0.0:0" {
                            state.set_parent_addr(
                                message.message_initiator_id.clone(),
                                message.sender_addr,
                            );

                            let nb_neighbours = state.get_nb_connected_neighbours();
                            let current_value = state
                                .attended_neighbours_nb_for_transaction_wave
                                .get(&message.message_initiator_id)
                                .copied()
                                .unwrap_or(nb_neighbours);

                            state
                                .attended_neighbours_nb_for_transaction_wave
                                .insert(message.message_initiator_id.clone(), current_value - 1);

                            log::debug!("Nombre de voisin : {}", current_value - 1);

                            diffuse = state
                                .attended_neighbours_nb_for_transaction_wave
                                .get(&message.message_initiator_id.clone())
                                .copied()
                                .unwrap_or(0)
                                > 0;
                        }
                        (state.get_site_id(), state.get_site_addr())
                    };

                    if diffuse {
                        let mut snd_msg = message.clone();
                        snd_msg.sender_id = local_site_id.to_string();
                        snd_msg.sender_addr = local_site_addr;
                        diffuse_message(&snd_msg).await?;
                    } else {
                        let (parent_addr, local_addr, site_id, clock) = {
                            let state = LOCAL_APP_STATE.lock().await;
                            (
                                state.get_the_parent_addr_for_wave(
                                    message.message_initiator_id.clone(),
                                ),
                                state.get_site_addr(),
                                state.get_site_id().to_string(),
                                state.get_clock(),
                            )
                        };
                        // Acquit message to parent
                        log::debug!(
                            "Réception d'un message de transaction, on est sur une feuille, on acquite, envoie à {}",
                            message.sender_addr.to_string().as_str()
                        );
                        send_message(
                            message.sender_addr,
                            MessageInfo::None,
                            None,
                            NetworkMessageCode::TransactionAcknowledgement,
                            local_addr,
                            site_id.as_str(),
                            &message.message_initiator_id.clone(),
                            message.message_initiator_addr,
                            message.clock.clone(),
                        )
                        .await?;

                        if message.sender_addr == parent_addr {
                            // réinitialisation s'il s'agit de la remontée après réception des rouges de tous les fils
                            let mut state = LOCAL_APP_STATE.lock().await;
                            let peer_count = state.get_nb_connected_neighbours();
                            state
                                .attended_neighbours_nb_for_transaction_wave
                                .insert(message.message_initiator_id.clone(), peer_count as i64);
                            state
                                .parent_addr_for_transaction_wave
                                .insert(message.message_initiator_id, "0.0.0.0:0".parse().unwrap());
                        }
                    }
                } else {
                    log::error!("Command is None for Transaction message");
                }
            }
            NetworkMessageCode::TransactionAcknowledgement => {
                // Message rouge
                let mut state = LOCAL_APP_STATE.lock().await;

                let nb_neighbours = state.get_nb_connected_neighbours();
                let current_value = state
                    .attended_neighbours_nb_for_transaction_wave
                    .get(&message.message_initiator_id)
                    .copied()
                    .unwrap_or(nb_neighbours);
                state
                    .attended_neighbours_nb_for_transaction_wave
                    .insert(message.message_initiator_id.clone(), current_value - 1);

                if state
                    .attended_neighbours_nb_for_transaction_wave
                    .get(&message.message_initiator_id)
                    .copied()
                    .unwrap_or(-1)
                    == 0
                {
                    if state
                        .parent_addr_for_transaction_wave
                        .get(&message.message_initiator_id)
                        .copied()
                        .unwrap_or("99.99.99.99:0".parse().unwrap())
                        == state.get_site_addr()
                    {
                        // on est chez le parent
                        // diffusion terminée
                        // Réinitialisation

                        println!("\x1b[1;31mDiffusion terminée et réussie !\x1b[0m");
                    } else {
                        log::debug!(
                            "On est de le noeud {}. On a reçu un rouge de tous nos fils: on acquite au parent {}",
                            state.get_site_addr().to_string().as_str(),
                            state
                                .get_the_parent_addr_for_wave(message.message_initiator_id.clone())
                                .to_string()
                                .as_str()
                        );
                        send_message(
                            state
                                .get_the_parent_addr_for_wave(message.message_initiator_id.clone()),
                            MessageInfo::None,
                            None,
                            NetworkMessageCode::TransactionAcknowledgement,
                            state.get_site_addr(),
                            &state.get_site_id().to_string(),
                            &message.message_initiator_id,
                            message.message_initiator_addr,
                            state.get_clock(),
                        )
                        .await?;
                    }

                    let peer_count = state.get_nb_connected_neighbours();
                    state
                        .attended_neighbours_nb_for_transaction_wave
                        .insert(message.message_initiator_id.clone(), peer_count as i64);
                    state
                        .parent_addr_for_transaction_wave
                        .insert(message.message_initiator_id, "0.0.0.0:0".parse().unwrap());
                }
            }

            NetworkMessageCode::Error => {
                log::debug!("Error message received: {:?}", message);
            }
            NetworkMessageCode::Disconnect => {
                {
                    let mut state = LOCAL_APP_STATE.lock().await;
                    state.remove_peer(message.message_initiator_addr).await;
                }

                log::debug!("Site {} disconnected", message.message_initiator_id);
            }
            NetworkMessageCode::SnapshotRequest => {
                let txs = crate::db::get_local_transaction_log()?;
                let summaries: Vec<_> = txs.iter().map(|t| t.into()).collect();

                let (site_id, clock, local_addr) = {
                    let st = LOCAL_APP_STATE.lock().await;
                    (st.get_site_id(), st.get_clock(), st.get_site_addr())
                };

                send_message(
                    message.sender_addr,
                    MessageInfo::SnapshotResponse(crate::message::SnapshotResponse {
                        site_id: site_id.clone(),
                        clock: clock.clone(),
                        tx_log: summaries,
                    }),
                    None,
                    NetworkMessageCode::SnapshotResponse,
                    local_addr,
                    &site_id,
                    &message.message_initiator_id,
                    message.message_initiator_addr,
                    clock.clone(),
                )
                .await?;
            }
            NetworkMessageCode::SnapshotResponse => {
                if let MessageInfo::SnapshotResponse(resp) = message.info {
                    let mut mgr = crate::snapshot::LOCAL_SNAPSHOT_MANAGER.lock().await;
                    if let Some(gs) = mgr.push(resp) {
                        log::info!("Global snapshot ready, hold per site : {:#?}", gs.missing);
                        crate::snapshot::persist(&gs).await.unwrap();
                    }
                }
            }
            NetworkMessageCode::Sync => {
                log::debug!("Sync message received: {:?}", message);
                on_sync().await;
            }
        }

        let mut state = LOCAL_APP_STATE.lock().await;
        state.update_clock(Some(&message.clock.clone())).await;
    }
}

#[cfg(feature = "server")]
/// Send a message to a specific peer
pub async fn send_message(
    recipient_address: std::net::SocketAddr,
    info: crate::message::MessageInfo,
    command: Option<crate::control::Command>,
    code: crate::message::NetworkMessageCode,
    local_addr: std::net::SocketAddr,
    local_site: &str,
    initiator_id: &str,
    initiator_addr: std::net::SocketAddr,
    sender_clock: crate::clock::Clock,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::message::Message;
    use rmp_serde::encode;

    if code == crate::message::NetworkMessageCode::Transaction && command.is_none() {
        log::error!("Command is None for Transaction message");
        return Err("Command is None for Transaction message".into());
    }

    let msg = Message {
        sender_id: local_site.to_string(),
        sender_addr: local_addr,
        message_initiator_id: initiator_id.to_string(),
        clock: sender_clock,
        command,
        info,
        code,
        message_initiator_addr: initiator_addr,
    };

    if recipient_address.ip().is_unspecified() || recipient_address.port() == 0 {
        log::warn!("Skipping invalid peer address {}", recipient_address);
        return Ok(());
    }

    let buf = encode::to_vec(&msg)?;

    let mut manager = NETWORK_MANAGER.lock().await;

    let sender = match manager.get_sender(&recipient_address) {
        Some(s) => s,
        None => {
            if let Err(e) = manager.create_connection(recipient_address).await {
                return Err(format!(
                    "error with connection to {}: {}",
                    recipient_address.to_string(),
                    e
                )
                .into());
            }
            match manager.get_sender(&recipient_address) {
                Some(s) => s,
                None => {
                    let err_msg =
                        format!("Sender not found after connecting to {}", recipient_address);
                    log::error!("{}", err_msg);
                    return Err(err_msg.into());
                }
            }
        }
    };

    match sender.send(buf).await {
        Ok(s) => s,
        Err(e) => {
            let err_msg = format!(
                "Impossible to send msg to {} due to error : {}",
                recipient_address, e
            );
            log::error!("{}", err_msg);
            return Err(err_msg.into());
        }
    };
    log::debug!("Sent message {:?} to {}", &msg, recipient_address);
    Ok(())
}

#[cfg(feature = "server")]
/// Implement our wave diffusion protocol
pub async fn diffuse_message(
    message: &crate::message::Message,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::state::LOCAL_APP_STATE;

    log::debug!("debut diffusion");

    let (local_addr, site_id, peer_addrs, parent_address) = {
        let state = LOCAL_APP_STATE.lock().await;
        (
            state.get_site_addr(),
            state.get_site_id(),
            state.get_connected_neighbours_addrs(),
            state.get_the_parent_addr_for_wave(message.message_initiator_id.clone()),
        )
    };

    for peer_addr in peer_addrs {
        let peer_addr_str = peer_addr.to_string();
        if peer_addr != parent_address {
            log::debug!("Sending message to: {}", peer_addr_str);

            if let Err(e) = send_message(
                peer_addr,
                message.info.clone(),
                message.command.clone(),
                message.code.clone(),
                local_addr,
                &site_id,
                &message.message_initiator_id,
                message.message_initiator_addr,
                message.clock.clone(),
            )
            .await
            {
                log::error!("❌ Impossible d’envoyer à {} : {}", peer_addr_str, e);
            }
        }
    }
    Ok(())
}

#[cfg(feature = "server")]
#[allow(unused)]
pub async fn on_sync() {
    // TODO: implement sync
}

#[cfg(test)]
#[cfg(feature = "server")]
mod tests {
    use super::*;
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn test_send_message() -> Result<(), Box<dyn std::error::Error>> {
        use crate::clock::Clock;
        use crate::message::{MessageInfo, NetworkMessageCode};

        let address: std::net::SocketAddr = "127.0.0.1:8081".parse().unwrap();
        let local_addr: std::net::SocketAddr = "127.0.0.1:8080".parse().unwrap();
        let local_site = "A";
        let clock = Clock::new();

        let _listener = TcpListener::bind(address).await?;

        let code = NetworkMessageCode::Discovery;

        let send_result = send_message(
            address,
            MessageInfo::None,
            None,
            code,
            local_addr,
            local_site,
            local_site,
            local_addr,
            clock,
        )
        .await;
        assert!(send_result.is_ok());
        Ok(())
    }
}
