//! Application state management for Peillute
//!
//! This module handles the global application state, including site information,
//! peer management, and logical clock synchronization.

#[cfg(feature = "server")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MutexTag {
    Request,
    Release,
    #[allow(dead_code)]
    Ack,
}

#[cfg(feature = "server")]
#[derive(Clone, Copy, Debug)]
pub struct MutexStamp {
    pub tag: MutexTag,
    pub date: i64,
}

#[cfg(feature = "server")]
use crate::clock::Clock;

#[cfg(feature = "server")]
/// Represents the global state of a Peillute node
pub struct AppState {
    // --- Site Info ---
    /// Unique identifier for this site
    pub site_id: String,
    /// Unique address for this site
    pub site_addr: std::net::SocketAddr,
    /// Number of deg(1) neighbours connected
    pub nb_connected_neighbours: i64,
    /// List of peer addresses given in arguments at the launch of the application
    pub peer_addrs: Vec<std::net::SocketAddr>,
    /// List of deg(1) neighbours connected addresses
    pub connected_neighbours_addrs: Vec<std::net::SocketAddr>,

    // --- Message Diffusion Info ---
    /// Adress of the parent (deg(1) neighbour for this site) for a specific wave from initiator id
    pub parent_addr_for_transaction_wave: std::collections::HashMap<String, std::net::SocketAddr>,
    /// Number of response expected from our direct neighbours (deg(1) neighbours for this site) = nb of connected neighbours - 1 (parent) for a specific wave initiator id
    pub attended_neighbours_nb_for_transaction_wave: std::collections::HashMap<String, i64>,

    // --- Logical Clocks ---
    /// Logical clock implementation for distributed synchronization
    pub clocks: crate::clock::Clock,

    // GLobal mutex
    pub global_mutex_fifo: std::collections::HashMap<String, MutexStamp>,
    pub waiting_sc: bool,
    pub in_sc: bool,
    pub notify_sc: std::sync::Arc<tokio::sync::Notify>,
    pub pending_commands: std::collections::VecDeque<crate::control::CriticalCommands>,
}

#[cfg(feature = "server")]
impl AppState {
    /// Creates a new AppState instance with the given configuration
    pub fn new(
        site_id: String,
        nb_neighbors: i64,
        peer_addrs: Vec<std::net::SocketAddr>,
        local_addr: std::net::SocketAddr,
    ) -> Self {
        let clocks = crate::clock::Clock::new();
        let parent_addr = std::collections::HashMap::new();
        let nb_of_attended_neighbors = std::collections::HashMap::new();
        let in_use_neighbors = Vec::new();
        let gm = std::collections::HashMap::new();
        let waiting_sc = false;
        let in_sc = false;

        Self {
            site_id,
            nb_connected_neighbours: nb_neighbors,
            peer_addrs,
            site_addr: local_addr,
            parent_addr_for_transaction_wave: parent_addr,
            attended_neighbours_nb_for_transaction_wave: nb_of_attended_neighbors,
            connected_neighbours_addrs: in_use_neighbors,
            clocks,
            global_mutex_fifo: gm,
            waiting_sc,
            in_sc,
            notify_sc: std::sync::Arc::new(tokio::sync::Notify::new()),
            pending_commands: std::collections::VecDeque::new(),
        }
    }

    // /// Adds a new peer to the network and updates the logical clock
    // #[allow(unused)]
    // pub fn add_peer(&mut self, site_id: &str, addr: std::net::SocketAddr) {
    //     if !self.peer_addrs.contains(&addr) {
    //         self.peer_addrs.push(addr);
    //         self.clocks.add_peer(site_id);
    //         self.attended_neighbours_nb_for_transaction_wave
    //             .insert(site_id.to_string(), self.peer_addrs.len() as i64);
    //         self.parent_addr_for_transaction_wave
    //             .insert(site_id.to_string(), "0.0.0.0:0".parse().unwrap());
    //     }
    // }

