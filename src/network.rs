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

/// If the user gave specific peers, we will only connect to those peers and not scan all the port range
pub async fn announce(ip: &str, start_port: u16, end_port: u16, selected_port: u16) {
    use crate::message::{MessageInfo, NetworkMessageCode};
    use crate::state::LOCAL_APP_STATE;

    let (local_addr, site_id, clocks, nb_peers, peer_addrs) = {
        let state = LOCAL_APP_STATE.lock().await;
        (
            state.local_addr,
            state.get_site_id().to_string(),
            state.get_clock().clone(),
            state.peer_addrs.clone().len(),
            state.get_peers(),

        )
    };

    let mut handles = Vec::new();

    if nb_peers > 0 {
        log::debug!("Manually connecting to peers based on args");
        for peer in peer_addrs.clone() {
            let site_id = site_id.clone();
            let clocks = clocks.clone();

            let handle = tokio::spawn(async move {
                let mut state = crate::state::LOCAL_APP_STATE.lock().await;
                state.increment_lamport();
                state.increment_vector_current();

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
            let address = format!("{}:{}", ip, port);
            let site_id = site_id.clone();
            let clocks = clocks.clone();

            let handle = tokio::spawn(async move {
                let mut state = crate::state::LOCAL_APP_STATE.lock().await;
                state.increment_lamport();
                state.increment_vector_current();

                let _ = send_message(
                    address.parse().unwrap(),
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

        log::debug!("Message received: {:?}", message);

        // tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        match message.code {
            NetworkMessageCode::Discovery => {

                let mut state = LOCAL_APP_STATE.lock().await;

                // return ack message if direct peer
                if !state.peer_addrs.iter().find(|addr| addr == &&message.sender_addr).is_none() {
                    if state.in_use_neighbors.iter().find(|addr| addr == &&message.sender_addr).is_none() {
                        state.in_use_neighbors.push(message.sender_addr);
                        state.nb_neighbors+=1;
                    }
                    send_message(message.sender_addr,
                                 MessageInfo::None,
                                 None,
                                 NetworkMessageCode::Acknowledgment,
                                 state.local_addr,
                                 state.get_site_id(),
                                 &message.message_initiator_id,
                                 message.message_initiator_addr,
                                 state.clocks.clone(),
                    ).await?;
                }


            }

            NetworkMessageCode::Acknowledgment => {
                let mut state = LOCAL_APP_STATE.lock().await;
                if message.message_initiator_addr == state.local_addr {
                    state.in_use_neighbors.push(message.sender_addr.clone());
                    state.nb_neighbors = state.in_use_neighbors.len() as i64;
                    for (site_id,nb_a_i) in state.nb_of_attended_neighbors.clone().iter(){
                        state.nb_of_attended_neighbors.insert(site_id.clone(), *nb_a_i+1);
                    }
                }

            }

            NetworkMessageCode::Transaction => {
                // messages bleus
                #[allow(unused)]
                if message.command.is_some() {

                    let site_id = {
                        let mut state = LOCAL_APP_STATE.lock().await;
                        (
                            state.get_site_id().to_string()
                        )
                    };

                    use crate::control::handle_command_from_network;
                    if let Err(e) = handle_command_from_network(
                        message.info.clone(),
                        message.clock.clone(),
                        &site_id
                    )
                    .await{
                        log::error!("Error handling command:\n{}", e);
                    }
                    // wave diffusion
                    let mut diffuse = false;
                    let (local_site_id,local_site_addr) = {
                        let mut state = LOCAL_APP_STATE.lock().await;
                        let parent_id = state.parent_addr.get(&message.message_initiator_id).unwrap_or(&"0.0.0.0:0".parse().unwrap()).to_string();
                        if parent_id == "0.0.0.0:0" {
                            state.set_parent_addr(message.message_initiator_id.clone(), message.sender_addr);

                            let nb_neighbours = state.nb_neighbors;
                            let current_value = state.nb_of_attended_neighbors.get(&message.message_initiator_id).copied().unwrap_or(nb_neighbours);

                            state.nb_of_attended_neighbors.insert(message.message_initiator_id.clone(), current_value - 1);

                            log::debug!("Nombre de voisin : {}", current_value-1);

                            diffuse = state.nb_of_attended_neighbors
                                .get(&message.message_initiator_id)
                                .copied()
                                .unwrap_or(0) > 0;
                        }
                        (state.site_id.clone(), state.local_addr.clone())
                    };

                    if diffuse {
                        let mut snd_msg = message.clone();
                        snd_msg.sender_id = local_site_id.to_string();
                        snd_msg.sender_addr = local_site_addr;
                        diffuse_message(&snd_msg).await?;
                    }else{
                        let (parent_addr, local_addr, site_id,clock) = {
                            let state = LOCAL_APP_STATE.lock().await;
                            (state.get_parent_addr(message.message_initiator_id.clone()),
                             &state.get_local_addr(),
                             &state.get_site_id().to_string(),
                             state.get_clock().clone())
                        };
                        // Acquit message to parent
                        log::debug!("Réception d'un message de transaction, on est sur une feuille, on acquite, envoie à {}", message.sender_addr.to_string().as_str());
                        send_message(message.sender_addr,
                                     MessageInfo::None,
                                     None,
                                     NetworkMessageCode::TransactionAcknowledgement,
                                     local_addr.parse().unwrap(),
                                     site_id,
                                     &message.message_initiator_id,
                                     message.message_initiator_addr,
                                     message.clock.clone(),
                        ).await?;

                        if message.sender_addr == parent_addr {
                            // réinitialisation s'il s'agit de la remontée après réception des rouges de tous les fils
                            let mut state = LOCAL_APP_STATE.lock().await;
                            let peer_count = state.in_use_neighbors.len();
                            state.nb_of_attended_neighbors.insert(
                                message.message_initiator_id.clone(),
                                peer_count as i64,
                            );
                            state.parent_addr.insert(
                                message.message_initiator_id.clone(),
                                "0.0.0.0:0".parse().unwrap()
                            );
                        }

                    }


                } else {
                    log::error!("Command is None for Transaction message");
                }
            }
            NetworkMessageCode::TransactionAcknowledgement => {
                // Message rouge
                let mut state = LOCAL_APP_STATE.lock().await;

                let nb_neighbours = state.nb_neighbors;
                let current_value = state.nb_of_attended_neighbors.get(&message.message_initiator_id).copied().unwrap_or(nb_neighbours);
                state.nb_of_attended_neighbors.insert(message.message_initiator_id.clone(), current_value - 1);

                if state.nb_of_attended_neighbors.get(&message.message_initiator_id.clone()).copied().unwrap_or(-1) == 0 {

                    if state.parent_addr.get(&message.message_initiator_id.clone()).copied().unwrap_or("99.99.99.99:0".parse().unwrap()) == state.local_addr {
                        // on est chez le parent
                        // diffusion terminée
                        // Réinitialisation

                        log::error!("Diffusion terminée et réussie !");

                    }else{
                        log::debug!("On est de le noeud {}. On a reçu un rouge de tous nos fils: on acquite au parent {}", state.local_addr.clone().to_string().as_str(),state.get_parent_addr(message.message_initiator_id.clone()).to_string().as_str());
                        send_message(state.get_parent_addr(message.message_initiator_id.clone()),
                                     MessageInfo::None,
                                     None,
                                     NetworkMessageCode::TransactionAcknowledgement,
                                     state.get_local_addr().parse().unwrap(),
                                     &state.get_site_id().to_string(),
                                     &message.message_initiator_id,
                                     message.message_initiator_addr,
                                     state.get_clock().clone()
                        ).await?;
                    }

                    let peer_count = state.in_use_neighbors.len();
                    state.nb_of_attended_neighbors.insert(
                        message.message_initiator_id.clone(),
                        peer_count as i64,
                    );
                    state.parent_addr.insert(
                        message.message_initiator_id.clone(),
                        "0.0.0.0:0".parse().unwrap()
                    );
                }
            }

            NetworkMessageCode::Error => {
                log::debug!("Error message received: {:?}", message);
            }
            NetworkMessageCode::Disconnect => {
                log::debug!("Disconnect message received: {:?}", message);
                let mut state = LOCAL_APP_STATE.lock().await;
                state.remove_peer(message.message_initiator_id.as_str(),message.message_initiator_addr);
            }
            NetworkMessageCode::SnapshotRequest => {
                let txs = crate::db::get_local_transaction_log()?;
                let summaries: Vec<_> = txs.iter().map(|t| t.into()).collect();

                let (site_id, clock, local_addr) = {
                    let st = LOCAL_APP_STATE.lock().await;
                    (
                        st.get_site_id().to_string(),
                        st.get_clock().clone(),
                        st.get_local_addr().to_string(),
                    )
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
                    local_addr.parse().unwrap(),
                    &site_id,
                    &message.message_initiator_id,
                    message.message_initiator_addr,
                    clock,
                )
                .await?;
            }

            NetworkMessageCode::SnapshotResponse => {
                if let MessageInfo::SnapshotResponse(resp) = message.info.clone() {
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
        state.increment_lamport();
        state.increment_vector_current();
        state.update_vector(&message.clock.get_vector());
    }
}

pub async fn send_message(
    address: std::net::SocketAddr,
    info: crate::message::MessageInfo,
    command: Option<crate::control::Command>,
    code: crate::message::NetworkMessageCode,
    local_addr: std::net::SocketAddr,
    local_site: &str,
    initiator_id: &str,
    initiator_addr: std::net::SocketAddr,
    clock: crate::clock::Clock,
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
        message_initiator_id : initiator_id.to_string(),
        clock: clock.clone(),
        command,
        info,
        code,
        message_initiator_addr: initiator_addr,
    };

    if address.ip().is_unspecified() || address.port() == 0 {
        log::warn!("Skipping invalid peer address {}", address);
        return Ok(());
    }

    let buf = encode::to_vec(&msg)?;

    let mut manager = NETWORK_MANAGER.lock().await;

    let sender = match manager.get_sender(&address) {
        Some(s) => s,
        None => {
            if let Err(e) = manager.create_connection(address).await {
                return Err(format!("error with connection to {}: {}", address.to_string(), e).into());
            }
            match manager.get_sender(&address) {
                Some(s) => s,
                None => {
                    let err_msg = format!("Sender not found after connecting to {}", address);
                    log::error!("{}", err_msg);
                    return Err(err_msg.into());
                }
            }
        }
    };


    match sender.send(buf).await{
        Ok(s) => s,
        Err(e) => {
            let err_msg = format!("Impossible to send msg to {} due to error : {}", address,e);
            log::error!("{}", err_msg);
            return Err(err_msg.into());
        }
    };
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
            state.get_local_addr(),
            state.get_site_id().to_string(),
            state.get_peers(),
            state.get_clock().clone(),
        )
    };

    for peer_addr in peer_addrs {
        let peer_addr_str = peer_addr;
        send_message(
            peer_addr_str,
            info.clone(),
            command.clone(),
            code.clone(),
            local_addr.parse().unwrap(),
            &site_id,
            &site_id,
            local_addr.parse().unwrap(),
            clock.clone(),
        )
        .await?;
    }
    Ok(())
}

// Vague de diffusion des messages
pub async fn diffuse_message(
    message: &crate::message::Message,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::state::LOCAL_APP_STATE;

    log::debug!("debut diffusion");

    let (local_addr, site_id, peer_addrs,parent_address) = {
        let state = LOCAL_APP_STATE.lock().await;
        (
            state.get_local_addr().to_string(),
            state.get_site_id().to_string(),
            state.get_peers(),
            state.get_parent_addr(message.message_initiator_id.clone()),
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
                local_addr.parse().unwrap(),
                &site_id,
                &message.message_initiator_id,
                message.message_initiator_addr,
                message.clock.clone(),
            ).await {
                log::error!("❌ Impossible d’envoyer à {} : {}", peer_addr_str, e);
            }
        }

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
            address.parse().unwrap(),
            MessageInfo::None,
            None,
            code,
            local_addr.parse().unwrap(),
            local_site,
            local_site,
            local_addr.parse().unwrap(),
            clock,
        )
        .await;
        assert!(send_result.is_ok());
        Ok(())
    }
}
