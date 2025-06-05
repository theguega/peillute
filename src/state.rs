//! Application state management for Peillute
//!
//! This module handles the global application state, including site information,
//! peer management, and logical clock synchronization.

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

        Self {
            site_id,
            nb_connected_neighbours: nb_neighbors,
            peer_addrs,
            site_addr: local_addr,
            parent_addr_for_transaction_wave: parent_addr,
            attended_neighbours_nb_for_transaction_wave: nb_of_attended_neighbors,
            connected_neighbours_addrs: in_use_neighbors,
            clocks,
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

    pub async fn update_clock(&mut self, site_id: &str, received_vc: Option<&Clock>) {
        // this wrapper is needed to ensure that the clock is saved
        // each time it is updated
        // please DO NOT call the `update_clock` method directly from the clock
        self.clocks.update_clock(site_id, received_vc);
        self.save_local_state().await;
    }

    pub async fn save_local_state(&self) {
        // this is likely to be called whenever the clocks are updated
        let _ = crate::db::update_local_state(&self.site_id.clone(), self.clocks.clone());
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