    // /// Removes a peer from the network
    // #[allow(unused)]
    // pub fn remove_peer(&mut self, site_id: &str, addr: std::net::SocketAddr) {
    //     if let Some(pos) = self.peer_addrs.iter().position(|x| *x == addr) {
    //         self.peer_addrs.remove(pos);
    //         self.nb_connected_neighbours -= 1;
    //         self.attended_neighbours_nb_for_transaction_wave
    //             .insert(site_id.to_string(), self.peer_addrs.len() as i64);
    //         self.parent_addr_for_transaction_wave
    //             .insert(site_id.to_string(), "0.0.0.0:0".parse().unwrap());
    //     }
    // }

    pub async fn acquire_mutex(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        use crate::message::{Message, MessageInfo, NetworkMessageCode};
        use crate::network::diffuse_message_without_lock;

        self.update_clock(None).await;

        self.global_mutex_fifo.insert(
            self.site_id.clone(),
            MutexStamp {
                tag: MutexTag::Request,
                date: self.clocks.get_lamport().clone(),
            },
        );
        self.waiting_sc = true;

        if self.nb_connected_neighbours == 0 {
            self.waiting_sc = false;
            self.in_sc = true;
            self.notify_sc.notify_waiters(); // wake up the CLI awaiting critical section
            return Ok(());
        }

        let msg = Message {
            sender_id: self.site_id.clone(),
            sender_addr: self.site_addr,
            message_initiator_id: self.site_id.clone(),
            message_initiator_addr: self.site_addr,
            clock: self.clocks.clone(),
            command: None,
            info: MessageInfo::AcquireMutex(crate::message::AcquireMutexPayload),
            code: NetworkMessageCode::AcquireMutex,
        };

        diffuse_message_without_lock(
            &msg,
            &self.get_site_addr(),
            &self.get_site_id(),
            &self.get_peers_addrs(),
            &&self.get_parent_addr(msg.message_initiator_id.clone()),
        )
        .await?;
        Ok(())
    }

    pub async fn release_mutex(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        use crate::message::{Message, MessageInfo, NetworkMessageCode};
        use crate::network::diffuse_message_without_lock;

        self.update_clock(None).await;

        let msg = Message {
            sender_id: self.site_id.clone(),
            sender_addr: self.site_addr,
            message_initiator_id: self.site_id.clone(),
            message_initiator_addr: self.site_addr,
            clock: self.clocks.clone(),
            command: None,
            info: MessageInfo::ReleaseMutex(crate::message::ReleaseMutexPayload),
            code: NetworkMessageCode::ReleaseGlobalMutex,
        };

        diffuse_message_without_lock(
            &msg,
            &self.get_site_addr(),
            &self.get_site_id(),
            &self.get_peers_addrs(),
            &&self.get_parent_addr(msg.message_initiator_id.clone()),
        )
        .await?;

        self.global_mutex_fifo.remove(&self.site_id);
        // on retire les requêtes locales
        self.in_sc = false;
        self.waiting_sc = false;
        Ok(())
    }

    pub fn try_enter_sc(&mut self) {
        // MUST BE CALLED ONLY AFTER A SUCCESSFUL WAVE AFTER ACQUIRE MUTEX
        // This function checks if the site can enter the critical section
        // It checks if the site is waiting for the critical section and if it can enter
        // based on the FIFO order of requests in the global mutex FIFO.

        // Pour respecter l'algo du poly il faut que la vague soit complete
        // c'est à dire que tout le monde ait répondu ACK pour appeller cette fonction
        // sinon on va entrer en section critique à un moment sans qu'un des peers ait noté notre demande
        if !self.waiting_sc {
            return;
        }
        let my_stamp = match self.global_mutex_fifo.get(&self.site_id) {
            Some(s) => *s,
            None => return, // No local request found
        };
        let me = (my_stamp.date, self.site_id.clone());

        // ici on compara les stamps des autres demandes, est-ce qu'on est le suivant dans la FIFO ?
        // si oui on peut entrer en section critique
        let ok = self.global_mutex_fifo.iter().all(|(id, stamp)| {
            if id == &self.site_id {
                true
            } else {
                match stamp.tag {
                    MutexTag::Request => me <= (stamp.date, id.clone()),
                    _ => true,
                }
            }
        });

        if ok {
            self.waiting_sc = false;
            self.in_sc = true;
            // All other sites are notified that we are in critical section
            self.notify_sc.notify_waiters(); // notifies worker to execute pending commands
                                             // We remove obsolete Releases
            self.global_mutex_fifo
                .retain(|_, s| s.tag != MutexTag::Release);
        }
    }

    /// Returns the local address as a string
    pub fn get_site_addr(&self) -> String {
        self.site_addr.to_string()
    }

    /// Returns the current site ID as &str
    pub fn get_site_id(&self) -> &str {
        self.site_id.as_str()
    }

    /// Returns a list of all peer addresses
    pub fn get_peers_addrs(&self) -> Vec<std::net::SocketAddr> {
        self.peer_addrs.clone()
    }

    /// Returns a list of peer addresses as strings
    pub fn get_peers_addrs_string(&self) -> Vec<String> {
        self.peer_addrs.iter().map(|x| x.to_string()).collect()
    }

    /// Returns a list of conncted neibhours as strings
    pub fn get_connected_neighbours_addrs_string(&self) -> Vec<String> {
        self.connected_neighbours_addrs
            .iter()
            .map(|x| x.to_string())
            .collect()
    }

    /// Returns a reference to the clock of the site
    pub fn get_clock(&self) -> &crate::clock::Clock {
        &self.clocks
    }

    /// Set the number of attended neighbors for the wave from initiator_id
    pub fn set_number_of_attended_neighbors(&mut self, initiator_id: String, n: i64) {
        self.attended_neighbours_nb_for_transaction_wave
            .insert(initiator_id, n);
    }

    /// Get the parent (neighbour deg(1)) address for the wave from initiator_id
    pub fn get_parent_addr(&self, initiator_id: String) -> std::net::SocketAddr {
        self.parent_addr_for_transaction_wave
            .get(&initiator_id)
            .copied()
            .unwrap_or("0.0.0.0:0".parse().unwrap())
    }

    /// Set the parent (neighbour deg(1)) address for the wave from initiator_id
    pub fn set_parent_addr(&mut self, initiator_id: String, peer_adr: std::net::SocketAddr) {
        self.parent_addr_for_transaction_wave
            .insert(initiator_id, peer_adr);
    }

    /// Returns the number of deg(1) neighbors connected
    #[allow(unused)]
    pub fn get_nb_connected_neighbours(&self) -> i64 {
        self.nb_connected_neighbours
    }

    pub async fn update_clock(&mut self, received_vc: Option<&Clock>) {
        // this wrapper is needed to ensure that the clock is saved
        // each time it is updated
        // please DO NOT call the `update_clock` method directly from the clock
        self.clocks.update_clock(&self.site_id, received_vc);
        self.save_local_state().await;
    }

    pub async fn save_local_state(&self) {
        // this is likely to be called whenever the clocks are updated
        let _ = crate::db::update_local_state(&self.site_id, self.clocks.clone());
    }
}

// Singleton
#[cfg(feature = "server")]
lazy_static::lazy_static! {
    pub static ref LOCAL_APP_STATE: std::sync::Arc<tokio::sync::Mutex<AppState>> =
        std::sync::Arc::new(tokio::sync::Mutex::new(AppState::new(
            "".to_string(), // empty site id at start
            0,
            Vec::new(),
            "0.0.0.0:0".parse().unwrap(),
        )));
}

#[cfg(test)]
#[cfg(feature = "server")]
mod tests {
    use super::*;

    #[test]
    fn test_new_state() {
        let site_id = "A".to_string();
        let num_sites = 2;
        let peer_addrs = vec![
            "127.0.0.1:8081".parse().unwrap(),
            "127.0.0.1:8082".parse().unwrap(),
        ];
        let local_addr: std::net::SocketAddr = format!("127.0.0.1:{}", 8080).parse().unwrap();
        let shared_state =
            AppState::new(site_id.clone(), num_sites, peer_addrs.clone(), local_addr);

        assert_eq!(shared_state.site_id, site_id);
        assert_eq!(shared_state.nb_connected_neighbours, num_sites);
        assert_eq!(shared_state.peer_addrs, peer_addrs);
        assert_eq!(shared_state.clocks.get_vector_clock_map().len(), 0); // Initially empty
    }
}
